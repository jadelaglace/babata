use std::path::PathBuf;

use babata_application::{
    CancelCollectionCommand, CollectorSessionService, RetryCollectionItemCommand,
    StartCollectionCommand,
};
use babata_domain::{
    CandidateEnvelope, CollectionSelection, CollectionSessionId, ItemId, SourceRouteId,
};
use babata_infrastructure::{
    AppConfig, FileAssetStore, SystemClock, open_collection_database,
    sources::providers::{
        bilibili_collection::BilibiliOpenCliAdapter,
        browser::BrowserCandidateAdapter,
        chatgpt::ChatGptOpenCliAdapter,
        doubao::DoubaoOpenCliAdapter,
        evernote::EvernoteNotesAdapter,
        feishu::FeishuCliAdapter,
        kimi::KimiOpenCliAdapter,
        local_files::{LocalFileStrategy, LocalFilesAdapter},
        onenote::OneNoteExportAdapter,
        wechat::WechatArticleOpenCliAdapter,
        wechat_recovery::WechatRecoveryAdapter,
        xiaohongshu::XiaohongshuOpenCliAdapter,
        yuque::YuqueOpenCliAdapter,
        zhihu::ZhihuOpenCliAdapter,
    },
};
use serde::Serialize;

#[derive(Debug, clap::Subcommand)]
pub enum CollectorCommand {
    Start {
        #[arg(long)]
        route: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        scope: String,
        #[arg(long)]
        authorisation: String,
        #[arg(long = "local-file-strategy", default_value = "opaque_copy")]
        local_file_strategy: String,
        #[arg(long = "candidate-envelope", hide = true)]
        candidate_envelopes: Vec<PathBuf>,
        #[arg(long = "acquisition-handoff")]
        acquisition_handoffs: Vec<PathBuf>,
    },
    Candidates {
        #[arg(long)]
        session: String,
    },
    Select {
        #[arg(long)]
        session: String,
        #[arg(long = "candidate", required = true)]
        candidates: Vec<String>,
        #[arg(long)]
        scope: String,
        #[arg(long)]
        authorisation: String,
        #[arg(long)]
        attachments: bool,
        #[arg(long)]
        confirm: bool,
    },
    Status {
        #[arg(long)]
        session: String,
    },
    Retry {
        #[arg(long)]
        session: String,
        #[arg(long)]
        candidate: String,
    },
    Cancel {
        #[arg(long)]
        session: String,
        #[arg(long)]
        reason: String,
    },
    Recollect {
        #[arg(long)]
        item: String,
        #[arg(long = "acquisition-handoff")]
        acquisition_handoffs: Vec<PathBuf>,
    },
    RecollectSession {
        #[arg(long)]
        session: String,
        #[arg(long = "acquisition-handoff")]
        acquisition_handoffs: Vec<PathBuf>,
    },
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CollectorExecution {
    Session(babata_domain::CollectionSession),
    Candidates(Vec<babata_domain::CandidateSummary>),
    Items(Vec<babata_domain::CollectionItemStatus>),
    Item(babata_domain::CollectionItemStatus),
    Recollection(babata_domain::RecollectionOutcome),
    Recollections(Vec<babata_domain::RecollectionOutcome>),
}

#[allow(clippy::too_many_lines)]
pub fn execute(
    command: CollectorCommand,
    config: &AppConfig,
) -> Result<CollectorExecution, babata_application::ApplicationError> {
    let browser_candidates = match &command {
        CollectorCommand::Start {
            candidate_envelopes,
            ..
        } => candidate_envelopes
            .iter()
            .map(|path| read_candidate(path))
            .collect::<Result<Vec<_>, _>>()?,
        _ => Vec::new(),
    };
    let active_route = match &command {
        CollectorCommand::Start { route, .. } => Some(route.as_str()),
        _ => None,
    };
    let local_file_strategy = match &command {
        CollectorCommand::Start {
            local_file_strategy,
            ..
        } => local_file_strategy.parse::<LocalFileStrategy>()?,
        _ => LocalFileStrategy::default(),
    };
    let acquisition_handoffs = match &command {
        CollectorCommand::Start {
            acquisition_handoffs,
            ..
        }
        | CollectorCommand::Recollect {
            acquisition_handoffs,
            ..
        }
        | CollectorCommand::RecollectSession {
            acquisition_handoffs,
            ..
        } => acquisition_handoffs.as_slice(),
        _ => &[],
    };
    let handoffs = partition_acquisition_handoffs(acquisition_handoffs)?;
    let repository = open_collection_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let adapters = source_adapters(
        config,
        active_route,
        &browser_candidates,
        &handoffs,
        local_file_strategy,
    )?;
    let service = CollectorSessionService::new(
        repository,
        FileAssetStore::new(config.paths()),
        SystemClock,
        adapters,
    );
    match command {
        CollectorCommand::Start {
            route,
            source,
            scope,
            authorisation,
            ..
        } => service
            .start(StartCollectionCommand {
                route_id: SourceRouteId(route),
                source_reference: source,
                scope_description: scope,
                authorisation_id: authorisation,
            })
            .map(CollectorExecution::Session),
        CollectorCommand::Candidates { session } => service
            .candidates(&CollectionSessionId::parse(session)?)
            .map(CollectorExecution::Candidates),
        CollectorCommand::Select {
            session,
            candidates,
            scope,
            authorisation,
            attachments,
            confirm,
        } => service
            .select(CollectionSelection {
                session_id: CollectionSessionId::parse(session)?,
                candidate_ids: candidates,
                scope_description: scope,
                confirmed: confirm,
                authorised_context: authorisation,
                requested_attachments: attachments,
            })
            .map(CollectorExecution::Items),
        CollectorCommand::Status { session } => service
            .status(&CollectionSessionId::parse(session)?)
            .map(CollectorExecution::Items),
        CollectorCommand::Retry { session, candidate } => service
            .retry(RetryCollectionItemCommand {
                session_id: CollectionSessionId::parse(session)?,
                candidate_id: candidate,
            })
            .map(CollectorExecution::Item),
        CollectorCommand::Cancel { session, reason } => service
            .cancel(CancelCollectionCommand {
                session_id: CollectionSessionId::parse(session)?,
                reason,
            })
            .map(CollectorExecution::Items),
        CollectorCommand::Recollect { item, .. } => service
            .recollect(&ItemId::parse(item)?)
            .map(CollectorExecution::Recollection),
        CollectorCommand::RecollectSession { session, .. } => service
            .recollect_session(&CollectionSessionId::parse(session)?)
            .map(CollectorExecution::Recollections),
    }
}

fn source_adapters(
    config: &AppConfig,
    active_route: Option<&str>,
    browser_candidates: &[CandidateEnvelope],
    handoffs: &AcquisitionHandoffPaths,
    local_file_strategy: LocalFileStrategy,
) -> Result<
    Vec<Box<dyn babata_application::ports::SourceAdapterPort>>,
    babata_application::ApplicationError,
> {
    let mut adapters: Vec<Box<dyn babata_application::ports::SourceAdapterPort>> = vec![
        Box::new(OneNoteExportAdapter),
        Box::new(EvernoteNotesAdapter::new(
            config
                .paths()
                .root()
                .join("04_runtime/provider-downloads/evernote"),
        )),
        Box::new(FeishuCliAdapter::new(
            config
                .paths()
                .root()
                .join("04_runtime/provider-downloads/feishu"),
        )),
        Box::new(KimiOpenCliAdapter),
        Box::new(DoubaoOpenCliAdapter::from_acquisition_handoffs(
            &handoffs.doubao,
        )?),
        Box::new(WechatRecoveryAdapter::favorites(
            &handoffs.wechat_favorites,
        )?),
        Box::new(WechatRecoveryAdapter::chats(&handoffs.wechat_chats)?),
        Box::new(ChatGptOpenCliAdapter),
        Box::new(ZhihuOpenCliAdapter::new(
            config
                .paths()
                .root()
                .join("04_runtime/provider-downloads/zhihu"),
        )),
        Box::new(XiaohongshuOpenCliAdapter::new(
            config
                .paths()
                .root()
                .join("04_runtime/provider-downloads/xiaohongshu"),
        )),
        Box::new(YuqueOpenCliAdapter::new(
            config
                .paths()
                .root()
                .join("04_runtime/provider-downloads/yuque"),
        )),
        Box::new(BilibiliOpenCliAdapter::new(
            config
                .paths()
                .root()
                .join("04_runtime/provider-downloads/bilibili"),
        )),
        Box::new(WechatArticleOpenCliAdapter::new(
            config
                .paths()
                .root()
                .join("04_runtime/provider-downloads/wechat"),
        )),
        Box::new(LocalFilesAdapter::new(local_file_strategy)),
    ];
    for route in [
        "source.douyin",
        "source.browser_pages",
        "source.browser_bookmarks",
    ] {
        let candidates = if active_route == Some(route) {
            browser_candidates.to_vec()
        } else {
            Vec::new()
        };
        adapters.push(Box::new(BrowserCandidateAdapter::for_route(
            SourceRouteId(route.to_owned()),
            candidates,
        )));
    }
    Ok(adapters)
}

#[derive(Debug, Default)]
struct AcquisitionHandoffPaths {
    doubao: Vec<PathBuf>,
    wechat_favorites: Vec<PathBuf>,
    wechat_chats: Vec<PathBuf>,
}

fn partition_acquisition_handoffs(
    paths: &[PathBuf],
) -> Result<AcquisitionHandoffPaths, babata_application::ApplicationError> {
    let mut partitioned = AcquisitionHandoffPaths::default();
    for path in paths {
        let bytes = std::fs::read(path).map_err(|error| {
            babata_application::ApplicationError::Asset(format!(
                "unable to read acquisition handoff: {:?}",
                error.kind()
            ))
        })?;
        let header: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| {
            babata_application::ApplicationError::Conflict(
                "acquisition handoff is invalid JSON".to_owned(),
            )
        })?;
        match header.get("provider").and_then(serde_json::Value::as_str) {
            Some("doubao") => partitioned.doubao.push(path.clone()),
            Some("wechat") => match header.get("route_id").and_then(serde_json::Value::as_str) {
                Some("source.wechat_favorites") => {
                    partitioned.wechat_favorites.push(path.clone());
                }
                Some("source.wechat_chats") => partitioned.wechat_chats.push(path.clone()),
                _ => {
                    return Err(babata_application::ApplicationError::Conflict(
                        "WeChat acquisition handoff has an unsupported route".to_owned(),
                    ));
                }
            },
            _ => {
                return Err(babata_application::ApplicationError::Conflict(
                    "acquisition handoff has an unsupported provider".to_owned(),
                ));
            }
        }
    }
    Ok(partitioned)
}

fn read_candidate(
    path: &std::path::Path,
) -> Result<CandidateEnvelope, babata_application::ApplicationError> {
    let bytes = std::fs::read(path).map_err(|error| {
        babata_application::ApplicationError::Asset(format!(
            "unable to read candidate envelope: {:?}",
            error.kind()
        ))
    })?;
    serde_json::from_slice(&bytes).map_err(|_| {
        babata_application::ApplicationError::Conflict(
            "candidate envelope is invalid JSON".to_owned(),
        )
    })
}

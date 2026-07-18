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
        bilibili_collection::BilibiliOpenCliAdapter, browser::BrowserCandidateAdapter,
        doubao::DoubaoOpenCliAdapter, feishu::FeishuCliAdapter, kimi::KimiOpenCliAdapter,
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
        #[arg(long = "candidate-envelope", hide = true)]
        candidate_envelopes: Vec<PathBuf>,
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
}

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
    let repository = open_collection_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let adapters = source_adapters(config, active_route, &browser_candidates);
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
        CollectorCommand::Recollect { item } => service
            .recollect(&ItemId::parse(item)?)
            .map(CollectorExecution::Recollection),
    }
}

fn source_adapters(
    config: &AppConfig,
    active_route: Option<&str>,
    browser_candidates: &[CandidateEnvelope],
) -> Vec<Box<dyn babata_application::ports::SourceAdapterPort>> {
    let mut adapters: Vec<Box<dyn babata_application::ports::SourceAdapterPort>> = vec![
        Box::new(FeishuCliAdapter::new(
            config
                .paths()
                .root()
                .join("04_runtime/provider-downloads/feishu"),
        )),
        Box::new(KimiOpenCliAdapter),
        Box::new(DoubaoOpenCliAdapter),
        Box::new(BilibiliOpenCliAdapter::new(
            config
                .paths()
                .root()
                .join("04_runtime/provider-downloads/bilibili"),
        )),
    ];
    for route in [
        "source.zhihu",
        "source.xiaohongshu",
        "source.douyin",
        "source.chatgpt",
        "source.yuque",
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
    adapters
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

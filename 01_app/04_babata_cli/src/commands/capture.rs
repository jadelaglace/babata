use babata_application::{
    AnnotateCommand, CandidateCaptureCommand, CaptureFileCommand, CaptureImportAsset,
    CaptureImportCommand, CaptureOutcome, CaptureService, CaptureTextCommand, CreateNoteCommand,
    ReviseCommand, RouteEvidenceCommand, WorkspaceService,
};
use babata_domain::{
    AssetRole, CandidateEnvelope, ContentType, ItemId, Metadata, RevisionId, RouteCoverage,
    SourceRouteId, UtcTimestamp,
};
use babata_infrastructure::{
    FileAssetStore, SqliteRawRepository, SystemClock,
    sources::providers::{browser, feishu},
};

use super::{RootCommand, workspace::NoteInput};

pub enum CaptureExecution {
    Single(Box<CaptureOutcome>),
    Batch(Vec<CaptureOutcome>),
}

fn single(outcome: CaptureOutcome) -> CaptureExecution {
    CaptureExecution::Single(Box::new(outcome))
}

#[allow(clippy::too_many_lines)]
pub fn execute(
    command: RootCommand,
    capture: CaptureService<SqliteRawRepository, FileAssetStore, SystemClock>,
    workspace: WorkspaceService<SqliteRawRepository, FileAssetStore, SystemClock>,
) -> Result<CaptureExecution, babata_application::ApplicationError> {
    match command {
        RootCommand::Capture(CaptureCommand::Text(input)) => {
            capture.capture_text(text_command(input)?).map(single)
        }
        RootCommand::Capture(CaptureCommand::File(input)) => {
            capture.capture_file(file_command(input)?).map(single)
        }
        RootCommand::Capture(CaptureCommand::Export(input)) => {
            capture.capture_export(file_command(input)?).map(single)
        }
        RootCommand::Capture(CaptureCommand::Candidate(input)) => capture
            .capture_candidate(CandidateCaptureCommand {
                candidate: read_candidate(&input.path)?,
            })
            .map(single),
        RootCommand::Capture(CaptureCommand::FeishuExport(input)) => {
            let export = feishu::read_markdown_export(&input.path)?;
            capture
                .capture_import(CaptureImportCommand {
                    provider: "feishu".to_owned(),
                    text: export.raw_text,
                    context: input.context,
                    locator: Some(input.locator.clone()),
                    native_id: Some(input.native_id),
                    identity: None,
                    content_type: ContentType::Document,
                    metadata: metadata_with_source_fields(
                        &input.metadata_json,
                        [
                            ("title", serde_json::Value::String(export.title)),
                            (
                                "import_format",
                                serde_json::Value::String("feishu_markdown_export".to_owned()),
                            ),
                            (
                                "attachment_count",
                                serde_json::Value::from(input.attachment.len()),
                            ),
                        ],
                    )?,
                    source_published_at: None,
                    assets: std::iter::once(CaptureImportAsset {
                        path: input.path.to_string_lossy().to_string(),
                        role: AssetRole::Export,
                    })
                    .chain(input.attachment.into_iter().map(|path| CaptureImportAsset {
                        path: path.to_string_lossy().to_string(),
                        role: AssetRole::Attachment,
                    }))
                    .collect(),
                    route_evidence: route_evidence(
                        "source.feishu",
                        input.authorized_test,
                        &input.locator,
                        RouteCoverage {
                            metadata: true,
                            attachments: true,
                            revisions: true,
                            limitations: vec![
                                "Markdown export only; no Wiki traversal or OpenAPI collection"
                                    .to_owned(),
                                "attachments are limited to files explicitly supplied to this command"
                                    .to_owned(),
                            ],
                        },
                    )?,
                })
                .map(single)
        }
        RootCommand::Capture(CaptureCommand::Bookmarks(input)) => {
            let bookmarks = browser::read_netscape_bookmarks(&input.path)?;
            let original_export = input.path.to_string_lossy().to_string();
            let outcomes = bookmarks
                .into_iter()
                .map(|bookmark| {
                    let context =
                        (!bookmark.folder_path.is_empty()).then_some(bookmark.folder_path.clone());
                    capture.capture_import(CaptureImportCommand {
                        provider: "browser".to_owned(),
                        text: format!("{}\n{}", bookmark.title, bookmark.url),
                        context,
                        locator: Some(bookmark.url.clone()),
                        native_id: None,
                        identity: Some(format!("bookmark:{}", bookmark.url)),
                        content_type: ContentType::WebPage,
                        metadata: metadata_with_source_fields(
                            &input.metadata_json,
                            [
                                ("title", serde_json::Value::String(bookmark.title)),
                                (
                                    "bookmark_folder",
                                    serde_json::Value::String(bookmark.folder_path),
                                ),
                                (
                                    "import_format",
                                    serde_json::Value::String("netscape_bookmarks_html".to_owned()),
                                ),
                            ],
                        )?,
                        source_published_at: None,
                        assets: vec![CaptureImportAsset {
                            path: original_export.clone(),
                            role: AssetRole::Export,
                        }],
                        route_evidence: route_evidence(
                            "source.browser",
                            input.authorized_test.clone(),
                            &bookmark.url,
                            RouteCoverage {
                                metadata: true,
                                attachments: false,
                                revisions: true,
                                limitations: vec![
                                    "Netscape bookmark HTML only; no page body is fetched"
                                        .to_owned(),
                                    "bookmark exports do not include page attachments".to_owned(),
                                ],
                            },
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(CaptureExecution::Batch(outcomes))
        }
        RootCommand::Workspace(super::WorkspaceCommand::Create(input))
        | RootCommand::Create(input) => {
            let (text, path, context, metadata) = note(input)?;
            workspace
                .create(CreateNoteCommand {
                    text,
                    path,
                    context,
                    metadata,
                })
                .map(single)
        }
        RootCommand::Workspace(super::WorkspaceCommand::Revise(input))
        | RootCommand::Revise(input) => {
            let (text, path, _context, metadata) = note(input.note)?;
            workspace
                .revise(ReviseCommand {
                    parent: RevisionId::parse(input.parent)?,
                    text,
                    path,
                    note: input.note_text,
                    metadata,
                })
                .map(single)
        }
        RootCommand::Workspace(super::WorkspaceCommand::Annotate(input))
        | RootCommand::Annotate(input) => {
            let (text, path, context, metadata) = note(input.note)?;
            match RevisionId::parse(&input.target) {
                Ok(revision) => workspace.annotate_revision(revision, text, path, metadata),
                Err(_) => workspace.annotate(AnnotateCommand {
                    target_item: ItemId::parse(input.target)?,
                    target_revision: None,
                    text,
                    path,
                    context,
                    metadata,
                }),
            }
            .map(single)
        }
        _ => unreachable!("non-capture commands are handled before service setup"),
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum CaptureCommand {
    Text(ExternalTextInput),
    File(ExternalFileInput),
    Export(ExternalFileInput),
    Candidate(CandidateInput),
    FeishuExport(FeishuExportInput),
    Bookmarks(BookmarksInput),
}

#[derive(Debug, clap::Args)]
pub struct ExternalTextInput {
    #[arg(long)]
    pub provider: String,
    #[arg(long)]
    pub text: String,
    #[arg(long)]
    pub context: Option<String>,
    #[arg(long)]
    pub locator: Option<String>,
    #[arg(long)]
    pub native_id: Option<String>,
    #[arg(long)]
    pub identity: Option<String>,
    #[arg(long, default_value = "{}")]
    pub metadata_json: String,
    #[arg(long)]
    pub source_published_at: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct ExternalFileInput {
    #[arg(long)]
    pub provider: String,
    #[arg(long)]
    pub path: std::path::PathBuf,
    #[arg(long)]
    pub context: Option<String>,
    #[arg(long)]
    pub locator: Option<String>,
    #[arg(long)]
    pub native_id: Option<String>,
    #[arg(long)]
    pub identity: Option<String>,
    #[arg(long, default_value = "{}")]
    pub metadata_json: String,
    #[arg(long)]
    pub source_published_at: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct CandidateInput {
    #[arg(long)]
    pub path: std::path::PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct FeishuExportInput {
    #[arg(long)]
    pub path: std::path::PathBuf,
    #[arg(long)]
    pub locator: String,
    #[arg(long)]
    pub native_id: String,
    #[arg(long)]
    pub context: Option<String>,
    #[arg(long = "attachment")]
    pub attachment: Vec<std::path::PathBuf>,
    #[arg(long, default_value = "{}")]
    pub metadata_json: String,
    #[arg(long)]
    pub authorized_test: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct BookmarksInput {
    #[arg(long)]
    pub path: std::path::PathBuf,
    #[arg(long, default_value = "{}")]
    pub metadata_json: String,
    #[arg(long)]
    pub authorized_test: Option<String>,
}

fn text_command(
    input: ExternalTextInput,
) -> Result<CaptureTextCommand, babata_application::ApplicationError> {
    Ok(CaptureTextCommand {
        provider: input.provider,
        text: input.text,
        context: input.context,
        locator: input.locator,
        native_id: input.native_id,
        identity: input.identity,
        metadata: Metadata::parse(&input.metadata_json)?,
        source_published_at: input
            .source_published_at
            .map(UtcTimestamp::parse)
            .transpose()?,
    })
}
fn file_command(
    input: ExternalFileInput,
) -> Result<CaptureFileCommand, babata_application::ApplicationError> {
    Ok(CaptureFileCommand {
        provider: input.provider,
        path: input.path.to_string_lossy().to_string(),
        context: input.context,
        locator: input.locator,
        native_id: input.native_id,
        identity: input.identity,
        metadata: Metadata::parse(&input.metadata_json)?,
        source_published_at: input
            .source_published_at
            .map(UtcTimestamp::parse)
            .transpose()?,
    })
}
fn note(
    input: NoteInput,
) -> Result<(String, Option<String>, Option<String>, Metadata), babata_application::ApplicationError>
{
    match (input.text, input.path) {
        (Some(text), None) => Ok((
            text,
            None,
            input.context,
            Metadata::parse(&input.metadata_json)?,
        )),
        (None, Some(path)) => {
            let text = std::fs::read_to_string(&path)
                .map_err(|error| babata_application::ApplicationError::Asset(error.to_string()))?;
            Ok((
                text,
                Some(path.to_string_lossy().to_string()),
                input.context,
                Metadata::parse(&input.metadata_json)?,
            ))
        }
        _ => Err(babata_application::ApplicationError::Domain(
            babata_domain::DomainError::Invalid {
                field: "text/path",
                value: "provide exactly one".to_owned(),
            },
        )),
    }
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

fn metadata_with_source_fields<const N: usize>(
    raw: &str,
    fields: [(&'static str, serde_json::Value); N],
) -> Result<Metadata, babata_application::ApplicationError> {
    let mut value: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| babata_domain::DomainError::MetadataMustBeObject)?;
    let object = value
        .as_object_mut()
        .ok_or(babata_domain::DomainError::MetadataMustBeObject)?;
    for (key, field) in fields {
        object.insert(key.to_owned(), field);
    }
    Metadata::parse(&value.to_string()).map_err(Into::into)
}

fn route_evidence(
    route_id: &str,
    authorization_id: Option<String>,
    source_reference: &str,
    coverage: RouteCoverage,
) -> Result<Option<RouteEvidenceCommand>, babata_application::ApplicationError> {
    authorization_id
        .map(|authorization_id| {
            if authorization_id.trim().is_empty() {
                return Err(babata_application::ApplicationError::Domain(
                    babata_domain::DomainError::Empty {
                        field: "authorized_test",
                    },
                ));
            }
            Ok(RouteEvidenceCommand {
                route_id: SourceRouteId(route_id.to_owned()),
                authorization_id,
                source_reference: source_reference.to_owned(),
                coverage,
            })
        })
        .transpose()
}

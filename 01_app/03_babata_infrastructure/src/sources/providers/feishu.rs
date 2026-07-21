use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use babata_application::{
    AcquisitionOutcome, ApplicationError, CaptureImportAsset, DiscoveredCandidate,
    ports::SourceAdapterPort,
};
use babata_domain::{
    AssetRole, CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus,
    CollectionSessionId, CommonSourceMetadata, ContentType, Metadata, RouteCoverage, Sha256,
    SourceAccessState, SourceMediaEntry, SourceRouteDescriptor, SourceRouteId, UtcTimestamp,
};
use quick_xml::{Reader, events::Event};
use serde_json::Value;
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

const ROUTE_ID: &str = "source.feishu";
const ADAPTER_VERSION: &str = "lark-cli/1";
const MAX_PAGES: usize = 10;
static DOWNLOAD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Default)]
pub struct FeishuConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(ROUTE_ID.to_owned()),
        provider: "feishu".to_owned(),
        status: CapabilityStatus::Disabled,
        activation_phase: "P4".to_owned(),
    }
}

#[derive(Debug, Clone)]
pub struct FeishuCliAdapter {
    download_root: PathBuf,
}

impl FeishuCliAdapter {
    pub fn new(download_root: PathBuf) -> Self {
        Self { download_root }
    }

    fn download_media(
        &self,
        document_id: &str,
        media: &[FeishuMediaReference],
    ) -> Result<Vec<CaptureImportAsset>, ApplicationError> {
        let batch = format!(
            "{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| ApplicationError::Asset(error.to_string()))?
                .as_nanos(),
            DOWNLOAD_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let output_dir = self
            .download_root
            .join(safe_path_component(document_id))
            .join(batch);
        fs::create_dir_all(&output_dir).map_err(|error| {
            ApplicationError::Asset(format!(
                "unable to create Feishu media download directory: {error}"
            ))
        })?;
        media
            .iter()
            .enumerate()
            .map(|(index, media)| {
                let prefix = format!("asset-{index:03}");
                run_media_download(&output_dir, media, &prefix)?;
                let mut matches = fs::read_dir(&output_dir)
                    .map_err(|error| ApplicationError::Asset(error.to_string()))?
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.is_file()
                            && path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .is_some_and(|name| name.starts_with(&prefix))
                    })
                    .collect::<Vec<_>>();
                matches.sort();
                if matches.len() != 1 {
                    return Err(ApplicationError::Asset(format!(
                        "Feishu media download produced {} files for one token",
                        matches.len()
                    )));
                }
                Ok(CaptureImportAsset {
                    path: matches[0].to_string_lossy().into_owned(),
                    role: if media.kind == FeishuMediaKind::Whiteboard {
                        AssetRole::Preview
                    } else {
                        AssetRole::Attachment
                    },
                })
            })
            .collect()
    }
}

impl SourceAdapterPort for FeishuCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        if let Some(scope) = source_reference.strip_prefix("wiki:") {
            discover_wiki(session_id, scope)
        } else if let Some(query) = source_reference.strip_prefix("drive:") {
            discover_drive(session_id, query)
        } else if let Some(document) = source_reference.strip_prefix("doc:") {
            discover_document(session_id, document)
        } else {
            Err(ApplicationError::Conflict(
                "Feishu source must be wiki:<space>[:parent], drive:<query>, or doc:<url-or-token>"
                    .to_owned(),
            ))
        }
    }

    fn collect(
        &self,
        candidate: &CandidateSummary,
        _prefetched: Option<&CandidateEnvelope>,
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let source = candidate.source_location.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Feishu candidate has no source location".to_owned())
        })?;
        let output = match run_lark(&[
            "docs",
            "+fetch",
            "--as=user",
            &format!("--doc={source}"),
            "--doc-format=xml",
            "--detail=simple",
            "--format=json",
        ]) {
            Ok(output) => output,
            Err(LarkFailure::Removed(reason)) => {
                return Ok(AcquisitionOutcome::Removed { reason });
            }
            Err(LarkFailure::Inaccessible(reason)) => {
                return Ok(AcquisitionOutcome::Inaccessible { reason });
            }
            Err(LarkFailure::Retryable(reason)) => {
                return Err(ApplicationError::Storage(reason));
            }
            Err(LarkFailure::Invalid(reason)) => {
                return Err(ApplicationError::Conflict(reason));
            }
        };
        let document = output
            .pointer("/data/document")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                ApplicationError::Integrity("lark-cli returned no document".to_owned())
            })?;
        let text = document
            .get("content")
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| {
                ApplicationError::Integrity("Feishu document content is empty".to_owned())
            })?
            .to_owned();
        let native_id = document
            .get("document_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| candidate.source_native_id.clone());
        let revision = document.get("revision_id").cloned().unwrap_or(Value::Null);
        let media = parse_media_references(&text)?;
        let content_fingerprint = feishu_content_fingerprint(
            native_id.as_deref().unwrap_or("unknown-document"),
            &revision,
            &media,
        );
        let assets = if requested_attachments {
            self.download_media(native_id.as_deref().unwrap_or("unknown-document"), &media)?
        } else {
            Vec::new()
        };
        let mut provider_document_metadata = document.clone();
        provider_document_metadata.remove("content");
        let common_metadata = common_metadata_for_document(candidate, document, &media)?;
        let metadata = Metadata::parse(
            &serde_json::json!({
                "title": common_metadata.title,
                "hierarchy": candidate.hierarchy,
                "feishu_revision_id": revision,
                "contains_media": !media.is_empty(),
                "media_count": media.len(),
                "requested_attachments": requested_attachments,
                "downloaded_asset_count": assets.len(),
                "content_fingerprint": content_fingerprint.as_str(),
                "adapter_version": ADAPTER_VERSION,
                "provider_document_metadata": provider_document_metadata,
            })
            .to_string(),
        )?;
        Ok(AcquisitionOutcome::Found {
            candidate: Box::new(CandidateEnvelope {
                protocol_version: "1".to_owned(),
                route_id: SourceRouteId(ROUTE_ID.to_owned()),
                source_reference: source.to_owned(),
                content_type: ContentType::Document,
                payload_sha256: Sha256::of_bytes(text.as_bytes()),
                metadata,
                payload: CandidatePayload::Text { text },
                context: Some(candidate.hierarchy.join(" / ")),
                native_id,
                common_metadata,
            }),
            assets,
        })
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: true,
            limitations: vec![
                "embedded images, files and whiteboard previews are downloaded only when attachments are explicitly requested".to_owned(),
                "embedded Sheets, Base, Slides and whiteboards require their own reader".to_owned(),
                "candidate discovery is capped at ten pages per explicit scope".to_owned(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum FeishuMediaKind {
    Image,
    File,
    Whiteboard,
}

impl FeishuMediaKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::File => "file",
            Self::Whiteboard => "whiteboard",
        }
    }
}

fn common_metadata_for_document(
    candidate: &CandidateSummary,
    document: &serde_json::Map<String, Value>,
    media: &[FeishuMediaReference],
) -> Result<CommonSourceMetadata, ApplicationError> {
    let mut common = candidate.effective_common_metadata();
    if let Some(title) = document
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        common.title = Some(title.to_owned());
    }
    if let Some(updated_at) = document.get("update_time_iso").and_then(Value::as_str) {
        common.source_updated_at = Some(to_utc_timestamp(updated_at)?);
    }
    if let Some(published_at) = document.get("create_time_iso").and_then(Value::as_str) {
        common.source_published_at = Some(to_utc_timestamp(published_at)?);
    }
    common.access_state = SourceAccessState::Accessible;
    common.media.entries = media
        .iter()
        .map(|reference| SourceMediaEntry {
            kind: reference.kind.as_str().to_owned(),
            media_type: None,
            duration_ms: None,
            width: None,
            height: None,
            page_count: None,
        })
        .collect();
    Ok(common)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct FeishuMediaReference {
    kind: FeishuMediaKind,
    token: String,
    name: Option<String>,
}

fn parse_media_references(text: &str) -> Result<Vec<FeishuMediaReference>, ApplicationError> {
    let mut reader = Reader::from_str(text);
    let mut media = Vec::new();
    loop {
        let event = reader
            .read_event()
            .map_err(|error| ApplicationError::Integrity(format!("invalid Feishu XML: {error}")))?;
        let element = match &event {
            Event::Start(element) | Event::Empty(element) => element,
            Event::Eof => break,
            _ => continue,
        };
        let kind = match element.name().as_ref() {
            b"img" => FeishuMediaKind::Image,
            b"source" => FeishuMediaKind::File,
            b"whiteboard" => FeishuMediaKind::Whiteboard,
            _ => continue,
        };
        let mut token = None;
        let mut source = None;
        let mut name = None;
        for attribute in element.attributes() {
            let attribute = attribute.map_err(|error| {
                ApplicationError::Integrity(format!("invalid Feishu XML attribute: {error}"))
            })?;
            let value = attribute
                .decode_and_unescape_value(reader.decoder())
                .map_err(|error| ApplicationError::Integrity(error.to_string()))?
                .into_owned();
            match attribute.key.as_ref() {
                b"token" => token = Some(value),
                b"src" => source = Some(value),
                b"name" => name = Some(value),
                _ => {}
            }
        }
        let token = token
            .or(source)
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| ApplicationError::Integrity("Feishu media has no token".to_owned()))?;
        let reference = FeishuMediaReference { kind, token, name };
        if !media.contains(&reference) {
            media.push(reference);
        }
    }
    Ok(media)
}

fn feishu_content_fingerprint(
    document_id: &str,
    revision: &Value,
    media: &[FeishuMediaReference],
) -> Sha256 {
    Sha256::of_bytes(
        serde_json::json!({
            "document_id": document_id,
            "revision_id": revision,
            "media": media,
        })
        .to_string()
        .as_bytes(),
    )
}

fn run_media_download(
    output_dir: &Path,
    media: &FeishuMediaReference,
    prefix: &str,
) -> Result<(), ApplicationError> {
    let executable = if cfg!(windows) {
        "lark-cli.cmd"
    } else {
        "lark-cli"
    };
    let mut command = Command::new(executable);
    command
        .current_dir(output_dir)
        .args([
            "docs",
            "+media-download",
            "--as=user",
            &format!("--token={}", media.token),
            &format!("--output=./{prefix}"),
            "--format=json",
        ])
        .env("LARKSUITE_CLI_NO_UPDATE_NOTIFIER", "1")
        .env("LARKSUITE_CLI_NO_SKILLS_NOTIFIER", "1");
    if media.kind == FeishuMediaKind::Whiteboard {
        command.arg("--type=whiteboard");
    }
    let output = command
        .output()
        .map_err(|error| ApplicationError::Asset(format!("unable to start lark-cli: {error}")))?;
    if output.status.success() {
        return Ok(());
    }
    let value = serde_json::from_slice::<Value>(&output.stderr).unwrap_or(Value::Null);
    let message = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("Feishu media download failed");
    Err(ApplicationError::Asset(message.to_owned()))
}

fn safe_path_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "unknown-document".to_owned()
    } else {
        sanitized
    }
}

fn discover_wiki(
    session_id: &CollectionSessionId,
    scope: &str,
) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
    let mut parts = scope.splitn(2, ':');
    let space = parts.next().unwrap_or_default();
    if space.trim().is_empty() {
        return Err(ApplicationError::Conflict(
            "wiki scope requires a space ID or my_library".to_owned(),
        ));
    }
    let parent = parts.next().filter(|value| !value.trim().is_empty());
    let mut args = vec![
        "wiki".to_owned(),
        "+node-list".to_owned(),
        "--as=user".to_owned(),
        format!("--space-id={space}"),
        "--page-all".to_owned(),
        format!("--page-limit={MAX_PAGES}"),
        "--page-size=50".to_owned(),
        "--format=json".to_owned(),
    ];
    if let Some(parent) = parent {
        args.push(format!("--parent-node-token={parent}"));
    }
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_lark(&arg_refs).map_err(lark_error)?;
    let nodes = output
        .pointer("/data/nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| ApplicationError::Integrity("lark-cli returned no Wiki nodes".to_owned()))?;
    nodes
        .iter()
        .filter(|node| node.get("obj_type").and_then(Value::as_str) == Some("docx"))
        .map(|node| {
            let node_token = required_string(node, "node_token")?;
            let object_token = required_string(node, "obj_token")?;
            let title = optional_nonempty(node, "title");
            let mut hierarchy = vec![format!("Wiki space {space}")];
            if let Some(parent) = parent {
                hierarchy.push(format!("Parent node {parent}"));
            }
            hierarchy.push(
                title
                    .clone()
                    .unwrap_or_else(|| "Untitled document".to_owned()),
            );
            let source_location = format!("https://my.feishu.cn/wiki/{node_token}");
            Ok(DiscoveredCandidate {
                summary: CandidateSummary {
                    candidate_id: candidate_id(&node_token),
                    session_id: session_id.clone(),
                    route_id: SourceRouteId(ROUTE_ID.to_owned()),
                    source_native_id: Some(object_token),
                    title,
                    source_location: Some(source_location),
                    hierarchy,
                    content_type: ContentType::Document,
                    source_updated_at: None,
                    attachment_available: None,
                    limitations: vec![
                        "Wiki node listing does not expose update time or attachment inventory"
                            .to_owned(),
                    ],
                    selection_capabilities: vec!["single".to_owned(), "visible_set".to_owned()],
                    common_metadata: CommonSourceMetadata::default(),
                }
                .with_common_from_legacy(),
                prefetched: None,
            })
        })
        .collect()
}

fn discover_drive(
    session_id: &CollectionSessionId,
    query: &str,
) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
    if query.chars().count() > 30 {
        return Err(ApplicationError::Conflict(
            "Feishu Drive query cannot exceed 30 characters".to_owned(),
        ));
    }
    let mut candidates = Vec::new();
    let mut page_token: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let mut args = vec![
            "drive".to_owned(),
            "+search".to_owned(),
            "--as=user".to_owned(),
            format!("--query={query}"),
            "--doc-types=docx".to_owned(),
            "--sort=edit_time".to_owned(),
            "--page-size=20".to_owned(),
            "--format=json".to_owned(),
        ];
        if let Some(token) = &page_token {
            args.push(format!("--page-token={token}"));
        }
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = run_lark(&arg_refs).map_err(lark_error)?;
        let results = output
            .pointer("/data/results")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ApplicationError::Integrity("lark-cli returned no Drive results".to_owned())
            })?;
        for result in results {
            let meta = result
                .get("result_meta")
                .and_then(Value::as_object)
                .ok_or_else(|| {
                    ApplicationError::Integrity("Drive result has no metadata".to_owned())
                })?;
            let url = meta
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| ApplicationError::Integrity("Drive result has no URL".to_owned()))?;
            let token = meta.get("token").and_then(Value::as_str).unwrap_or(url);
            let title = result
                .get("title_highlighted")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned);
            let updated = meta
                .get("update_time_iso")
                .and_then(Value::as_str)
                .map(to_utc_timestamp)
                .transpose()?;
            candidates.push(DiscoveredCandidate {
                summary: CandidateSummary {
                    candidate_id: candidate_id(token),
                    session_id: session_id.clone(),
                    route_id: SourceRouteId(ROUTE_ID.to_owned()),
                    source_native_id: Some(token.to_owned()),
                    title: title.clone(),
                    source_location: Some(url.to_owned()),
                    hierarchy: vec![
                        "Feishu Drive search".to_owned(),
                        if query.is_empty() {
                            "Recent documents"
                        } else {
                            query
                        }
                        .to_owned(),
                        title.unwrap_or_else(|| "Untitled document".to_owned()),
                    ],
                    content_type: ContentType::Document,
                    source_updated_at: updated,
                    attachment_available: None,
                    limitations: vec![
                        "search metadata does not include attachment inventory".to_owned(),
                    ],
                    selection_capabilities: vec![
                        "single".to_owned(),
                        "visible_set".to_owned(),
                        "explicit_search_scope".to_owned(),
                    ],
                    common_metadata: CommonSourceMetadata::default(),
                }
                .with_common_from_legacy(),
                prefetched: None,
            });
        }
        let has_more = output
            .pointer("/data/has_more")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        page_token = output
            .pointer("/data/page_token")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        if !has_more || page_token.is_none() {
            break;
        }
    }
    Ok(candidates)
}

fn discover_document(
    session_id: &CollectionSessionId,
    document: &str,
) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
    if document.trim().is_empty() {
        return Err(ApplicationError::Conflict(
            "doc scope requires a URL or token".to_owned(),
        ));
    }
    Ok(vec![DiscoveredCandidate {
        summary: CandidateSummary {
            candidate_id: candidate_id(document),
            session_id: session_id.clone(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_native_id: token_from_reference(document),
            title: None,
            source_location: Some(document.to_owned()),
            hierarchy: vec!["Explicit Feishu document".to_owned()],
            content_type: ContentType::Document,
            source_updated_at: None,
            attachment_available: None,
            limitations: vec![
                "single-document discovery defers title, update time and media inventory until read"
                    .to_owned(),
            ],
            selection_capabilities: vec!["single".to_owned()],
            common_metadata: CommonSourceMetadata::default(),
        }
        .with_common_from_legacy(),
        prefetched: None,
    }])
}

fn run_lark(args: &[&str]) -> Result<Value, LarkFailure> {
    let executable = if cfg!(windows) {
        "lark-cli.cmd"
    } else {
        "lark-cli"
    };
    let output = Command::new(executable)
        .args(args)
        .env("LARKSUITE_CLI_NO_UPDATE_NOTIFIER", "1")
        .env("LARKSUITE_CLI_NO_SKILLS_NOTIFIER", "1")
        .output()
        .map_err(|error| LarkFailure::Invalid(format!("unable to start lark-cli: {error}")))?;
    let bytes = if output.status.success() {
        &output.stdout
    } else {
        &output.stderr
    };
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| LarkFailure::Invalid("lark-cli returned a non-JSON response".to_owned()))?;
    if output.status.success() && value.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok(value);
    }
    let message = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("lark-cli request failed")
        .to_owned();
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("not found") || normalized.contains("not exist") {
        Err(LarkFailure::Removed(message))
    } else if normalized.contains("permission")
        || normalized.contains("forbidden")
        || normalized.contains("unauthorized")
    {
        Err(LarkFailure::Inaccessible(message))
    } else if normalized.contains("rate")
        || normalized.contains("timeout")
        || normalized.contains("network")
    {
        Err(LarkFailure::Retryable(message))
    } else {
        Err(LarkFailure::Invalid(message))
    }
}

fn candidate_id(value: &str) -> String {
    format!(
        "feishu_{}",
        &Sha256::of_bytes(value.as_bytes()).as_str()[..24]
    )
}

fn required_string(value: &Value, key: &str) -> Result<String, ApplicationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApplicationError::Integrity(format!("Feishu result has no {key}")))
}

fn optional_nonempty(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn token_from_reference(reference: &str) -> Option<String> {
    reference
        .split(['/', '#', '?'])
        .rfind(|part| !part.is_empty())
        .map(str::to_owned)
}

fn to_utc_timestamp(value: &str) -> Result<UtcTimestamp, ApplicationError> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| {
        ApplicationError::Integrity("Feishu returned an invalid update timestamp".to_owned())
    })?;
    let canonical = parsed
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    UtcTimestamp::parse(canonical).map_err(Into::into)
}

fn lark_error(error: LarkFailure) -> ApplicationError {
    match error {
        LarkFailure::Removed(reason) => ApplicationError::NotFound(reason),
        LarkFailure::Inaccessible(reason) | LarkFailure::Invalid(reason) => {
            ApplicationError::Conflict(reason)
        }
        LarkFailure::Retryable(reason) => ApplicationError::Storage(reason),
    }
}

enum LarkFailure {
    Removed(String),
    Inaccessible(String),
    Retryable(String),
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeishuMarkdownExport {
    pub title: String,
    pub raw_text: String,
}

pub fn read_markdown_export(path: &Path) -> Result<FeishuMarkdownExport, ApplicationError> {
    let raw_text = fs::read_to_string(path).map_err(|error| {
        ApplicationError::Asset(format!("unable to read Feishu export: {:?}", error.kind()))
    })?;
    let title = raw_text
        .lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .filter(|title| !title.is_empty())
                .map(str::to_owned)
        })
        .ok_or_else(|| ApplicationError::Asset("Feishu export title is unavailable".to_owned()))?;
    if raw_text.trim().is_empty() {
        return Err(ApplicationError::Asset(
            "Feishu export must contain text".to_owned(),
        ));
    }
    Ok(FeishuMarkdownExport { title, raw_text })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn markdown_export_uses_heading_as_title() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("export.md");
        fs::write(&path, "# A Feishu document\n\nBody").unwrap();
        let export = read_markdown_export(&path).unwrap();
        assert_eq!(export.title, "A Feishu document");
        assert_eq!(export.raw_text, "# A Feishu document\n\nBody");
    }

    #[test]
    fn timestamps_are_normalized_to_utc() {
        assert_eq!(
            to_utc_timestamp("2026-07-17T21:09:42+08:00")
                .unwrap()
                .as_str(),
            "2026-07-17T13:09:42Z"
        );
    }

    #[test]
    fn explicit_document_discovery_emits_typed_hierarchy_and_limitations() {
        let candidates = discover_document(
            &CollectionSessionId::new(),
            "https://my.feishu.cn/docx/document-token",
        )
        .unwrap();
        let common = &candidates[0].summary.common_metadata;
        assert_eq!(common.hierarchy[0].name, "Explicit Feishu document");
        assert_eq!(common.limitations[0].code, "provider_reported");
        assert_eq!(common.context.as_deref(), Some("Explicit Feishu document"));
    }

    #[test]
    fn collected_document_can_correct_title_and_supply_typed_source_times() {
        let candidate = discover_document(
            &CollectionSessionId::new(),
            "https://my.feishu.cn/docx/document-token",
        )
        .unwrap()
        .remove(0)
        .summary;
        let document = serde_json::json!({
            "title": "Canonical Feishu title",
            "create_time_iso": "2026-07-17T20:00:00+08:00",
            "update_time_iso": "2026-07-17T21:09:42+08:00"
        });
        let common = common_metadata_for_document(
            &candidate,
            document.as_object().unwrap(),
            &[FeishuMediaReference {
                kind: FeishuMediaKind::Image,
                token: "image-token".to_owned(),
                name: None,
            }],
        )
        .unwrap();
        assert_eq!(common.title.as_deref(), Some("Canonical Feishu title"));
        assert_eq!(
            common
                .source_published_at
                .as_ref()
                .map(UtcTimestamp::as_str),
            Some("2026-07-17T12:00:00Z")
        );
        assert_eq!(
            common.source_updated_at.as_ref().map(UtcTimestamp::as_str),
            Some("2026-07-17T13:09:42Z")
        );
        assert_eq!(common.media.entries[0].kind, "image");
    }

    #[test]
    fn media_references_are_parsed_structurally_and_deduplicated() {
        let media = parse_media_references(
            r#"<title>Sample</title><img src="img-a" href="https://signed.example/one"/><source token="file-a" name="report &amp; notes.pdf"/><whiteboard token="board-a"/><img src="img-a" href="https://signed.example/two"/>"#,
        )
        .unwrap();
        assert_eq!(media.len(), 3);
        assert_eq!(media[0].kind, FeishuMediaKind::Image);
        assert_eq!(media[0].token, "img-a");
        assert_eq!(media[1].kind, FeishuMediaKind::File);
        assert_eq!(media[1].name.as_deref(), Some("report & notes.pdf"));
        assert_eq!(media[2].kind, FeishuMediaKind::Whiteboard);
    }

    #[test]
    fn content_fingerprint_uses_official_revision_and_stable_media_tokens() {
        let first =
            parse_media_references(r#"<img src="img-a" href="https://signed.example/one"/>"#)
                .unwrap();
        let refreshed_url =
            parse_media_references(r#"<img src="img-a" href="https://signed.example/two"/>"#)
                .unwrap();
        assert_eq!(
            feishu_content_fingerprint("doc-a", &serde_json::json!(42), &first),
            feishu_content_fingerprint("doc-a", &serde_json::json!(42), &refreshed_url)
        );
        assert_ne!(
            feishu_content_fingerprint("doc-a", &serde_json::json!(42), &first),
            feishu_content_fingerprint("doc-a", &serde_json::json!(43), &first)
        );
    }
}

use std::{
    fs,
    path::PathBuf,
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
    CollectionSessionId, ContentType, Metadata, RouteCoverage, Sha256, SourceRouteDescriptor,
    SourceRouteId,
};
use serde_json::Value;

const ROUTE_ID: &str = "source.yuque";
const ADAPTER_VERSION: &str = "yuque-official-markdown+opencli-web/2";
static DOWNLOAD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Default)]
pub struct YuqueConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(ROUTE_ID.to_owned()),
        provider: "yuque".to_owned(),
        status: CapabilityStatus::Disabled,
        activation_phase: "P4".to_owned(),
    }
}

#[derive(Debug, Clone)]
pub struct YuqueOpenCliAdapter {
    download_root: PathBuf,
}

impl YuqueOpenCliAdapter {
    pub fn new(download_root: PathBuf) -> Self {
        Self { download_root }
    }

    fn download_images(
        &self,
        document_id: &str,
        images: &[Value],
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
        let output_dir = self.download_root.join(document_id).join(batch);
        fs::create_dir_all(&output_dir).map_err(|error| {
            ApplicationError::Asset(format!("unable to create Yuque image directory: {error}"))
        })?;
        images
            .iter()
            .enumerate()
            .map(|(index, image)| {
                let url = required_string(image, "Url")?;
                let token = required_string(image, "Token")?;
                if !is_yuque_image_url(&url) {
                    return Err(ApplicationError::Integrity(
                        "Yuque returned an unsupported image host".to_owned(),
                    ));
                }
                let extension = token
                    .rsplit_once('.')
                    .map(|(_, extension)| extension)
                    .filter(|extension| {
                        extension.chars().all(|value| value.is_ascii_alphanumeric())
                    })
                    .unwrap_or("bin");
                let path = output_dir.join(format!("image-{index:03}.{extension}"));
                let output = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" })
                    .args([
                        "--location",
                        "--fail",
                        "--silent",
                        "--show-error",
                        "--referer",
                        "https://www.yuque.com/",
                        "--user-agent",
                        "Mozilla/5.0",
                        "--output",
                    ])
                    .arg(&path)
                    .arg(&url)
                    .output()
                    .map_err(|error| {
                        ApplicationError::Asset(format!(
                            "unable to start Yuque image download: {error}"
                        ))
                    })?;
                if !output.status.success() {
                    return Err(ApplicationError::Asset(format!(
                        "Yuque image download failed: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    )));
                }
                if fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0) == 0 {
                    return Err(ApplicationError::Asset(
                        "Yuque image download produced an empty file".to_owned(),
                    ));
                }
                Ok(CaptureImportAsset {
                    path: path.to_string_lossy().into_owned(),
                    role: AssetRole::Attachment,
                })
            })
            .collect()
    }
}

impl SourceAdapterPort for YuqueOpenCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let limit = parse_recent_scope(source_reference)?;
        let output = run_opencli(&[
            "web",
            "yuque-recent-full",
            "--limit",
            &limit.to_string(),
            "--window",
            "background",
            "--site-session",
            "persistent",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ])?;
        let rows = output.as_array().ok_or_else(|| {
            ApplicationError::Integrity("Yuque recent-full result was not an array".to_owned())
        })?;
        rows.iter()
            .map(|row| {
                let id = required_string(row, "Id")?;
                let title = required_string(row, "Title")?;
                let owner = required_string(row, "Owner")?;
                let book = required_string(row, "Book")?;
                let url = required_string(row, "Url")?;
                Ok(DiscoveredCandidate {
                    summary: CandidateSummary {
                        candidate_id: format!("yuque_{id}"),
                        session_id: session_id.clone(),
                        route_id: SourceRouteId(ROUTE_ID.to_owned()),
                        source_native_id: Some(id),
                        title: Some(title.clone()),
                        source_location: Some(url),
                        hierarchy: vec!["Yuque".to_owned(), owner, book, title],
                        content_type: ContentType::Document,
                        source_updated_at: None,
                        attachment_available: None,
                        limitations: vec![
                            "dashboard date has day precision and no media inventory".to_owned(),
                        ],
                        selection_capabilities: vec![
                            "single".to_owned(),
                            "visible_set".to_owned(),
                            "recent_count".to_owned(),
                        ],
                        common_metadata: babata_domain::CommonSourceMetadata::default(),
                    }
                    .with_common_from_legacy(),
                    prefetched: None,
                })
            })
            .collect()
    }

    fn collect(
        &self,
        candidate: &CandidateSummary,
        _prefetched: Option<&CandidateEnvelope>,
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let source = candidate.source_location.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Yuque candidate has no source URL".to_owned())
        })?;
        let document_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Yuque candidate has no document ID".to_owned())
        })?;
        let output = match run_opencli(&[
            "web",
            "yuque-detail-full",
            source,
            "--window",
            "background",
            "--site-session",
            "persistent",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ]) {
            Ok(output) => output,
            Err(ApplicationError::NotFound(reason)) => {
                return Ok(AcquisitionOutcome::Removed { reason });
            }
            Err(ApplicationError::Conflict(reason)) => {
                return Ok(AcquisitionOutcome::Inaccessible { reason });
            }
            Err(error) => return Err(error),
        };
        acquisition_from_detail(self, candidate, document_id, requested_attachments, &output)
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: true,
            limitations: vec![
                "official Markdown export, rendered HTML and inline media are covered".to_owned(),
                "full book pagination, files, tables, boards and comments are not yet covered"
                    .to_owned(),
                "candidate discovery is bounded to the dashboard recent-document table".to_owned(),
            ],
        }
    }
}

fn acquisition_from_detail(
    adapter: &YuqueOpenCliAdapter,
    candidate: &CandidateSummary,
    document_id: &str,
    requested_attachments: bool,
    output: &Value,
) -> Result<AcquisitionOutcome, ApplicationError> {
    let row = output
        .as_array()
        .and_then(|rows| rows.first())
        .ok_or_else(|| ApplicationError::Integrity("Yuque returned no document".to_owned()))?;
    let source_url = required_string(row, "Url")?;
    let text = required_string(row, "Text")?;
    let html = required_string(row, "Html")?;
    let markdown = required_string(row, "Markdown")?;
    let images = row
        .get("Images")
        .and_then(Value::as_array)
        .ok_or_else(|| ApplicationError::Integrity("Yuque Images was not an array".to_owned()))?;
    let tokens = images
        .iter()
        .map(|image| required_string(image, "Token"))
        .collect::<Result<Vec<_>, _>>()?;
    let assets = if requested_attachments {
        adapter.download_images(document_id, images)?
    } else {
        Vec::new()
    };
    let payload = serde_json::to_string_pretty(&serde_json::json!({
        "platform": "yuque",
        "document_id": document_id,
        "title": row.get("Title"),
        "markdown": markdown,
        "text": text,
        "html": html,
        "images": images,
    }))
    .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let content_fingerprint = yuque_content_fingerprint(document_id, &markdown, &tokens);
    let metadata = Metadata::parse(
        &serde_json::json!({
            "title": candidate.title,
            "hierarchy": candidate.hierarchy,
            "text_length": text.chars().count(),
            "html_length": html.chars().count(),
            "markdown_length": markdown.chars().count(),
            "inline_image_count": images.len(),
            "requested_attachments": requested_attachments,
            "downloaded_asset_count": assets.len(),
            "attachments_covered": requested_attachments && assets.len() == images.len(),
            "content_fingerprint": content_fingerprint.as_str(),
            "official_export": "markdown_endpoint",
            "adapter_version": ADAPTER_VERSION,
        })
        .to_string(),
    )?;
    Ok(AcquisitionOutcome::Found {
        candidate: Box::new(CandidateEnvelope {
            protocol_version: "1".to_owned(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_reference: source_url,
            content_type: ContentType::Document,
            payload_sha256: Sha256::of_bytes(payload.as_bytes()),
            metadata,
            payload: CandidatePayload::Text { text: payload },
            context: Some(candidate.hierarchy.join(" / ")),
            native_id: Some(document_id.to_owned()),
            common_metadata: candidate.effective_common_metadata(),
        }),
        assets,
    })
}

fn parse_recent_scope(value: &str) -> Result<usize, ApplicationError> {
    let count = value
        .strip_prefix("recent:")
        .ok_or_else(|| ApplicationError::Conflict("Yuque scope must be recent:<count>".to_owned()))?
        .parse::<usize>()
        .map_err(|_| ApplicationError::Conflict("invalid Yuque recent count".to_owned()))?;
    if !(1..=20).contains(&count) {
        return Err(ApplicationError::Conflict(
            "Yuque recent count must be between 1 and 20".to_owned(),
        ));
    }
    Ok(count)
}

fn yuque_content_fingerprint(document_id: &str, markdown: &str, tokens: &[String]) -> Sha256 {
    Sha256::of_bytes(
        serde_json::json!({"document_id": document_id, "markdown": markdown, "images": tokens})
            .to_string()
            .as_bytes(),
    )
}

fn run_opencli(args: &[&str]) -> Result<Value, ApplicationError> {
    let output = Command::new(if cfg!(windows) {
        "opencli.cmd"
    } else {
        "opencli"
    })
    .args(args)
    .output()
    .map_err(|error| ApplicationError::Storage(format!("unable to start OpenCLI: {error}")))?;
    let bytes = if output.status.success() || output.stderr.iter().all(u8::is_ascii_whitespace) {
        &output.stdout
    } else {
        &output.stderr
    };
    let value: Value = serde_json::from_slice(bytes).map_err(|_| {
        let text = String::from_utf8_lossy(bytes)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        ApplicationError::Storage(format!(
            "OpenCLI Yuque returned a non-JSON response: {}",
            text.chars().take(240).collect::<String>()
        ))
    })?;
    if output.status.success() {
        return Ok(value);
    }
    let message = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("OpenCLI Yuque command failed")
        .to_owned();
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("not found") || normalized.contains("no document") {
        Err(ApplicationError::NotFound(message))
    } else if normalized.contains("login")
        || normalized.contains("permission")
        || normalized.contains("auth")
    {
        Err(ApplicationError::Conflict(message))
    } else {
        Err(ApplicationError::Storage(message))
    }
}

fn required_string(value: &Value, key: &str) -> Result<String, ApplicationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApplicationError::Integrity(format!("Yuque result has no {key}")))
}

fn is_yuque_image_url(value: &str) -> bool {
    value.starts_with("https://cdn.nlark.com/yuque/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_scope_is_explicit_and_bounded() {
        assert_eq!(parse_recent_scope("recent:8").unwrap(), 8);
        assert!(parse_recent_scope("recent:21").is_err());
        assert!(parse_recent_scope("all").is_err());
    }

    #[test]
    fn fingerprint_tracks_text_and_stable_media_tokens() {
        let tokens = vec!["a.png".to_owned(), "b.png".to_owned()];
        assert_eq!(
            yuque_content_fingerprint("doc", "body", &tokens),
            yuque_content_fingerprint("doc", "body", &tokens)
        );
        assert_ne!(
            yuque_content_fingerprint("doc", "body", &tokens),
            yuque_content_fingerprint("doc", "changed", &tokens)
        );
    }

    #[test]
    fn media_host_is_restricted_to_yuque_cdn() {
        assert!(is_yuque_image_url(
            "https://cdn.nlark.com/yuque/0/2024/png/example.png"
        ));
        assert!(!is_yuque_image_url("https://example.com/image.png"));
    }
}

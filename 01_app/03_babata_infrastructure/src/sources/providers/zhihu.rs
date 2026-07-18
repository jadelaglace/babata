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

const ROUTE_ID: &str = "source.zhihu";
const ADAPTER_VERSION: &str = "opencli-zhihu/1";
static DOWNLOAD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Default)]
pub struct ZhihuConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(ROUTE_ID.to_owned()),
        provider: "zhihu".to_owned(),
        status: CapabilityStatus::Disabled,
        activation_phase: "P4".to_owned(),
    }
}

#[derive(Debug, Clone)]
pub struct ZhihuOpenCliAdapter {
    download_root: PathBuf,
}

impl ZhihuOpenCliAdapter {
    pub fn new(download_root: PathBuf) -> Self {
        Self { download_root }
    }

    fn download_images(
        &self,
        answer_id: &str,
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
        let output_dir = self.download_root.join(answer_id).join(batch);
        fs::create_dir_all(&output_dir).map_err(|error| {
            ApplicationError::Asset(format!("unable to create Zhihu image directory: {error}"))
        })?;
        images
            .iter()
            .enumerate()
            .map(|(index, image)| {
                let url = required_string(image, "Url")?;
                if !is_zhihu_image_url(&url) {
                    return Err(ApplicationError::Integrity(
                        "Zhihu returned an unsupported image host".to_owned(),
                    ));
                }
                let path = output_dir.join(format!("image-{index:03}.jpg"));
                let output = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" })
                    .args([
                        "--location",
                        "--fail",
                        "--silent",
                        "--show-error",
                        "--referer",
                        "https://www.zhihu.com/",
                        "--user-agent",
                        "Mozilla/5.0",
                        "--output",
                    ])
                    .arg(&path)
                    .arg(&url)
                    .output()
                    .map_err(|error| {
                        ApplicationError::Asset(format!(
                            "unable to start Zhihu image download: {error}"
                        ))
                    })?;
                if !output.status.success() {
                    return Err(ApplicationError::Asset(format!(
                        "Zhihu image download failed: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    )));
                }
                if fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0) == 0 {
                    return Err(ApplicationError::Asset(
                        "Zhihu image download produced an empty file".to_owned(),
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

impl SourceAdapterPort for ZhihuOpenCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let (collection_id, limit) = parse_collection_scope(source_reference)?;
        let output = run_opencli(&[
            "zhihu",
            "collection",
            collection_id,
            "--offset",
            "0",
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
            ApplicationError::Integrity("OpenCLI Zhihu collection was not an array".to_owned())
        })?;
        rows.iter()
            .map(|row| {
                let url = required_string(row, "url")?;
                let item_type = required_string(row, "type")?;
                let native_id = numeric_tail(&url).ok_or_else(|| {
                    ApplicationError::Integrity("Zhihu candidate URL has no numeric ID".to_owned())
                })?;
                let title = required_string(row, "title")?;
                Ok(DiscoveredCandidate {
                    summary: CandidateSummary {
                        candidate_id: format!("zhihu_{item_type}_{native_id}"),
                        session_id: session_id.clone(),
                        route_id: SourceRouteId(ROUTE_ID.to_owned()),
                        source_native_id: Some(native_id),
                        title: Some(title.clone()),
                        source_location: Some(url),
                        hierarchy: vec![
                            "Zhihu".to_owned(),
                            format!("Collection {collection_id}"),
                            title,
                        ],
                        content_type: ContentType::Document,
                        source_updated_at: None,
                        attachment_available: None,
                        limitations: if item_type == "answer" {
                            vec!["collection listing does not expose media inventory".to_owned()]
                        } else {
                            vec![format!(
                                "{item_type} detail collection is not yet covered by this answer adapter"
                            )]
                        },
                        selection_capabilities: vec![
                            "single".to_owned(),
                            "visible_set".to_owned(),
                            "collection_count".to_owned(),
                        ],
                    },
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
            ApplicationError::Integrity("Zhihu candidate has no source URL".to_owned())
        })?;
        if !source.contains("/answer/") {
            return Ok(AcquisitionOutcome::Inaccessible {
                reason: "this Zhihu adapter currently closes answer candidates only".to_owned(),
            });
        }
        let answer_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Zhihu answer has no native ID".to_owned())
        })?;
        let output = match run_opencli(&[
            "zhihu",
            "answer-detail-full",
            answer_id,
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
        acquisition_from_detail(
            self,
            candidate,
            answer_id,
            source,
            requested_attachments,
            &output,
        )
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: true,
            limitations: vec![
                "answer text, raw HTML and inline original images are covered".to_owned(),
                "articles, pins, videos and comment threads are not yet covered".to_owned(),
                "candidate discovery is bounded to one explicit collection and count".to_owned(),
            ],
        }
    }
}

fn acquisition_from_detail(
    adapter: &ZhihuOpenCliAdapter,
    candidate: &CandidateSummary,
    answer_id: &str,
    source: &str,
    requested_attachments: bool,
    output: &Value,
) -> Result<AcquisitionOutcome, ApplicationError> {
    let row = output
        .as_array()
        .and_then(|rows| rows.first())
        .ok_or_else(|| {
            ApplicationError::Integrity("Zhihu detail-full returned no answer".to_owned())
        })?;
    let content_text = required_string(row, "ContentText")?;
    let content_html = required_string(row, "ContentHtml")?;
    let images = row
        .get("Images")
        .and_then(Value::as_array)
        .ok_or_else(|| ApplicationError::Integrity("Zhihu Images is not an array".to_owned()))?;
    let assets = if requested_attachments {
        adapter.download_images(answer_id, images)?
    } else {
        Vec::new()
    };
    let payload = serde_json::to_string_pretty(&serde_json::json!({
        "platform": "zhihu",
        "answer_id": answer_id,
        "question_id": row.get("QuestionId"),
        "question_title": row.get("QuestionTitle"),
        "author": row.get("Author"),
        "created_at": row.get("CreatedAt"),
        "updated_at": row.get("UpdatedAt"),
        "content_text": content_text,
        "content_html": content_html,
        "images": images,
    }))
    .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let stable_images = images
        .iter()
        .map(|image| required_string(image, "Token"))
        .collect::<Result<Vec<_>, _>>()?;
    let content_fingerprint = Sha256::of_bytes(
        serde_json::json!({
            "updated_at": row.get("UpdatedAt"),
            "content_text": content_text,
            "images": stable_images,
        })
        .to_string()
        .as_bytes(),
    );
    let metadata = Metadata::parse(
        &serde_json::json!({
            "title": candidate.title,
            "author": row.get("Author"),
            "question_id": row.get("QuestionId"),
            "created_at": row.get("CreatedAt"),
            "updated_at": row.get("UpdatedAt"),
            "inline_image_count": images.len(),
            "requested_attachments": requested_attachments,
            "downloaded_asset_count": assets.len(),
            "attachments_covered": requested_attachments && assets.len() == images.len(),
            "content_fingerprint": content_fingerprint.as_str(),
            "adapter_version": ADAPTER_VERSION,
        })
        .to_string(),
    )?;
    Ok(AcquisitionOutcome::Found {
        candidate: CandidateEnvelope {
            protocol_version: "1".to_owned(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_reference: source.to_owned(),
            content_type: ContentType::Document,
            payload_sha256: Sha256::of_bytes(payload.as_bytes()),
            metadata,
            payload: CandidatePayload::Text { text: payload },
            context: Some(candidate.hierarchy.join(" / ")),
            native_id: Some(answer_id.to_owned()),
        },
        assets,
    })
}

fn parse_collection_scope(value: &str) -> Result<(&str, usize), ApplicationError> {
    let mut parts = value.split(':');
    if parts.next() != Some("collection") {
        return Err(ApplicationError::Conflict(
            "Zhihu scope must be collection:<id>:<count>".to_owned(),
        ));
    }
    let id = parts.next().unwrap_or_default();
    let count = parts
        .next()
        .unwrap_or_default()
        .parse::<usize>()
        .map_err(|_| ApplicationError::Conflict("invalid Zhihu collection count".to_owned()))?;
    if parts.next().is_some() || !id.chars().all(|character| character.is_ascii_digit()) {
        return Err(ApplicationError::Conflict(
            "Zhihu collection ID must be numeric".to_owned(),
        ));
    }
    if !(1..=100).contains(&count) {
        return Err(ApplicationError::Conflict(
            "Zhihu collection count must be between 1 and 100".to_owned(),
        ));
    }
    Ok((id, count))
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
            "OpenCLI Zhihu returned a non-JSON response: {}",
            text.chars().take(240).collect::<String>()
        ))
    })?;
    if output.status.success() {
        return Ok(value);
    }
    let message = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("OpenCLI Zhihu command failed")
        .to_owned();
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("not found") || normalized.contains("no zhihu answer") {
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
        .ok_or_else(|| ApplicationError::Integrity(format!("Zhihu result has no {key}")))
}

fn numeric_tail(value: &str) -> Option<String> {
    value
        .split(['/', '?', '#'])
        .rfind(|part| !part.is_empty() && part.chars().all(|character| character.is_ascii_digit()))
        .map(str::to_owned)
}

fn is_zhihu_image_url(value: &str) -> bool {
    ["picx", "pic1", "pic2", "pic3", "pic4", "pica"]
        .iter()
        .any(|host| value.starts_with(&format!("https://{host}.zhimg.com/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_scope_is_explicit_and_bounded() {
        assert_eq!(
            parse_collection_scope("collection:711587450:28").unwrap(),
            ("711587450", 28)
        );
        assert!(parse_collection_scope("collection:711587450:101").is_err());
        assert!(parse_collection_scope("all").is_err());
    }

    #[test]
    fn image_host_allowlist_rejects_unrelated_urls() {
        assert!(is_zhihu_image_url(
            "https://picx.zhimg.com/v2-example_r.jpg?source=x"
        ));
        assert!(!is_zhihu_image_url("https://example.com/image.jpg"));
    }
}

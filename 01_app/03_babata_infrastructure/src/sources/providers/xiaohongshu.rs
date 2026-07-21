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

const ROUTE_ID: &str = "source.xiaohongshu";
const ADAPTER_VERSION: &str = "opencli-xiaohongshu/1";
static DOWNLOAD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Default)]
pub struct XiaohongshuConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(ROUTE_ID.to_owned()),
        provider: "xiaohongshu".to_owned(),
        status: CapabilityStatus::Disabled,
        activation_phase: "P4".to_owned(),
    }
}

#[derive(Debug, Clone)]
pub struct XiaohongshuOpenCliAdapter {
    download_root: PathBuf,
}

impl XiaohongshuOpenCliAdapter {
    pub fn new(download_root: PathBuf) -> Self {
        Self { download_root }
    }

    fn download_media(
        &self,
        note_id: &str,
        source_url: &str,
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
        let output_dir = self.download_root.join(note_id).join(batch);
        fs::create_dir_all(&output_dir).map_err(|error| {
            ApplicationError::Asset(format!(
                "unable to create Xiaohongshu media directory: {error}"
            ))
        })?;
        let output_arg = output_dir.to_string_lossy().into_owned();
        let result = run_opencli(&[
            "xiaohongshu",
            "download",
            source_url,
            "--output",
            &output_arg,
            "--window",
            "background",
            "--site-session",
            "persistent",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ])?;
        let rows = result.as_array().ok_or_else(|| {
            ApplicationError::Integrity("Xiaohongshu download result was not an array".to_owned())
        })?;
        if rows
            .iter()
            .any(|row| row.get("status").and_then(Value::as_str) != Some("success"))
        {
            return Err(ApplicationError::Asset(
                "Xiaohongshu reported an incomplete media download".to_owned(),
            ));
        }
        let mut paths = Vec::new();
        collect_files(&output_dir, &mut paths)?;
        paths.sort();
        if paths.is_empty() || paths.len() != rows.len() {
            return Err(ApplicationError::Asset(format!(
                "Xiaohongshu downloaded {} files for {} media rows",
                paths.len(),
                rows.len()
            )));
        }
        Ok(paths
            .into_iter()
            .map(|path| CaptureImportAsset {
                path: path.to_string_lossy().into_owned(),
                role: AssetRole::Attachment,
            })
            .collect())
    }
}

impl SourceAdapterPort for XiaohongshuOpenCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let (profile_id, limit) = parse_saved_scope(source_reference)?;
        let rows = saved_notes(profile_id, limit)?;
        rows.iter()
            .map(|row| {
                let id = required_string(row, "id")?;
                let listed_title = row
                    .get("title")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty());
                let title = listed_title.map_or_else(
                    || format!("Untitled Xiaohongshu note {id}"),
                    str::to_owned,
                );
                let url = required_string(row, "url")?;
                Ok(DiscoveredCandidate {
                    summary: CandidateSummary {
                        candidate_id: format!("xiaohongshu_{id}"),
                        session_id: session_id.clone(),
                        route_id: SourceRouteId(ROUTE_ID.to_owned()),
                        source_native_id: Some(id),
                        title: Some(title.clone()),
                        source_location: Some(url),
                        hierarchy: vec![
                            "Xiaohongshu".to_owned(),
                            format!("Saved profile {profile_id}"),
                            title,
                        ],
                        content_type: ContentType::Document,
                        source_updated_at: None,
                        attachment_available: None,
                        limitations: [
                            Some("saved listing does not expose media count or update time".to_owned()),
                            listed_title.is_none().then(|| "saved listing returned no title; note ID is used as the candidate label".to_owned()),
                        ]
                        .into_iter()
                        .flatten()
                        .collect(),
                        selection_capabilities: vec![
                            "single".to_owned(),
                            "visible_set".to_owned(),
                            "saved_count".to_owned(),
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
        let note_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Xiaohongshu candidate has no note ID".to_owned())
        })?;
        let source_url = refreshed_source_url(candidate).unwrap_or_else(|| {
            candidate
                .source_location
                .clone()
                .unwrap_or_else(|| note_id.to_owned())
        });
        let detail = match run_opencli(&[
            "xiaohongshu",
            "note",
            &source_url,
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
            note_id,
            &source_url,
            requested_attachments,
            &detail,
        )
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: true,
            limitations: vec![
                "saved-note text, tags, images and videos are covered".to_owned(),
                "saved albums, files, comments and liked notes are not yet covered".to_owned(),
                "candidate discovery is bounded to one explicit profile and count".to_owned(),
            ],
        }
    }
}

fn acquisition_from_detail(
    adapter: &XiaohongshuOpenCliAdapter,
    candidate: &CandidateSummary,
    note_id: &str,
    source_url: &str,
    requested_attachments: bool,
    detail: &Value,
) -> Result<AcquisitionOutcome, ApplicationError> {
    let rows = detail.as_array().ok_or_else(|| {
        ApplicationError::Integrity("Xiaohongshu note result was not an array".to_owned())
    })?;
    let field = |name: &str| {
        rows.iter()
            .find(|row| row.get("field").and_then(Value::as_str) == Some(name))
            .and_then(|row| row.get("value"))
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    let content = field("content");
    if content.trim().is_empty() {
        return Err(ApplicationError::Integrity(
            "Xiaohongshu note content is empty".to_owned(),
        ));
    }
    let assets = if requested_attachments {
        adapter.download_media(note_id, source_url)?
    } else {
        Vec::new()
    };
    let payload = serde_json::to_string_pretty(&serde_json::json!({
        "platform": "xiaohongshu",
        "note_id": note_id,
        "title": field("title"),
        "author": field("author"),
        "content": content,
        "tags": field("tags"),
        "likes": field("likes"),
        "collects": field("collects"),
        "comments": field("comments"),
    }))
    .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let content_fingerprint = xiaohongshu_content_fingerprint(
        note_id,
        field("title"),
        field("author"),
        content,
        field("tags"),
    );
    let metadata = Metadata::parse(
        &serde_json::json!({
            "title": candidate.title,
            "author": field("author"),
            "tags": field("tags"),
            "likes": field("likes"),
            "collects": field("collects"),
            "comments": field("comments"),
            "requested_attachments": requested_attachments,
            "downloaded_asset_count": assets.len(),
            "attachments_covered": requested_attachments && !assets.is_empty(),
            "content_fingerprint": content_fingerprint.as_str(),
            "adapter_version": ADAPTER_VERSION,
        })
        .to_string(),
    )?;
    Ok(AcquisitionOutcome::Found {
        candidate: Box::new(CandidateEnvelope {
            protocol_version: "1".to_owned(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_reference: source_url.to_owned(),
            content_type: ContentType::Document,
            payload_sha256: Sha256::of_bytes(payload.as_bytes()),
            metadata,
            payload: CandidatePayload::Text { text: payload },
            context: Some(candidate.hierarchy.join(" / ")),
            native_id: Some(note_id.to_owned()),
            common_metadata: candidate.effective_common_metadata(),
        }),
        assets,
    })
}

fn parse_saved_scope(value: &str) -> Result<(&str, usize), ApplicationError> {
    let mut parts = value.split(':');
    if parts.next() != Some("saved") {
        return Err(ApplicationError::Conflict(
            "Xiaohongshu scope must be saved:<profile-id>:<count>".to_owned(),
        ));
    }
    let profile = parts.next().unwrap_or_default();
    let count = parts
        .next()
        .unwrap_or_default()
        .parse::<usize>()
        .map_err(|_| ApplicationError::Conflict("invalid saved-note count".to_owned()))?;
    if parts.next().is_some()
        || profile.is_empty()
        || !profile
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(ApplicationError::Conflict(
            "Xiaohongshu profile ID must be hexadecimal".to_owned(),
        ));
    }
    if !(1..=50).contains(&count) {
        return Err(ApplicationError::Conflict(
            "Xiaohongshu saved-note count must be between 1 and 50".to_owned(),
        ));
    }
    Ok((profile, count))
}

fn xiaohongshu_content_fingerprint(
    note_id: &str,
    title: &str,
    author: &str,
    content: &str,
    tags: &str,
) -> Sha256 {
    Sha256::of_bytes(
        serde_json::json!({
            "note_id": note_id,
            "title": title,
            "author": author,
            "content": content,
            "tags": tags,
        })
        .to_string()
        .as_bytes(),
    )
}

fn saved_notes(profile_id: &str, limit: usize) -> Result<Vec<Value>, ApplicationError> {
    let output = run_opencli(&[
        "xiaohongshu",
        "saved",
        "--id",
        profile_id,
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
    output.as_array().cloned().ok_or_else(|| {
        ApplicationError::Integrity("Xiaohongshu saved result was not an array".to_owned())
    })
}

fn refreshed_source_url(candidate: &CandidateSummary) -> Option<String> {
    let profile = candidate
        .hierarchy
        .iter()
        .find_map(|part| part.strip_prefix("Saved profile "))?;
    let note_id = candidate.source_native_id.as_deref()?;
    saved_notes(profile, 50)
        .ok()?
        .into_iter()
        .find(|row| row.get("id").and_then(Value::as_str) == Some(note_id))
        .and_then(|row| row.get("url").and_then(Value::as_str).map(str::to_owned))
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
            "OpenCLI Xiaohongshu returned a non-JSON response: {}",
            text.chars().take(240).collect::<String>()
        ))
    })?;
    if output.status.success() {
        return Ok(value);
    }
    let message = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("OpenCLI Xiaohongshu command failed")
        .to_owned();
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("not found") || normalized.contains("no note") {
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
        .ok_or_else(|| ApplicationError::Integrity(format!("Xiaohongshu result has no {key}")))
}

fn collect_files(root: &PathBuf, files: &mut Vec<PathBuf>) -> Result<(), ApplicationError> {
    for entry in fs::read_dir(root).map_err(|error| ApplicationError::Asset(error.to_string()))? {
        let path = entry
            .map_err(|error| ApplicationError::Asset(error.to_string()))?
            .path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_scope_is_explicit_and_bounded() {
        assert_eq!(
            parse_saved_scope("saved:5d0da3b50000000010037438:20").unwrap(),
            ("5d0da3b50000000010037438", 20)
        );
        assert!(parse_saved_scope("saved:abc:51").is_err());
        assert!(parse_saved_scope("all").is_err());
    }

    #[test]
    fn content_fingerprint_tracks_body_without_live_counts() {
        let first = xiaohongshu_content_fingerprint("note", "title", "author", "body", "tag");
        let same = xiaohongshu_content_fingerprint("note", "title", "author", "body", "tag");
        let changed = xiaohongshu_content_fingerprint("note", "title", "author", "changed", "tag");
        assert_eq!(first, same);
        assert_ne!(first, changed);
    }
}

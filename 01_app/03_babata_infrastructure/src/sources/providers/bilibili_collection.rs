use std::{fs, path::PathBuf, process::Command};

use babata_application::{
    AcquisitionOutcome, ApplicationError, CaptureImportAsset, DiscoveredCandidate,
    ports::SourceAdapterPort,
};
use babata_domain::{
    AssetRole, CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus,
    CollectionSessionId, ContentType, Metadata, RouteCoverage, Sha256, SourceRouteDescriptor,
    SourceRouteId,
};
use serde_json::{Map, Value};

const ROUTE_ID: &str = "source.bilibili";
const ADAPTER_VERSION: &str = "opencli-bilibili/1";

#[derive(Debug, Clone)]
pub struct BilibiliOpenCliAdapter {
    download_root: PathBuf,
}

impl BilibiliOpenCliAdapter {
    pub fn new(download_root: PathBuf) -> Self {
        Self { download_root }
    }

    fn download_video(&self, bvid: &str) -> Result<String, ApplicationError> {
        let output_dir = self.download_root.join(bvid);
        fs::create_dir_all(&output_dir).map_err(|error| {
            ApplicationError::Asset(format!(
                "unable to create Bilibili download directory: {error}"
            ))
        })?;
        let output_dir_text = output_dir.to_string_lossy().into_owned();
        run_opencli(&[
            "bilibili",
            "download",
            bvid,
            "--quality",
            "480p",
            "--output",
            &output_dir_text,
            "--window",
            "background",
            "--site-session",
            "ephemeral",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ])?;
        let mut media = fs::read_dir(&output_dir)
            .map_err(|error| ApplicationError::Asset(error.to_string()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(bvid))
                    && path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| {
                            matches!(
                                extension.to_ascii_lowercase().as_str(),
                                "mp4" | "mkv" | "webm"
                            )
                        })
            })
            .collect::<Vec<_>>();
        media.sort();
        let path = media.into_iter().next().ok_or_else(|| {
            ApplicationError::Asset("Bilibili download completed without a media file".to_owned())
        })?;
        Ok(path.to_string_lossy().into_owned())
    }
}

impl SourceAdapterPort for BilibiliOpenCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        SourceRouteDescriptor {
            id: SourceRouteId(ROUTE_ID.to_owned()),
            provider: "bilibili".to_owned(),
            status: CapabilityStatus::Disabled,
            activation_phase: "P4".to_owned(),
        }
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let limit = source_reference
            .strip_prefix("history:")
            .ok_or_else(|| {
                ApplicationError::Conflict(
                    "Bilibili scope must be history:<count>; account-wide all is never implicit"
                        .to_owned(),
                )
            })?
            .parse::<usize>()
            .map_err(|_| ApplicationError::Conflict("invalid Bilibili history count".to_owned()))?;
        if !(1..=100).contains(&limit) {
            return Err(ApplicationError::Conflict(
                "Bilibili history count must be between 1 and 100".to_owned(),
            ));
        }
        let output = run_opencli(&[
            "bilibili",
            "history",
            "--limit",
            &limit.to_string(),
            "--window",
            "background",
            "--site-session",
            "ephemeral",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ])?;
        let rows = output.as_array().ok_or_else(|| {
            ApplicationError::Integrity("OpenCLI Bilibili history was not an array".to_owned())
        })?;
        rows.iter().map(|row| {
            let title = required_string(row, "title")?;
            let author = required_string(row, "author")?;
            let url = required_string(row, "url")?;
            let bvid = bvid_from_url(&url)?;
            Ok(DiscoveredCandidate {
                summary: CandidateSummary {
                    candidate_id: format!("bilibili_{bvid}"),
                    session_id: session_id.clone(),
                    route_id: SourceRouteId(ROUTE_ID.to_owned()),
                    source_native_id: Some(bvid),
                    title: Some(title.clone()),
                    source_location: Some(url),
                    hierarchy: vec![
                        "Bilibili".to_owned(), "Watch history".to_owned(), author, title,
                    ],
                    content_type: ContentType::Document,
                    source_updated_at: None,
                    attachment_available: Some(true),
                    limitations: vec![
                        "candidate discovery is bounded to the requested watch-history window".to_owned(),
                        "video media is downloaded only when attachments are explicitly requested".to_owned(),
                    ],
                    selection_capabilities: vec![
                        "single".to_owned(), "visible_set".to_owned(),
                        "history_count".to_owned(), "media_download".to_owned(),
                    ],
                },
                prefetched: None,
            })
        }).collect()
    }

    fn collect(
        &self,
        candidate: &CandidateSummary,
        _prefetched: Option<&CandidateEnvelope>,
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let bvid = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Bilibili candidate has no BV ID".to_owned())
        })?;
        let video_rows = run_opencli(&[
            "bilibili",
            "video",
            bvid,
            "--window",
            "background",
            "--site-session",
            "ephemeral",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ])?;
        let video = field_rows(&video_rows)?;
        let subtitle = run_optional(&[
            "bilibili",
            "subtitle",
            bvid,
            "--window",
            "background",
            "--site-session",
            "ephemeral",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ]);
        let summary = run_optional(&[
            "bilibili",
            "summary",
            bvid,
            "--window",
            "background",
            "--site-session",
            "ephemeral",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ]);
        let assets = if requested_attachments {
            vec![CaptureImportAsset {
                path: self.download_video(bvid)?,
                role: AssetRole::Original,
            }]
        } else {
            Vec::new()
        };
        let content_fingerprint =
            bilibili_content_fingerprint(bvid, &video, subtitle.as_ref(), summary.as_ref());
        let payload = serde_json::to_string_pretty(&serde_json::json!({
            "platform": "bilibili",
            "bvid": bvid,
            "title": candidate.title,
            "video": video,
            "subtitle": subtitle,
            "official_ai_summary": summary,
        }))
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        let metadata = Metadata::parse(
            &serde_json::json!({
                "title": candidate.title,
                "bvid": bvid,
                "adapter_version": ADAPTER_VERSION,
                "subtitle_available": subtitle.is_some(),
                "official_ai_summary_available": summary.is_some(),
                "requested_attachments": requested_attachments,
                "downloaded_asset_count": assets.len(),
                "content_fingerprint": content_fingerprint.as_str(),
            })
            .to_string(),
        )?;
        Ok(AcquisitionOutcome::Found {
            candidate: CandidateEnvelope {
                protocol_version: "1".to_owned(),
                route_id: SourceRouteId(ROUTE_ID.to_owned()),
                source_reference: candidate
                    .source_location
                    .clone()
                    .unwrap_or_else(|| format!("https://www.bilibili.com/video/{bvid}")),
                content_type: ContentType::Document,
                payload_sha256: Sha256::of_bytes(payload.as_bytes()),
                metadata,
                payload: CandidatePayload::Text { text: payload },
                context: Some("Bilibili / Watch history".to_owned()),
                native_id: Some(bvid.to_owned()),
            },
            assets,
        })
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: true,
            limitations: vec![
                "video metadata, available subtitles and official AI summary are preserved"
                    .to_owned(),
                "selected free video media is downloaded through yt-dlp at 480p for the P4 proof"
                    .to_owned(),
                "comments, danmaku and multi-part expansion are outside the current pilot"
                    .to_owned(),
            ],
        }
    }
}

fn field_rows(value: &Value) -> Result<Map<String, Value>, ApplicationError> {
    let rows = value.as_array().ok_or_else(|| {
        ApplicationError::Integrity("OpenCLI Bilibili video was not an array".to_owned())
    })?;
    let mut fields = Map::new();
    for row in rows {
        fields.insert(
            required_string(row, "field")?,
            row.get("value").cloned().unwrap_or(Value::Null),
        );
    }
    Ok(fields)
}

fn bilibili_content_fingerprint(
    bvid: &str,
    video: &Map<String, Value>,
    subtitle: Option<&Value>,
    summary: Option<&Value>,
) -> Sha256 {
    let mut stable_video = video.clone();
    for volatile_counter in [
        "view", "like", "coin", "favorite", "share", "reply", "danmaku",
    ] {
        stable_video.remove(volatile_counter);
    }
    Sha256::of_bytes(
        serde_json::json!({
            "bvid": bvid,
            "video": stable_video,
            "subtitle": subtitle,
            "official_ai_summary": summary,
        })
        .to_string()
        .as_bytes(),
    )
}

fn run_optional(args: &[&str]) -> Option<Value> {
    run_opencli(args).ok()
}

fn run_opencli(args: &[&str]) -> Result<Value, ApplicationError> {
    let executable = if cfg!(windows) {
        "opencli.cmd"
    } else {
        "opencli"
    };
    let output = Command::new(executable)
        .args(args)
        .output()
        .map_err(|error| ApplicationError::Asset(format!("unable to start OpenCLI: {error}")))?;
    let bytes = if output.status.success() {
        &output.stdout
    } else {
        &output.stderr
    };
    let value: Value = serde_json::from_slice(bytes).map_err(|_| {
        ApplicationError::Integrity("OpenCLI returned a non-JSON response".to_owned())
    })?;
    if output.status.success() {
        Ok(value)
    } else {
        let message = value
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("OpenCLI Bilibili command failed")
            .to_owned();
        Err(ApplicationError::Storage(message))
    }
}

fn bvid_from_url(url: &str) -> Result<String, ApplicationError> {
    url.split('/')
        .find(|part| part.starts_with("BV") && part.len() >= 10)
        .map(str::to_owned)
        .ok_or_else(|| ApplicationError::Integrity("Bilibili history URL has no BV ID".to_owned()))
}

fn required_string(value: &Value, key: &str) -> Result<String, ApplicationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApplicationError::Integrity(format!("OpenCLI result has no {key}")))
}

#[cfg(test)]
mod tests {
    use super::bilibili_content_fingerprint;
    use serde_json::{Map, Value, json};

    fn fields(value: Value) -> Map<String, Value> {
        value.as_object().expect("test value is an object").clone()
    }

    #[test]
    fn content_fingerprint_ignores_live_counters_but_tracks_content() {
        let first = fields(json!({
            "title": "stable title",
            "description": "stable description",
            "view": "100",
            "like": "10",
            "favorite": "3"
        }));
        let counters_changed = fields(json!({
            "title": "stable title",
            "description": "stable description",
            "view": "101",
            "like": "11",
            "favorite": "4"
        }));
        let content_changed = fields(json!({
            "title": "revised title",
            "description": "stable description",
            "view": "101",
            "like": "11",
            "favorite": "4"
        }));
        let subtitle = json!({"body": "subtitle"});
        let summary = json!({"body": "summary"});

        let initial =
            bilibili_content_fingerprint("BV1TEST", &first, Some(&subtitle), Some(&summary));
        let after_counters = bilibili_content_fingerprint(
            "BV1TEST",
            &counters_changed,
            Some(&subtitle),
            Some(&summary),
        );
        let after_content = bilibili_content_fingerprint(
            "BV1TEST",
            &content_changed,
            Some(&subtitle),
            Some(&summary),
        );

        assert_eq!(initial, after_counters);
        assert_ne!(initial, after_content);
    }
}

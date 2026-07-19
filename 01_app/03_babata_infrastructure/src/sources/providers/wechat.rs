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
    CollectionSessionId, ContentType, Metadata, RouteCoverage, Sha256, SourceRouteDescriptor,
    SourceRouteId,
};
use serde_json::{Value, json};

const ROUTE_ID: &str = "source.wechat_articles";
const ADAPTER_VERSION: &str = "pc-wechat-ui+opencli-weixin/1";
static DOWNLOAD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Default)]
pub struct WechatConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(ROUTE_ID.to_owned()),
        provider: "wechat".to_owned(),
        status: CapabilityStatus::Disabled,
        activation_phase: "P4".to_owned(),
    }
}

#[derive(Debug, Clone)]
pub struct WechatArticleOpenCliAdapter {
    download_root: PathBuf,
}

impl WechatArticleOpenCliAdapter {
    pub fn new(download_root: PathBuf) -> Self {
        Self { download_root }
    }

    fn download(&self, url: &str, native_id: &str) -> Result<DownloadedArticle, ApplicationError> {
        let batch = format!(
            "{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| ApplicationError::Asset(error.to_string()))?
                .as_nanos(),
            DOWNLOAD_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let article_root = self.download_root.join(native_id);
        let output_dir = article_root.join(batch);
        fs::create_dir_all(&output_dir).map_err(|error| {
            ApplicationError::Asset(format!(
                "unable to create WeChat article download directory: {error}"
            ))
        })?;
        let output_dir_text = output_dir.to_string_lossy().into_owned();
        let output = run_opencli(&[
            "weixin",
            "download",
            "--url",
            url,
            "--output",
            &output_dir_text,
            "--download-images",
            "true",
            "--window",
            "background",
            "--site-session",
            "ephemeral",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ])?;
        downloaded_article(&article_root, &output)
    }
}

impl SourceAdapterPort for WechatArticleOpenCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let native_id = article_id(source_reference)?;
        Ok(vec![DiscoveredCandidate {
            summary: CandidateSummary {
                candidate_id: format!("wechat_article_{native_id}"),
                session_id: session_id.clone(),
                route_id: SourceRouteId(ROUTE_ID.to_owned()),
                source_native_id: Some(native_id.clone()),
                title: Some(format!("WeChat article {native_id}")),
                source_location: Some(source_reference.to_owned()),
                hierarchy: vec![
                    "WeChat".to_owned(),
                    "Favorites".to_owned(),
                    "Official account article".to_owned(),
                ],
                content_type: ContentType::Document,
                source_updated_at: None,
                attachment_available: Some(true),
                limitations: vec![
                    "the candidate was selected in the official PC WeChat Favorites UI".to_owned(),
                    "this route retrieves one already selected public article URL".to_owned(),
                ],
                selection_capabilities: vec!["single".to_owned(), "known_url".to_owned()],
            },
            prefetched: None,
        }])
    }

    fn collect(
        &self,
        candidate: &CandidateSummary,
        _prefetched: Option<&CandidateEnvelope>,
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let url = candidate.source_location.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("WeChat article candidate has no URL".to_owned())
        })?;
        let native_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("WeChat article candidate has no native ID".to_owned())
        })?;
        let mut article = self.download(url, native_id)?;
        if !requested_attachments {
            article
                .assets
                .retain(|asset| asset.role == AssetRole::Export);
        }
        acquisition(candidate, native_id, url, article, requested_attachments)
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: true,
            limitations: vec![
                "candidate discovery remains the official PC WeChat Favorites UI".to_owned(),
                "only one already selected public mp.weixin.qq.com article URL is retrieved"
                    .to_owned(),
                "OpenCLI Markdown and downloaded inline media are covered; raw HTML is covered only when pre-staged by the Agent"
                    .to_owned(),
                "favorite traversal, non-article favorite types, comments, chats and account history are not covered"
                    .to_owned(),
            ],
        }
    }
}

#[derive(Debug)]
struct DownloadedArticle {
    title: String,
    author: Option<String>,
    publish_time: Option<String>,
    markdown: String,
    assets: Vec<CaptureImportAsset>,
    stable_asset_hashes: Vec<String>,
    evidence: Value,
}

fn downloaded_article(
    article_root: &Path,
    output: &Value,
) -> Result<DownloadedArticle, ApplicationError> {
    let row = output
        .as_array()
        .and_then(|rows| rows.first())
        .ok_or_else(|| {
            ApplicationError::Integrity("OpenCLI returned no WeChat article".to_owned())
        })?;
    if row.get("status").and_then(Value::as_str) != Some("success") {
        return Err(ApplicationError::Storage(
            "OpenCLI did not report a successful WeChat article download".to_owned(),
        ));
    }
    let title = required_string(row, "title")?;
    let article_root = fs::canonicalize(article_root).map_err(|error| {
        ApplicationError::Asset(format!("unable to resolve WeChat download root: {error}"))
    })?;
    let saved =
        fs::canonicalize(PathBuf::from(required_string(row, "saved")?)).map_err(|error| {
            ApplicationError::Asset(format!(
                "unable to resolve downloaded WeChat Markdown: {error}"
            ))
        })?;
    if !saved.starts_with(&article_root) {
        return Err(ApplicationError::Integrity(
            "OpenCLI returned a WeChat download outside the authorised article directory"
                .to_owned(),
        ));
    }
    let markdown = fs::read_to_string(&saved).map_err(|error| {
        ApplicationError::Asset(format!(
            "unable to read downloaded WeChat Markdown: {error}"
        ))
    })?;
    if markdown.trim().is_empty() {
        return Err(ApplicationError::Integrity(
            "downloaded WeChat Markdown is empty".to_owned(),
        ));
    }

    let mut assets = vec![CaptureImportAsset {
        path: saved.to_string_lossy().into_owned(),
        role: AssetRole::Export,
    }];
    let mut stable_asset_hashes = Vec::new();
    if let Some(parent) = saved.parent() {
        for path in files_below(parent)? {
            if path == saved {
                continue;
            }
            stable_asset_hashes.push(hash_file(&path)?);
            assets.push(CaptureImportAsset {
                path: path.to_string_lossy().into_owned(),
                role: AssetRole::Attachment,
            });
        }
    }
    stable_asset_hashes.sort();

    let raw_html = article_root.join("source.html");
    if raw_html.is_file() {
        assets.push(CaptureImportAsset {
            path: raw_html.to_string_lossy().into_owned(),
            role: AssetRole::Export,
        });
    }
    let evidence_path = article_root.join("evidence.json");
    let evidence = if evidence_path.is_file() {
        let bytes = fs::read(&evidence_path).map_err(|error| {
            ApplicationError::Asset(format!("unable to read WeChat evidence: {error}"))
        })?;
        serde_json::from_slice(&bytes).map_err(|_| {
            ApplicationError::Integrity("WeChat evidence is invalid JSON".to_owned())
        })?
    } else {
        json!({})
    };

    Ok(DownloadedArticle {
        title,
        author: optional_string(row, "author"),
        publish_time: optional_string(row, "publish_time"),
        markdown,
        assets,
        stable_asset_hashes,
        evidence,
    })
}

fn acquisition(
    candidate: &CandidateSummary,
    native_id: &str,
    url: &str,
    article: DownloadedArticle,
    requested_attachments: bool,
) -> Result<babata_application::AcquisitionOutcome, ApplicationError> {
    let content_fingerprint = Sha256::of_bytes(
        json!({
            "native_id": native_id,
            "markdown": article.markdown,
            "assets": article.stable_asset_hashes,
        })
        .to_string()
        .as_bytes(),
    );
    let downloaded_attachment_count = article
        .assets
        .iter()
        .filter(|asset| asset.role == AssetRole::Attachment)
        .count();
    let payload = serde_json::to_string_pretty(&json!({
        "platform": "wechat",
        "kind": "official_account_article",
        "native_id": native_id,
        "title": article.title,
        "author": article.author,
        "publish_time": article.publish_time,
        "source_url": url,
        "markdown": article.markdown,
        "ui_evidence": article.evidence,
    }))
    .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let metadata = Metadata::parse(
        &json!({
            "title": article.title,
            "author": article.author,
            "publish_time": article.publish_time,
            "hierarchy": candidate.hierarchy,
            "requested_attachments": requested_attachments,
            "downloaded_attachment_count": downloaded_attachment_count,
            "export_count": article.assets.len() - downloaded_attachment_count,
            "content_fingerprint": content_fingerprint.as_str(),
            "adapter_version": ADAPTER_VERSION,
            "ui_evidence": article.evidence,
        })
        .to_string(),
    )?;
    Ok(AcquisitionOutcome::Found {
        candidate: CandidateEnvelope {
            protocol_version: "1".to_owned(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_reference: url.to_owned(),
            content_type: ContentType::Document,
            payload_sha256: Sha256::of_bytes(payload.as_bytes()),
            metadata,
            payload: CandidatePayload::Text { text: payload },
            context: Some("WeChat / Favorites / Official account article".to_owned()),
            native_id: Some(native_id.to_owned()),
        },
        assets: article.assets,
    })
}

fn article_id(url: &str) -> Result<String, ApplicationError> {
    let id = url
        .strip_prefix("https://mp.weixin.qq.com/s/")
        .and_then(|tail| tail.split(['?', '#']).next())
        .filter(|value| {
            !value.is_empty()
                && value.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
        })
        .ok_or_else(|| {
            ApplicationError::Conflict(
                "WeChat article source must be an https://mp.weixin.qq.com/s/<id> URL".to_owned(),
            )
        })?;
    Ok(id.to_owned())
}

fn files_below(root: &Path) -> Result<Vec<PathBuf>, ApplicationError> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in
            fs::read_dir(&directory).map_err(|error| ApplicationError::Asset(error.to_string()))?
        {
            let entry = entry.map_err(|error| ApplicationError::Asset(error.to_string()))?;
            let file_type = entry
                .file_type()
                .map_err(|error| ApplicationError::Asset(error.to_string()))?;
            let path = entry.path();
            if file_type.is_symlink() {
                return Err(ApplicationError::Integrity(
                    "WeChat download contains an unsupported symbolic link".to_owned(),
                ));
            }
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn hash_file(path: &Path) -> Result<String, ApplicationError> {
    fs::read(path)
        .map(|bytes| Sha256::of_bytes(&bytes).as_str().to_owned())
        .map_err(|error| ApplicationError::Asset(error.to_string()))
}

fn run_opencli(args: &[&str]) -> Result<Value, ApplicationError> {
    let output = Command::new(if cfg!(windows) {
        "opencli.cmd"
    } else {
        "opencli"
    })
    .args(args)
    .output()
    .map_err(|error| ApplicationError::Asset(format!("unable to start OpenCLI: {error}")))?;
    let bytes = if output.status.success() {
        &output.stdout
    } else {
        &output.stderr
    };
    let value: Value = serde_json::from_slice(bytes).map_err(|_| {
        ApplicationError::Integrity("OpenCLI WeChat returned a non-JSON response".to_owned())
    })?;
    if output.status.success() {
        Ok(value)
    } else {
        let message = value
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("OpenCLI WeChat download failed")
            .to_owned();
        Err(ApplicationError::Storage(message))
    }
}

fn required_string(value: &Value, key: &str) -> Result<String, ApplicationError> {
    optional_string(value, key)
        .ok_or_else(|| ApplicationError::Integrity(format!("OpenCLI result has no {key}")))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-")
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use babata_application::AcquisitionOutcome;
    use babata_domain::{
        AssetRole, CandidateSummary, CollectionSessionId, ContentType, SourceRouteId,
    };
    use serde_json::json;
    use tempfile::tempdir;

    use super::{acquisition, article_id, downloaded_article};

    #[test]
    fn only_public_short_article_urls_are_accepted() {
        assert_eq!(
            article_id("https://mp.weixin.qq.com/s/Va9tXvh6qWoOkog9SIbOOg").unwrap(),
            "Va9tXvh6qWoOkog9SIbOOg"
        );
        assert!(article_id("https://example.test/s/not-wechat").is_err());
    }

    #[test]
    fn download_path_cannot_escape_the_authorised_article_directory() {
        let temp = tempdir().unwrap();
        let article_root = temp.path().join("article");
        fs::create_dir_all(&article_root).unwrap();
        let outside = temp.path().join("outside.md");
        fs::write(&outside, "# Outside").unwrap();
        let result = downloaded_article(
            &article_root,
            &json!([{
                "title": "Title",
                "status": "success",
                "saved": outside,
            }]),
        );
        assert!(result.is_err());
    }

    #[test]
    fn downloaded_markdown_media_and_pre_staged_html_share_one_acquisition() {
        let temp = tempdir().unwrap();
        let article_root = temp.path().join("article");
        let output = article_root.join("batch").join("title");
        fs::create_dir_all(&output).unwrap();
        let markdown = output.join("title.md");
        let image = output.join("image.png");
        fs::write(&markdown, "# Title\n\nBody").unwrap();
        fs::write(&image, b"image bytes").unwrap();
        fs::write(article_root.join("source.html"), b"<html>source</html>").unwrap();
        fs::write(
            article_root.join("evidence.json"),
            serde_json::to_vec(&json!({"wechat_version": "4.1.11.55"})).unwrap(),
        )
        .unwrap();
        let downloaded = downloaded_article(
            &article_root,
            &json!([{
                "title": "Title",
                "author": "Account",
                "publish_time": "6月2日 08:56",
                "status": "success",
                "saved": markdown,
            }]),
        )
        .unwrap();
        let candidate = CandidateSummary {
            candidate_id: "wechat_article_id".to_owned(),
            session_id: CollectionSessionId::new(),
            route_id: SourceRouteId("source.wechat_articles".to_owned()),
            source_native_id: Some("id".to_owned()),
            title: Some("Title".to_owned()),
            source_location: Some("https://mp.weixin.qq.com/s/id".to_owned()),
            hierarchy: vec!["WeChat".to_owned(), "Favorites".to_owned()],
            content_type: ContentType::Document,
            source_updated_at: None,
            attachment_available: Some(true),
            limitations: Vec::new(),
            selection_capabilities: vec!["single".to_owned()],
        };
        let AcquisitionOutcome::Found { candidate, assets } = acquisition(
            &candidate,
            "id",
            "https://mp.weixin.qq.com/s/id",
            downloaded,
            true,
        )
        .unwrap() else {
            panic!("expected found acquisition");
        };
        assert_eq!(candidate.native_id.as_deref(), Some("id"));
        assert_eq!(assets.len(), 3);
        assert_eq!(
            assets
                .iter()
                .filter(|asset| asset.role == AssetRole::Export)
                .count(),
            2
        );
        assert_eq!(
            assets
                .iter()
                .filter(|asset| asset.role == AssetRole::Attachment)
                .count(),
            1
        );
    }
}

use std::{
    collections::HashSet,
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

use babata_application::{
    AcquisitionOutcome, ApplicationError, CaptureImportAsset, DiscoveredCandidate,
    ports::SourceAdapterPort,
};
use babata_domain::{
    AssetRole, CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus,
    CollectionSessionId, CommonSourceMetadata, ContentType, Metadata, RouteCoverage, Sha256,
    SourceAccessState, SourceHierarchyNode, SourceLimitation, SourceMediaEntry,
    SourceMediaMetadata, SourceRouteDescriptor, SourceRouteId,
};
use serde::Deserialize;
use sha2::{Digest, Sha256 as Sha256Hasher};

const ROUTE_ID: &str = "source.yitang";
const MANIFEST_SCHEMA: &str = "babata.yitang-source-manifest/v1";
const DOCUMENT_SCHEMA: &str = "babata.yitang-rendered-document/v1";
const ADAPTER_VERSION: &str = "yitang-prepared-manifest/1";

#[derive(Debug, Clone, Default)]
pub struct YitangPreparedManifestAdapter;

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(ROUTE_ID.to_owned()),
        provider: "yitang".to_owned(),
        status: CapabilityStatus::Enabled,
        activation_phase: "usage-0.2".to_owned(),
    }
}

#[derive(Debug, Deserialize)]
struct PreparedManifest {
    schema: String,
    readiness: String,
    acquisition_root: String,
    summary: ManifestSummary,
    items: Vec<PreparedItem>,
}

#[derive(Debug, Deserialize)]
struct ManifestSummary {
    source_items: usize,
    assets: usize,
    images: usize,
    unresolved: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct PreparedItem {
    stable_identity: String,
    replay_sid: String,
    replay_url: String,
    document_acl_token: String,
    document_token: String,
    document_url: String,
    title: String,
    subtitle: Option<String>,
    begin_time: Option<String>,
    hls_url: String,
    audio_url: String,
    duration_seconds_source: f64,
    duration_seconds_local: f64,
    document: PreparedDocument,
    content_fingerprint: String,
    assets: Vec<PreparedAsset>,
    readiness: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PreparedDocument {
    traversal_complete: bool,
    block_count: usize,
    nonempty_block_count: usize,
    heading_count: usize,
    image_count: usize,
    markdown_relative_path: String,
    blocks_relative_path: String,
    text_relative_path: String,
    html_relative_path: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PreparedAsset {
    relative_path: String,
    local_path: String,
    role: String,
    media_type: String,
    source_url: String,
    byte_size: u64,
    sha256: String,
}

struct ValidatedManifest {
    path: PathBuf,
    items: Vec<PreparedItem>,
}

impl SourceAdapterPort for YitangPreparedManifestAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let manifest = read_manifest(source_reference)?;
        manifest
            .items
            .iter()
            .map(|item| discovered_candidate(session_id, &manifest.path, item))
            .collect()
    }

    fn collect(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&CandidateEnvelope>,
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        if !requested_attachments {
            return Err(ApplicationError::Conflict(
                "Yitang replay/document candidates require all declared attachments".to_owned(),
            ));
        }
        let expected = prefetched.ok_or_else(|| {
            ApplicationError::Integrity("Yitang candidate envelope is missing".to_owned())
        })?;
        let manifest_path = metadata_string(&expected.metadata, "prepared_manifest_path")?;
        let native_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Yitang candidate has no stable identity".to_owned())
        })?;
        let manifest = read_manifest(&manifest_path)?;
        let item = manifest
            .items
            .iter()
            .find(|item| item.stable_identity == native_id)
            .ok_or_else(|| ApplicationError::NotFound(format!("Yitang item: {native_id}")))?;
        found(&manifest.path, item)
    }

    fn collect_recollection(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&CandidateEnvelope>,
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        match self.collect(candidate, prefetched, requested_attachments) {
            Err(ApplicationError::NotFound(reason)) => Ok(AcquisitionOutcome::Removed { reason }),
            result => result,
        }
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: true,
            limitations: vec![
                "only explicitly authorised replay/document pairs in validated prepared manifests are accepted"
                    .to_owned(),
                "signed-in browser acquisition excludes credentials and user interaction data"
                    .to_owned(),
                "document links are references and are not recursively collected".to_owned(),
            ],
        }
    }
}

fn read_manifest(source_reference: &str) -> Result<ValidatedManifest, ApplicationError> {
    let path = fs::canonicalize(source_reference).map_err(|error| {
        ApplicationError::Asset(format!(
            "unable to open Yitang prepared manifest: {:?}",
            error.kind()
        ))
    })?;
    if !path.is_file() {
        return Err(ApplicationError::Conflict(
            "Yitang source reference must be a prepared manifest file".to_owned(),
        ));
    }
    let bytes = fs::read(&path).map_err(|error| ApplicationError::Asset(error.to_string()))?;
    let manifest: PreparedManifest = serde_json::from_slice(&bytes).map_err(|_| {
        ApplicationError::Integrity("Yitang prepared manifest is invalid JSON".to_owned())
    })?;
    if manifest.schema != MANIFEST_SCHEMA || manifest.readiness != "prepared" {
        return Err(ApplicationError::Conflict(
            "Yitang prepared manifest has an unsupported schema or readiness".to_owned(),
        ));
    }
    let root = fs::canonicalize(&manifest.acquisition_root).map_err(|error| {
        ApplicationError::Asset(format!(
            "unable to inspect Yitang acquisition root: {error}"
        ))
    })?;
    if !root.is_dir() || path.parent() != Some(root.as_path()) {
        return Err(ApplicationError::Conflict(
            "Yitang manifest must live at its acquisition root".to_owned(),
        ));
    }
    if manifest.items.len() != 1
        || manifest.summary.source_items != manifest.items.len()
        || manifest.summary.unresolved != 0
    {
        return Err(ApplicationError::Integrity(
            "Yitang manifest must contain one complete source item".to_owned(),
        ));
    }
    let item = &manifest.items[0];
    if manifest.summary.assets != item.assets.len()
        || manifest.summary.images != item.document.image_count
    {
        return Err(ApplicationError::Integrity(
            "Yitang manifest summary is inconsistent".to_owned(),
        ));
    }
    validate_item(&root, item)?;
    Ok(ValidatedManifest {
        path,
        items: manifest.items,
    })
}

#[allow(clippy::too_many_lines)]
fn validate_item(root: &Path, item: &PreparedItem) -> Result<(), ApplicationError> {
    let expected_replay = format!("https://air.yitang.top/live/{}", item.replay_sid);
    let expected_document = format!(
        "https://yitang.top/fs-doc/{}/{}",
        item.document_acl_token, item.document_token
    );
    let expected_identity = format!(
        "replay:{}|document:{}",
        item.replay_sid, item.document_token
    );
    let valid_acl = item.document_acl_token.len() == 32
        && item
            .document_acl_token
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    if !valid_native_component(&item.replay_sid, 6, 64)
        || !valid_native_component(&item.document_token, 8, 128)
        || !valid_acl
        || item.replay_url != expected_replay
        || item.document_url != expected_document
        || item.stable_identity != expected_identity
        || item.title.trim().is_empty()
        || item.readiness != "prepared"
        || item.duration_seconds_source <= 0.0
        || item.duration_seconds_local <= 0.0
        || (item.duration_seconds_source - item.duration_seconds_local).abs() > 2.0
        || !item.hls_url.starts_with("https://cdn.yitang.top/")
        || !item.audio_url.starts_with("https://cdn.yitang.top/")
        || !item.document.traversal_complete
        || item.document.block_count == 0
        || item.document.nonempty_block_count == 0
        || item.document.nonempty_block_count > item.document.block_count
        || item.document.heading_count == 0
        || item.document.image_count == 0
    {
        return Err(ApplicationError::Integrity(
            "Yitang manifest contains an invalid or incomplete source pairing".to_owned(),
        ));
    }

    let expected_exports = HashSet::from([
        item.document.markdown_relative_path.as_str(),
        item.document.blocks_relative_path.as_str(),
        item.document.text_relative_path.as_str(),
        item.document.html_relative_path.as_str(),
    ]);
    if expected_exports.len() != 4 {
        return Err(ApplicationError::Integrity(
            "Yitang document exports must have distinct paths".to_owned(),
        ));
    }

    let mut relative_paths = HashSet::new();
    let mut local_paths = HashSet::new();
    let mut original_count = 0_usize;
    let mut export_paths = HashSet::new();
    let mut image_count = 0_usize;
    for asset in &item.assets {
        let role = asset_role(&asset.role)?;
        let relative = controlled_relative_path(&asset.relative_path)?;
        if !relative_paths.insert(asset.relative_path.as_str()) {
            return Err(ApplicationError::Integrity(
                "Yitang manifest contains a duplicate asset path".to_owned(),
            ));
        }
        let expected_path = fs::canonicalize(root.join(relative)).map_err(|error| {
            ApplicationError::Asset(format!(
                "unable to inspect Yitang asset: {:?}",
                error.kind()
            ))
        })?;
        let declared_path = fs::canonicalize(&asset.local_path).map_err(|error| {
            ApplicationError::Asset(format!(
                "unable to inspect Yitang asset: {:?}",
                error.kind()
            ))
        })?;
        if expected_path != declared_path
            || !expected_path.starts_with(root)
            || !expected_path.is_file()
            || !local_paths.insert(expected_path.clone())
            || asset.media_type.trim().is_empty()
        {
            return Err(ApplicationError::Integrity(
                "Yitang manifest contains an unsafe or duplicate local asset".to_owned(),
            ));
        }
        let expected_sha256 = Sha256::parse(asset.sha256.clone())?;
        let (actual_size, actual_sha256) = hash_file(&expected_path)?;
        if actual_size != asset.byte_size || actual_sha256 != expected_sha256 {
            return Err(ApplicationError::Integrity(
                "Yitang asset size or SHA-256 no longer matches the manifest".to_owned(),
            ));
        }
        match role {
            AssetRole::Original => {
                original_count += 1;
                if asset.source_url != item.replay_url || asset.media_type != "video/mp4" {
                    return Err(ApplicationError::Integrity(
                        "Yitang original media identity is inconsistent".to_owned(),
                    ));
                }
            }
            AssetRole::Export => {
                export_paths.insert(asset.relative_path.as_str());
                if asset.source_url != item.document_url {
                    return Err(ApplicationError::Integrity(
                        "Yitang document export identity is inconsistent".to_owned(),
                    ));
                }
            }
            AssetRole::Attachment => {
                image_count += 1;
                if !asset.relative_path.starts_with("document/images/")
                    || !asset.media_type.starts_with("image/")
                    || !asset.source_url.starts_with("https://cdn.yitang.top/")
                {
                    return Err(ApplicationError::Integrity(
                        "Yitang document image identity is inconsistent".to_owned(),
                    ));
                }
            }
            _ => {
                return Err(ApplicationError::Integrity(
                    "Yitang manifest contains an unsupported asset role".to_owned(),
                ));
            }
        }
    }
    if original_count != 1
        || export_paths != expected_exports
        || image_count != item.document.image_count
        || item.assets.len() != 5 + item.document.image_count
    {
        return Err(ApplicationError::Integrity(
            "Yitang replay, document exports, or image denominator is incomplete".to_owned(),
        ));
    }

    let markdown =
        fs::read_to_string(root.join(&item.document.markdown_relative_path)).map_err(|_| {
            ApplicationError::Integrity("Yitang Markdown export is not readable UTF-8".to_owned())
        })?;
    if markdown.trim().is_empty() || !markdown.contains(&item.title) {
        return Err(ApplicationError::Integrity(
            "Yitang Markdown export has no readable document body".to_owned(),
        ));
    }
    validate_blocks(root, item)?;
    if Sha256::parse(item.content_fingerprint.clone())? != content_fingerprint(item) {
        return Err(ApplicationError::Integrity(
            "Yitang manifest content fingerprint is inconsistent".to_owned(),
        ));
    }
    Ok(())
}

fn validate_blocks(root: &Path, item: &PreparedItem) -> Result<(), ApplicationError> {
    let bytes = fs::read(root.join(&item.document.blocks_relative_path))
        .map_err(|error| ApplicationError::Asset(error.to_string()))?;
    let blocks: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| {
        ApplicationError::Integrity("Yitang rendered block export is invalid JSON".to_owned())
    })?;
    let count = u64::try_from(item.document.block_count).ok();
    let image_count = u64::try_from(item.document.image_count).ok();
    if blocks.get("schema").and_then(serde_json::Value::as_str) != Some(DOCUMENT_SCHEMA)
        || blocks.get("title").and_then(serde_json::Value::as_str) != Some(&item.title)
        || blocks
            .get("document_token")
            .and_then(serde_json::Value::as_str)
            != Some(&item.document_token)
        || blocks.get("acl_token").and_then(serde_json::Value::as_str)
            != Some(&item.document_acl_token)
        || blocks
            .pointer("/traversal/reached_bottom")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        || blocks
            .get("block_count")
            .and_then(serde_json::Value::as_u64)
            != count
        || blocks
            .get("image_count")
            .and_then(serde_json::Value::as_u64)
            != image_count
    {
        return Err(ApplicationError::Integrity(
            "Yitang rendered block export does not match the source pairing".to_owned(),
        ));
    }
    Ok(())
}

fn controlled_relative_path(value: &str) -> Result<PathBuf, ApplicationError> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(ApplicationError::Conflict(
            "Yitang asset path must be a controlled relative path".to_owned(),
        ));
    }
    Ok(path.to_path_buf())
}

fn valid_native_component(value: &str, minimum: usize, maximum: usize) -> bool {
    (minimum..=maximum).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn asset_role(value: &str) -> Result<AssetRole, ApplicationError> {
    match value {
        "original" => Ok(AssetRole::Original),
        "export" => Ok(AssetRole::Export),
        "attachment" => Ok(AssetRole::Attachment),
        _ => Err(ApplicationError::Integrity(format!(
            "unsupported Yitang asset role: {value}"
        ))),
    }
}

fn hash_file(path: &Path) -> Result<(u64, Sha256), ApplicationError> {
    let mut file = File::open(path)
        .map_err(|error| ApplicationError::Asset(format!("{}: {error}", path.display())))?;
    let mut hasher = Sha256Hasher::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut byte_size = 0_u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| ApplicationError::Asset(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        byte_size +=
            u64::try_from(read).map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    }
    Ok((
        byte_size,
        Sha256::parse(format!("{:x}", hasher.finalize()))?,
    ))
}

fn content_fingerprint(item: &PreparedItem) -> Sha256 {
    let mut lines = vec![
        format!("replay_sid={}", item.replay_sid),
        format!("replay_url={}", item.replay_url),
        format!("document_acl_token={}", item.document_acl_token),
        format!("document_token={}", item.document_token),
        format!("document_url={}", item.document_url),
    ];
    lines.extend(item.assets.iter().map(|asset| {
        format!(
            "asset={}|{}|{}|{}",
            asset.relative_path, asset.role, asset.byte_size, asset.sha256
        )
    }));
    Sha256::of_bytes(lines.join("\n").as_bytes())
}

fn discovered_candidate(
    session_id: &CollectionSessionId,
    manifest_path: &Path,
    item: &PreparedItem,
) -> Result<DiscoveredCandidate, ApplicationError> {
    Ok(DiscoveredCandidate {
        summary: CandidateSummary {
            candidate_id: format!("yitang_{}_{}", item.replay_sid, item.document_token),
            session_id: session_id.clone(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_native_id: Some(item.stable_identity.clone()),
            title: Some(item.title.clone()),
            source_location: Some(item.replay_url.clone()),
            hierarchy: vec!["一堂".to_owned(), item.title.clone()],
            content_type: ContentType::Video,
            source_updated_at: None,
            attachment_available: Some(true),
            limitations: limitations(),
            selection_capabilities: vec![
                "single".to_owned(),
                "prepared_manifest".to_owned(),
                "paired_replay_document".to_owned(),
                "all_attachments_required".to_owned(),
            ],
            common_metadata: common_metadata(item),
        },
        prefetched: Some(envelope(manifest_path, item)?),
    })
}

fn found(
    manifest_path: &Path,
    item: &PreparedItem,
) -> Result<AcquisitionOutcome, ApplicationError> {
    let assets = item
        .assets
        .iter()
        .map(|asset| {
            Ok(CaptureImportAsset {
                path: asset.local_path.clone(),
                role: asset_role(&asset.role)?,
                expected_sha256: Some(Sha256::parse(asset.sha256.clone())?),
                selected_relative_path: Some(asset.relative_path.clone()),
                expected_byte_size: Some(asset.byte_size),
                ..CaptureImportAsset::default()
            })
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    Ok(AcquisitionOutcome::Found {
        candidate: Box::new(envelope(manifest_path, item)?),
        assets,
    })
}

fn envelope(
    manifest_path: &Path,
    item: &PreparedItem,
) -> Result<CandidateEnvelope, ApplicationError> {
    let root = manifest_path.parent().ok_or_else(|| {
        ApplicationError::Integrity("Yitang manifest has no acquisition root".to_owned())
    })?;
    let markdown = fs::read_to_string(root.join(&item.document.markdown_relative_path))
        .map_err(|error| ApplicationError::Asset(error.to_string()))?;
    let payload_sha256 = Sha256::of_bytes(markdown.as_bytes());
    let metadata = Metadata::parse(
        &serde_json::json!({
            "adapter_version": ADAPTER_VERSION,
            "prepared_manifest_path": manifest_path.to_string_lossy(),
            "content_fingerprint": item.content_fingerprint,
            "stable_identity": item.stable_identity,
            "replay_sid": item.replay_sid,
            "document_acl_token": item.document_acl_token,
            "document_token": item.document_token,
            "document_url": item.document_url,
            "begin_time": item.begin_time,
            "block_count": item.document.block_count,
            "heading_count": item.document.heading_count,
            "image_count": item.document.image_count,
            "asset_count": item.assets.len(),
        })
        .to_string(),
    )?;
    Ok(CandidateEnvelope {
        protocol_version: "1".to_owned(),
        route_id: SourceRouteId(ROUTE_ID.to_owned()),
        source_reference: item.replay_url.clone(),
        content_type: ContentType::Video,
        payload_sha256,
        metadata,
        payload: CandidatePayload::Text { text: markdown },
        context: Some(format!("一堂 / {}", item.title)),
        native_id: Some(item.stable_identity.clone()),
        common_metadata: common_metadata(item),
    })
}

fn common_metadata(item: &PreparedItem) -> CommonSourceMetadata {
    CommonSourceMetadata {
        title: Some(item.title.clone()),
        language: Some("zh-CN".to_owned()),
        hierarchy: vec![SourceHierarchyNode {
            kind: Some("series".to_owned()),
            name: "一堂".to_owned(),
            native_id: None,
            locator: Some("https://yitang.top/".to_owned()),
        }],
        context: Some(format!(
            "paired replay {} and document {}{}",
            item.replay_sid,
            item.document_token,
            item.subtitle
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(|value| format!(" / {value}"))
                .unwrap_or_default()
        )),
        limitations: limitations()
            .into_iter()
            .map(|detail| SourceLimitation {
                code: "prepared_manifest_boundary".to_owned(),
                detail,
            })
            .collect(),
        access_state: SourceAccessState::Accessible,
        media: SourceMediaMetadata {
            entries: vec![
                SourceMediaEntry {
                    kind: "video".to_owned(),
                    media_type: Some("video/mp4".to_owned()),
                    duration_ms: Some((item.duration_seconds_local * 1000.0).round() as u64),
                    width: None,
                    height: None,
                    page_count: None,
                },
                SourceMediaEntry {
                    kind: "document".to_owned(),
                    media_type: Some("text/markdown".to_owned()),
                    duration_ms: None,
                    width: None,
                    height: None,
                    page_count: None,
                },
                SourceMediaEntry {
                    kind: "image_collection".to_owned(),
                    media_type: Some("image/*".to_owned()),
                    duration_ms: None,
                    width: None,
                    height: None,
                    page_count: u32::try_from(item.document.image_count).ok(),
                },
            ],
            ..SourceMediaMetadata::default()
        },
        ..CommonSourceMetadata::default()
    }
}

fn limitations() -> Vec<String> {
    vec![
        "the signed-in browser was used only for one explicitly authorised replay/document pair"
            .to_owned(),
        "browser identity, credentials, discussion, reactions, and encrypted API bodies are excluded"
            .to_owned(),
        "document links are references and were not recursively collected".to_owned(),
    ]
}

fn metadata_string(metadata: &Metadata, field: &str) -> Result<String, ApplicationError> {
    serde_json::from_str::<serde_json::Value>(&metadata.to_json())
        .map_err(|_| ApplicationError::Integrity("Yitang metadata is invalid JSON".to_owned()))?
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApplicationError::Integrity(format!("Yitang metadata has no {field}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use babata_application::{
        CollectorSessionService, StartCollectionCommand, ports::RawRepositoryPort,
    };
    use babata_domain::{CollectionItemState, CollectionSelection, RecollectionState};
    use tempfile::tempdir;

    use crate::{
        FileAssetStore, SystemClock, open_collection_database,
        paths::{DataPaths, ensure_layout},
    };

    #[allow(clippy::too_many_lines)]
    fn fixture(root: &Path) -> PathBuf {
        fs::create_dir_all(root.join("media")).unwrap();
        fs::create_dir_all(root.join("document/images")).unwrap();
        for (relative, body) in [
            ("media/replay_1.mp4", b"video-audio".as_slice()),
            (
                "document/document.md",
                b"# Fixture title\nBody\n".as_slice(),
            ),
            ("document/rendered.txt", b"Fixture title\nBody\n".as_slice()),
            (
                "document/source-rendered.html",
                b"<h1>Fixture title</h1><p>Body</p>".as_slice(),
            ),
            ("document/images/001.png", b"image".as_slice()),
        ] {
            fs::write(root.join(relative), body).unwrap();
        }
        fs::write(
            root.join("document/blocks.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema": DOCUMENT_SCHEMA,
                "title": "Fixture title",
                "document_token": "document_1",
                "acl_token": "0123456789abcdef0123456789abcdef",
                "traversal": {"reached_bottom": true},
                "block_count": 2,
                "image_count": 1
            }))
            .unwrap(),
        )
        .unwrap();

        let replay_url = "https://air.yitang.top/live/replay_1";
        let document_url = "https://yitang.top/fs-doc/0123456789abcdef0123456789abcdef/document_1";
        let definitions = [
            ("media/replay_1.mp4", "original", "video/mp4", replay_url),
            (
                "document/document.md",
                "export",
                "text/markdown",
                document_url,
            ),
            (
                "document/blocks.json",
                "export",
                "application/json",
                document_url,
            ),
            (
                "document/rendered.txt",
                "export",
                "text/plain",
                document_url,
            ),
            (
                "document/source-rendered.html",
                "export",
                "text/html",
                document_url,
            ),
            (
                "document/images/001.png",
                "attachment",
                "image/png",
                "https://cdn.yitang.top/image/001.png",
            ),
        ];
        let assets = definitions
            .iter()
            .map(|(relative, role, media_type, source_url)| {
                let path = root.join(relative);
                let (byte_size, sha256) = hash_file(&path).unwrap();
                serde_json::json!({
                    "relative_path": relative,
                    "local_path": path,
                    "role": role,
                    "media_type": media_type,
                    "source_url": source_url,
                    "byte_size": byte_size,
                    "sha256": sha256
                })
            })
            .collect::<Vec<_>>();
        let mut item = serde_json::json!({
            "stable_identity": "replay:replay_1|document:document_1",
            "replay_sid": "replay_1",
            "replay_url": replay_url,
            "document_acl_token": "0123456789abcdef0123456789abcdef",
            "document_token": "document_1",
            "document_url": document_url,
            "title": "Fixture title",
            "subtitle": null,
            "begin_time": null,
            "hls_url": "https://cdn.yitang.top/replay/replay_1.m3u8",
            "audio_url": "https://cdn.yitang.top/replay/replay_1.mp3",
            "duration_seconds_source": 10.0,
            "duration_seconds_local": 10.0,
            "document": {
                "traversal_complete": true,
                "block_count": 2,
                "nonempty_block_count": 2,
                "heading_count": 1,
                "image_count": 1,
                "markdown_relative_path": "document/document.md",
                "blocks_relative_path": "document/blocks.json",
                "text_relative_path": "document/rendered.txt",
                "html_relative_path": "document/source-rendered.html"
            },
            "content_fingerprint": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "assets": assets,
            "readiness": "prepared"
        });
        let parsed: PreparedItem = serde_json::from_value(item.clone()).unwrap();
        item["content_fingerprint"] =
            serde_json::Value::String(content_fingerprint(&parsed).to_string());
        let manifest = serde_json::json!({
            "schema": MANIFEST_SCHEMA,
            "readiness": "prepared",
            "acquisition_root": root,
            "summary": {"source_items": 1, "assets": 6, "images": 1, "unresolved": 0},
            "items": [item]
        });
        let path = root.join("prepared-manifest.json");
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        path
    }

    #[test]
    fn discovers_composite_identity_and_requires_attachments() {
        let temp = tempdir().unwrap();
        let path = fixture(temp.path());
        let candidates = YitangPreparedManifestAdapter
            .discover(&CollectionSessionId::new(), &path.to_string_lossy())
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].summary.source_native_id.as_deref(),
            Some("replay:replay_1|document:document_1")
        );
        assert!(
            YitangPreparedManifestAdapter
                .collect(
                    &candidates[0].summary,
                    candidates[0].prefetched.as_ref(),
                    false,
                )
                .is_err()
        );
    }

    #[test]
    fn changed_missing_asset_and_changed_pairing_fail_closed() {
        let changed = tempdir().unwrap();
        let path = fixture(changed.path());
        fs::write(changed.path().join("document/images/001.png"), b"other").unwrap();
        assert!(read_manifest(&path.to_string_lossy()).is_err());

        let missing = tempdir().unwrap();
        let path = fixture(missing.path());
        fs::remove_file(missing.path().join("document/images/001.png")).unwrap();
        assert!(read_manifest(&path.to_string_lossy()).is_err());

        let pairing = tempdir().unwrap();
        let path = fixture(pairing.path());
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["items"][0]["document_url"] =
            serde_json::Value::String("https://yitang.top/fs-doc/wrong/pair".to_owned());
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(read_manifest(&path.to_string_lossy()).is_err());
    }

    #[test]
    fn collector_saves_all_assets_and_recollects_unchanged() {
        let source = tempdir().unwrap();
        let manifest_path = fixture(source.path());
        let data = tempdir().unwrap();
        let paths = DataPaths::new(data.path().to_path_buf());
        ensure_layout(&paths).unwrap();
        let repository = open_collection_database(&paths, 1_000).unwrap();
        let service = CollectorSessionService::new(
            repository.clone(),
            FileAssetStore::new(paths),
            SystemClock,
            vec![Box::new(YitangPreparedManifestAdapter)],
        );
        let session = service
            .start(StartCollectionCommand {
                route_id: SourceRouteId(ROUTE_ID.to_owned()),
                source_reference: manifest_path.to_string_lossy().into_owned(),
                scope_description: "one explicit Yitang pair".to_owned(),
                authorisation_id: "auth-yitang-fixture".to_owned(),
            })
            .unwrap();
        let candidates = service.candidates(&session.session_id).unwrap();
        let statuses = service
            .select(CollectionSelection {
                session_id: session.session_id.clone(),
                candidate_ids: vec![candidates[0].candidate_id.clone()],
                scope_description: "one explicit Yitang pair".to_owned(),
                confirmed: true,
                authorised_context: "auth-yitang-fixture".to_owned(),
                requested_attachments: true,
            })
            .unwrap();
        assert_eq!(statuses[0].state, CollectionItemState::Saved);
        let item_id = statuses[0].item_id.as_ref().unwrap();
        let detail = repository.load_detail(item_id).unwrap();
        assert_eq!(detail.provider, "yitang");
        assert_eq!(detail.assets.len(), 6);
        assert_eq!(detail.revisions.len(), 1);

        let recollection = service.recollect(item_id).unwrap();
        assert_eq!(recollection.state, RecollectionState::Unchanged);
        assert!(recollection.new_revision_id.is_none());
        assert_eq!(repository.load_detail(item_id).unwrap().revisions.len(), 1);
    }
}

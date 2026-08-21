use std::{collections::HashSet, fs, path::Path};

use babata_application::{
    AcquisitionOutcome, ApplicationError, CaptureImportAsset, DiscoveredCandidate,
    ports::SourceAdapterPort,
};
use babata_domain::{
    AssetRole, CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus,
    CollectionSessionId, CommonSourceMetadata, ContentType, Metadata, RouteCoverage, Sha256,
    SourceAccessState, SourceAuthor, SourceHierarchyNode, SourceLimitation, SourceMediaEntry,
    SourceMediaMetadata, SourceRouteDescriptor, SourceRouteId,
};
use serde::Deserialize;

const ROUTE_ID: &str = "source.youtube";
const MANIFEST_SCHEMA: &str = "babata.external-course-source-manifest/v1";
const ADAPTER_VERSION: &str = "youtube-prepared-cache/1";

#[derive(Debug, Clone, Default)]
pub struct YouTubePreparedCacheAdapter;

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(ROUTE_ID.to_owned()),
        provider: "youtube".to_owned(),
        status: CapabilityStatus::Enabled,
        activation_phase: "P7".to_owned(),
    }
}

#[derive(Debug, Deserialize)]
struct PreparedManifest {
    schema: String,
    readiness: String,
    summary: ManifestSummary,
    items: Vec<PreparedItem>,
}

#[derive(Debug, Deserialize)]
struct ManifestSummary {
    source_items: usize,
    local_mp4: usize,
    mapped: usize,
    unmapped_source: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct PreparedItem {
    course_slug: String,
    course_title: String,
    video_id: String,
    original_title: String,
    source_url: String,
    playlist_id: String,
    playlist_title: String,
    playlist_position_observed: u32,
    playlist_count_observed: u32,
    duration_seconds_source: Option<f64>,
    channel_id: Option<String>,
    channel_title: Option<String>,
    local_mapping: Mapping,
    local_media: LocalMedia,
    readiness: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Mapping {
    mapping_confidence: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LocalMedia {
    local_path: String,
    local_filename: String,
    size_bytes: u64,
    sha256: String,
    duration_seconds_local: f64,
    embedded_subtitle_streams: u32,
    streams: Vec<MediaStream>,
}

#[derive(Debug, Clone, Deserialize)]
struct MediaStream {
    codec_type: String,
    width: Option<u32>,
    height: Option<u32>,
}

impl SourceAdapterPort for YouTubePreparedCacheAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let manifest_path = canonical_manifest_path(source_reference)?;
        let items = read_manifest(&manifest_path)?;
        items
            .iter()
            .map(|item| discovered_candidate(session_id, &manifest_path, item))
            .collect()
    }

    fn collect(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&CandidateEnvelope>,
        _requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let expected = prefetched.ok_or_else(|| {
            ApplicationError::Integrity("YouTube candidate envelope is missing".to_owned())
        })?;
        let manifest_path = metadata_string(&expected.metadata, "prepared_manifest_path")?;
        let video_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("YouTube candidate has no video ID".to_owned())
        })?;
        let item = read_manifest(Path::new(&manifest_path))?
            .into_iter()
            .find(|item| item.video_id == video_id)
            .ok_or_else(|| ApplicationError::NotFound(format!("YouTube video ID: {video_id}")))?;
        found(&manifest_path, &item)
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
                "only explicitly authorized, prepared YouTube cache manifests are accepted"
                    .to_owned(),
                "playlist position is observed mutable metadata, not stable identity".to_owned(),
                "embedded subtitles are inventoried but excluded from transcript authority"
                    .to_owned(),
            ],
        }
    }
}

fn canonical_manifest_path(source_reference: &str) -> Result<std::path::PathBuf, ApplicationError> {
    let path = fs::canonicalize(source_reference).map_err(|error| {
        ApplicationError::Asset(format!(
            "unable to open YouTube cache manifest: {:?}",
            error.kind()
        ))
    })?;
    if !path.is_file() {
        return Err(ApplicationError::Conflict(
            "YouTube source reference must be a prepared manifest file".to_owned(),
        ));
    }
    Ok(path)
}

fn read_manifest(path: &Path) -> Result<Vec<PreparedItem>, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::Asset(format!(
            "unable to read YouTube cache manifest: {:?}",
            error.kind()
        ))
    })?;
    let manifest: PreparedManifest = serde_json::from_slice(&bytes).map_err(|_| {
        ApplicationError::Integrity("YouTube cache manifest is invalid JSON".to_owned())
    })?;
    if manifest.schema != MANIFEST_SCHEMA || manifest.readiness != "prepared" {
        return Err(ApplicationError::Conflict(
            "YouTube cache manifest has an unsupported schema or readiness".to_owned(),
        ));
    }
    if manifest.items.is_empty()
        || manifest.summary.source_items != manifest.items.len()
        || manifest.summary.local_mp4 != manifest.items.len()
        || manifest.summary.mapped != manifest.items.len()
        || manifest.summary.unmapped_source != 0
    {
        return Err(ApplicationError::Integrity(
            "YouTube cache manifest summary is incomplete or inconsistent".to_owned(),
        ));
    }
    let mut video_ids = HashSet::new();
    let mut local_paths = HashSet::new();
    for item in &manifest.items {
        validate_item(item)?;
        if !video_ids.insert(item.video_id.as_str()) {
            return Err(ApplicationError::Integrity(
                "YouTube cache manifest contains duplicate video IDs".to_owned(),
            ));
        }
        let local_path = fs::canonicalize(&item.local_media.local_path).map_err(|error| {
            ApplicationError::Asset(format!(
                "unable to inspect prepared YouTube media: {:?}",
                error.kind()
            ))
        })?;
        if !local_path.is_file() || !local_paths.insert(local_path.clone()) {
            return Err(ApplicationError::Integrity(
                "YouTube cache manifest contains a missing or duplicate local media path"
                    .to_owned(),
            ));
        }
        let metadata = fs::metadata(&local_path)
            .map_err(|error| ApplicationError::Asset(error.to_string()))?;
        if metadata.len() != item.local_media.size_bytes {
            return Err(ApplicationError::Integrity(
                "prepared YouTube media size no longer matches the manifest".to_owned(),
            ));
        }
    }
    Ok(manifest.items)
}

fn validate_item(item: &PreparedItem) -> Result<(), ApplicationError> {
    let valid_id = item.video_id.len() == 11
        && item
            .video_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
    let canonical_url = format!("https://www.youtube.com/watch?v={}", item.video_id);
    let has_video = item
        .local_media
        .streams
        .iter()
        .any(|stream| stream.codec_type == "video");
    if !valid_id
        || item.original_title.trim().is_empty()
        || item.playlist_id.trim().is_empty()
        || item.playlist_title.trim().is_empty()
        || item.course_slug.trim().is_empty()
        || item.course_title.trim().is_empty()
        || item.source_url != canonical_url
        || item.playlist_position_observed == 0
        || item.playlist_position_observed > item.playlist_count_observed
        || !matches!(
            item.local_mapping.mapping_confidence.as_str(),
            "exact" | "high"
        )
        || item.readiness != "prepared"
        || item.local_media.local_filename.trim().is_empty()
        || item.local_media.size_bytes == 0
        || item.local_media.duration_seconds_local <= 0.0
        || !has_video
    {
        return Err(ApplicationError::Integrity(
            "YouTube cache manifest contains an invalid or unresolved item".to_owned(),
        ));
    }
    Sha256::parse(item.local_media.sha256.clone())?;
    Ok(())
}

fn discovered_candidate(
    session_id: &CollectionSessionId,
    manifest_path: &Path,
    item: &PreparedItem,
) -> Result<DiscoveredCandidate, ApplicationError> {
    let envelope = envelope(manifest_path, item)?;
    Ok(DiscoveredCandidate {
        summary: CandidateSummary {
            candidate_id: format!("youtube_{}", item.video_id),
            session_id: session_id.clone(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_native_id: Some(item.video_id.clone()),
            title: Some(item.original_title.clone()),
            source_location: Some(item.source_url.clone()),
            hierarchy: vec![
                "YouTube".to_owned(),
                item.playlist_title.clone(),
                item.original_title.clone(),
            ],
            content_type: ContentType::Video,
            source_updated_at: None,
            attachment_available: Some(true),
            limitations: limitations(item),
            selection_capabilities: vec![
                "single".to_owned(),
                "explicit_playlist".to_owned(),
                "prepared_cache".to_owned(),
                "stable_video_id".to_owned(),
            ],
            common_metadata: common_metadata(item),
        },
        prefetched: Some(envelope),
    })
}

fn found(manifest_path: &str, item: &PreparedItem) -> Result<AcquisitionOutcome, ApplicationError> {
    if !Path::new(&item.local_media.local_path).is_file() {
        return Err(ApplicationError::NotFound(format!(
            "prepared YouTube media: {}",
            item.local_media.local_path
        )));
    }
    Ok(AcquisitionOutcome::Found {
        candidate: Box::new(envelope(Path::new(manifest_path), item)?),
        assets: vec![CaptureImportAsset {
            path: item.local_media.local_path.clone(),
            role: AssetRole::Original,
            expected_sha256: Some(Sha256::parse(item.local_media.sha256.clone())?),
            expected_byte_size: Some(item.local_media.size_bytes),
            ..CaptureImportAsset::default()
        }],
    })
}

fn envelope(
    manifest_path: &Path,
    item: &PreparedItem,
) -> Result<CandidateEnvelope, ApplicationError> {
    let payload = serde_json::to_string_pretty(&serde_json::json!({
        "schema": "babata.youtube-source-record/v1",
        "video_id": item.video_id,
        "original_title": item.original_title,
        "source_url": item.source_url,
        "playlist_id": item.playlist_id,
        "playlist_title": item.playlist_title,
        "playlist_position_observed": item.playlist_position_observed,
        "playlist_count_observed": item.playlist_count_observed,
        "duration_seconds_source": item.duration_seconds_source,
        "source_mp4_sha256": item.local_media.sha256,
    }))
    .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let payload_sha256 = Sha256::of_bytes(payload.as_bytes());
    let metadata = Metadata::parse(
        &serde_json::json!({
            "adapter_version": ADAPTER_VERSION,
            "prepared_manifest_path": manifest_path.to_string_lossy(),
            "content_fingerprint": payload_sha256.as_str(),
            "video_id": item.video_id,
            "playlist_id": item.playlist_id,
            "playlist_title": item.playlist_title,
            "playlist_position_observed": item.playlist_position_observed,
            "playlist_count_observed": item.playlist_count_observed,
            "mapping_confidence": item.local_mapping.mapping_confidence,
            "embedded_subtitle_streams": item.local_media.embedded_subtitle_streams,
        })
        .to_string(),
    )?;
    Ok(CandidateEnvelope {
        protocol_version: "1".to_owned(),
        route_id: SourceRouteId(ROUTE_ID.to_owned()),
        source_reference: item.source_url.clone(),
        content_type: ContentType::Video,
        payload_sha256,
        metadata,
        payload: CandidatePayload::Text { text: payload },
        context: Some(format!("YouTube / {}", item.playlist_title)),
        native_id: Some(item.video_id.clone()),
        common_metadata: common_metadata(item),
    })
}

fn common_metadata(item: &PreparedItem) -> CommonSourceMetadata {
    let video_stream = item
        .local_media
        .streams
        .iter()
        .find(|stream| stream.codec_type == "video");
    CommonSourceMetadata {
        title: Some(item.original_title.clone()),
        authors: item
            .channel_title
            .as_ref()
            .map(|title| SourceAuthor {
                display_name: title.clone(),
                native_id: item.channel_id.clone(),
                locator: item
                    .channel_id
                    .as_ref()
                    .map(|id| format!("https://www.youtube.com/channel/{id}")),
            })
            .into_iter()
            .collect(),
        language: Some("en".to_owned()),
        hierarchy: vec![SourceHierarchyNode {
            kind: Some("playlist".to_owned()),
            name: item.playlist_title.clone(),
            native_id: Some(item.playlist_id.clone()),
            locator: Some(format!(
                "https://www.youtube.com/playlist?list={}",
                item.playlist_id
            )),
        }],
        context: Some(format!(
            "{} / observed position {} of {}",
            item.course_title, item.playlist_position_observed, item.playlist_count_observed
        )),
        limitations: limitations(item)
            .into_iter()
            .map(|detail| SourceLimitation {
                code: "prepared_cache_boundary".to_owned(),
                detail,
            })
            .collect(),
        access_state: SourceAccessState::Accessible,
        media: SourceMediaMetadata {
            entries: vec![SourceMediaEntry {
                kind: "video".to_owned(),
                media_type: Some("video/mp4".to_owned()),
                duration_ms: Some((item.local_media.duration_seconds_local * 1000.0).round() as u64),
                width: video_stream.and_then(|stream| stream.width),
                height: video_stream.and_then(|stream| stream.height),
                page_count: None,
            }],
            ..SourceMediaMetadata::default()
        },
        ..CommonSourceMetadata::default()
    }
}

fn limitations(item: &PreparedItem) -> Vec<String> {
    let mut values = vec![
        "playlist position is an observed mutable field and is not the stable item identity"
            .to_owned(),
        "source was acquired before formal registration and admitted from a validated local cache manifest"
            .to_owned(),
    ];
    if item.local_media.embedded_subtitle_streams > 0 {
        values.push(format!(
            "{} embedded subtitle stream(s) were inventoried but are excluded from transcript authority",
            item.local_media.embedded_subtitle_streams
        ));
    }
    values
}

fn metadata_string(metadata: &Metadata, field: &str) -> Result<String, ApplicationError> {
    serde_json::from_str::<serde_json::Value>(&metadata.to_json())
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApplicationError::Integrity(format!("YouTube metadata has no {field}")))
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

    fn manifest(path: &Path, duplicate: bool, sha256: &str) {
        let media = path.parent().unwrap().join("video.mp4");
        fs::write(&media, b"video").unwrap();
        let item = serde_json::json!({
            "course_slug": "cpp",
            "course_title": "C++",
            "video_id": "18c3MTX0PK0",
            "original_title": "Welcome to C++",
            "source_url": "https://www.youtube.com/watch?v=18c3MTX0PK0",
            "playlist_id": "playlist",
            "playlist_title": "C++",
            "playlist_position_observed": 1,
            "playlist_count_observed": if duplicate { 2 } else { 1 },
            "duration_seconds_source": 5.0,
            "channel_id": "channel",
            "channel_title": "The Cherno",
            "local_mapping": {"mapping_confidence": "high"},
            "local_media": {
                "local_path": media,
                "local_filename": "video.mp4",
                "size_bytes": 5,
                "sha256": sha256,
                "duration_seconds_local": 5.0,
                "embedded_subtitle_streams": 0,
                "streams": [{"codec_type": "video", "width": 1920, "height": 1080}]
            },
            "readiness": "prepared"
        });
        let items = if duplicate {
            vec![item.clone(), item]
        } else {
            vec![item]
        };
        fs::write(
            path,
            serde_json::to_vec(&serde_json::json!({
                "schema": MANIFEST_SCHEMA,
                "readiness": "prepared",
                "summary": {
                    "source_items": items.len(),
                    "local_mp4": items.len(),
                    "mapped": items.len(),
                    "unmapped_source": 0
                },
                "items": items
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn prepared_manifest_discovers_video_identity() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("manifest.json");
        manifest(&path, false, Sha256::of_bytes(b"video").as_str());
        let candidates = YouTubePreparedCacheAdapter
            .discover(&CollectionSessionId::new(), &path.to_string_lossy())
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].summary.source_native_id.as_deref(),
            Some("18c3MTX0PK0")
        );
        assert_eq!(candidates[0].summary.content_type, ContentType::Video);
    }

    #[test]
    fn duplicate_video_id_is_rejected() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("manifest.json");
        manifest(&path, true, Sha256::of_bytes(b"video").as_str());
        assert!(read_manifest(&path).is_err());
    }

    #[test]
    fn malformed_sha256_is_rejected() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("manifest.json");
        manifest(&path, false, "not-a-sha256");
        assert!(read_manifest(&path).is_err());
    }

    #[test]
    fn collector_saves_hash_bound_video_and_recollects_unchanged() {
        let source = tempdir().unwrap();
        let manifest_path = source.path().join("manifest.json");
        manifest(&manifest_path, false, Sha256::of_bytes(b"video").as_str());
        let data = tempdir().unwrap();
        let paths = DataPaths::new(data.path().to_path_buf());
        ensure_layout(&paths).unwrap();
        let repository = open_collection_database(&paths, 1_000).unwrap();
        let service = CollectorSessionService::new(
            repository.clone(),
            FileAssetStore::new(paths.clone()),
            SystemClock,
            vec![Box::new(YouTubePreparedCacheAdapter)],
        );
        let session = service
            .start(StartCollectionCommand {
                route_id: SourceRouteId(ROUTE_ID.to_owned()),
                source_reference: manifest_path.to_string_lossy().into_owned(),
                scope_description: "one explicit YouTube video".to_owned(),
                authorisation_id: "auth-youtube-fixture".to_owned(),
            })
            .unwrap();
        let candidates = service.candidates(&session.session_id).unwrap();
        let statuses = service
            .select(CollectionSelection {
                session_id: session.session_id.clone(),
                candidate_ids: vec![candidates[0].candidate_id.clone()],
                scope_description: "one explicit YouTube video".to_owned(),
                confirmed: true,
                authorised_context: "auth-youtube-fixture".to_owned(),
                requested_attachments: true,
            })
            .unwrap();
        assert_eq!(statuses[0].state, CollectionItemState::Saved);
        let item_id = statuses[0].item_id.as_ref().unwrap();
        let detail = repository.load_detail(item_id).unwrap();
        assert_eq!(detail.provider, "youtube");
        assert_eq!(detail.source_native_id.as_deref(), Some("18c3MTX0PK0"));
        assert_eq!(
            detail.common_metadata.title.as_deref(),
            Some("Welcome to C++")
        );
        assert_eq!(detail.assets.len(), 1);
        assert_eq!(
            detail.assets[0].sha256.as_deref(),
            Some(Sha256::of_bytes(b"video").as_str())
        );
        assert!(
            detail.assets[0]
                .logical_path
                .starts_with("01_raw/assets/sha256/")
        );

        let recollection = service.recollect(item_id).unwrap();
        assert_eq!(recollection.state, RecollectionState::Unchanged);
        assert!(recollection.new_revision_id.is_none());
    }
}

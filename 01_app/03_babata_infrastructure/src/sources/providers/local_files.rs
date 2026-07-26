use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use babata_application::{
    AcquisitionOutcome, ApplicationError, CaptureImportAsset, DiscoveredCandidate,
    ports::SourceAdapterPort,
};
use babata_domain::{
    AssetIntegrityMethod, AssetRole, CandidateEnvelope, CandidatePayload, CandidateSummary,
    CapabilityStatus, CollectionSessionId, CommonSourceMetadata, ContentType, Metadata,
    RouteCoverage, Sha256, SourceAccessState, SourceHierarchyNode, SourceRouteDescriptor,
    SourceRouteId,
};
use serde::{Deserialize, Serialize};

const ROUTE_ID: &str = "source.local_files";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalFileStrategy {
    FullSha256,
    #[default]
    OpaqueCopy,
}

impl std::str::FromStr for LocalFileStrategy {
    type Err = ApplicationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "full_sha256" | "copy_and_sha256" => Ok(Self::FullSha256),
            "opaque_copy" => Ok(Self::OpaqueCopy),
            _ => Err(ApplicationError::Conflict(format!(
                "unknown local file strategy: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalFilesAdapter {
    strategy: LocalFileStrategy,
}

impl Default for LocalFilesAdapter {
    fn default() -> Self {
        Self::new(LocalFileStrategy::default())
    }
}

impl LocalFilesAdapter {
    pub fn new(strategy: LocalFileStrategy) -> Self {
        Self { strategy }
    }
}

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(ROUTE_ID.to_owned()),
        provider: "local_files".to_owned(),
        status: CapabilityStatus::Enabled,
        activation_phase: "P7".to_owned(),
    }
}

#[derive(Debug, Clone)]
struct InventoryFile {
    path: PathBuf,
    relative_path: String,
    byte_size: u64,
    modified_unix_nanos: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InventoryCounts {
    inventoried_files: usize,
    inventoried_bytes: u64,
}

#[derive(Debug)]
struct BatchInventory {
    root: PathBuf,
    files: Vec<InventoryFile>,
    counts: InventoryCounts,
}

impl BatchInventory {
    fn read(root: &Path) -> Result<Self, ApplicationError> {
        let root = fs::canonicalize(root).map_err(asset_io("canonicalize local batch"))?;
        if !root.is_dir() {
            return Err(ApplicationError::Conflict(
                "local file source must be a directory".to_owned(),
            ));
        }
        let mut paths = Vec::new();
        inventory_paths(&root, &root, &mut paths)?;
        paths.sort_by(|left, right| left.1.cmp(&right.1));
        if paths.is_empty() {
            return Err(ApplicationError::Conflict(
                "local file source contains no regular files".to_owned(),
            ));
        }
        let files = paths
            .into_iter()
            .map(|(path, relative_path, metadata)| InventoryFile {
                path,
                relative_path,
                byte_size: metadata.len(),
                modified_unix_nanos: modified_unix_nanos(&metadata),
            })
            .collect::<Vec<_>>();
        let counts = InventoryCounts {
            inventoried_files: files.len(),
            inventoried_bytes: files.iter().map(|file| file.byte_size).sum(),
        };
        Ok(Self {
            root,
            files,
            counts,
        })
    }

    fn integrity_method(strategy: LocalFileStrategy) -> AssetIntegrityMethod {
        match strategy {
            LocalFileStrategy::FullSha256 => AssetIntegrityMethod::Sha256V1,
            LocalFileStrategy::OpaqueCopy => AssetIntegrityMethod::SizeSnapshotV1,
        }
    }

    fn envelope(
        &self,
        file: &InventoryFile,
        strategy: LocalFileStrategy,
    ) -> Result<CandidateEnvelope, ApplicationError> {
        let integrity_method = Self::integrity_method(strategy);
        let fingerprint = source_fingerprint(file);
        let payload = serde_json::to_string(&serde_json::json!({
            "schema": "local_file_item_v1",
            "batch_root": self.root.to_string_lossy(),
            "relative_path": file.relative_path,
            "byte_size": file.byte_size,
            "modified_unix_nanos": file.modified_unix_nanos.map(|value| value.to_string()),
            "integrity_method": integrity_method,
        }))
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        let metadata = Metadata::parse(
            &serde_json::json!({
                "adapter_version": "local-files-batch/1",
                "strategy": strategy,
                "integrity_method": integrity_method,
                "content_fingerprint": fingerprint,
                "batch_root": self.root.to_string_lossy(),
                "selected_relative_path": file.relative_path,
                "byte_size": file.byte_size,
                "modified_unix_nanos": file.modified_unix_nanos.map(|value| value.to_string()),
                "batch_counts": self.counts,
            })
            .to_string(),
        )?;
        let content_type = content_type(&file.path);
        Ok(CandidateEnvelope {
            protocol_version: "1".to_owned(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_reference: file.path.to_string_lossy().into_owned(),
            content_type,
            payload_sha256: Sha256::of_bytes(payload.as_bytes()),
            metadata,
            payload: CandidatePayload::Text { text: payload },
            context: Some(self.root.to_string_lossy().into_owned()),
            native_id: Some(file.relative_path.clone()),
            common_metadata: CommonSourceMetadata {
                title: file
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned()),
                hierarchy: hierarchy(&file.relative_path),
                context: Some(format!("local batch: {}", self.root.to_string_lossy())),
                access_state: SourceAccessState::Accessible,
                ..CommonSourceMetadata::default()
            },
        })
    }
}

impl SourceAdapterPort for LocalFilesAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let inventory = BatchInventory::read(Path::new(source_reference))?;
        inventory
            .files
            .iter()
            .map(|file| {
                let envelope = inventory.envelope(file, self.strategy)?;
                let summary = summary(session_id, &inventory, file, &envelope, self.strategy);
                Ok(DiscoveredCandidate {
                    summary,
                    prefetched: Some(envelope),
                })
            })
            .collect()
    }

    fn collect(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&CandidateEnvelope>,
        _requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let expected = prefetched.ok_or_else(|| {
            ApplicationError::Integrity("local file candidate envelope is missing".to_owned())
        })?;
        let strategy = strategy_from_metadata(&expected.metadata)?;
        let current = current_envelope(candidate, expected, strategy)?;
        if source_fingerprint_from_metadata(&expected.metadata)?
            != source_fingerprint_from_metadata(&current.metadata)?
        {
            return Err(ApplicationError::Conflict(
                "source file changed after batch inventory".to_owned(),
            ));
        }
        found(current)
    }

    fn collect_recollection(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&CandidateEnvelope>,
        _requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let expected = prefetched.ok_or_else(|| {
            ApplicationError::Integrity("local file candidate envelope is missing".to_owned())
        })?;
        let strategy = strategy_from_metadata(&expected.metadata)?;
        match current_envelope(candidate, expected, strategy) {
            Ok(envelope) => found(envelope),
            Err(ApplicationError::NotFound(reason)) => Ok(AcquisitionOutcome::Removed { reason }),
            Err(error) => Err(error),
        }
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: true,
            limitations: vec![
                "size_snapshot_v1 proves the selected size/source snapshot, not byte identity"
                    .to_owned(),
            ],
        }
    }
}

fn found(envelope: CandidateEnvelope) -> Result<AcquisitionOutcome, ApplicationError> {
    let value: serde_json::Value = serde_json::from_str(&envelope.metadata.to_json())
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let integrity_method: AssetIntegrityMethod =
        serde_json::from_value(value.get("integrity_method").cloned().ok_or_else(|| {
            ApplicationError::Integrity("integrity method is missing".to_owned())
        })?)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let byte_size = value.get("byte_size").and_then(serde_json::Value::as_u64);
    let modified = value
        .get("modified_unix_nanos")
        .and_then(serde_json::Value::as_str)
        .map(str::parse::<u128>)
        .transpose()
        .map_err(|_| ApplicationError::Integrity("modified timestamp is invalid".to_owned()))?;
    let relative = value
        .get("selected_relative_path")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let path = envelope.source_reference.clone();
    Ok(AcquisitionOutcome::Found {
        candidate: Box::new(envelope),
        assets: vec![CaptureImportAsset {
            path,
            role: AssetRole::Original,
            expected_sha256: None,
            integrity_method,
            selected_relative_path: relative,
            expected_byte_size: byte_size,
            expected_modified_unix_nanos: modified,
        }],
    })
}

fn current_envelope(
    candidate: &CandidateSummary,
    expected: &CandidateEnvelope,
    strategy: LocalFileStrategy,
) -> Result<CandidateEnvelope, ApplicationError> {
    let path = candidate.source_location.as_deref().ok_or_else(|| {
        ApplicationError::Integrity("local file candidate has no source path".to_owned())
    })?;
    if !Path::new(path).is_file() {
        return Err(ApplicationError::NotFound(format!(
            "local source file is no longer present: {path}"
        )));
    }
    let root = candidate
        .common_metadata
        .context
        .as_deref()
        .and_then(|context| context.strip_prefix("local batch: "))
        .ok_or_else(|| ApplicationError::Integrity("local batch root is missing".to_owned()))?;
    let relative = candidate
        .source_native_id
        .as_deref()
        .ok_or_else(|| ApplicationError::Integrity("local relative path is missing".to_owned()))?;
    let metadata = fs::metadata(path).map_err(asset_io("inspect local file"))?;
    if !metadata.is_file() {
        return Err(ApplicationError::NotFound(format!(
            "local source file is no longer present: {path}"
        )));
    }
    let expected_metadata: serde_json::Value =
        serde_json::from_str(&expected.metadata.to_json())
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let counts: InventoryCounts = serde_json::from_value(
        expected_metadata
            .get("batch_counts")
            .cloned()
            .ok_or_else(|| ApplicationError::Integrity("batch counts are missing".to_owned()))?,
    )
    .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let file = InventoryFile {
        path: PathBuf::from(path),
        relative_path: relative.to_owned(),
        byte_size: metadata.len(),
        modified_unix_nanos: modified_unix_nanos(&metadata),
    };
    BatchInventory {
        root: PathBuf::from(root),
        files: vec![file.clone()],
        counts,
    }
    .envelope(&file, strategy)
}

fn summary(
    session_id: &CollectionSessionId,
    inventory: &BatchInventory,
    file: &InventoryFile,
    envelope: &CandidateEnvelope,
    strategy: LocalFileStrategy,
) -> CandidateSummary {
    CandidateSummary {
        candidate_id: format!(
            "local_{}",
            &Sha256::of_bytes(file.relative_path.as_bytes()).as_str()[..24]
        ),
        session_id: session_id.clone(),
        route_id: SourceRouteId(ROUTE_ID.to_owned()),
        source_native_id: Some(file.relative_path.clone()),
        title: file
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
        source_location: Some(file.path.to_string_lossy().into_owned()),
        hierarchy: hierarchy(&file.relative_path)
            .into_iter()
            .map(|node| node.name)
            .collect(),
        content_type: envelope.content_type,
        source_updated_at: None,
        attachment_available: Some(true),
        limitations: Vec::new(),
        selection_capabilities: vec![
            "single".to_owned(),
            "explicit_batch".to_owned(),
            format!("strategy:{}", strategy_name(strategy)),
            format!("batch_files:{}", inventory.counts.inventoried_files),
            format!(
                "opaque_copy_files:{}",
                if strategy == LocalFileStrategy::OpaqueCopy {
                    inventory.counts.inventoried_files
                } else {
                    0
                }
            ),
            format!(
                "sha256_files:{}",
                if strategy == LocalFileStrategy::FullSha256 {
                    inventory.counts.inventoried_files
                } else {
                    0
                }
            ),
            format!("inventoried_bytes:{}", inventory.counts.inventoried_bytes),
        ],
        common_metadata: envelope.common_metadata.clone(),
    }
}

fn inventory_paths(
    root: &Path,
    directory: &Path,
    output: &mut Vec<(PathBuf, String, fs::Metadata)>,
) -> Result<(), ApplicationError> {
    let mut entries = fs::read_dir(directory)
        .map_err(asset_io("read local batch directory"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(asset_io("read local batch entry"))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(asset_io("inspect local batch entry"))?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            inventory_paths(root, &path, output)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| ApplicationError::Integrity("local path escaped batch".to_owned()))?
                .to_string_lossy()
                .replace('\\', "/");
            let metadata = entry.metadata().map_err(asset_io("inspect local file"))?;
            output.push((path, relative, metadata));
        }
    }
    Ok(())
}

fn modified_unix_nanos(metadata: &fs::Metadata) -> Option<u128> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos())
}

fn source_fingerprint(file: &InventoryFile) -> String {
    Sha256::of_bytes(
        format!(
            "{}:{}:{}",
            file.relative_path,
            file.byte_size,
            file.modified_unix_nanos
                .map_or_else(|| "unknown".to_owned(), |value| value.to_string())
        )
        .as_bytes(),
    )
    .to_string()
}

fn source_fingerprint_from_metadata(metadata: &Metadata) -> Result<String, ApplicationError> {
    serde_json::from_str::<serde_json::Value>(&metadata.to_json())
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?
        .get("content_fingerprint")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ApplicationError::Integrity("source fingerprint is missing".to_owned()))
}

fn strategy_from_metadata(metadata: &Metadata) -> Result<LocalFileStrategy, ApplicationError> {
    let value: serde_json::Value = serde_json::from_str(&metadata.to_json())
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    value
        .get("strategy")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ApplicationError::Integrity("local file strategy is missing".to_owned()))?
        .parse()
}

fn hierarchy(relative: &str) -> Vec<SourceHierarchyNode> {
    Path::new(relative)
        .components()
        .map(|component| SourceHierarchyNode {
            kind: Some("local_path".to_owned()),
            name: component.as_os_str().to_string_lossy().into_owned(),
            native_id: None,
            locator: None,
        })
        .collect()
}

fn content_type(path: &Path) -> ContentType {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "txt" | "md" | "csv" | "json" | "xml" | "html" => ContentType::Document,
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" => ContentType::Image,
        "mp3" | "wav" | "m4a" | "flac" | "ogg" => ContentType::Audio,
        "mp4" | "mkv" | "mov" | "avi" | "webm" => ContentType::Video,
        "zip" | "7z" | "rar" | "tar" | "gz" => ContentType::Archive,
        _ => ContentType::Unknown,
    }
}

fn strategy_name(strategy: LocalFileStrategy) -> &'static str {
    match strategy {
        LocalFileStrategy::FullSha256 => "full_sha256",
        LocalFileStrategy::OpaqueCopy => "opaque_copy",
    }
}

fn asset_io(context: &'static str) -> impl FnOnce(std::io::Error) -> ApplicationError {
    move |error| ApplicationError::Asset(format!("{context}: {:?}", error.kind()))
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

    #[test]
    fn opaque_copy_never_hashes_or_deduplicates_same_size_files() {
        let temporary = tempdir().unwrap();
        fs::write(temporary.path().join("unique.txt"), b"u").unwrap();
        fs::write(temporary.path().join("same-a.txt"), b"aa").unwrap();
        fs::write(temporary.path().join("same-b.txt"), b"bb").unwrap();
        let adapter = LocalFilesAdapter::default();
        let candidates = adapter
            .discover(
                &CollectionSessionId::new(),
                &temporary.path().to_string_lossy(),
            )
            .unwrap();
        assert_eq!(candidates.len(), 3);
        for candidate in candidates {
            let asset = match adapter
                .collect(&candidate.summary, candidate.prefetched.as_ref(), true)
                .unwrap()
            {
                AcquisitionOutcome::Found { assets, .. } => assets.into_iter().next().unwrap(),
                _ => panic!("expected found"),
            };
            assert_eq!(asset.integrity_method, AssetIntegrityMethod::SizeSnapshotV1);
        }
    }

    #[test]
    fn full_sha256_marks_every_file_for_streaming_hash() {
        let temporary = tempdir().unwrap();
        fs::write(temporary.path().join("one.txt"), b"1").unwrap();
        fs::write(temporary.path().join("two.txt"), b"22").unwrap();
        let adapter = LocalFilesAdapter::new(LocalFileStrategy::FullSha256);
        let candidates = adapter
            .discover(
                &CollectionSessionId::new(),
                &temporary.path().to_string_lossy(),
            )
            .unwrap();
        for candidate in candidates {
            let asset = match adapter
                .collect(&candidate.summary, candidate.prefetched.as_ref(), true)
                .unwrap()
            {
                AcquisitionOutcome::Found { assets, .. } => assets.into_iter().next().unwrap(),
                _ => panic!("expected found"),
            };
            assert_eq!(asset.integrity_method, AssetIntegrityMethod::Sha256V1);
        }
    }

    #[test]
    fn collector_persists_independent_opaque_copies_and_recollects_unchanged() {
        let source = tempdir().unwrap();
        fs::write(source.path().join("unique.txt"), b"u").unwrap();
        fs::write(source.path().join("equal-a.txt"), b"same").unwrap();
        fs::write(source.path().join("equal-b.txt"), b"same").unwrap();
        fs::write(source.path().join("unequal.txt"), b"else").unwrap();
        let data = tempdir().unwrap();
        let paths = DataPaths::new(data.path().to_path_buf());
        ensure_layout(&paths).unwrap();
        let repository = open_collection_database(&paths, 1_000).unwrap();
        let service = CollectorSessionService::new(
            repository.clone(),
            FileAssetStore::new(paths.clone()),
            SystemClock,
            vec![Box::new(LocalFilesAdapter::default())],
        );
        let session = service
            .start(StartCollectionCommand {
                route_id: SourceRouteId(ROUTE_ID.to_owned()),
                source_reference: source.path().to_string_lossy().into_owned(),
                scope_description: "explicit local batch".to_owned(),
                authorisation_id: "auth-local".to_owned(),
            })
            .unwrap();
        let candidates = service.candidates(&session.session_id).unwrap();
        let statuses = service
            .select(CollectionSelection {
                session_id: session.session_id.clone(),
                candidate_ids: candidates
                    .iter()
                    .map(|candidate| candidate.candidate_id.clone())
                    .collect(),
                scope_description: "explicit local batch".to_owned(),
                confirmed: true,
                authorised_context: "auth-local".to_owned(),
                requested_attachments: true,
            })
            .unwrap();
        assert!(
            statuses
                .iter()
                .all(|status| status.state == CollectionItemState::Saved),
            "{statuses:#?}"
        );

        let mut logical_paths = Vec::new();
        for status in &statuses {
            let detail = repository
                .load_detail(status.item_id.as_ref().unwrap())
                .unwrap();
            let asset = &detail.assets[0];
            logical_paths.push(asset.logical_path.clone());
            assert_eq!(asset.integrity_method, AssetIntegrityMethod::SizeSnapshotV1);
            assert!(asset.sha256.is_none());
            assert!(asset.logical_path.starts_with("01_raw/assets/opaque/"));
        }
        logical_paths.sort();
        logical_paths.dedup();
        assert_eq!(
            logical_paths.len(),
            4,
            "urgent intake must retain every selected file independently"
        );

        let outcome = service
            .recollect(statuses[0].item_id.as_ref().unwrap())
            .unwrap();
        assert_eq!(outcome.state, RecollectionState::Unchanged);
        assert!(outcome.new_revision_id.is_none());
    }
}

use babata_domain::{BackupClass, Sha256, SnapshotId, UtcTimestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifestEntry {
    pub relative_path: String,
    pub sha256: Sha256,
    pub byte_size: u64,
    pub class: BackupClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub schema_version: u32,
    pub snapshot_id: SnapshotId,
    pub created_at: UtcTimestamp,
    pub entries: Vec<SnapshotManifestEntry>,
}

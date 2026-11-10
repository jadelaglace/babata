use babata_domain::{BackupClass, Sha256, SnapshotId, UtcTimestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifestEntry {
    #[serde(default = "data_alias")]
    pub root_alias: String,
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
    #[serde(default)]
    pub roots: Vec<SnapshotRoot>,
    #[serde(default)]
    pub exclusions: Vec<SnapshotExclusion>,
    #[serde(default)]
    pub finished_at: Option<UtcTimestamp>,
    pub entries: Vec<SnapshotManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRoot {
    pub alias: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotExclusion {
    pub root_alias: String,
    pub relative_path: String,
    pub reason: String,
}

fn data_alias() -> String {
    "data".to_owned()
}

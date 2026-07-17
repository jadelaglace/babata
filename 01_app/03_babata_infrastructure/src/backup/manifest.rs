use babata_domain::{BackupClass, Sha256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifestEntry {
    pub relative_path: String,
    pub sha256: Sha256,
    pub class: BackupClass,
}

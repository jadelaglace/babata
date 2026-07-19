use babata_domain::{AssetId, AssetRole, LogicalPath, Sha256};

use crate::ApplicationError;

#[derive(Debug, Clone)]
pub struct StagedAsset {
    pub asset_id: AssetId,
    pub role: AssetRole,
    pub staging_key: String,
    pub logical_path: LogicalPath,
    pub sha256: Sha256,
    pub byte_size: u64,
    pub media_type: Option<String>,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizeAssetOutcome {
    Created,
    Reused,
}

pub trait AssetStorePort {
    fn begin_operation(&self, operation_id: &str) -> Result<(), ApplicationError>;
    fn preserve_operation(
        &self,
        operation_id: &str,
        revision_id: &str,
        failure_code: &str,
    ) -> Result<(), ApplicationError>;
    fn complete_operation(&self, operation_id: &str) -> Result<(), ApplicationError>;
    fn stage(
        &self,
        source: &str,
        role: AssetRole,
        operation_id: &str,
    ) -> Result<StagedAsset, ApplicationError>;
    fn hash(&self, source: &str) -> Result<Sha256, ApplicationError>;
    fn finalize(&self, asset: &StagedAsset) -> Result<FinalizeAssetOutcome, ApplicationError>;
    fn discard_stage(&self, asset: &StagedAsset) -> Result<(), ApplicationError>;
    fn open(&self, logical_path: &LogicalPath) -> Result<Vec<u8>, ApplicationError>;
    fn verify(&self, asset: &StagedAsset) -> Result<bool, ApplicationError>;
    /// Hash the finalized bytes behind a logical path under the data root.
    fn hash_logical(&self, logical_path: &LogicalPath) -> Result<Sha256, ApplicationError>;
    /// Copy an external file into managed C1 storage and return its logical path.
    fn import_derived_file(&self, source: &str) -> Result<(LogicalPath, Sha256), ApplicationError>;
    fn quarantine_finalized(
        &self,
        asset: &StagedAsset,
        operation_id: &str,
        outcome: FinalizeAssetOutcome,
    ) -> Result<(), ApplicationError>;
}

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
    fn quarantine_finalized(
        &self,
        asset: &StagedAsset,
        operation_id: &str,
        outcome: FinalizeAssetOutcome,
    ) -> Result<(), ApplicationError>;
}

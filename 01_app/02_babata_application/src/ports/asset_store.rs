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
    /// Stage an external derivative under a recoverable operation. Final bytes
    /// use a content-addressed path in managed C1 storage.
    fn stage_derived_file(
        &self,
        source: &str,
        operation_id: &str,
    ) -> Result<StagedAsset, ApplicationError> {
        let mut staged = self.stage(source, AssetRole::Derived, operation_id)?;
        staged.logical_path = LogicalPath::parse(format!(
            "02_derived/files/sha256/{}/{}",
            &staged.sha256.as_str()[..2],
            staged.sha256
        ))
        .map_err(ApplicationError::from)?;
        Ok(staged)
    }
    fn quarantine_finalized(
        &self,
        asset: &StagedAsset,
        operation_id: &str,
        outcome: FinalizeAssetOutcome,
    ) -> Result<(), ApplicationError>;
}

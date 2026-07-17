use babata_domain::SnapshotRef;

use crate::{ApplicationError, BackupOutcome, OperationStatus};

#[derive(Debug, Default, Clone, Copy)]
pub struct OpsService;

impl OpsService {
    pub fn status(&self) -> Result<OperationStatus, ApplicationError> {
        unavailable()
    }

    pub fn doctor(&self) -> Result<OperationStatus, ApplicationError> {
        unavailable()
    }

    pub fn backup(&self) -> Result<BackupOutcome, ApplicationError> {
        unavailable()
    }

    pub fn restore_verify(
        &self,
        _snapshot: &SnapshotRef,
    ) -> Result<OperationStatus, ApplicationError> {
        unavailable()
    }
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable("ops.backup", "P8"))
}

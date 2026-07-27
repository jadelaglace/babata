use babata_domain::SnapshotId;

use crate::{ApplicationError, BackupOutcome, OperationStatus, RestoreVerificationOutcome};

pub trait BackupDriverPort {
    fn status(&self) -> Result<OperationStatus, ApplicationError>;
    fn doctor(&self) -> Result<OperationStatus, ApplicationError>;
    fn snapshot(&self) -> Result<BackupOutcome, ApplicationError>;
    fn restore_verify(
        &self,
        snapshot: &SnapshotId,
        target: Option<&str>,
    ) -> Result<RestoreVerificationOutcome, ApplicationError>;
    fn verify_restored(
        &self,
        snapshot: &SnapshotId,
        target: &str,
    ) -> Result<RestoreVerificationOutcome, ApplicationError>;
}

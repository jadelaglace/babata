use babata_domain::SnapshotId;

use crate::{
    ApplicationError, BackupOutcome, OperationStatus, RestoreVerificationOutcome,
    ports::BackupDriverPort,
};

pub struct OpsService<D> {
    driver: D,
}

impl<D> OpsService<D>
where
    D: BackupDriverPort,
{
    pub fn new(driver: D) -> Self {
        Self { driver }
    }

    pub fn status(&self) -> Result<OperationStatus, ApplicationError> {
        self.driver.status()
    }

    pub fn doctor(&self) -> Result<OperationStatus, ApplicationError> {
        self.driver.doctor()
    }

    pub fn backup(&self) -> Result<BackupOutcome, ApplicationError> {
        self.driver.snapshot()
    }

    pub fn restore_verify(
        &self,
        snapshot: &SnapshotId,
        target: Option<&str>,
    ) -> Result<RestoreVerificationOutcome, ApplicationError> {
        self.driver.restore_verify(snapshot, target)
    }

    pub fn verify_restored(
        &self,
        snapshot: &SnapshotId,
        target: &str,
    ) -> Result<RestoreVerificationOutcome, ApplicationError> {
        self.driver.verify_restored(snapshot, target)
    }
}

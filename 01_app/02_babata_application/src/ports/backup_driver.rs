use babata_domain::SnapshotRef;

use crate::ApplicationError;

pub trait BackupDriverPort {
    fn snapshot(&self) -> Result<SnapshotRef, ApplicationError>;
    fn restore(&self, snapshot: &SnapshotRef, target: &str) -> Result<(), ApplicationError>;
    fn verify(&self, snapshot: &SnapshotRef) -> Result<bool, ApplicationError>;
}

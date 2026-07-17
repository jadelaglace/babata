use babata_application::ApplicationError;

#[derive(Debug, Clone, Default)]
pub struct SqliteSnapshotConfig {
    pub enabled: bool,
}

pub fn unavailable() -> ApplicationError {
    ApplicationError::capability_unavailable("backup.sqlite_snapshot", "P8")
}

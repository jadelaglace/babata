use babata_application::ApplicationError;

#[derive(Debug, Clone, Default)]
pub struct ResticConfig {
    pub enabled: bool,
    pub repository: Option<String>,
}

pub fn unavailable() -> ApplicationError {
    ApplicationError::capability_unavailable("backup.restic", "P8")
}

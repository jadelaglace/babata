use babata_application::ApplicationError;

#[derive(Debug, Clone, Default)]
pub struct LocalExtractConfig {
    pub enabled: bool,
}

pub fn unavailable() -> ApplicationError {
    ApplicationError::capability_unavailable("processing.local_extract", "P5")
}

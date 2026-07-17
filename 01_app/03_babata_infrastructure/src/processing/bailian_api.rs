use babata_application::ApplicationError;

#[derive(Debug, Clone, Default)]
pub struct BailianApiConfig {
    pub enabled: bool,
    pub endpoint: Option<String>,
}

pub fn unavailable() -> ApplicationError {
    ApplicationError::capability_unavailable("processing.bailian_api", "P5+")
}

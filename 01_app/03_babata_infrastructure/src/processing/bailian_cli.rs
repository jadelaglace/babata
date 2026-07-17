use babata_application::ApplicationError;

#[derive(Debug, Clone, Default)]
pub struct BailianCliConfig {
    pub enabled: bool,
    pub executable: Option<String>,
}

pub fn unavailable() -> ApplicationError {
    ApplicationError::capability_unavailable("processing.bailian_cli", "P5")
}

use babata_application::ApplicationError;

#[derive(Debug, Clone, Default)]
pub struct YtDlpConfig {
    pub enabled: bool,
    pub executable: Option<String>,
}

pub fn unavailable() -> ApplicationError {
    ApplicationError::capability_unavailable("tools.yt_dlp", "P4/P7")
}

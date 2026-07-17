use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("local API is disabled")]
    Disabled,
    #[error("missing or invalid installation token")]
    Unauthorized,
    #[error(transparent)]
    Application(#[from] babata_application::ApplicationError),
}

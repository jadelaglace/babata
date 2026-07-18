use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("local API is disabled")]
    Disabled,
    #[error("missing or invalid installation token")]
    Unauthorized,
    #[error("browser origin is not allowed")]
    OriginForbidden,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("request payload exceeds the local API limit")]
    PayloadTooLarge,
    #[error("local API I/O failure: {0}")]
    Io(String),
    #[error(transparent)]
    Application(#[from] babata_application::ApplicationError),
}

impl ApiError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Disabled => "capability_unavailable",
            Self::Unauthorized => "unauthorized",
            Self::OriginForbidden => "origin_forbidden",
            Self::InvalidRequest(_) => "invalid_request",
            Self::PayloadTooLarge => "payload_too_large",
            Self::Io(_) => "io_failed",
            Self::Application(error) => error.code(),
        }
    }

    pub fn status_code(&self) -> u16 {
        match self {
            Self::Unauthorized => 401,
            Self::OriginForbidden => 403,
            Self::InvalidRequest(_)
            | Self::Application(babata_application::ApplicationError::Domain(_)) => 400,
            Self::PayloadTooLarge => 413,
            Self::Disabled
            | Self::Application(babata_application::ApplicationError::CapabilityUnavailable {
                ..
            }) => 503,
            Self::Application(babata_application::ApplicationError::NotFound(_)) => 404,
            Self::Application(babata_application::ApplicationError::Conflict(_)) => 409,
            Self::Io(_) | Self::Application(_) => 500,
        }
    }
}

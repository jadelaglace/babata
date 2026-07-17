use babata_domain::{CapabilityId, DomainError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("storage failure: {0}")]
    Storage(String),
    #[error("asset failure: {0}")]
    Asset(String),
    #[error("integrity failure: {0}")]
    Integrity(String),
    #[error("capability_unavailable: {capability} (activation phase: {activation_phase})")]
    CapabilityUnavailable {
        capability: CapabilityId,
        activation_phase: String,
    },
}

impl ApplicationError {
    pub fn capability_unavailable(
        capability: impl Into<String>,
        activation_phase: impl Into<String>,
    ) -> Self {
        Self::CapabilityUnavailable {
            capability: CapabilityId::new(capability),
            activation_phase: activation_phase.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::CapabilityUnavailable { .. } => "capability_unavailable",
            Self::Domain(_) => "validation_failed",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Storage(_) | Self::Asset(_) => "io_failed",
            Self::Integrity(_) => "integrity_failed",
        }
    }
}

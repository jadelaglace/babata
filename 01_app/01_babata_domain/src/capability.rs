use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityId(pub String);

impl CapabilityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for CapabilityId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Unavailable,
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub status: CapabilityStatus,
    pub activation_phase: String,
    pub reason: Option<String>,
}

impl CapabilityDescriptor {
    pub fn enabled(id: impl Into<String>, activation_phase: impl Into<String>) -> Self {
        Self {
            id: CapabilityId::new(id),
            status: CapabilityStatus::Enabled,
            activation_phase: activation_phase.into(),
            reason: None,
        }
    }

    pub fn unavailable(id: impl Into<String>, activation_phase: impl Into<String>) -> Self {
        Self {
            id: CapabilityId::new(id),
            status: CapabilityStatus::Unavailable,
            activation_phase: activation_phase.into(),
            reason: Some("capability is not activated in the current phase".to_owned()),
        }
    }
}

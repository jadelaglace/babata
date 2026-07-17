pub mod candidate;
pub mod providers;
pub mod registry;

use babata_application::ApplicationError;
use babata_domain::{CapabilityStatus, SourceRouteDescriptor, SourceRouteId};

pub(crate) fn descriptor(id: &str, provider: &str, phase: &str) -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(id.to_owned()),
        provider: provider.to_owned(),
        status: CapabilityStatus::Unavailable,
        activation_phase: phase.to_owned(),
    }
}

pub(crate) fn unavailable(id: &str, phase: &str) -> ApplicationError {
    ApplicationError::capability_unavailable(id, phase)
}

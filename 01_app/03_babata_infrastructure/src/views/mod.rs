pub mod datasette;
pub mod exports;
pub mod manifest;
pub mod obsidian;
pub mod output;
pub mod sublibrary;

use babata_domain::{CapabilityStatus, ViewDescriptor, ViewId, ViewKind};

pub(crate) fn descriptor(kind: ViewKind, phase: &str) -> ViewDescriptor {
    ViewDescriptor {
        id: ViewId::new(),
        kind,
        status: CapabilityStatus::Unavailable,
        activation_phase: phase.to_owned(),
    }
}

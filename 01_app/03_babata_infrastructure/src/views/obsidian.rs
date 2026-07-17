use babata_application::ApplicationError;
use babata_domain::{ViewDescriptor, ViewKind};

pub fn descriptor() -> ViewDescriptor {
    super::descriptor(ViewKind::Obsidian, "P6")
}

pub fn unavailable() -> ApplicationError {
    ApplicationError::capability_unavailable("views.obsidian", "P6")
}

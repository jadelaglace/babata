use babata_application::ApplicationError;
use babata_domain::{ViewDescriptor, ViewKind};

pub fn descriptor() -> ViewDescriptor {
    super::descriptor(ViewKind::Export, "P6")
}

pub fn unavailable() -> ApplicationError {
    ApplicationError::capability_unavailable("views.exports", "P6")
}

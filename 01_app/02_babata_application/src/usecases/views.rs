use babata_domain::{ViewDescriptor, ViewId};

use crate::{ApplicationError, ViewBuildCommand, ViewBuildOutcome};

#[derive(Debug, Default, Clone, Copy)]
pub struct ViewService;

impl ViewService {
    pub fn list(&self) -> Result<Vec<ViewDescriptor>, ApplicationError> {
        unavailable()
    }

    pub fn build(&self, _command: ViewBuildCommand) -> Result<ViewBuildOutcome, ApplicationError> {
        unavailable()
    }

    #[doc(hidden)]
    pub fn _owner_marker(&self, _view_id: &ViewId) {}
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable("views", "P6"))
}

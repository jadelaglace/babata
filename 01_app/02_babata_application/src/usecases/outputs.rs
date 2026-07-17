use babata_domain::{OutputBuild, OutputId, OutputKind, OutputScope};

use crate::ApplicationError;

#[derive(Debug, Default, Clone, Copy)]
pub struct OutputService;

impl OutputService {
    pub fn list(&self) -> Result<Vec<OutputKind>, ApplicationError> {
        unavailable()
    }

    pub fn build(
        &self,
        _kind: OutputKind,
        _scope: OutputScope,
    ) -> Result<OutputBuild, ApplicationError> {
        unavailable()
    }

    pub fn status(&self, _output_id: &OutputId) -> Result<OutputBuild, ApplicationError> {
        unavailable()
    }

    pub fn verify(&self, _output_id: &OutputId) -> Result<bool, ApplicationError> {
        unavailable()
    }
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable("outputs", "P6"))
}

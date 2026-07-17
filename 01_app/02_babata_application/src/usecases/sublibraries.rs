use babata_domain::{SublibraryDefinition, SublibraryId, ViewDescriptor};

use crate::ApplicationError;

#[derive(Debug, Default, Clone, Copy)]
pub struct SublibraryService;

impl SublibraryService {
    pub fn create(
        &self,
        _definition: SublibraryDefinition,
    ) -> Result<SublibraryDefinition, ApplicationError> {
        unavailable()
    }

    pub fn revise(
        &self,
        _definition: SublibraryDefinition,
    ) -> Result<SublibraryDefinition, ApplicationError> {
        unavailable()
    }

    pub fn show(
        &self,
        _sublibrary_id: &SublibraryId,
    ) -> Result<SublibraryDefinition, ApplicationError> {
        unavailable()
    }

    pub fn materialize(
        &self,
        _sublibrary_id: &SublibraryId,
    ) -> Result<ViewDescriptor, ApplicationError> {
        unavailable()
    }
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable(
        "sublibraries",
        "P6",
    ))
}

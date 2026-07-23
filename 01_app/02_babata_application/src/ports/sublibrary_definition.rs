use babata_domain::{SublibraryDefinition, SublibraryId};

use crate::ApplicationError;

pub trait SublibraryDefinitionPort {
    fn list_latest(&self) -> Result<Vec<SublibraryDefinition>, ApplicationError>;
    fn list_versions(
        &self,
        sublibrary_id: &SublibraryId,
    ) -> Result<Vec<SublibraryDefinition>, ApplicationError>;
    fn find(
        &self,
        sublibrary_id: &SublibraryId,
        version: Option<u32>,
    ) -> Result<Option<SublibraryDefinition>, ApplicationError>;
}

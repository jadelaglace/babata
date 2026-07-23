use babata_domain::{SublibraryId, SublibraryMaterialization, SublibraryMaterializationDocument};

use crate::ApplicationError;

pub trait SublibraryMaterializerPort {
    fn build(
        &self,
        document: &SublibraryMaterializationDocument,
    ) -> Result<SublibraryMaterialization, ApplicationError>;
    fn status(
        &self,
        sublibrary_id: &SublibraryId,
        version: u32,
    ) -> Result<SublibraryMaterialization, ApplicationError>;
    fn verify(
        &self,
        document: &SublibraryMaterializationDocument,
    ) -> Result<SublibraryMaterialization, ApplicationError>;
    fn delete(
        &self,
        sublibrary_id: &SublibraryId,
        version: u32,
    ) -> Result<SublibraryMaterialization, ApplicationError>;
}

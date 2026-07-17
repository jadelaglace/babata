use babata_domain::{CapabilityDescriptor, PipelineId, ProviderTaskRef, RevisionId};

use crate::ApplicationError;

pub trait ProcessProviderPort {
    fn describe(&self) -> CapabilityDescriptor;
    fn prepare(
        &self,
        pipeline_id: &PipelineId,
        revision_id: &RevisionId,
    ) -> Result<String, ApplicationError>;
    fn submit(&self, prepared: &str) -> Result<ProviderTaskRef, ApplicationError>;
    fn poll(&self, task: &ProviderTaskRef) -> Result<String, ApplicationError>;
    fn cancel(&self, task: &ProviderTaskRef) -> Result<(), ApplicationError>;
    fn fetch(&self, task: &ProviderTaskRef) -> Result<Vec<u8>, ApplicationError>;
}

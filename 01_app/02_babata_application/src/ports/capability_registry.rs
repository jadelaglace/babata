use babata_domain::{CapabilityDescriptor, CapabilityId};

use crate::ApplicationError;

pub trait CapabilityRegistryPort {
    fn list(&self) -> Result<Vec<CapabilityDescriptor>, ApplicationError>;
    fn get(&self, id: &CapabilityId) -> Result<Option<CapabilityDescriptor>, ApplicationError>;
}

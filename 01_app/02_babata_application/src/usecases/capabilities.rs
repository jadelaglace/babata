use babata_domain::CapabilityDescriptor;

use crate::{ApplicationError, ports::CapabilityRegistryPort};

pub struct CapabilityService<R> {
    registry: R,
}

impl<R> CapabilityService<R>
where
    R: CapabilityRegistryPort,
{
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub fn list(&self) -> Result<Vec<CapabilityDescriptor>, ApplicationError> {
        self.registry.list()
    }
}

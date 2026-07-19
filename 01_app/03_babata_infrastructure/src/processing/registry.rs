use babata_application::{
    ApplicationError,
    ports::{
        ProcessProviderPort, ProviderExecutionOutcome, ProviderExecutionRequest, ProviderIdentity,
    },
};
use babata_domain::{CapabilityDescriptor, PipelineId, ProviderTaskRef};

use super::{
    bailian_cli::{BailianCliConfig, BailianCliProvider},
    local_extract::LocalExtractProvider,
};

#[derive(Debug, Clone, Default)]
pub struct ProcessProviderRouter {
    local: LocalExtractProvider,
    bailian: BailianCliProvider,
}

impl ProcessProviderRouter {
    pub fn detect() -> Self {
        Self {
            local: LocalExtractProvider,
            bailian: BailianCliProvider::new(BailianCliConfig::detect()),
        }
    }

    pub fn descriptors(&self) -> Vec<CapabilityDescriptor> {
        vec![
            self.local.describe(),
            self.bailian.describe(),
            CapabilityDescriptor::unavailable("processing.bailian_api", "P5+"),
        ]
    }
}

impl ProcessProviderPort for ProcessProviderRouter {
    fn describe(&self, pipeline_id: &PipelineId) -> CapabilityDescriptor {
        match pipeline_id.as_str() {
            "local_extract_text" => self.local.describe(),
            "bailian_summary" => self.bailian.describe(),
            other => CapabilityDescriptor::unavailable(format!("processing.{other}"), "P5+"),
        }
    }

    fn identity(&self, pipeline_id: &PipelineId) -> Result<ProviderIdentity, ApplicationError> {
        match pipeline_id.as_str() {
            "local_extract_text" => Ok(self.local.identity()),
            "bailian_summary" => Ok(self.bailian.identity()),
            other => Err(ApplicationError::capability_unavailable(
                format!("processing.{other}"),
                "P5+",
            )),
        }
    }

    fn execute(
        &self,
        request: &ProviderExecutionRequest,
    ) -> Result<ProviderExecutionOutcome, ApplicationError> {
        match request.pipeline_id.as_str() {
            "local_extract_text" => self.local.execute(request),
            "bailian_summary" => self.bailian.execute(request),
            other => Err(ApplicationError::capability_unavailable(
                format!("processing.{other}"),
                "P5+",
            )),
        }
    }

    fn cancel(&self, task: &ProviderTaskRef) -> Result<(), ApplicationError> {
        match task.provider.as_str() {
            "local_extract" => Ok(()),
            "bailian_cli" => self.bailian.cancel(task),
            other => Err(ApplicationError::capability_unavailable(
                format!("processing.{other}"),
                "P5+",
            )),
        }
    }
}

pub fn processing_descriptors() -> Vec<CapabilityDescriptor> {
    ProcessProviderRouter::detect().descriptors()
}

#[cfg(test)]
mod tests {
    use super::*;
    use babata_domain::CapabilityStatus;

    #[test]
    fn local_provider_is_enabled_without_promoting_the_api_provider() {
        let descriptors = ProcessProviderRouter::detect().descriptors();
        assert!(descriptors.iter().any(|descriptor| {
            descriptor.id.0.as_str() == "processing.local_extract"
                && descriptor.status == CapabilityStatus::Enabled
        }));
        assert!(descriptors.iter().any(|descriptor| {
            descriptor.id.0.as_str() == "processing.bailian_api"
                && descriptor.status == CapabilityStatus::Unavailable
        }));
    }
}

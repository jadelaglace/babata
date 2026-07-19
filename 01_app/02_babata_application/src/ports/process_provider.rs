use babata_domain::{
    CapabilityDescriptor, DerivativeKind, JobId, Metadata, PipelineId, ProviderTaskRef, RevisionId,
    Sha256,
};

use crate::ApplicationError;

#[derive(Debug, Clone)]
pub struct ProviderExecutionRequest {
    pub job_id: JobId,
    pub pipeline_id: PipelineId,
    pub revision_id: RevisionId,
    pub input_sha256: Sha256,
    pub input_text: String,
}

#[derive(Debug, Clone)]
pub struct ProviderExecutionOutcome {
    pub task: ProviderTaskRef,
    pub kind: DerivativeKind,
    pub provider: String,
    pub tool_or_model: String,
    pub tool_version: String,
    pub content_text: Option<String>,
    pub content_json: Option<String>,
    pub media_type: Option<String>,
    pub language: Option<String>,
    pub params: Metadata,
    pub usage: Metadata,
    pub loss_notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderIdentity {
    pub kind: DerivativeKind,
    pub provider: String,
    pub tool_or_model: String,
    pub tool_version: String,
}

pub trait ProcessProviderPort {
    fn describe(&self, pipeline_id: &PipelineId) -> CapabilityDescriptor;
    fn identity(&self, pipeline_id: &PipelineId) -> Result<ProviderIdentity, ApplicationError>;
    fn execute(
        &self,
        request: &ProviderExecutionRequest,
    ) -> Result<ProviderExecutionOutcome, ApplicationError>;
    fn cancel(&self, task: &ProviderTaskRef) -> Result<(), ApplicationError>;
}

impl ProcessProviderPort for () {
    fn describe(&self, pipeline_id: &PipelineId) -> CapabilityDescriptor {
        CapabilityDescriptor::unavailable(format!("processing.{}", pipeline_id.as_str()), "P5")
    }

    fn identity(&self, pipeline_id: &PipelineId) -> Result<ProviderIdentity, ApplicationError> {
        Err(ApplicationError::capability_unavailable(
            format!("processing.{}", pipeline_id.as_str()),
            "P5",
        ))
    }

    fn execute(
        &self,
        request: &ProviderExecutionRequest,
    ) -> Result<ProviderExecutionOutcome, ApplicationError> {
        Err(ApplicationError::capability_unavailable(
            format!("processing.{}", request.pipeline_id.as_str()),
            "P5",
        ))
    }

    fn cancel(&self, task: &ProviderTaskRef) -> Result<(), ApplicationError> {
        Err(ApplicationError::capability_unavailable(
            format!("processing.{}", task.provider),
            "P5",
        ))
    }
}

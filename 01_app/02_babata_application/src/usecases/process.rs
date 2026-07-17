use babata_domain::{JobId, PipelineId};

use crate::{ApplicationError, EnqueueProcessCommand, ProcessJobOutcome};

#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessService;

impl ProcessService {
    pub fn enqueue(
        &self,
        _command: EnqueueProcessCommand,
    ) -> Result<ProcessJobOutcome, ApplicationError> {
        unavailable()
    }

    pub fn run_once(&self) -> Result<ProcessJobOutcome, ApplicationError> {
        unavailable()
    }

    pub fn status(&self, _job_id: &JobId) -> Result<ProcessJobOutcome, ApplicationError> {
        unavailable()
    }

    pub fn retry(&self, _job_id: &JobId) -> Result<ProcessJobOutcome, ApplicationError> {
        unavailable()
    }

    pub fn cancel(&self, _job_id: &JobId) -> Result<ProcessJobOutcome, ApplicationError> {
        unavailable()
    }

    pub fn list_pipelines(&self) -> Result<Vec<PipelineId>, ApplicationError> {
        unavailable()
    }
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable("processing", "P5"))
}

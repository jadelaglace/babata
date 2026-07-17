use babata_domain::{JobId, JobRef, PipelineId, ProcessingState, UtcTimestamp};

use crate::ApplicationError;

pub trait JobRepositoryPort {
    fn enqueue(&self, pipeline_id: &PipelineId) -> Result<JobRef, ApplicationError>;
    fn claim(&self, worker_id: &str) -> Result<Option<JobRef>, ApplicationError>;
    fn heartbeat(&self, job_id: &JobId, at: &UtcTimestamp) -> Result<(), ApplicationError>;
    fn complete(&self, job_id: &JobId) -> Result<(), ApplicationError>;
    fn fail(&self, job_id: &JobId, message: &str) -> Result<(), ApplicationError>;
    fn retry(&self, job_id: &JobId) -> Result<(), ApplicationError>;
    fn cancel(&self, job_id: &JobId) -> Result<ProcessingState, ApplicationError>;
}

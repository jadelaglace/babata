use babata_domain::{JobId, ProcessJob, ProviderTaskRef, RunId, UtcTimestamp};

use crate::ApplicationError;

pub trait JobRepositoryPort {
    fn enqueue(&self, job: &ProcessJob) -> Result<(), ApplicationError>;
    fn get(&self, job_id: &JobId) -> Result<Option<ProcessJob>, ApplicationError>;
    fn claim(
        &self,
        worker_id: &str,
        at: &UtcTimestamp,
        lease_seconds: u32,
    ) -> Result<Option<ProcessJob>, ApplicationError>;
    fn heartbeat(
        &self,
        job_id: &JobId,
        worker_id: &str,
        at: &UtcTimestamp,
        lease_seconds: u32,
    ) -> Result<ProcessJob, ApplicationError>;
    fn complete(
        &self,
        job_id: &JobId,
        worker_id: &str,
        run_id: &RunId,
        task: &ProviderTaskRef,
        at: &UtcTimestamp,
    ) -> Result<ProcessJob, ApplicationError>;
    #[allow(clippy::too_many_arguments)]
    fn fail(
        &self,
        job_id: &JobId,
        worker_id: &str,
        error_code: &str,
        error_message: &str,
        run_id: Option<&RunId>,
        task: Option<&ProviderTaskRef>,
        at: &UtcTimestamp,
    ) -> Result<ProcessJob, ApplicationError>;
    fn retry(&self, parent_id: &JobId, retry: &ProcessJob) -> Result<(), ApplicationError>;
    fn cancel(&self, job_id: &JobId, at: &UtcTimestamp) -> Result<ProcessJob, ApplicationError>;
}

impl JobRepositoryPort for () {
    fn enqueue(&self, _job: &ProcessJob) -> Result<(), ApplicationError> {
        Err(unavailable())
    }

    fn get(&self, _job_id: &JobId) -> Result<Option<ProcessJob>, ApplicationError> {
        Err(unavailable())
    }

    fn claim(
        &self,
        _worker_id: &str,
        _at: &UtcTimestamp,
        _lease_seconds: u32,
    ) -> Result<Option<ProcessJob>, ApplicationError> {
        Err(unavailable())
    }

    fn heartbeat(
        &self,
        _job_id: &JobId,
        _worker_id: &str,
        _at: &UtcTimestamp,
        _lease_seconds: u32,
    ) -> Result<ProcessJob, ApplicationError> {
        Err(unavailable())
    }

    fn complete(
        &self,
        _job_id: &JobId,
        _worker_id: &str,
        _run_id: &RunId,
        _task: &ProviderTaskRef,
        _at: &UtcTimestamp,
    ) -> Result<ProcessJob, ApplicationError> {
        Err(unavailable())
    }

    fn fail(
        &self,
        _job_id: &JobId,
        _worker_id: &str,
        _error_code: &str,
        _error_message: &str,
        _run_id: Option<&RunId>,
        _task: Option<&ProviderTaskRef>,
        _at: &UtcTimestamp,
    ) -> Result<ProcessJob, ApplicationError> {
        Err(unavailable())
    }

    fn retry(&self, _parent_id: &JobId, _retry: &ProcessJob) -> Result<(), ApplicationError> {
        Err(unavailable())
    }

    fn cancel(&self, _job_id: &JobId, _at: &UtcTimestamp) -> Result<ProcessJob, ApplicationError> {
        Err(unavailable())
    }
}

fn unavailable() -> ApplicationError {
    ApplicationError::capability_unavailable("processing.job_queue", "P5")
}

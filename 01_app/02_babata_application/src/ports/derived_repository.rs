use babata_domain::{DerivativeId, DerivativeRef, ProcessRun, RevisionId, RunId};

use crate::ApplicationError;

pub trait DerivedRepositoryPort {
    fn create_run(&self, run: &ProcessRun) -> Result<(), ApplicationError>;
    fn update_run(&self, run: &ProcessRun) -> Result<(), ApplicationError>;
    fn get_run(&self, run_id: &RunId) -> Result<Option<ProcessRun>, ApplicationError>;
    fn list_runs_for_revision(
        &self,
        revision_id: &RevisionId,
    ) -> Result<Vec<ProcessRun>, ApplicationError>;
    fn add_derivative(&self, derivative: &DerivativeRef) -> Result<(), ApplicationError>;
    fn get_derivative(
        &self,
        derivative_id: &DerivativeId,
    ) -> Result<Option<DerivativeRef>, ApplicationError>;
    fn list_derivatives(&self, run_id: &RunId) -> Result<Vec<DerivativeRef>, ApplicationError>;
}

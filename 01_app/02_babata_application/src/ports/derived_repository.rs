use babata_domain::{DerivativeId, DerivativeRef, ProcessRun, RevisionId, RunId};

use crate::ApplicationError;

/// One atomic C1 commit: a process run plus the derivatives it produced.
#[derive(Debug, Clone)]
pub struct ProcessCommit {
    pub run: ProcessRun,
    pub derivatives: Vec<DerivativeRef>,
}

impl ProcessCommit {
    #[must_use]
    pub fn new(run: ProcessRun) -> Self {
        Self {
            run,
            derivatives: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_derivative(mut self, derivative: DerivativeRef) -> Self {
        self.derivatives.push(derivative);
        self
    }
}

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
    /// Persist a run and its derivatives in one transaction so a succeeded run
    /// always has its outputs recorded (no partial C1 commits).
    fn commit_run(&self, commit: &ProcessCommit) -> Result<(), ApplicationError>;
}
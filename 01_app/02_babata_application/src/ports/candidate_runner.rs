use babata_domain::CandidateEnvelope;

use crate::{ApplicationError, CaptureOutcome};

pub trait CandidateRunnerPort {
    fn run(&self, candidate: &CandidateEnvelope) -> Result<CaptureOutcome, ApplicationError>;
    fn validate(&self, candidate: &CandidateEnvelope) -> Result<(), ApplicationError>;
}

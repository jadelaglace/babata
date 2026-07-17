use babata_application::{ApplicationError, CaptureOutcome, ports::CandidateRunnerPort};
use babata_domain::CandidateEnvelope;

#[derive(Debug, Default, Clone, Copy)]
pub struct RustCandidateRunner;

impl CandidateRunnerPort for RustCandidateRunner {
    fn run(&self, _candidate: &CandidateEnvelope) -> Result<CaptureOutcome, ApplicationError> {
        Err(super::unavailable("capture.candidate", "P4"))
    }

    fn validate(&self, candidate: &CandidateEnvelope) -> Result<(), ApplicationError> {
        if candidate.protocol_version == "1" {
            Ok(())
        } else {
            Err(ApplicationError::Conflict(
                "unsupported candidate protocol version".to_owned(),
            ))
        }
    }
}

use babata_application::ApplicationError;

use crate::app::WorkerApp;

pub fn run(_app: &WorkerApp) -> Result<(), ApplicationError> {
    Err(ApplicationError::capability_unavailable("worker", "P5"))
}

pub fn claim_once(_app: &WorkerApp) -> Result<(), ApplicationError> {
    Err(ApplicationError::capability_unavailable(
        "worker.claim",
        "P5",
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn disabled_worker_does_not_claim_or_run() {
        let app = crate::app::build();
        assert!(super::run(&app).is_err());
        assert!(super::claim_once(&app).is_err());
    }
}

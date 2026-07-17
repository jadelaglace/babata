use babata_application::ApplicationError;

pub fn heartbeat(_worker_id: &str) -> Result<(), ApplicationError> {
    Err(ApplicationError::capability_unavailable(
        "worker.heartbeat",
        "P5",
    ))
}

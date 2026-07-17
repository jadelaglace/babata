use babata_application::ApplicationError;

#[derive(Debug, Default, Clone, Copy)]
pub struct SqliteJobRepository;

impl SqliteJobRepository {
    pub fn unavailable() -> ApplicationError {
        ApplicationError::capability_unavailable("storage.jobs", "P5")
    }
}

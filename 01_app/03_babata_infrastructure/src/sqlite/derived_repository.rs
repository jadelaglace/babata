use babata_application::ApplicationError;

#[derive(Debug, Default, Clone, Copy)]
pub struct SqliteDerivedRepository;

impl SqliteDerivedRepository {
    pub fn unavailable() -> ApplicationError {
        ApplicationError::capability_unavailable("storage.derived", "P5")
    }
}

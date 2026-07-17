use babata_application::ApplicationError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DisabledCommandRunner;

impl DisabledCommandRunner {
    pub fn run(&self, _spec: &CommandSpec) -> Result<(), ApplicationError> {
        Err(ApplicationError::capability_unavailable(
            "tools.command_runner",
            "P4+",
        ))
    }
}

use babata_application::{ApplicationError, ports::OutputBuilderPort};
use babata_domain::{OutputBuild, OutputId, OutputKind, OutputScope};

#[derive(Debug, Clone, Copy)]
pub struct UnavailableOutputBuilder {
    kind: OutputKind,
}

impl UnavailableOutputBuilder {
    pub fn new(kind: OutputKind) -> Self {
        Self { kind }
    }
}

impl OutputBuilderPort for UnavailableOutputBuilder {
    fn describe(&self) -> OutputKind {
        self.kind
    }

    fn build(&self, _scope: &OutputScope) -> Result<OutputBuild, ApplicationError> {
        unavailable()
    }

    fn status(&self, _output_id: &OutputId) -> Result<OutputBuild, ApplicationError> {
        unavailable()
    }

    fn verify(&self, _output_id: &OutputId) -> Result<bool, ApplicationError> {
        unavailable()
    }
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable("outputs", "P6"))
}

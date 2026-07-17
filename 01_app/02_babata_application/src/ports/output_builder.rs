use babata_domain::{OutputBuild, OutputId, OutputKind, OutputScope};

use crate::ApplicationError;

pub trait OutputBuilderPort {
    fn describe(&self) -> OutputKind;
    fn build(&self, scope: &OutputScope) -> Result<OutputBuild, ApplicationError>;
    fn status(&self, output_id: &OutputId) -> Result<OutputBuild, ApplicationError>;
    fn verify(&self, output_id: &OutputId) -> Result<bool, ApplicationError>;
}

use babata_domain::{OutputBuild, OutputDocument, OutputId, OutputKind, OutputVerification};

use crate::ApplicationError;

pub trait OutputBuilderPort {
    fn supported_kinds(&self) -> Vec<OutputKind>;
    fn build(&self, document: &OutputDocument) -> Result<OutputBuild, ApplicationError>;
    fn rebuild(&self, document: &OutputDocument) -> Result<OutputBuild, ApplicationError>;
    fn status(&self, output_id: &OutputId) -> Result<OutputBuild, ApplicationError>;
    fn verify(&self, document: &OutputDocument) -> Result<OutputVerification, ApplicationError>;
    fn delete(&self, output_id: &OutputId) -> Result<OutputBuild, ApplicationError>;
}

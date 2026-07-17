use babata_domain::{BuildTarget, ViewDescriptor};

use crate::ApplicationError;

pub trait ViewBuilderPort {
    fn describe(&self) -> ViewDescriptor;
    fn build(&self, target: &BuildTarget) -> Result<(), ApplicationError>;
    fn verify(&self, target: &BuildTarget) -> Result<bool, ApplicationError>;
}

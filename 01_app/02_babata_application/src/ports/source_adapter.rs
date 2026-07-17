use babata_domain::{CandidateEnvelope, CandidateSummary, RouteCoverage, SourceRouteDescriptor};

use crate::ApplicationError;

pub trait SourceAdapterPort {
    fn describe(&self) -> SourceRouteDescriptor;
    fn discover(&self, source_reference: &str) -> Result<Vec<CandidateSummary>, ApplicationError>;
    fn collect(&self, source_reference: &str) -> Result<CandidateEnvelope, ApplicationError>;
    fn coverage(&self) -> RouteCoverage;
}

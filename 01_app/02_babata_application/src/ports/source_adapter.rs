use babata_domain::{CandidateSummary, CollectionSessionId, RouteCoverage, SourceRouteDescriptor};

use crate::{AcquisitionOutcome, ApplicationError, DiscoveredCandidate};

pub trait SourceAdapterPort {
    fn describe(&self) -> SourceRouteDescriptor;
    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError>;
    fn collect(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&babata_domain::CandidateEnvelope>,
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError>;
    fn coverage(&self) -> RouteCoverage;
}

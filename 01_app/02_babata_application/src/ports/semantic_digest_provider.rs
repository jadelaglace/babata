use babata_domain::{
    DerivativeEvidence, ItemId, Metadata, RevisionId, SemanticCandidatePackage, Sha256,
    UtcTimestamp,
};

use crate::ApplicationError;

#[derive(Debug, Clone)]
pub struct SemanticDigestRequest {
    pub source_item_id: ItemId,
    pub source_revision_id: RevisionId,
    pub source_input_sha256: Sha256,
    pub evidence: Vec<DerivativeEvidence>,
    pub review_context: String,
    pub generated_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
pub struct SemanticDigestOutcome {
    pub package: SemanticCandidatePackage,
    pub provider_task_id: String,
    pub usage: Metadata,
}

pub trait SemanticDigestProviderPort {
    fn execute(
        &self,
        request: &SemanticDigestRequest,
    ) -> Result<SemanticDigestOutcome, ApplicationError>;
}

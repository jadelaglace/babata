use babata_domain::ScoreProfile;

use crate::{
    ApplicationError, CreateScoreProfileCommand, FirstPartySemanticOutcome,
    IngestSemanticCandidateCommand, RecordRelevanceScoreCommand, RecordSuggestionReviewCommand,
    RegisterFirstPartySemanticCommand, SemanticCoreSnapshot, SemanticEntryDetail,
    SemanticIngestOutcome,
};

pub trait KnowledgeCoreRepositoryPort {
    fn ingest_machine_candidate(
        &self,
        command: &IngestSemanticCandidateCommand,
    ) -> Result<SemanticIngestOutcome, ApplicationError>;
    fn load_semantic_snapshot(
        &self,
        suggestion_id: &str,
    ) -> Result<SemanticCoreSnapshot, ApplicationError>;
    fn record_suggestion_review(
        &self,
        command: &RecordSuggestionReviewCommand,
    ) -> Result<(), ApplicationError>;
    fn create_score_profile(
        &self,
        command: &CreateScoreProfileCommand,
    ) -> Result<(), ApplicationError>;
    fn list_score_profiles(&self) -> Result<Vec<ScoreProfile>, ApplicationError>;
    fn register_first_party_semantic(
        &self,
        command: &RegisterFirstPartySemanticCommand,
    ) -> Result<FirstPartySemanticOutcome, ApplicationError>;
    fn load_semantic_entry(
        &self,
        semantic_id: &str,
    ) -> Result<SemanticEntryDetail, ApplicationError>;
    fn record_relevance_score(
        &self,
        command: &RecordRelevanceScoreCommand,
    ) -> Result<crate::RelevanceScoreDetail, ApplicationError>;
}

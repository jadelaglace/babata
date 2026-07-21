use babata_domain::ScoreProfile;

use crate::{
    ApplicationError, ChangeMapNodeTagCommand, ChangeMapParentCommand,
    ChangeSemanticMapAssignmentCommand, CreateMapNodeCommand, CreateScoreProfileCommand,
    EvolveMapNodeCommand, FirstPartySemanticOutcome, IngestSemanticCandidateCommand, MapNodeDetail,
    RecordRelevanceScoreCommand, RecordSuggestionReviewCommand, RegisterFirstPartySemanticCommand,
    SemanticCoreSnapshot, SemanticEntryDetail, SemanticIngestOutcome,
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
    fn create_map_node(
        &self,
        command: &CreateMapNodeCommand,
    ) -> Result<MapNodeDetail, ApplicationError>;
    fn evolve_map_node(
        &self,
        command: &EvolveMapNodeCommand,
    ) -> Result<MapNodeDetail, ApplicationError>;
    fn change_map_parent(
        &self,
        command: &ChangeMapParentCommand,
    ) -> Result<MapNodeDetail, ApplicationError>;
    fn change_semantic_map_assignment(
        &self,
        command: &ChangeSemanticMapAssignmentCommand,
    ) -> Result<MapNodeDetail, ApplicationError>;
    fn change_map_node_tag(
        &self,
        command: &ChangeMapNodeTagCommand,
    ) -> Result<MapNodeDetail, ApplicationError>;
    fn load_map_node(&self, map_node_id: &str) -> Result<MapNodeDetail, ApplicationError>;
    fn record_relevance_score(
        &self,
        command: &RecordRelevanceScoreCommand,
    ) -> Result<crate::RelevanceScoreDetail, ApplicationError>;
}

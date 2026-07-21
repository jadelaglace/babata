use babata_domain::{
    CandidateEnvelope, CandidateSummary, CollectionItemState, CollectionItemStatus,
    CollectionSelection, CollectionSession, CollectionSessionId, CollectionSessionState, ItemId,
    RecollectionOutcome, RevisionId, UtcTimestamp,
};

use crate::{ApplicationError, ports::NewSourceObservation};

pub trait CollectionRepositoryPort {
    fn create_session(&self, session: &CollectionSession) -> Result<(), ApplicationError>;
    fn update_session_state(
        &self,
        session_id: &CollectionSessionId,
        state: CollectionSessionState,
        updated_at: &UtcTimestamp,
    ) -> Result<(), ApplicationError>;
    fn session(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<Option<CollectionSession>, ApplicationError>;
    fn save_candidates(
        &self,
        candidates: &[(CandidateSummary, Option<CandidateEnvelope>)],
    ) -> Result<(), ApplicationError>;
    fn candidates(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<Vec<CandidateSummary>, ApplicationError>;
    fn candidate(
        &self,
        session_id: &CollectionSessionId,
        candidate_id: &str,
    ) -> Result<Option<(CandidateSummary, Option<CandidateEnvelope>)>, ApplicationError>;
    fn enqueue_selection(
        &self,
        selection: &CollectionSelection,
        now: &UtcTimestamp,
    ) -> Result<Vec<CollectionItemStatus>, ApplicationError>;
    fn collection_items(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<Vec<CollectionItemStatus>, ApplicationError>;
    fn claim_item(
        &self,
        session_id: &CollectionSessionId,
        candidate_id: &str,
        now: &UtcTimestamp,
    ) -> Result<Option<CollectionItemStatus>, ApplicationError>;
    fn cancel_session(
        &self,
        session_id: &CollectionSessionId,
        reason: &str,
        now: &UtcTimestamp,
    ) -> Result<Vec<CollectionItemStatus>, ApplicationError>;
    #[allow(clippy::too_many_arguments)]
    fn transition_item(
        &self,
        session_id: &CollectionSessionId,
        candidate_id: &str,
        state: CollectionItemState,
        reason: Option<&str>,
        retryable: bool,
        item_id: Option<&ItemId>,
        revision_id: Option<&RevisionId>,
        increment_attempt: bool,
        now: &UtcTimestamp,
    ) -> Result<CollectionItemStatus, ApplicationError>;
    fn latest_saved_for_item(
        &self,
        item_id: &ItemId,
    ) -> Result<Option<(CollectionSessionId, String)>, ApplicationError>;
    fn record_recollection(
        &self,
        outcome: &RecollectionOutcome,
        observation: Option<&NewSourceObservation>,
    ) -> Result<(), ApplicationError>;
}

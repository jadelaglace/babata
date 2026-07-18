use serde::{Deserialize, Serialize};

use crate::{CollectionSessionId, ContentType, ItemId, RevisionId, SourceRouteId, UtcTimestamp};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateSummary {
    pub candidate_id: String,
    pub session_id: CollectionSessionId,
    pub route_id: SourceRouteId,
    pub source_native_id: Option<String>,
    pub title: Option<String>,
    pub source_location: Option<String>,
    pub hierarchy: Vec<String>,
    pub content_type: ContentType,
    pub source_updated_at: Option<UtcTimestamp>,
    pub attachment_available: Option<bool>,
    pub limitations: Vec<String>,
    pub selection_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSelection {
    pub session_id: CollectionSessionId,
    pub candidate_ids: Vec<String>,
    pub scope_description: String,
    pub confirmed: bool,
    pub authorised_context: String,
    pub requested_attachments: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionSessionState {
    Discovering,
    AwaitingSelection,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSession {
    pub session_id: CollectionSessionId,
    pub route_id: SourceRouteId,
    pub source_reference: String,
    pub scope_description: String,
    pub authorisation_id: String,
    pub state: CollectionSessionState,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionItemState {
    Queued,
    Running,
    Saved,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionItemStatus {
    pub session_id: CollectionSessionId,
    pub candidate_id: String,
    pub state: CollectionItemState,
    pub attempt_count: u32,
    pub reason: Option<String>,
    pub retryable: bool,
    pub requested_attachments: bool,
    pub item_id: Option<ItemId>,
    pub revision_id: Option<RevisionId>,
    pub updated_at: UtcTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecollectionState {
    Changed,
    Unchanged,
    Inaccessible,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecollectionOutcome {
    pub item_id: ItemId,
    pub state: RecollectionState,
    pub previous_revision_id: RevisionId,
    pub new_revision_id: Option<RevisionId>,
    pub reason: Option<String>,
    pub checked_at: UtcTimestamp,
}

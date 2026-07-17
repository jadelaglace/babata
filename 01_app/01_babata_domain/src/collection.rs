use serde::{Deserialize, Serialize};

use crate::{CollectionSessionId, ContentType, SourceRouteId, UtcTimestamp};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateSummary {
    pub candidate_id: String,
    pub session_id: CollectionSessionId,
    pub route_id: SourceRouteId,
    pub title: Option<String>,
    pub source_location: Option<String>,
    pub content_type: ContentType,
    pub source_updated_at: Option<UtcTimestamp>,
    pub attachment_available: Option<bool>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSelection {
    pub session_id: CollectionSessionId,
    pub candidate_ids: Vec<String>,
    pub scope_description: String,
    pub confirmed: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecollectionState {
    Changed,
    Unchanged,
    Inaccessible,
    Removed,
}

use serde::{Deserialize, Serialize};

use crate::{ItemId, KnowledgeId, RevisionId, UtcTimestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    Record,
    Judgment,
    Relation,
    Classification,
    Model,
    Score,
    Analysis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeRecord {
    pub id: KnowledgeId,
    pub kind: KnowledgeKind,
    pub target_item_id: Option<ItemId>,
    pub target_revision_id: Option<RevisionId>,
    pub content: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSuggestion {
    pub suggestion_id: String,
    pub target_item_id: ItemId,
    pub content: String,
    pub model: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionDecisionKind {
    Accept,
    Modify,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestionDecision {
    pub suggestion_id: String,
    pub decision: SuggestionDecisionKind,
    pub human_record_id: Option<KnowledgeId>,
}

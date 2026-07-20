use serde::{Deserialize, Serialize};

use crate::{ItemId, KnowledgeId, RevisionId, UtcTimestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    MapDirection,
    Knowledge,
    Case,
    Log,
    Insight,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeVersion {
    pub ordinal: u32,
    pub first_party_revision_id: RevisionId,
    pub title: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeRecord {
    pub id: KnowledgeId,
    pub kind: KnowledgeKind,
    pub author: String,
    pub first_party_item_id: ItemId,
    pub source_item_id: ItemId,
    pub source_revision_id: RevisionId,
    pub created_at: UtcTimestamp,
    pub versions: Vec<KnowledgeVersion>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_kind_wire_values_are_stable() {
        assert_eq!(
            serde_json::to_string(&KnowledgeKind::MapDirection).unwrap(),
            "\"map_direction\""
        );
        assert_eq!(
            serde_json::to_string(&KnowledgeKind::Knowledge).unwrap(),
            "\"knowledge\""
        );
    }
}

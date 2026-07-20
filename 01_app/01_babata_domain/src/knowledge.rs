use serde::{Deserialize, Serialize};

use crate::ItemId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRealm {
    KnowledgeMap,
    KnowledgeAndCases,
    CognitiveTrail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    MapDirection,
    Knowledge,
    Case,
    Log,
    Insight,
}

impl KnowledgeKind {
    pub const fn realm(self) -> KnowledgeRealm {
        match self {
            Self::MapDirection => KnowledgeRealm::KnowledgeMap,
            Self::Knowledge | Self::Case => KnowledgeRealm::KnowledgeAndCases,
            Self::Log | Self::Insight => KnowledgeRealm::CognitiveTrail,
        }
    }
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
    pub first_party_item_id: Option<ItemId>,
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
        assert_eq!(
            KnowledgeKind::MapDirection.realm(),
            KnowledgeRealm::KnowledgeMap
        );
        assert_eq!(
            KnowledgeKind::Knowledge.realm(),
            KnowledgeRealm::KnowledgeAndCases
        );
        assert_eq!(
            KnowledgeKind::Case.realm(),
            KnowledgeRealm::KnowledgeAndCases
        );
        assert_eq!(KnowledgeKind::Log.realm(), KnowledgeRealm::CognitiveTrail);
        assert_eq!(
            KnowledgeKind::Insight.realm(),
            KnowledgeRealm::CognitiveTrail
        );
    }
}

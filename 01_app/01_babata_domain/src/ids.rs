use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::DomainError;

macro_rules! opaque_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new() -> Self {
                Self(format!("{}{}", $prefix, ulid::Ulid::new()))
            }

            pub fn parse(value: impl AsRef<str>) -> Result<Self, DomainError> {
                let value = value.as_ref();
                if !value.starts_with($prefix) || value.len() != $prefix.len() + 26 {
                    return Err(DomainError::Invalid {
                        field: stringify!($name),
                        value: value.to_owned(),
                    });
                }
                ulid::Ulid::from_string(&value[$prefix.len()..]).map_err(|_| {
                    DomainError::Invalid {
                        field: stringify!($name),
                        value: value.to_owned(),
                    }
                })?;
                Ok(Self(value.to_owned()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = DomainError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }
    };
}

opaque_id!(ItemId, "item_");
opaque_id!(RevisionId, "rev_");
opaque_id!(AssetId, "asset_");
opaque_id!(AssetAttachmentId, "asset_attachment_");
opaque_id!(SourceId, "source_");
opaque_id!(CollectionId, "collection_");
opaque_id!(RelationId, "relation_");
opaque_id!(RunId, "run_");
opaque_id!(JobId, "job_");
opaque_id!(DerivativeId, "derivative_");
opaque_id!(ViewId, "view_");
opaque_id!(SnapshotId, "snapshot_");
opaque_id!(CollectionSessionId, "session_");
opaque_id!(SourceObservationId, "observation_");
opaque_id!(KnowledgeId, "knowledge_");
opaque_id!(MapNodeId, "mapnode_");
opaque_id!(SemanticId, "semantic_");
opaque_id!(SuggestionId, "suggestion_");
opaque_id!(SuggestionReviewId, "suggestion_review_");
opaque_id!(ScoreProfileId, "score_profile_");
opaque_id!(ScoreId, "score_");
opaque_id!(TagId, "tag_");
opaque_id!(SemanticRelationId, "semantic_relation_");
opaque_id!(DenseExpressionId, "expression_");
opaque_id!(MapNodeEventId, "map_event_");
opaque_id!(MapEdgeEventId, "map_edge_event_");
opaque_id!(SemanticMapEventId, "semantic_map_event_");
opaque_id!(MapTagEventId, "map_tag_event_");
opaque_id!(SublibraryId, "sublibrary_");
opaque_id!(OutputId, "output_");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        let id = ItemId::new();
        assert_eq!(ItemId::parse(id.to_string()).unwrap(), id);
    }

    #[test]
    fn ids_reject_wrong_prefix() {
        assert!(RevisionId::parse("item_01J00000000000000000000000").is_err());
    }
}

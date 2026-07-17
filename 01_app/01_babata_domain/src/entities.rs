use serde::{Deserialize, Serialize};

use crate::{
    AssetId, AssetRole, ContentType, ItemId, LogicalPath, Metadata, RelationId, RelationKind,
    RevisionId, RevisionKind, Sha256, SourceId, SourceKind, UtcTimestamp,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    pub id: SourceId,
    pub kind: SourceKind,
    pub provider: String,
    pub account_or_workspace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawItem {
    pub id: ItemId,
    pub source_id: SourceId,
    pub content_type: ContentType,
    pub source_identity_key: Option<String>,
    pub first_captured_at: UtcTimestamp,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRevision {
    pub id: RevisionId,
    pub item_id: ItemId,
    pub parent_revision_id: Option<RevisionId>,
    pub kind: RevisionKind,
    pub ordinal: u32,
    pub captured_at: UtcTimestamp,
    pub raw_text: Option<String>,
    pub text_sha256: Option<Sha256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRef {
    pub id: AssetId,
    pub revision_id: RevisionId,
    pub role: AssetRole,
    pub logical_path: LogicalPath,
    pub sha256: Sha256,
    pub byte_size: u64,
    pub media_type: Option<String>,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: RelationId,
    pub from_item_id: ItemId,
    pub from_revision_id: Option<RevisionId>,
    pub kind: RelationKind,
    pub to_item_id: ItemId,
    pub to_revision_id: Option<RevisionId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ItemId, Metadata, SourceId, UtcTimestamp};

    #[test]
    fn entity_keeps_immutable_identity() {
        let item = RawItem {
            id: ItemId::new(),
            source_id: SourceId::new(),
            content_type: ContentType::Text,
            source_identity_key: None,
            first_captured_at: UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap(),
            metadata: Metadata::empty(),
        };
        assert!(item.source_identity_key.is_none());
    }
}

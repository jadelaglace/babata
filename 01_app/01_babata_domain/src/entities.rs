use serde::{Deserialize, Serialize};

use crate::{
    AssetId, AssetRole, ContentType, ItemId, LogicalPath, Metadata, RawState, RelationId,
    RelationKind, RevisionId, RevisionKind, Sha256, SourceId, SourceKind, UtcTimestamp,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    pub id: SourceId,
    pub kind: SourceKind,
    pub provider: String,
    pub display_name: Option<String>,
    pub account_or_workspace: Option<String>,
    pub base_locator: Option<String>,
    pub metadata: Metadata,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawItem {
    pub id: ItemId,
    pub source_id: SourceId,
    pub source_native_id: Option<String>,
    pub source_locator: Option<String>,
    pub content_type: ContentType,
    pub source_identity_key: Option<String>,
    pub source_published_at: Option<UtcTimestamp>,
    pub source_updated_at: Option<UtcTimestamp>,
    pub first_captured_at: UtcTimestamp,
    pub metadata: Metadata,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRevision {
    pub id: RevisionId,
    pub item_id: ItemId,
    pub parent_revision_id: Option<RevisionId>,
    pub kind: RevisionKind,
    pub ordinal: u32,
    pub captured_at: UtcTimestamp,
    pub authored_at: Option<UtcTimestamp>,
    pub revision_note: Option<String>,
    pub raw_text: Option<String>,
    pub text_sha256: Option<Sha256>,
    pub metadata: Metadata,
    pub state: RawState,
    pub created_at: UtcTimestamp,
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
    pub state: RawState,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: RelationId,
    pub from_item_id: ItemId,
    pub from_revision_id: Option<RevisionId>,
    pub kind: RelationKind,
    pub to_item_id: ItemId,
    pub to_revision_id: Option<RevisionId>,
    pub metadata: Metadata,
    pub created_at: UtcTimestamp,
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
            source_native_id: None,
            source_locator: None,
            content_type: ContentType::Text,
            source_identity_key: None,
            source_published_at: None,
            source_updated_at: None,
            first_captured_at: UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap(),
            metadata: Metadata::empty(),
            created_at: UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap(),
        };
        assert!(item.source_identity_key.is_none());
    }
}

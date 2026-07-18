use babata_domain::{
    AssetId, AssetRole, CollectionId, ContentType, ItemId, Metadata, RawState, RelationKind,
    RevisionId, RevisionKind, RouteCoverage, RouteEvidence, Sha256, SourceId, SourceKind,
    UtcTimestamp,
};

use crate::{ApplicationError, RecordDetail};

#[derive(Debug, Clone)]
pub struct NewSource {
    pub id: SourceId,
    pub kind: SourceKind,
    pub provider: String,
    pub account_or_workspace: Option<String>,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
pub struct NewItem {
    pub id: ItemId,
    pub source_id: SourceId,
    pub source_native_id: Option<String>,
    pub source_locator: Option<String>,
    pub source_identity_key: Option<String>,
    pub content_type: ContentType,
    pub source_published_at: Option<UtcTimestamp>,
    pub first_captured_at: UtcTimestamp,
    pub metadata: Metadata,
}

#[derive(Debug, Clone)]
pub struct NewCollection {
    pub id: CollectionId,
    pub source_id: SourceId,
    pub native_id: String,
    pub collection_kind: String,
    pub title: String,
    pub observed_at: UtcTimestamp,
    pub metadata: Metadata,
}

#[derive(Debug, Clone)]
pub struct NewRevision {
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
}

#[derive(Debug, Clone)]
pub struct NewCaptureOperation {
    pub operation_id: String,
    pub item_id: ItemId,
    pub revision_id: RevisionId,
    pub source_native_id: Option<String>,
    pub source_locator: Option<String>,
    pub source_published_at: Option<UtcTimestamp>,
    pub metadata: Metadata,
    pub started_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
pub struct NewAsset {
    pub id: AssetId,
    pub revision_id: RevisionId,
    pub role: AssetRole,
    pub logical_path: String,
    pub sha256: Sha256,
    pub byte_size: u64,
    pub media_type: Option<String>,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewRelation {
    pub kind: RelationKind,
    pub from_item_id: ItemId,
    pub from_revision_id: Option<RevisionId>,
    pub to_item_id: ItemId,
    pub to_revision_id: Option<RevisionId>,
}

#[derive(Debug, Clone)]
pub struct PersistGraph {
    pub operation: NewCaptureOperation,
    pub source: NewSource,
    pub collection: Option<NewCollection>,
    pub item: NewItem,
    pub revision: NewRevision,
    pub assets: Vec<NewAsset>,
    pub relations: Vec<NewRelation>,
}

#[derive(Debug, Clone)]
pub struct NewRouteEvidence {
    pub route_id: String,
    pub authorization_id: String,
    pub source_reference: String,
    pub item_id: ItemId,
    pub revision_id: RevisionId,
    pub coverage: RouteCoverage,
    pub reimported: bool,
    pub recorded_at: UtcTimestamp,
}

pub trait RawRepositoryPort {
    fn find_source(
        &self,
        kind: SourceKind,
        provider: &str,
        account_or_workspace: Option<&str>,
    ) -> Result<Option<NewSource>, ApplicationError>;
    fn find_item(&self, item_id: &ItemId) -> Result<Option<NewItem>, ApplicationError>;
    fn find_revision(
        &self,
        revision_id: &RevisionId,
    ) -> Result<Option<NewRevision>, ApplicationError>;
    fn find_revision_state(
        &self,
        revision_id: &RevisionId,
    ) -> Result<Option<RawState>, ApplicationError>;
    fn find_by_source_identity(
        &self,
        source_id: &SourceId,
        identity: &str,
    ) -> Result<Option<(NewItem, NewRevision)>, ApplicationError>;
    fn next_ordinal(&self, item_id: &ItemId) -> Result<u32, ApplicationError>;
    fn find_duplicate_text(
        &self,
        item_id: &ItemId,
        hash: &Sha256,
    ) -> Result<Option<RevisionId>, ApplicationError>;
    fn insert_capture_graph(&self, graph: &PersistGraph) -> Result<(), ApplicationError>;
    fn mark_ready(&self, revision_id: &RevisionId) -> Result<(), ApplicationError>;
    fn quarantine(
        &self,
        revision_id: &RevisionId,
        failure_code: &str,
    ) -> Result<(), ApplicationError>;
    fn load_detail(&self, item_id: &ItemId) -> Result<RecordDetail, ApplicationError>;
    fn record_route_evidence(&self, evidence: &NewRouteEvidence) -> Result<(), ApplicationError>;
    fn route_evidence(&self, route_id: &str) -> Result<Vec<RouteEvidence>, ApplicationError>;
}

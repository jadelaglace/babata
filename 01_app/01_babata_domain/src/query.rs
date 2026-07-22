use serde::{Deserialize, Serialize};

use crate::{
    ContentType, DerivativeId, ItemId, KnowledgeKind, KnowledgeRealm, RevisionId, SourceId,
    SourceKind, UtcTimestamp,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    #[default]
    Relevance,
    Newest,
    Interest,
    Strategy,
    Consensus,
    WeightedScore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchRecordKind {
    RawItem,
    SemanticEntry,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct QueryFilter {
    pub text: Option<String>,
    pub source_kind: Option<SourceKind>,
    pub provider: Option<String>,
    pub content_type: Option<ContentType>,
    pub captured_from: Option<UtcTimestamp>,
    pub captured_to: Option<UtcTimestamp>,
    pub semantic_kind: Option<KnowledgeKind>,
    pub realm: Option<KnowledgeRealm>,
    pub state: Option<String>,
    pub access_state: Option<String>,
    pub person: Option<String>,
    pub map_node: Option<String>,
    pub tag: Option<String>,
    pub relation_kind: Option<String>,
    pub related_to: Option<String>,
    pub processing_state: Option<String>,
    pub origin_kind: Option<String>,
    pub review_state: Option<String>,
    pub restricted: Option<bool>,
    pub missing: Option<bool>,
    pub media_only: Option<bool>,
    pub attachment_only: Option<bool>,
    pub profile_id: Option<String>,
    pub min_interest: Option<u8>,
    pub min_strategy: Option<u8>,
    pub min_consensus: Option<u8>,
    pub min_weighted_score: Option<u16>,
    pub sort: SearchSort,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PageCursor(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchMapRef {
    pub map_node_id: String,
    pub name: String,
    pub level: String,
    pub lifecycle: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchScoreRef {
    pub score_id: String,
    pub profile_id: String,
    pub profile_ordinal: u32,
    pub interest_weight: u8,
    pub strategy_weight: u8,
    pub consensus_weight: u8,
    pub interest: u8,
    pub strategy: u8,
    pub consensus: u8,
    pub weighted_score: u16,
    pub rationale: String,
    pub provenance_kind: String,
    pub author: String,
    pub created_at: UtcTimestamp,
    pub eligible_for_surface: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfacingReasonKind {
    Direction,
    Relevance,
    Time,
    Relation,
    TextMatch,
    FilterMatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfacingReason {
    pub kind: SurfacingReasonKind,
    pub explanation: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchRecordMarker {
    Restricted,
    Missing,
    MediaOnly,
    AttachmentOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgmentStatus {
    pub human_judgment: bool,
    pub confirmed_fact: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSummary {
    pub record_id: String,
    pub record_kind: SearchRecordKind,
    pub item_id: Option<ItemId>,
    pub revision_id: Option<RevisionId>,
    pub semantic_id: Option<String>,
    pub source_id: SourceId,
    pub source_locator: Option<String>,
    pub source_native_id: Option<String>,
    pub title: String,
    pub excerpt: Option<String>,
    pub source_kind: SourceKind,
    pub provider: String,
    pub content_type: ContentType,
    pub semantic_kind: Option<KnowledgeKind>,
    pub realm: Option<KnowledgeRealm>,
    pub state: String,
    pub processing_state: String,
    pub origin_kind: String,
    pub review_state: Option<String>,
    pub access_state: String,
    pub judgment: JudgmentStatus,
    pub event_at: UtcTimestamp,
    pub markers: Vec<SearchRecordMarker>,
    pub limitations: Vec<String>,
    pub people: Vec<String>,
    pub map_nodes: Vec<SearchMapRef>,
    pub tags: Vec<String>,
    pub score: Option<SearchScoreRef>,
    pub reasons: Vec<SurfacingReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRevisionRef {
    pub revision_id: RevisionId,
    pub parent_revision_id: Option<RevisionId>,
    pub ordinal: u32,
    pub kind: String,
    pub state: String,
    pub captured_at: UtcTimestamp,
    pub authored_at: Option<UtcTimestamp>,
    pub text_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchAssetRef {
    pub asset_id: String,
    pub revision_id: RevisionId,
    pub role: String,
    pub logical_path: String,
    pub media_type: Option<String>,
    pub state: String,
    pub missing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDerivativeRef {
    pub derivative_id: DerivativeId,
    pub run_id: String,
    pub revision_id: RevisionId,
    pub kind: String,
    pub processing_state: String,
    pub output_sha256: Option<String>,
    pub logical_path: Option<String>,
    pub media_type: Option<String>,
    pub invalidated: bool,
    pub missing: bool,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRelationRef {
    pub direction: String,
    pub relation_kind: String,
    pub related_record_id: Option<String>,
    pub related_entity_id: String,
    pub related_title: Option<String>,
    pub evidence: Option<String>,
    pub broken: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRecordDetail {
    pub record: RecordSummary,
    pub revisions: Vec<SearchRevisionRef>,
    pub assets: Vec<SearchAssetRef>,
    pub derivatives: Vec<SearchDerivativeRef>,
    pub relations: Vec<SearchRelationRef>,
    pub score_history: Vec<SearchScoreRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionStatus {
    pub state: String,
    pub schema_version: u32,
    pub built_at: Option<UtcTimestamp>,
    pub raw_items: u64,
    pub semantic_entries: u64,
    pub relations: u64,
    pub source_fingerprint: Option<String>,
}

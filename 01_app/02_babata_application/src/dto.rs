use babata_domain::{
    AssetAttachmentId, AssetId, AssetRole, BuildTarget, CandidateEnvelope, CandidateSummary,
    CollectionId, CollectionSessionId, ContentType, DerivativeId, DerivativeKind, DerivativeRef,
    FirstPartySemanticDefinition, HealthState, ItemId, LogicalPath, Metadata, PageCursor,
    PipelineId, ProcessJob, ProcessRun, ProcessingState, QueryFilter, RawState, RecordSummary,
    RelationKind, RevisionId, RouteCoverage, RunId, ScoreProfile, SemanticCandidatePackage,
    SemanticPayload, Sha256, SnapshotRef, SourceId, SourceKind, SourceRouteId,
    SuggestionDecisionKind, UtcTimestamp, ViewDescriptor, ViewId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CaptureTextCommand {
    pub provider: String,
    pub text: String,
    pub context: Option<String>,
    pub locator: Option<String>,
    pub native_id: Option<String>,
    pub identity: Option<String>,
    pub metadata: Metadata,
    pub source_published_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone)]
pub struct CaptureFileCommand {
    pub provider: String,
    pub path: String,
    pub context: Option<String>,
    pub locator: Option<String>,
    pub native_id: Option<String>,
    pub identity: Option<String>,
    pub metadata: Metadata,
    pub source_published_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureImportAsset {
    pub path: String,
    pub role: AssetRole,
}

#[derive(Debug, Clone)]
pub struct AttachRecoveredAssetsCommand {
    pub revision_id: RevisionId,
    pub assets: Vec<CaptureImportAsset>,
    pub reason: String,
    pub metadata: Metadata,
}

#[derive(Debug, Clone)]
pub struct CaptureImportCommand {
    pub provider: String,
    pub text: String,
    pub context: Option<String>,
    pub locator: Option<String>,
    pub native_id: Option<String>,
    pub identity: Option<String>,
    pub content_type: ContentType,
    pub metadata: Metadata,
    pub source_published_at: Option<UtcTimestamp>,
    pub assets: Vec<CaptureImportAsset>,
    pub route_evidence: Option<RouteEvidenceCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEvidenceCommand {
    pub route_id: SourceRouteId,
    pub authorization_id: String,
    pub source_reference: String,
    pub coverage: RouteCoverage,
}

#[derive(Debug, Clone)]
pub struct CaptureExportCommand(pub CaptureFileCommand);

#[derive(Debug, Clone)]
pub struct CreateNoteCommand {
    pub text: String,
    pub path: Option<String>,
    pub context: Option<String>,
    pub metadata: Metadata,
}

#[derive(Debug, Clone)]
pub struct ReviseCommand {
    pub parent: RevisionId,
    pub text: String,
    pub path: Option<String>,
    pub note: Option<String>,
    pub metadata: Metadata,
}

#[derive(Debug, Clone)]
pub struct AnnotateCommand {
    pub target_item: ItemId,
    pub target_revision: Option<RevisionId>,
    pub text: String,
    pub path: Option<String>,
    pub context: Option<String>,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureOutcome {
    pub operation_id: String,
    pub item_id: ItemId,
    pub revision_id: RevisionId,
    pub asset_ids: Vec<AssetId>,
    pub status: String,
    pub duplicate_of: Option<RevisionId>,
    pub reimported: bool,
    pub warnings: Vec<String>,
    pub record: Option<RecordDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordDetail {
    pub item_id: ItemId,
    pub source_id: SourceId,
    pub source_kind: SourceKind,
    pub provider: String,
    pub source_account_or_workspace: Option<String>,
    pub content_type: ContentType,
    pub source_native_id: Option<String>,
    pub source_locator: Option<String>,
    pub source_identity_key: Option<String>,
    pub source_published_at: Option<UtcTimestamp>,
    pub source_updated_at: Option<UtcTimestamp>,
    pub first_captured_at: UtcTimestamp,
    pub metadata: Metadata,
    pub collections: Vec<CollectionDetail>,
    pub revisions: Vec<RevisionDetail>,
    pub assets: Vec<AssetDetail>,
    pub asset_attachments: Vec<AssetAttachmentDetail>,
    pub relations: Vec<RelationDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionDetail {
    pub collection_id: CollectionId,
    pub parent_collection_id: Option<CollectionId>,
    pub native_id: Option<String>,
    pub locator: Option<String>,
    pub kind: String,
    pub title: Option<String>,
    pub metadata: Metadata,
    pub observed_at: UtcTimestamp,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionDetail {
    pub revision_id: RevisionId,
    pub parent_revision_id: Option<RevisionId>,
    pub kind: String,
    pub ordinal: u32,
    pub captured_at: UtcTimestamp,
    pub authored_at: Option<UtcTimestamp>,
    pub revision_note: Option<String>,
    pub raw_text: Option<String>,
    pub text_sha256: Option<String>,
    pub metadata: Metadata,
    pub state: RawState,
    pub created_at: UtcTimestamp,
    pub provenance: Option<CaptureProvenanceDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureProvenanceDetail {
    pub operation_id: String,
    pub source_native_id: Option<String>,
    pub source_locator: Option<String>,
    pub source_published_at: Option<UtcTimestamp>,
    pub metadata: Metadata,
    pub state: RawState,
    pub failure_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDetail {
    pub asset_id: AssetId,
    pub revision_id: RevisionId,
    pub role: AssetRole,
    pub logical_path: String,
    pub sha256: String,
    pub byte_size: u64,
    pub media_type: Option<String>,
    pub original_filename: Option<String>,
    pub state: RawState,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAttachmentDetail {
    pub operation_id: AssetAttachmentId,
    pub revision_id: RevisionId,
    pub asset_ids: Vec<AssetId>,
    pub reason: String,
    pub metadata: Metadata,
    pub state: RawState,
    pub failure_code: Option<String>,
    pub started_at: UtcTimestamp,
    pub completed_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationDetail {
    pub relation_id: babata_domain::RelationId,
    pub kind: RelationKind,
    pub from_item_id: ItemId,
    pub from_revision_id: Option<RevisionId>,
    pub to_item_id: ItemId,
    pub to_revision_id: Option<RevisionId>,
    pub metadata: Metadata,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnqueueProcessCommand {
    pub pipeline_id: PipelineId,
    pub revision_id: RevisionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub filter: QueryFilter,
    pub cursor: Option<PageCursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPage {
    pub records: Vec<RecordSummary>,
    pub next_cursor: Option<PageCursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteCollectCommand {
    pub route_id: SourceRouteId,
    pub source_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewBuildCommand {
    pub view_id: ViewId,
    pub target: BuildTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStatus {
    pub health: HealthState,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateCaptureCommand {
    pub candidate: CandidateEnvelope,
    pub assets: Vec<CaptureImportAsset>,
    pub route_evidence: Option<RouteEvidenceCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartCollectionCommand {
    pub route_id: SourceRouteId,
    pub source_reference: String,
    pub scope_description: String,
    pub authorisation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryCollectionItemCommand {
    pub session_id: CollectionSessionId,
    pub candidate_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelCollectionCommand {
    pub session_id: CollectionSessionId,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct DiscoveredCandidate {
    pub summary: CandidateSummary,
    pub prefetched: Option<CandidateEnvelope>,
}

#[derive(Debug, Clone)]
pub enum AcquisitionOutcome {
    Found {
        candidate: CandidateEnvelope,
        assets: Vec<CaptureImportAsset>,
    },
    Skipped {
        reason: String,
    },
    Inaccessible {
        reason: String,
    },
    Removed {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessJobOutcome {
    pub status: String,
    pub job: Option<ProcessJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDerivativeCommand {
    pub pipeline_id: PipelineId,
    pub revision_id: RevisionId,
    pub item_id: Option<ItemId>,
    pub input_sha256: Sha256,
    pub kind: DerivativeKind,
    pub provider: String,
    pub tool_or_model: Option<String>,
    pub tool_version: Option<String>,
    pub retry_of_run_id: Option<RunId>,
    pub params: Metadata,
    pub usage: Metadata,
    pub loss_notes: Option<String>,
    pub content_text: Option<String>,
    pub content_json: Option<String>,
    pub logical_path: Option<LogicalPath>,
    pub source_file: Option<String>,
    pub media_type: Option<String>,
    pub language: Option<String>,
    pub input_asset_id: Option<AssetId>,
    pub output_sha256: Option<Sha256>,
    pub derivative_loss_notes: Option<String>,
    pub derivative_metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterFailureCommand {
    pub pipeline_id: PipelineId,
    pub revision_id: RevisionId,
    pub item_id: Option<ItemId>,
    pub input_sha256: Sha256,
    pub kind: DerivativeKind,
    pub provider: String,
    pub tool_or_model: Option<String>,
    pub tool_version: Option<String>,
    pub retry_of_run_id: Option<RunId>,
    pub params: Metadata,
    pub error_code: String,
    pub error_message: Option<String>,
    pub loss_notes: Option<String>,
    pub input_asset_id: Option<AssetId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDerivativeOutcome {
    pub run_id: RunId,
    pub derivative_id: Option<DerivativeId>,
    pub pipeline_id: PipelineId,
    pub kind: Option<DerivativeKind>,
    pub state: ProcessingState,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowProcessRunOutcome {
    pub run: ProcessRun,
    pub derivatives: Vec<DerivativeRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeReviewContext {
    pub target: RecordDetail,
    pub target_revision_id: RevisionId,
    pub process_runs: Vec<ShowProcessRunOutcome>,
}

#[derive(Debug, Clone)]
pub struct IngestSemanticCandidateCommand {
    pub source_derivative_id: DerivativeId,
    pub source_output_sha256: Sha256,
    pub package: SemanticCandidatePackage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticIngestOutcome {
    pub suggestion_id: String,
    pub semantic_ids: Vec<String>,
    pub map_node_ids: Vec<String>,
    pub profile_id: String,
    pub review_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDigestAndIngestOutcome {
    pub run_id: RunId,
    pub derivative_id: DerivativeId,
    pub ingest: SemanticIngestOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSuggestionDetail {
    pub suggestion_id: String,
    pub source_item_id: ItemId,
    pub source_revision_id: RevisionId,
    pub source_derivative_id: DerivativeId,
    pub source_output_sha256: Sha256,
    pub provider: String,
    pub model: String,
    pub model_version: String,
    pub prompt_version: String,
    pub generated_at: UtcTimestamp,
    pub evidence_derivatives: Vec<babata_domain::DerivativeEvidence>,
    pub limitations: Vec<String>,
    pub review_state: String,
    pub downstream_eligibility: SuggestionDownstreamEligibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionDownstreamEligibility {
    pub eligible_uses: Vec<SuggestionDownstreamUse>,
    pub human_judgment: bool,
    pub confirmed_fact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionDownstreamUse {
    Search,
    Surfacing,
    RelationNavigation,
    SublibraryCandidate,
    OutputCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEntryDetail {
    pub semantic_id: String,
    pub kind: babata_domain::KnowledgeKind,
    pub realm: babata_domain::KnowledgeRealm,
    pub origin_kind: String,
    pub author: String,
    pub title: String,
    pub payload: SemanticPayload,
    pub map_nodes: Vec<MapNodeDetail>,
    pub tags: Vec<String>,
    pub dense_expressions: Vec<DenseExpressionDetail>,
    pub scores: Vec<RelevanceScoreDetail>,
    pub outgoing_relations: Vec<SemanticRelationDetail>,
    pub incoming_relations: Vec<SemanticRelationDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapNodeDetail {
    pub map_node_id: String,
    pub level: babata_domain::MapNodeLevel,
    pub canonical_key: String,
    pub name: String,
    pub lifecycle: babata_domain::MapNodeLifecycle,
    pub parent_node_ids: Vec<String>,
    pub child_node_ids: Vec<String>,
    pub tags: Vec<String>,
    pub semantic_ids: Vec<String>,
    pub scores: Vec<RelevanceScoreDetail>,
    pub node_events: Vec<MapNodeEventDetail>,
    pub edge_events: Vec<MapEdgeEventDetail>,
    pub assignment_events: Vec<SemanticMapAssignmentEventDetail>,
    pub tag_events: Vec<MapNodeTagEventDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapNodeEventDetail {
    pub event_id: String,
    pub kind: String,
    pub previous_name: Option<String>,
    pub current_name: Option<String>,
    pub merged_into_map_node_id: Option<String>,
    pub rationale: String,
    pub provenance_kind: String,
    pub author: String,
    pub suggestion_id: Option<String>,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapEdgeEventDetail {
    pub event_id: String,
    pub parent_node_id: String,
    pub child_node_id: String,
    pub kind: String,
    pub rationale: String,
    pub provenance_kind: String,
    pub author: String,
    pub suggestion_id: Option<String>,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMapAssignmentEventDetail {
    pub event_id: String,
    pub semantic_id: String,
    pub map_node_id: String,
    pub kind: String,
    pub rationale: String,
    pub provenance_kind: String,
    pub author: String,
    pub suggestion_id: Option<String>,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapNodeTagEventDetail {
    pub event_id: String,
    pub tag: String,
    pub kind: String,
    pub rationale: String,
    pub provenance_kind: String,
    pub author: String,
    pub suggestion_id: Option<String>,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenseExpressionDetail {
    pub expression_id: String,
    pub kind: babata_domain::DenseExpressionKind,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceScoreDetail {
    pub score_id: String,
    pub target_kind: babata_domain::RelevanceTargetKind,
    pub target_id: String,
    pub profile_id: String,
    pub interest: u8,
    pub strategy: u8,
    pub consensus: u8,
    pub weighted_score: u16,
    pub rationale: String,
    pub provenance_kind: String,
    pub author: String,
    pub suggestion_id: Option<String>,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRelationDetail {
    pub relation_id: String,
    pub from_semantic_id: String,
    pub kind: String,
    pub to_semantic_id: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionReviewDetail {
    pub review_id: String,
    pub decision: SuggestionDecisionKind,
    pub reason: Option<String>,
    pub first_party_item_id: Option<ItemId>,
    pub first_party_revision_id: Option<RevisionId>,
    pub reviewer: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCoreSnapshot {
    pub suggestion: ModelSuggestionDetail,
    pub entries: Vec<SemanticEntryDetail>,
    pub relations: Vec<SemanticRelationDetail>,
    pub reviews: Vec<SuggestionReviewDetail>,
}

#[derive(Debug, Clone)]
pub struct RecordSuggestionReviewCommand {
    pub suggestion_id: String,
    pub decision: SuggestionDecisionKind,
    pub reason: Option<String>,
    pub first_party_item_id: Option<ItemId>,
    pub first_party_revision_id: Option<RevisionId>,
    pub reviewer: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
pub struct CreateScoreProfileCommand {
    pub profile: ScoreProfile,
}

#[derive(Debug, Clone)]
pub struct CreateMapNodeCommand {
    pub level: babata_domain::MapNodeLevel,
    pub name: String,
    pub parent_node_ids: Vec<String>,
    pub tags: Vec<String>,
    pub rationale: String,
    pub author_kind: String,
    pub author: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
pub enum EvolveMapNodeAction {
    Rename { name: String },
    Deactivate,
    Merge { target_map_node_id: String },
}

#[derive(Debug, Clone)]
pub struct EvolveMapNodeCommand {
    pub map_node_id: String,
    pub action: EvolveMapNodeAction,
    pub rationale: String,
    pub author_kind: String,
    pub author: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Copy)]
pub enum AssignmentChange {
    Assign,
    Unassign,
}

#[derive(Debug, Clone)]
pub struct ChangeMapParentCommand {
    pub parent_map_node_id: String,
    pub child_map_node_id: String,
    pub change: AssignmentChange,
    pub rationale: String,
    pub author_kind: String,
    pub author: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
pub struct ChangeSemanticMapAssignmentCommand {
    pub semantic_id: String,
    pub map_node_id: String,
    pub change: AssignmentChange,
    pub rationale: String,
    pub author_kind: String,
    pub author: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
pub struct ChangeMapNodeTagCommand {
    pub map_node_id: String,
    pub tag: String,
    pub change: AssignmentChange,
    pub rationale: String,
    pub author_kind: String,
    pub author: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
pub struct RegisterFirstPartySemanticCommand {
    pub item_id: ItemId,
    pub revision_id: RevisionId,
    pub definition: FirstPartySemanticDefinition,
    pub author: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstPartySemanticOutcome {
    pub semantic_id: String,
    pub kind: babata_domain::KnowledgeKind,
    pub realm: babata_domain::KnowledgeRealm,
    pub origin_kind: String,
    pub first_party_item_id: ItemId,
    pub first_party_revision_id: RevisionId,
}

#[derive(Debug, Clone)]
pub struct RecordRelevanceScoreCommand {
    pub target_kind: babata_domain::RelevanceTargetKind,
    pub target_id: String,
    pub components: babata_domain::RelevanceComponents,
    pub author_kind: String,
    pub author: String,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone)]
pub struct DenseExpressionPreviewDocument {
    pub semantic_id: String,
    pub source_sha256: Sha256,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenseExpressionPreviewOutcome {
    pub semantic_id: String,
    pub logical_path: String,
    pub source_sha256: Sha256,
    pub output_sha256: Sha256,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewBuildOutcome {
    pub descriptor: ViewDescriptor,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupOutcome {
    pub snapshot: SnapshotRef,
}

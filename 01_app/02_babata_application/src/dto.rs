use babata_domain::{
    AssetId, AssetRole, BuildTarget, CandidateEnvelope, CandidateSummary, CollectionId,
    CollectionSessionId, ContentType, HealthState, ItemId, JobId, Metadata, PageCursor, PipelineId,
    QueryFilter, RawState, RecordSummary, RelationKind, RevisionId, RouteCoverage, SnapshotRef,
    SourceId, SourceKind, SourceRouteId, UtcTimestamp, ViewDescriptor, ViewId,
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
    pub content_type: ContentType,
    pub source_native_id: Option<String>,
    pub source_locator: Option<String>,
    pub source_identity_key: Option<String>,
    pub metadata: Metadata,
    pub collections: Vec<CollectionDetail>,
    pub revisions: Vec<RevisionDetail>,
    pub assets: Vec<AssetDetail>,
    pub relations: Vec<RelationDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionDetail {
    pub collection_id: CollectionId,
    pub native_id: Option<String>,
    pub kind: String,
    pub title: Option<String>,
    pub observed_at: UtcTimestamp,
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
    pub role: AssetRole,
    pub logical_path: String,
    pub sha256: String,
    pub byte_size: u64,
    pub media_type: Option<String>,
    pub original_filename: Option<String>,
    pub state: RawState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationDetail {
    pub kind: RelationKind,
    pub from_item_id: ItemId,
    pub from_revision_id: Option<RevisionId>,
    pub to_item_id: ItemId,
    pub to_revision_id: Option<RevisionId>,
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
    pub job_id: JobId,
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

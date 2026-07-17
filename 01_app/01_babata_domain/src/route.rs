use serde::{Deserialize, Serialize};

use crate::{CapabilityStatus, ContentType, ItemId, Metadata, RevisionId, Sha256, UtcTimestamp};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceRouteId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRouteDescriptor {
    pub id: SourceRouteId,
    pub provider: String,
    pub status: CapabilityStatus,
    pub activation_phase: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCoverage {
    pub metadata: bool,
    pub attachments: bool,
    pub revisions: bool,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEvidence {
    pub route_id: SourceRouteId,
    pub authorization_id: String,
    pub source_reference: String,
    pub item_id: ItemId,
    pub revision_id: RevisionId,
    pub coverage: RouteCoverage,
    pub reimported: bool,
    pub recorded_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateEnvelope {
    pub protocol_version: String,
    pub route_id: SourceRouteId,
    pub source_reference: String,
    pub content_type: ContentType,
    pub payload_sha256: Sha256,
    pub metadata: Metadata,
    pub payload: CandidatePayload,
    pub context: Option<String>,
    pub native_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CandidatePayload {
    Text { text: String },
}

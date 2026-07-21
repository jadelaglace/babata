use serde::{Deserialize, Serialize};

use crate::{
    CapabilityStatus, CommonSourceMetadata, ContentType, ItemId, Metadata, RevisionId, Sha256,
    UtcTimestamp,
};

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
    #[serde(default)]
    pub common_metadata: CommonSourceMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CandidatePayload {
    Text { text: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_candidate_envelope_without_common_metadata_still_decodes() {
        let envelope: CandidateEnvelope = serde_json::from_value(serde_json::json!({
            "protocolVersion": "1",
            "routeId": "source.fixture",
            "sourceReference": "https://example.test/item",
            "contentType": "document",
            "payloadSha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "metadata": {"unknown_provider_field": {"kept": true}},
            "payload": {"kind": "text", "text": "fixture"},
            "context": null,
            "nativeId": "fixture-1"
        }))
        .unwrap();
        assert_eq!(
            envelope.common_metadata.schema,
            crate::COMMON_SOURCE_METADATA_SCHEMA_V1
        );
        assert!(
            envelope
                .metadata
                .to_json()
                .contains("unknown_provider_field")
        );
    }
}

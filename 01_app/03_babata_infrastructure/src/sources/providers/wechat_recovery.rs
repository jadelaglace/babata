use std::{collections::HashMap, fs, path::PathBuf};

use babata_application::{
    AcquisitionOutcome, ApplicationError, CaptureImportAsset, DiscoveredCandidate,
    ports::SourceAdapterPort,
};
use babata_domain::{
    AssetRole, CandidateEnvelope, CandidateSummary, CapabilityStatus, CollectionSessionId,
    Metadata, RouteCoverage, Sha256, SourceRouteDescriptor, SourceRouteId,
};
use serde::Deserialize;
use serde_json::{Value, json};

pub const FAVORITES_ROUTE_ID: &str = "source.wechat_favorites";
pub const CHATS_ROUTE_ID: &str = "source.wechat_chats";
const HANDOFF_PROTOCOL: &str = "babata/wechat-recovery-handoff/1";
const INTERNAL_ASSETS: &str = "_babata_acquisition_assets";

#[derive(Debug, Deserialize)]
struct WechatRecoveryHandoff {
    protocol_version: String,
    provider: String,
    route_id: String,
    candidate: CandidateEnvelope,
}

#[derive(Debug, Deserialize)]
struct HandoffAsset {
    path: PathBuf,
    role: AssetRole,
    sha256: Sha256,
}

#[derive(Debug, Clone)]
pub struct WechatRecoveryAdapter {
    route_id: SourceRouteId,
    handoffs: HashMap<String, CandidateEnvelope>,
}

impl WechatRecoveryAdapter {
    pub fn favorites(paths: &[PathBuf]) -> Result<Self, ApplicationError> {
        Self::from_handoffs(FAVORITES_ROUTE_ID, paths)
    }

    pub fn chats(paths: &[PathBuf]) -> Result<Self, ApplicationError> {
        Self::from_handoffs(CHATS_ROUTE_ID, paths)
    }

    fn from_handoffs(route_id: &str, paths: &[PathBuf]) -> Result<Self, ApplicationError> {
        let mut handoffs = HashMap::new();
        for path in paths {
            let bytes = fs::read(path).map_err(|error| {
                ApplicationError::Asset(format!(
                    "unable to read WeChat recovery handoff: {:?}",
                    error.kind()
                ))
            })?;
            let handoff: WechatRecoveryHandoff = serde_json::from_slice(&bytes).map_err(|_| {
                ApplicationError::Conflict("WeChat recovery handoff is invalid JSON".to_owned())
            })?;
            if handoff.provider != "wechat" || handoff.protocol_version != HANDOFF_PROTOCOL {
                return Err(ApplicationError::Conflict(
                    "WeChat recovery handoff has an invalid provider or protocol".to_owned(),
                ));
            }
            if handoff.route_id != route_id {
                continue;
            }
            validate_envelope(route_id, &handoff.candidate)?;
            let native_id = handoff.candidate.native_id.clone().ok_or_else(|| {
                ApplicationError::Integrity("WeChat recovery handoff has no native ID".to_owned())
            })?;
            if handoffs
                .insert(native_id.clone(), handoff.candidate)
                .is_some()
            {
                return Err(ApplicationError::Conflict(format!(
                    "duplicate WeChat recovery handoff for {native_id}"
                )));
            }
        }
        Ok(Self {
            route_id: SourceRouteId(route_id.to_owned()),
            handoffs,
        })
    }

    fn envelope_for(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&CandidateEnvelope>,
    ) -> Result<CandidateEnvelope, ApplicationError> {
        let native_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("WeChat recovery candidate has no native ID".to_owned())
        })?;
        let envelope = self
            .handoffs
            .get(native_id)
            .or(prefetched)
            .cloned()
            .ok_or_else(|| {
                ApplicationError::Integrity(
                    "WeChat recovery candidate has no acquisition handoff".to_owned(),
                )
            })?;
        validate_envelope(&self.route_id.0, &envelope)?;
        if envelope.native_id.as_deref() != Some(native_id) {
            return Err(ApplicationError::Integrity(
                "WeChat recovery handoff does not match the selected candidate".to_owned(),
            ));
        }
        Ok(envelope)
    }
}

impl SourceAdapterPort for WechatRecoveryAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor(&self.route_id.0)
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        if !source_reference.starts_with("recovery:") {
            return Err(ApplicationError::Conflict(
                "WeChat recovery source must be recovery:<batch-id>".to_owned(),
            ));
        }
        if self.handoffs.is_empty() {
            return Err(ApplicationError::Conflict(
                "WeChat recovery collection requires at least one acquisition handoff".to_owned(),
            ));
        }
        let mut native_ids = self.handoffs.keys().cloned().collect::<Vec<_>>();
        native_ids.sort();
        native_ids
            .into_iter()
            .map(|native_id| {
                let envelope = self.handoffs.get(&native_id).cloned().ok_or_else(|| {
                    ApplicationError::Integrity("WeChat recovery handoff disappeared".to_owned())
                })?;
                let metadata: Value =
                    serde_json::from_str(&envelope.metadata.to_json()).map_err(|_| {
                        ApplicationError::Integrity("invalid handoff metadata".to_owned())
                    })?;
                let title = metadata
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                Ok(DiscoveredCandidate {
                    summary: CandidateSummary {
                        candidate_id: format!("wechat_recovery_{native_id}"),
                        session_id: session_id.clone(),
                        route_id: self.route_id.clone(),
                        source_native_id: Some(native_id),
                        title,
                        source_location: Some(envelope.source_reference.clone()),
                        hierarchy: vec!["WeChat".to_owned(), "P7 C0-A2 sample".to_owned()],
                        content_type: envelope.content_type,
                        source_updated_at: envelope.common_metadata.source_updated_at.clone(),
                        attachment_available: Some(true),
                        limitations: vec![
                            "candidate is limited to an explicitly selected local Recovery handoff"
                                .to_owned(),
                        ],
                        selection_capabilities: vec!["explicit_batch".to_owned()],
                        common_metadata: envelope.common_metadata.clone(),
                    }
                    .with_common_from_legacy(),
                    prefetched: Some(envelope),
                })
            })
            .collect()
    }

    fn collect(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&CandidateEnvelope>,
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let mut envelope = self.envelope_for(candidate, prefetched)?;
        let mut metadata: Value = serde_json::from_str(&envelope.metadata.to_json())
            .map_err(|_| ApplicationError::Integrity("invalid handoff metadata".to_owned()))?;
        let raw_assets = metadata
            .as_object_mut()
            .and_then(|object| object.remove(INTERNAL_ASSETS))
            .ok_or_else(|| {
                ApplicationError::Integrity(
                    "WeChat recovery handoff has no validated assets".to_owned(),
                )
            })?;
        let handoff_assets: Vec<HandoffAsset> =
            serde_json::from_value(raw_assets).map_err(|_| {
                ApplicationError::Integrity("WeChat recovery handoff assets are invalid".to_owned())
            })?;
        if handoff_assets.is_empty() {
            return Err(ApplicationError::Integrity(
                "WeChat recovery handoff has no validated assets".to_owned(),
            ));
        }
        let assets = if requested_attachments {
            handoff_assets
                .into_iter()
                .map(|asset| CaptureImportAsset {
                    path: asset.path.to_string_lossy().into_owned(),
                    role: asset.role,
                    expected_sha256: Some(asset.sha256),
                    ..CaptureImportAsset::default()
                })
                .collect()
        } else {
            handoff_assets
                .into_iter()
                .filter(|asset| asset.role == AssetRole::Export)
                .map(|asset| CaptureImportAsset {
                    path: asset.path.to_string_lossy().into_owned(),
                    role: asset.role,
                    expected_sha256: Some(asset.sha256),
                    ..CaptureImportAsset::default()
                })
                .collect()
        };
        envelope.metadata = Metadata::parse(&metadata.to_string())?;
        Ok(AcquisitionOutcome::Found {
            candidate: Box::new(envelope),
            assets,
        })
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: true,
            limitations: vec![
                "only the explicitly selected C0-A2 Recovery sample is registered".to_owned(),
                "unselected WeChat stage-one data remains C0-A1 outside the formal database"
                    .to_owned(),
            ],
        }
    }
}

pub fn favorites_descriptor() -> SourceRouteDescriptor {
    descriptor(FAVORITES_ROUTE_ID)
}

pub fn chats_descriptor() -> SourceRouteDescriptor {
    descriptor(CHATS_ROUTE_ID)
}

fn descriptor(route_id: &str) -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(route_id.to_owned()),
        provider: "wechat".to_owned(),
        status: CapabilityStatus::Enabled,
        activation_phase: "P7".to_owned(),
    }
}

fn validate_envelope(route_id: &str, envelope: &CandidateEnvelope) -> Result<(), ApplicationError> {
    if envelope.protocol_version != "1" || envelope.route_id.0 != route_id {
        return Err(ApplicationError::Conflict(
            "WeChat recovery handoff candidate has an invalid protocol or route".to_owned(),
        ));
    }
    let babata_domain::CandidatePayload::Text { text } = &envelope.payload;
    if Sha256::of_bytes(text.as_bytes()) != envelope.payload_sha256 {
        return Err(ApplicationError::Integrity(
            "WeChat recovery handoff payload hash is invalid".to_owned(),
        ));
    }
    let metadata: Value = serde_json::from_str(&envelope.metadata.to_json())
        .map_err(|_| ApplicationError::Integrity("invalid handoff metadata".to_owned()))?;
    if metadata.get("sovereignty_depth") != Some(&json!("C0-A2"))
        || metadata.get("management_readiness") != Some(&json!("prepared"))
        || metadata
            .get("content_fingerprint")
            .and_then(Value::as_str)
            .is_none()
        || metadata
            .get(INTERNAL_ASSETS)
            .and_then(Value::as_array)
            .is_none()
    {
        return Err(ApplicationError::Integrity(
            "WeChat recovery handoff is not a prepared C0-A2 candidate".to_owned(),
        ));
    }
    Ok(())
}

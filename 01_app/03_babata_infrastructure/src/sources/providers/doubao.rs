use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use babata_application::{
    AcquisitionOutcome, ApplicationError, CaptureImportAsset, DiscoveredCandidate,
    ports::SourceAdapterPort,
};
use babata_domain::{
    AssetRole, CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus,
    CollectionSessionId, ContentType, Metadata, RouteCoverage, Sha256, SourceRouteDescriptor,
    SourceRouteId, UtcTimestamp,
};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const ROUTE_ID: &str = "source.doubao";
const ADAPTER_VERSION: &str = "doubao-structured/3";
const HANDOFF_PROTOCOL_VERSION: &str = "1";
const HANDOFF_ROUTE: &str = "chrome_native";
const INTERNAL_ASSETS: &str = "_doubao_acquisition_assets";
const INTERNAL_ROUTE: &str = "_doubao_acquisition_route";

#[derive(Debug, PartialEq, Eq)]
enum DoubaoScope {
    Recent(usize),
    Conversations(Vec<String>),
    Conversation(String),
}

#[derive(Debug, Default, Clone)]
pub struct DoubaoOpenCliAdapter {
    handoffs: HashMap<String, CandidateEnvelope>,
}

#[derive(Debug, Deserialize)]
struct DoubaoAcquisitionHandoff {
    protocol_version: String,
    provider: String,
    acquisition_route: String,
    conversation_id: String,
    source_url: String,
    conversation_info: Value,
    messages: Vec<Value>,
    has_more: bool,
    message_cursor: String,
    assets: Vec<DoubaoHandoffAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct DoubaoHandoffAsset {
    path: String,
    file_name: String,
    byte_size: u64,
    md5: String,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclaredAttachment {
    file_name: String,
    byte_size: u64,
    md5: String,
}

impl DoubaoOpenCliAdapter {
    pub fn from_acquisition_handoffs(paths: &[PathBuf]) -> Result<Self, ApplicationError> {
        let mut handoffs = HashMap::new();
        for path in paths {
            let (conversation_id, envelope) = read_acquisition_handoff(path)?;
            if handoffs.insert(conversation_id.clone(), envelope).is_some() {
                return Err(ApplicationError::Conflict(format!(
                    "duplicate Doubao acquisition handoff for conversation {conversation_id}"
                )));
            }
        }
        Ok(Self { handoffs })
    }
}

impl SourceAdapterPort for DoubaoOpenCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        SourceRouteDescriptor {
            id: SourceRouteId(ROUTE_ID.to_owned()),
            provider: "doubao".to_owned(),
            status: CapabilityStatus::Enabled,
            activation_phase: "P7".to_owned(),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let limit = match parse_scope(source_reference)? {
            DoubaoScope::Conversation(conversation_id) => {
                if let Some(envelope) = self.handoffs.get(&conversation_id) {
                    return handoff_candidate(session_id, envelope)
                        .map(|candidate| vec![candidate]);
                }
                if !self.handoffs.is_empty() {
                    return Err(ApplicationError::Conflict(format!(
                        "Doubao acquisition handoff does not match conversation {conversation_id}"
                    )));
                }
                return Ok(vec![explicit_candidate(
                    session_id,
                    conversation_id,
                    "Explicit conversation",
                    "single",
                )]);
            }
            DoubaoScope::Conversations(conversation_ids) => {
                if !self.handoffs.is_empty() {
                    return conversation_ids
                        .iter()
                        .map(|conversation_id| {
                            self.handoffs.get(conversation_id).ok_or_else(|| {
                                ApplicationError::Conflict(format!(
                                    "Doubao acquisition handoff is missing conversation {conversation_id}"
                                ))
                            })
                        })
                        .map(|envelope| {
                            envelope.and_then(|envelope| handoff_candidate(session_id, envelope))
                        })
                        .collect();
                }
                return Ok(conversation_ids
                    .into_iter()
                    .map(|conversation_id| {
                        explicit_candidate(session_id, conversation_id, "Explicit batch", "batch")
                    })
                    .collect());
            }
            DoubaoScope::Recent(limit) => {
                if !self.handoffs.is_empty() {
                    return Err(ApplicationError::Conflict(
                        "Doubao acquisition handoffs require an explicit conversation scope"
                            .to_owned(),
                    ));
                }
                limit
            }
        };
        let output = run_opencli(&[
            "doubao",
            "history-full",
            "--limit",
            &limit.to_string(),
            "--window",
            "background",
            "--site-session",
            "persistent",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ])?;
        let rows = output.as_array().ok_or_else(|| {
            ApplicationError::Integrity("OpenCLI Doubao history-full was not an array".to_owned())
        })?;
        rows.iter()
            .map(|row| {
                let conversation_id = required_string(row, "Id")?;
                let title = required_string(row, "Title")?;
                let url = required_string(row, "Url")?;
                Ok(DiscoveredCandidate {
                    summary: CandidateSummary {
                        candidate_id: format!("doubao_{conversation_id}"),
                        session_id: session_id.clone(),
                        route_id: SourceRouteId(ROUTE_ID.to_owned()),
                        source_native_id: Some(conversation_id),
                        title: Some(title.clone()),
                        source_location: Some(url),
                        hierarchy: vec![
                            "Doubao".to_owned(),
                            "Recent conversations".to_owned(),
                            title,
                        ],
                        content_type: ContentType::Document,
                        source_updated_at: unix_seconds_timestamp(row, "UpdatedAt")?,
                        attachment_available: None,
                        limitations: vec![
                            "candidate discovery is bounded to the requested recent window"
                                .to_owned(),
                            "history metadata does not declare message attachments".to_owned(),
                        ],
                        selection_capabilities: vec![
                            "single".to_owned(),
                            "visible_set".to_owned(),
                            "recent_count".to_owned(),
                        ],
                        common_metadata: babata_domain::CommonSourceMetadata::default(),
                    }
                    .with_common_from_legacy(),
                    prefetched: None,
                })
            })
            .collect()
    }

    #[allow(clippy::too_many_lines)]
    fn collect(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&CandidateEnvelope>,
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let conversation_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Doubao candidate has no conversation ID".to_owned())
        })?;
        if let Some(envelope) = self.handoffs.get(conversation_id) {
            return collect_handoff(candidate, envelope, requested_attachments);
        }
        if !self.handoffs.is_empty() {
            return Err(ApplicationError::Conflict(format!(
                "Doubao acquisition handoff does not match conversation {conversation_id}"
            )));
        }
        if let Some(envelope) = prefetched.filter(|envelope| is_handoff_envelope(envelope)) {
            return collect_handoff(candidate, envelope, requested_attachments);
        }
        let output = run_opencli(&[
            "doubao",
            "detail-full",
            conversation_id,
            "--window",
            "background",
            "--site-session",
            "persistent",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ])?;
        let rows = output.as_array().ok_or_else(|| {
            ApplicationError::Integrity("OpenCLI Doubao detail-full was not an array".to_owned())
        })?;
        let row = rows.first().ok_or_else(|| {
            ApplicationError::Integrity("Doubao returned no structured conversation".to_owned())
        })?;
        if row.get("HasMore").and_then(Value::as_bool) != Some(false) {
            return Err(ApplicationError::Integrity(
                "Doubao message pagination was incomplete; C0 was not written".to_owned(),
            ));
        }
        validate_opencli_attachment_coverage(row, requested_attachments)?;
        let info = row
            .get("Info")
            .filter(|value| value.is_object())
            .ok_or_else(|| {
                ApplicationError::Integrity("Doubao detail-full has no Info object".to_owned())
            })?;
        let messages = row
            .get("Messages")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ApplicationError::Integrity("Doubao detail-full has no Messages array".to_owned())
            })?;
        let title = info
            .pointer("/conversation_info/name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .or_else(|| candidate.title.clone());
        let payload = serde_json::to_string_pretty(&serde_json::json!({
            "platform": "doubao",
            "conversation_id": conversation_id,
            "title": title,
            "conversation_info": info,
            "messages": messages,
        }))
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        let content_fingerprint =
            doubao_content_fingerprint(conversation_id, title.as_deref(), messages, &[]);
        let metadata = Metadata::parse(
            &serde_json::json!({
                "title": title,
                "conversation_id": conversation_id,
                "message_count": messages.len(),
                "message_cursor": row.get("MessageCursor").and_then(Value::as_str),
                "attachment_key_count": row.get("AttachmentKeyCount").and_then(Value::as_u64),
                "media_key_count": row.get("MediaKeyCount").and_then(Value::as_u64),
                "response_bytes": row.get("ResponseBytes").and_then(Value::as_u64),
                "content_fingerprint": content_fingerprint.as_str(),
                "adapter_version": ADAPTER_VERSION,
                "structured_page_response": true,
                "complete_message_chain": true,
                "attachments_covered": false,
            })
            .to_string(),
        )?;
        Ok(AcquisitionOutcome::Found {
            candidate: Box::new(CandidateEnvelope {
                protocol_version: "1".to_owned(),
                route_id: SourceRouteId(ROUTE_ID.to_owned()),
                source_reference: candidate
                    .source_location
                    .clone()
                    .unwrap_or_else(|| format!("https://www.doubao.com/chat/{conversation_id}")),
                content_type: ContentType::Document,
                payload_sha256: Sha256::of_bytes(payload.as_bytes()),
                metadata,
                payload: CandidatePayload::Text { text: payload },
                context: Some("Doubao / Recent conversations".to_owned()),
                native_id: Some(conversation_id.to_owned()),
                common_metadata: candidate.effective_common_metadata(),
            }),
            assets: Vec::new(),
        })
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: true,
            limitations: vec![
                "the complete conversation/info and chain/single structures are preserved"
                    .to_owned(),
                "conversation chains reporting has_more are rejected until pagination is proven"
                    .to_owned(),
                "original attachments are covered only when a validated Chrome-native acquisition handoff is supplied"
                    .to_owned(),
                "OpenCLI remains the structured-text fallback and does not download attachment binaries"
                    .to_owned(),
            ],
        }
    }
}

#[allow(clippy::too_many_lines)]
fn read_acquisition_handoff(path: &Path) -> Result<(String, CandidateEnvelope), ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::Asset(format!(
            "unable to read Doubao acquisition handoff: {:?}",
            error.kind()
        ))
    })?;
    let handoff: DoubaoAcquisitionHandoff = serde_json::from_slice(&bytes).map_err(|_| {
        ApplicationError::Conflict("Doubao acquisition handoff is invalid JSON".to_owned())
    })?;
    if handoff.protocol_version != HANDOFF_PROTOCOL_VERSION {
        return Err(ApplicationError::Conflict(
            "unsupported Doubao acquisition handoff protocol version".to_owned(),
        ));
    }
    if handoff.provider != "doubao" || handoff.acquisition_route != HANDOFF_ROUTE {
        return Err(ApplicationError::Conflict(
            "Doubao acquisition handoff has an invalid provider or route".to_owned(),
        ));
    }
    if !is_conversation_id(&handoff.conversation_id) {
        return Err(ApplicationError::Conflict(
            "Doubao acquisition handoff has an invalid conversation ID".to_owned(),
        ));
    }
    let expected_url = format!("https://www.doubao.com/chat/{}", handoff.conversation_id);
    if handoff.source_url != expected_url {
        return Err(ApplicationError::Conflict(
            "Doubao acquisition handoff source URL does not match its conversation ID".to_owned(),
        ));
    }
    if handoff.has_more {
        return Err(ApplicationError::Integrity(
            "Doubao acquisition handoff has incomplete message pagination; C0 was not written"
                .to_owned(),
        ));
    }
    if handoff.message_cursor.trim().is_empty() {
        return Err(ApplicationError::Integrity(
            "Doubao acquisition handoff has no message cursor".to_owned(),
        ));
    }
    if handoff
        .conversation_info
        .get("conversation_id")
        .and_then(Value::as_str)
        .is_some_and(|value| value != handoff.conversation_id)
    {
        return Err(ApplicationError::Integrity(
            "Doubao conversation info does not match the handoff conversation ID".to_owned(),
        ));
    }
    let title = handoff
        .conversation_info
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            ApplicationError::Integrity("Doubao acquisition handoff has no title".to_owned())
        })?
        .to_owned();
    let declared = validate_message_chain(&handoff.conversation_id, &handoff.messages)?;
    let assets = validate_handoff_assets(&declared, handoff.assets)?;
    let stable_assets = stable_asset_values(&assets);
    let content_fingerprint = doubao_content_fingerprint(
        &handoff.conversation_id,
        Some(&title),
        &handoff.messages,
        &stable_assets,
    );
    let payload = serde_json::to_string_pretty(&serde_json::json!({
        "platform": "doubao",
        "acquisition_route": HANDOFF_ROUTE,
        "conversation_id": handoff.conversation_id,
        "title": title,
        "conversation_info": handoff.conversation_info,
        "messages": handoff.messages,
        "has_more": false,
        "message_cursor": handoff.message_cursor,
    }))
    .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let metadata = Metadata::parse(
        &serde_json::json!({
            "title": title,
            "conversation_id": handoff.conversation_id,
            "message_count": handoff.messages.len(),
            "message_cursor": handoff.message_cursor,
            "attachment_declared_count": declared.len(),
            "attachment_sha256": stable_assets,
            "content_fingerprint": content_fingerprint.as_str(),
            "adapter_version": ADAPTER_VERSION,
            "acquisition_route": HANDOFF_ROUTE,
            "structured_page_response": true,
            "complete_message_chain": true,
            "attachments_covered": true,
            INTERNAL_ROUTE: HANDOFF_ROUTE,
            INTERNAL_ASSETS: assets,
        })
        .to_string(),
    )?;
    let conversation_id = handoff.conversation_id;
    Ok((
        conversation_id.clone(),
        CandidateEnvelope {
            protocol_version: HANDOFF_PROTOCOL_VERSION.to_owned(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_reference: expected_url,
            content_type: ContentType::Document,
            payload_sha256: Sha256::of_bytes(payload.as_bytes()),
            metadata,
            payload: CandidatePayload::Text { text: payload },
            context: Some("Doubao / Chrome-native acquisition".to_owned()),
            native_id: Some(conversation_id),
            common_metadata: babata_domain::CommonSourceMetadata::default(),
        },
    ))
}

fn handoff_candidate(
    session_id: &CollectionSessionId,
    envelope: &CandidateEnvelope,
) -> Result<DiscoveredCandidate, ApplicationError> {
    let payload = handoff_payload(envelope)?;
    let conversation_id = required_string(&payload, "conversation_id")?;
    let title = required_string(&payload, "title")?;
    let attachment_count = internal_assets(&envelope.metadata)?.len();
    Ok(DiscoveredCandidate {
        summary: CandidateSummary {
            candidate_id: format!("doubao_{conversation_id}"),
            session_id: session_id.clone(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_native_id: Some(conversation_id),
            title: Some(title.clone()),
            source_location: Some(envelope.source_reference.clone()),
            hierarchy: vec!["Doubao".to_owned(), "Chrome acquisition".to_owned(), title],
            content_type: ContentType::Document,
            source_updated_at: None,
            attachment_available: Some(attachment_count > 0),
            limitations: Vec::new(),
            selection_capabilities: vec!["single".to_owned(), "explicit_id".to_owned()],
            common_metadata: envelope.common_metadata.clone(),
        }
        .with_common_from_legacy(),
        prefetched: Some(envelope.clone()),
    })
}

fn collect_handoff(
    candidate: &CandidateSummary,
    envelope: &CandidateEnvelope,
    requested_attachments: bool,
) -> Result<AcquisitionOutcome, ApplicationError> {
    let conversation_id = candidate.source_native_id.as_deref().ok_or_else(|| {
        ApplicationError::Integrity("Doubao candidate has no conversation ID".to_owned())
    })?;
    if envelope.route_id.0 != ROUTE_ID
        || envelope.native_id.as_deref() != Some(conversation_id)
        || envelope.source_reference != format!("https://www.doubao.com/chat/{conversation_id}")
        || candidate.source_location.as_deref() != Some(envelope.source_reference.as_str())
    {
        return Err(ApplicationError::Integrity(
            "Doubao acquisition handoff does not match the selected candidate".to_owned(),
        ));
    }
    let payload = handoff_payload(envelope)?;
    if payload.get("has_more").and_then(Value::as_bool) != Some(false) {
        return Err(ApplicationError::Integrity(
            "Doubao acquisition handoff has incomplete message pagination; C0 was not written"
                .to_owned(),
        ));
    }
    let messages = payload
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ApplicationError::Integrity(
                "Doubao acquisition handoff has no Messages array".to_owned(),
            )
        })?;
    let title = payload
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let declared = validate_message_chain(conversation_id, messages)?;
    let handoff_assets = validate_handoff_assets(&declared, internal_assets(&envelope.metadata)?)?;
    let stable_assets = stable_asset_values(&handoff_assets);
    let expected_fingerprint =
        doubao_content_fingerprint(conversation_id, title, messages, &stable_assets);
    let mut metadata = metadata_object(&envelope.metadata)?;
    if metadata.get("content_fingerprint").and_then(Value::as_str)
        != Some(expected_fingerprint.as_str())
    {
        return Err(ApplicationError::Integrity(
            "Doubao acquisition handoff fingerprint is invalid".to_owned(),
        ));
    }
    metadata.remove(INTERNAL_ASSETS);
    metadata.remove(INTERNAL_ROUTE);
    metadata.insert(
        "downloaded_asset_count".to_owned(),
        Value::from(if requested_attachments {
            handoff_assets.len()
        } else {
            0
        }),
    );
    metadata.insert(
        "attachments_covered".to_owned(),
        Value::Bool(requested_attachments || declared.is_empty()),
    );
    let CandidatePayload::Text { text } = &envelope.payload;
    if Sha256::of_bytes(text.as_bytes()) != envelope.payload_sha256 {
        return Err(ApplicationError::Integrity(
            "Doubao acquisition handoff payload hash is invalid".to_owned(),
        ));
    }
    let assets = if requested_attachments {
        handoff_assets
            .into_iter()
            .map(|asset| {
                Ok(CaptureImportAsset {
                    path: asset.path,
                    role: AssetRole::Attachment,
                    expected_sha256: Some(Sha256::parse(asset.sha256)?),
                    ..CaptureImportAsset::default()
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?
    } else {
        Vec::new()
    };
    let mut clean = envelope.clone();
    clean.metadata = Metadata::parse(&Value::Object(metadata).to_string())?;
    Ok(AcquisitionOutcome::Found {
        candidate: Box::new(clean),
        assets,
    })
}

fn is_handoff_envelope(envelope: &CandidateEnvelope) -> bool {
    metadata_object(&envelope.metadata)
        .ok()
        .and_then(|metadata| metadata.get(INTERNAL_ROUTE).cloned())
        .and_then(|value| value.as_str().map(str::to_owned))
        .as_deref()
        == Some(HANDOFF_ROUTE)
}

fn handoff_payload(envelope: &CandidateEnvelope) -> Result<Value, ApplicationError> {
    let CandidatePayload::Text { text } = &envelope.payload;
    serde_json::from_str(text).map_err(|_| {
        ApplicationError::Integrity("Doubao acquisition payload is invalid JSON".to_owned())
    })
}

fn metadata_object(metadata: &Metadata) -> Result<Map<String, Value>, ApplicationError> {
    serde_json::from_str::<Value>(&metadata.to_json())
        .ok()
        .and_then(|value| value.as_object().cloned())
        .ok_or_else(|| ApplicationError::Integrity("Doubao metadata is invalid".to_owned()))
}

fn internal_assets(metadata: &Metadata) -> Result<Vec<DoubaoHandoffAsset>, ApplicationError> {
    let object = metadata_object(metadata)?;
    serde_json::from_value(object.get(INTERNAL_ASSETS).cloned().ok_or_else(|| {
        ApplicationError::Integrity("Doubao acquisition handoff has no validated assets".to_owned())
    })?)
    .map_err(|_| {
        ApplicationError::Integrity("Doubao acquisition asset metadata is invalid".to_owned())
    })
}

fn validate_message_chain(
    conversation_id: &str,
    messages: &[Value],
) -> Result<Vec<DeclaredAttachment>, ApplicationError> {
    if messages.is_empty() {
        return Err(ApplicationError::Integrity(
            "Doubao acquisition handoff has no messages".to_owned(),
        ));
    }
    let mut message_ids = HashSet::new();
    let mut attachments = Vec::new();
    for message in messages {
        if required_string(message, "conversation_id")? != conversation_id {
            return Err(ApplicationError::Integrity(
                "Doubao message belongs to another conversation".to_owned(),
            ));
        }
        let message_id = required_string(message, "message_id")?;
        if !message_ids.insert(message_id) {
            return Err(ApplicationError::Integrity(
                "Doubao acquisition handoff contains duplicate message IDs".to_owned(),
            ));
        }
        if message.get("content_type").and_then(Value::as_u64) == Some(20) {
            let raw = required_string(message, "content")?;
            let content: Value = serde_json::from_str(&raw).map_err(|_| {
                ApplicationError::Integrity(
                    "Doubao attachment message contains invalid JSON".to_owned(),
                )
            })?;
            let entities = content
                .get("entities")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    ApplicationError::Integrity(
                        "Doubao attachment message has no entities".to_owned(),
                    )
                })?;
            for file in entities
                .iter()
                .filter_map(|entity| entity.pointer("/entity_content/file"))
            {
                let file_name = required_string(file, "file_name")?;
                let md5 = required_string(file, "md5")?.to_ascii_lowercase();
                let byte_size = file.get("size").and_then(Value::as_u64).ok_or_else(|| {
                    ApplicationError::Integrity(
                        "Doubao attachment metadata has no valid size".to_owned(),
                    )
                })?;
                if byte_size == 0
                    || md5.len() != 32
                    || !md5.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err(ApplicationError::Integrity(
                        "Doubao attachment metadata has an invalid size or MD5".to_owned(),
                    ));
                }
                attachments.push(DeclaredAttachment {
                    file_name,
                    byte_size,
                    md5,
                });
            }
        }
    }
    let unique = attachments
        .iter()
        .map(|attachment| attachment.file_name.as_str())
        .collect::<HashSet<_>>();
    if unique.len() != attachments.len() {
        return Err(ApplicationError::Integrity(
            "Doubao acquisition handoff contains duplicate attachment names".to_owned(),
        ));
    }
    Ok(attachments)
}

fn validate_handoff_assets(
    declared: &[DeclaredAttachment],
    mut assets: Vec<DoubaoHandoffAsset>,
) -> Result<Vec<DoubaoHandoffAsset>, ApplicationError> {
    if declared.len() != assets.len() {
        return Err(ApplicationError::Integrity(format!(
            "Doubao attachment coverage is incomplete: declared {}, received {}; C0 was not written",
            declared.len(),
            assets.len()
        )));
    }
    assets.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    if assets
        .windows(2)
        .any(|pair| pair[0].file_name == pair[1].file_name)
    {
        return Err(ApplicationError::Integrity(
            "Doubao acquisition handoff contains duplicate asset names".to_owned(),
        ));
    }
    for asset in &mut assets {
        let expected = declared
            .iter()
            .find(|expected| expected.file_name == asset.file_name)
            .ok_or_else(|| {
                ApplicationError::Integrity(
                    "Doubao acquisition asset is not declared by the conversation".to_owned(),
                )
            })?;
        asset.md5.make_ascii_lowercase();
        asset.sha256.make_ascii_lowercase();
        if asset.byte_size != expected.byte_size || asset.md5 != expected.md5 {
            return Err(ApplicationError::Integrity(
                "Doubao acquisition asset metadata does not match the conversation".to_owned(),
            ));
        }
        let path = Path::new(&asset.path);
        if !path.is_absolute()
            || path.file_name().and_then(|value| value.to_str()) != Some(asset.file_name.as_str())
            || path
                .extension()
                .and_then(|value| value.to_str())
                .is_none_or(|value| !value.eq_ignore_ascii_case("docx"))
        {
            return Err(ApplicationError::Integrity(
                "Doubao acquisition asset path or filename is invalid".to_owned(),
            ));
        }
        let bytes = fs::read(path).map_err(|error| {
            ApplicationError::Asset(format!(
                "unable to read Doubao acquisition asset: {:?}",
                error.kind()
            ))
        })?;
        if bytes.len() as u64 != asset.byte_size
            || !bytes.starts_with(b"PK\x03\x04")
            || format!("{:x}", Md5::digest(&bytes)) != asset.md5
            || Sha256::of_bytes(&bytes).as_str() != asset.sha256
        {
            return Err(ApplicationError::Integrity(
                "Doubao acquisition asset bytes do not match the declared original; C0 was not written"
                    .to_owned(),
            ));
        }
        Sha256::parse(&asset.sha256)?;
    }
    Ok(assets)
}

fn stable_asset_values(assets: &[DoubaoHandoffAsset]) -> Vec<Value> {
    assets
        .iter()
        .map(|asset| {
            serde_json::json!({
                "file_name": asset.file_name,
                "byte_size": asset.byte_size,
                "sha256": asset.sha256,
            })
        })
        .collect()
}

fn parse_scope(source_reference: &str) -> Result<DoubaoScope, ApplicationError> {
    if let Some(raw) = source_reference.strip_prefix("recent:") {
        let limit = raw
            .parse::<usize>()
            .map_err(|_| ApplicationError::Conflict("invalid Doubao recent count".to_owned()))?;
        if !(1..=20).contains(&limit) {
            return Err(ApplicationError::Conflict(
                "Doubao recent count must be between 1 and 20".to_owned(),
            ));
        }
        return Ok(DoubaoScope::Recent(limit));
    }
    if let Some(raw) = source_reference.strip_prefix("conversations:") {
        let conversation_ids = raw
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if !(1..=20).contains(&conversation_ids.len()) {
            return Err(ApplicationError::Conflict(
                "Doubao explicit batch must contain between 1 and 20 conversation IDs".to_owned(),
            ));
        }
        if conversation_ids
            .iter()
            .any(|conversation_id| !is_conversation_id(conversation_id))
        {
            return Err(ApplicationError::Conflict(
                "Doubao explicit batch contains an invalid conversation ID".to_owned(),
            ));
        }
        let unique = conversation_ids.iter().collect::<HashSet<_>>();
        if unique.len() != conversation_ids.len() {
            return Err(ApplicationError::Conflict(
                "Doubao explicit batch contains duplicate conversation IDs".to_owned(),
            ));
        }
        return Ok(DoubaoScope::Conversations(conversation_ids));
    }
    if let Some(conversation_id) = source_reference.strip_prefix("conversation:")
        && is_conversation_id(conversation_id)
    {
        return Ok(DoubaoScope::Conversation(conversation_id.to_owned()));
    }
    Err(ApplicationError::Conflict(
        "Doubao scope must be recent:<count>, conversation:<id>, or conversations:<id,...>; account-wide all is never implicit"
            .to_owned(),
    ))
}

fn is_conversation_id(value: &str) -> bool {
    value.len() >= 8 && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn explicit_candidate(
    session_id: &CollectionSessionId,
    conversation_id: String,
    hierarchy_scope: &str,
    selection_kind: &str,
) -> DiscoveredCandidate {
    let title = format!("Doubao conversation {conversation_id}");
    DiscoveredCandidate {
        summary: CandidateSummary {
            candidate_id: format!("doubao_{conversation_id}"),
            session_id: session_id.clone(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_native_id: Some(conversation_id.clone()),
            title: Some(title.clone()),
            source_location: Some(format!("https://www.doubao.com/chat/{conversation_id}")),
            hierarchy: vec!["Doubao".to_owned(), hierarchy_scope.to_owned(), title],
            content_type: ContentType::Document,
            source_updated_at: None,
            attachment_available: None,
            limitations: vec![
                "the conversation was explicitly discovered in the signed-in Chrome sidebar"
                    .to_owned(),
                "history metadata does not declare message attachments".to_owned(),
            ],
            selection_capabilities: vec![selection_kind.to_owned(), "explicit_id".to_owned()],
            common_metadata: babata_domain::CommonSourceMetadata::default(),
        }
        .with_common_from_legacy(),
        prefetched: None,
    }
}

fn doubao_content_fingerprint(
    conversation_id: &str,
    title: Option<&str>,
    messages: &[Value],
    assets: &[Value],
) -> Sha256 {
    let stable_messages = messages
        .iter()
        .map(stable_message_value)
        .collect::<Vec<_>>();
    Sha256::of_bytes(
        serde_json::json!({
            "conversation_id": conversation_id,
            "title": title,
            "messages": stable_messages,
            "assets": assets,
        })
        .to_string()
        .as_bytes(),
    )
}

fn stable_message_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter(|(key, _)| key.as_str() != "block_id")
                .map(|(key, value)| {
                    let stable = if matches!(key.as_str(), "info" | "tag_info") {
                        stable_embedded_json(value)
                    } else {
                        stable_message_value(value)
                    };
                    (key.clone(), stable)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(stable_message_value).collect()),
        Value::String(value) => Value::String(stable_url(value)),
        _ => value.clone(),
    }
}

fn stable_embedded_json(value: &Value) -> Value {
    let Value::String(raw) = value else {
        return stable_message_value(value);
    };
    serde_json::from_str::<Value>(raw)
        .map(|parsed| stable_message_value(&parsed).to_string())
        .map_or_else(|_| Value::String(stable_url(raw)), Value::String)
}

fn stable_url(value: &str) -> String {
    const SIGNED_HOST: &str = "flow-imagex-sign.byteimg.com";
    if !value.starts_with("https://") && !value.starts_with("http://") {
        return value.to_owned();
    }
    let Some(host_at) = value.find(SIGNED_HOST) else {
        return value.to_owned();
    };
    let path_at = host_at + SIGNED_HOST.len();
    let path = value[path_at..].split('?').next().unwrap_or_default();
    format!("https://{SIGNED_HOST}{path}")
}

fn run_opencli(args: &[&str]) -> Result<Value, ApplicationError> {
    let executable = if cfg!(windows) {
        "opencli.cmd"
    } else {
        "opencli"
    };
    let output = Command::new(executable)
        .args(args)
        .output()
        .map_err(|error| ApplicationError::Asset(format!("unable to start OpenCLI: {error}")))?;
    let bytes = if output.status.success() {
        &output.stdout
    } else {
        &output.stderr
    };
    let value: Value = serde_json::from_slice(bytes).map_err(|_| {
        ApplicationError::Integrity("OpenCLI returned a non-JSON response".to_owned())
    })?;
    if output.status.success() {
        Ok(value)
    } else {
        let message = value
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("OpenCLI Doubao command failed")
            .to_owned();
        Err(ApplicationError::Storage(message))
    }
}

fn validate_opencli_attachment_coverage(
    row: &Value,
    requested_attachments: bool,
) -> Result<(), ApplicationError> {
    let reported_attachment_key_count = row
        .get("AttachmentKeyCount")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let declared_attachment_count = row
        .get("Messages")
        .and_then(Value::as_array)
        .map(|messages| count_declared_message_files(messages))
        .unwrap_or_default();
    let attachment_key_count = reported_attachment_key_count.max(declared_attachment_count);
    if requested_attachments && attachment_key_count > 0 {
        return Err(ApplicationError::Integrity(format!(
            "Doubao declares {attachment_key_count} attachment files but the text-only OpenCLI result has no validated original binaries; supply a Chrome-native acquisition handoff; C0 was not written"
        )));
    }
    Ok(())
}

fn count_declared_message_files(messages: &[Value]) -> u64 {
    messages
        .iter()
        .filter(|message| message.get("content_type").and_then(Value::as_u64) == Some(20))
        .filter_map(|message| message.get("content").and_then(Value::as_str))
        .filter_map(|content| serde_json::from_str::<Value>(content).ok())
        .filter_map(|content| content.get("entities").and_then(Value::as_array).cloned())
        .map(|entities| {
            entities
                .iter()
                .filter(|entity| entity.pointer("/entity_content/file").is_some())
                .count() as u64
        })
        .sum()
}

fn required_string(value: &Value, key: &str) -> Result<String, ApplicationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApplicationError::Integrity(format!("OpenCLI result has no {key}")))
}

fn unix_seconds_timestamp(
    value: &Value,
    key: &str,
) -> Result<Option<UtcTimestamp>, ApplicationError> {
    let Some(raw) = value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let seconds = raw
        .parse::<i64>()
        .map_err(|_| ApplicationError::Integrity(format!("OpenCLI result has an invalid {key}")))?;
    let timestamp = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| ApplicationError::Integrity(format!("OpenCLI result has an invalid {key}")))?;
    let canonical = timestamp
        .format(&Rfc3339)
        .map_err(|_| ApplicationError::Integrity(format!("OpenCLI result has an invalid {key}")))?;
    UtcTimestamp::parse(canonical).map(Some).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use babata_application::ports::SourceAdapterPort;
    use babata_domain::{AssetRole, CapabilityStatus, CollectionSessionId, Sha256};
    use md5::{Digest, Md5};
    use tempfile::TempDir;

    use super::{
        DoubaoOpenCliAdapter, DoubaoScope, doubao_content_fingerprint, parse_scope,
        validate_opencli_attachment_coverage,
    };
    use serde_json::json;

    const CONVERSATION_ID: &str = "21060420230098690";

    fn handoff_fixture(temp: &TempDir, include_asset: bool) -> (PathBuf, PathBuf) {
        let file_name = "lesson.docx";
        let asset_path = temp.path().join(file_name);
        let bytes = b"PK\x03\x04fixture-docx";
        fs::write(&asset_path, bytes).unwrap();
        let md5 = format!("{:x}", Md5::digest(bytes));
        let sha256 = Sha256::of_bytes(bytes);
        let message_content = json!({
            "entities": [{
                "entity_type": 1,
                "entity_content": {"file": {
                    "file_name": file_name,
                    "md5": md5,
                    "size": bytes.len(),
                    "file_type": 1
                }}
            }]
        })
        .to_string();
        let assets = if include_asset {
            vec![json!({
                "path": asset_path,
                "file_name": file_name,
                "byte_size": bytes.len(),
                "md5": md5,
                "sha256": sha256.as_str(),
            })]
        } else {
            Vec::new()
        };
        let handoff = json!({
            "protocol_version": "1",
            "provider": "doubao",
            "acquisition_route": "chrome_native",
            "conversation_id": CONVERSATION_ID,
            "source_url": format!("https://www.doubao.com/chat/{CONVERSATION_ID}"),
            "conversation_info": {
                "conversation_id": CONVERSATION_ID,
                "name": "Strategic Leadership W1"
            },
            "messages": [{
                "conversation_id": CONVERSATION_ID,
                "message_id": "message-1",
                "content_type": 20,
                "content": message_content
            }],
            "has_more": false,
            "message_cursor": "1",
            "assets": assets
        });
        let handoff_path = temp.path().join("handoff.json");
        fs::write(&handoff_path, handoff.to_string()).unwrap();
        (handoff_path, asset_path)
    }

    #[test]
    fn scope_is_explicit_and_bounded() {
        assert_eq!(parse_scope("recent:1").unwrap(), DoubaoScope::Recent(1));
        assert_eq!(parse_scope("recent:20").unwrap(), DoubaoScope::Recent(20));
        assert_eq!(
            parse_scope("conversation:38434881297298946").unwrap(),
            DoubaoScope::Conversation("38434881297298946".to_owned())
        );
        assert_eq!(
            parse_scope("conversations:38434881297298946,38435737678224898").unwrap(),
            DoubaoScope::Conversations(vec![
                "38434881297298946".to_owned(),
                "38435737678224898".to_owned(),
            ])
        );
        assert!(parse_scope("recent:0").is_err());
        assert!(parse_scope("recent:21").is_err());
        assert!(parse_scope("conversations:").is_err());
        assert!(parse_scope("conversations:38434881297298946,,38435737678224898").is_err());
        assert!(parse_scope("conversations:38434881297298946,not-an-id").is_err());
        assert!(parse_scope("conversations:38434881297298946,38434881297298946").is_err());
        assert!(
            parse_scope(&format!(
                "conversations:{}",
                (0..21)
                    .map(|index| format!("{index:08}"))
                    .collect::<Vec<_>>()
                    .join(",")
            ))
            .is_err()
        );
        assert!(parse_scope("conversation:1234567").is_err());
        assert!(parse_scope("conversation:not-an-id").is_err());
        assert!(parse_scope("all").is_err());
    }

    #[test]
    fn route_is_enabled_after_real_bounded_batch_evidence() {
        let descriptor = DoubaoOpenCliAdapter::default().describe();
        assert_eq!(descriptor.status, CapabilityStatus::Enabled);
        assert_eq!(descriptor.activation_phase, "P7");
    }

    #[test]
    fn content_fingerprint_ignores_transient_blocks_and_signed_media_urls() {
        let first = vec![json!({
            "message_id": "message-1",
            "content": "stable",
            "content_block": [{
                "block_id": "generated-one",
                "content": {"text_block": {"text": "stable"}},
                "meta_info": [{
                    "info": r#"{"media":[{"image":{"item_id":"image-1","thumb_url":"https://p11-flow-imagex-sign.byteimg.com/path/image.jpeg?x-signature=one"}}]}"#
                }]
            }]
        })];
        let refreshed = vec![json!({
            "message_id": "message-1",
            "content": "stable",
            "content_block": [{
                "block_id": "generated-two",
                "content": {"text_block": {"text": "stable"}},
                "meta_info": [{
                    "info": r#"{"media":[{"image":{"item_id":"image-1","thumb_url":"https://p26-flow-imagex-sign.byteimg.com/path/image.jpeg?x-signature=two"}}]}"#
                }]
            }]
        })];
        let changed = vec![json!({
            "message_id": "message-1",
            "content": "changed",
            "content_block": [{
                "block_id": "generated-three",
                "content": {"text_block": {"text": "changed"}}
            }]
        })];

        assert_eq!(
            doubao_content_fingerprint("conversation-1", Some("title"), &first, &[]),
            doubao_content_fingerprint("conversation-1", Some("title"), &refreshed, &[])
        );
        assert_ne!(
            doubao_content_fingerprint("conversation-1", Some("title"), &first, &[]),
            doubao_content_fingerprint("conversation-1", Some("title"), &changed, &[])
        );
    }

    #[test]
    fn opencli_fallback_rejects_declared_attachments_when_originals_were_requested() {
        let row = json!({"AttachmentKeyCount": 7});
        let error = validate_opencli_attachment_coverage(&row, true).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Chrome-native acquisition handoff")
        );

        validate_opencli_attachment_coverage(&row, false).unwrap();
        validate_opencli_attachment_coverage(&json!({"AttachmentKeyCount": 0}), true).unwrap();

        let nested_row = json!({
            "AttachmentKeyCount": 0,
            "Messages": [{
                "content_type": 20,
                "content": json!({
                    "entities": [{"entity_content": {"file": {"key": "original.docx"}}}]
                }).to_string()
            }]
        });
        let error = validate_opencli_attachment_coverage(&nested_row, true).unwrap_err();
        assert!(error.to_string().contains("declares 1 attachment files"));
    }

    #[test]
    fn chrome_handoff_prefetches_complete_assets_and_sanitizes_final_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let (handoff_path, asset_path) = handoff_fixture(&temp, true);
        let adapter = DoubaoOpenCliAdapter::from_acquisition_handoffs(&[handoff_path]).unwrap();
        let discovered = adapter
            .discover(
                &CollectionSessionId::new(),
                &format!("conversation:{CONVERSATION_ID}"),
            )
            .unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].summary.attachment_available, Some(true));
        let acquisition = adapter
            .collect(
                &discovered[0].summary,
                discovered[0].prefetched.as_ref(),
                true,
            )
            .unwrap();
        let babata_application::AcquisitionOutcome::Found { candidate, assets } = acquisition
        else {
            panic!("handoff should produce a candidate");
        };
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].role, AssetRole::Attachment);
        assert_eq!(assets[0].path, asset_path.to_string_lossy());
        let metadata = candidate.metadata.to_json();
        assert!(!metadata.contains("_doubao_acquisition"));
        assert!(!metadata.contains(temp.path().to_string_lossy().as_ref()));
        assert!(metadata.contains("\"attachments_covered\":true"));
    }

    #[test]
    fn chrome_handoff_rejects_missing_or_changed_originals_before_c0() {
        let missing_temp = tempfile::tempdir().unwrap();
        let (missing_handoff, _) = handoff_fixture(&missing_temp, false);
        let missing =
            DoubaoOpenCliAdapter::from_acquisition_handoffs(&[missing_handoff]).unwrap_err();
        assert!(missing.to_string().contains("coverage is incomplete"));

        let changed_temp = tempfile::tempdir().unwrap();
        let (changed_handoff, asset_path) = handoff_fixture(&changed_temp, true);
        let adapter = DoubaoOpenCliAdapter::from_acquisition_handoffs(&[changed_handoff]).unwrap();
        let discovered = adapter
            .discover(
                &CollectionSessionId::new(),
                &format!("conversation:{CONVERSATION_ID}"),
            )
            .unwrap();
        fs::write(asset_path, b"PK\x03\x04changed").unwrap();
        let changed = adapter
            .collect(
                &discovered[0].summary,
                discovered[0].prefetched.as_ref(),
                true,
            )
            .unwrap_err();
        assert!(changed.to_string().contains("bytes do not match"));
    }
}

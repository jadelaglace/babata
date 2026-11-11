use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use babata_application::{
    AcquisitionOutcome, ApplicationError, CaptureImportAsset, DiscoveredCandidate,
    ports::SourceAdapterPort,
};
use babata_domain::{
    AssetRole, CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus,
    CollectionSessionId, CommonSourceMetadata, ContentType, Metadata, RouteCoverage, Sha256,
    SourceRouteDescriptor, SourceRouteId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const ROUTE: &str = "source.yuanbao";
const INTERNAL: &str = "_yuanbao_assets";

/// Consumes explicit native acquisitions; browser credentials and network IO stay outside the writer.
#[derive(Debug, Default, Clone)]
pub struct YuanbaoNativeAdapter {
    handoffs: HashMap<String, Handoff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Handoff {
    protocol_version: String,
    provider: String,
    conversation_id: String,
    agent_id: String,
    title: String,
    has_more: bool,
    messages: Vec<Value>,
    #[serde(default)]
    media_info: Value,
    assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Asset {
    url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    path: String,
    sha256: String,
    bytes: u64,
}

fn invalid(reason: &str) -> ApplicationError {
    ApplicationError::Integrity(format!("Yuanbao: {reason}; C0 was not written"))
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

impl Handoff {
    fn source_url(&self) -> String {
        format!(
            "https://yuanbao.tencent.com/chat/{}/{}",
            self.agent_id, self.conversation_id
        )
    }
}

// Citation URLs are references. Only native typed media are required binary dependencies.
fn media_urls(value: &Value, required: &mut HashSet<String>) -> Result<(), ApplicationError> {
    match value {
        Value::Object(map) => {
            let typed_media = map
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|kind| matches!(kind, "image" | "file" | "audio" | "video"))
                || map
                    .get("mediaId")
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.is_empty());
            if typed_media {
                let mut found = false;
                for field in ["url", "resourceUrl", "OriginUrl"] {
                    if let Some(url) = map
                        .get(field)
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                    {
                        if !url.starts_with("https://") && !url.starts_with("http://") {
                            return Err(invalid("invalid native media URL"));
                        }
                        required.insert(url.to_owned());
                        found = true;
                    }
                }
                if !found {
                    return Err(invalid("native media has no downloadable URL"));
                }
            }
            for nested in map.values() {
                media_urls(nested, required)?;
            }
        }
        Value::Array(values) => {
            for nested in values {
                media_urls(nested, required)?;
            }
        }
        Value::String(text) => {
            if let Ok(nested) = serde_json::from_str::<Value>(text)
                && (nested.is_object() || nested.is_array())
            {
                media_urls(&nested, required)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate(h: &Handoff) -> Result<(), ApplicationError> {
    if h.protocol_version != "1"
        || h.provider != "yuanbao"
        || !valid_id(&h.conversation_id)
        || !valid_id(&h.agent_id)
        || h.title.trim().is_empty()
        || h.has_more
        || h.messages.is_empty()
    {
        return Err(invalid(
            "invalid identity, empty messages or incomplete pagination",
        ));
    }
    let mut ids = HashSet::new();
    let mut required = HashSet::new();
    for row in &h.messages {
        let id = row
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| invalid("missing native message ID"))?;
        if row.get("conversationId").and_then(Value::as_str) != Some(h.conversation_id.as_str())
            || !ids.insert(id)
            || !matches!(
                row.get("speaker").and_then(Value::as_str),
                Some("human" | "ai")
            )
            || row
                .get("speechesV2")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
        {
            return Err(invalid(
                "foreign/duplicate message or unsupported native message shape",
            ));
        }
        media_urls(&row["speechesV2"], &mut required)?;
    }
    media_urls(&h.media_info, &mut required)?;
    let mut supplied = HashSet::new();
    for asset in &h.assets {
        if !supplied.insert(asset.url.clone()) {
            return Err(invalid("duplicate media URL"));
        }
        let data = fs::read(&asset.path).map_err(|_| invalid("missing media file"))?;
        if data.is_empty()
            || data.len() as u64 != asset.bytes
            || Sha256::of_bytes(&data).as_str() != asset.sha256
        {
            return Err(invalid("media size/hash mismatch"));
        }
    }
    if supplied != required {
        return Err(invalid("native media coverage mismatch"));
    }
    Ok(())
}

fn timestamp(seconds: Option<i64>) -> Option<UtcTimestamp> {
    let utc = OffsetDateTime::from_unix_timestamp(seconds?).ok()?;
    UtcTimestamp::parse(utc.format(&Rfc3339).ok()?).ok()
}

fn envelope(h: &Handoff, internal: bool) -> Result<CandidateEnvelope, ApplicationError> {
    let mut public = h.clone();
    for asset in &mut public.assets {
        asset.path.clear();
    }
    public.assets.sort_by(|a, b| a.url.cmp(&b.url));
    let text = serde_json::to_string(&public).map_err(|_| invalid("invalid native payload"))?;
    let hash = Sha256::of_bytes(text.as_bytes());
    let mut metadata = serde_json::json!({"title": h.title, "conversation_id": h.conversation_id,
        "message_count": h.messages.len(), "downloaded_asset_count": h.assets.len(),
        "content_fingerprint": hash.as_str(), "adapter_version": "yuanbao-native/1",
        "complete_selected_message_chain": true, "attachments_covered": true,
        "original_upload_bytes_verified": false});
    if internal {
        metadata[INTERNAL] =
            serde_json::to_value(&h.assets).map_err(|_| invalid("invalid assets"))?;
    }
    let times: Vec<_> = h
        .messages
        .iter()
        .filter_map(|row| row["createTime"].as_i64())
        .collect();
    Ok(CandidateEnvelope {
        protocol_version: "1".to_owned(),
        route_id: SourceRouteId(ROUTE.to_owned()),
        source_reference: h.source_url(),
        content_type: ContentType::Document,
        payload_sha256: hash,
        payload: CandidatePayload::Text { text },
        metadata: Metadata::parse(&metadata.to_string())?,
        context: Some("Yuanbao / explicitly selected native conversations".to_owned()),
        native_id: Some(h.conversation_id.clone()),
        common_metadata: CommonSourceMetadata {
            title: Some(h.title.clone()),
            source_published_at: timestamp(times.iter().min().copied()),
            source_updated_at: timestamp(times.iter().max().copied()),
            ..CommonSourceMetadata::default()
        },
    })
}

fn restore(prefetched: &CandidateEnvelope) -> Result<Handoff, ApplicationError> {
    let CandidatePayload::Text { text } = &prefetched.payload;
    if Sha256::of_bytes(text.as_bytes()) != prefetched.payload_sha256
        || prefetched.route_id.0 != ROUTE
    {
        return Err(invalid("prefetched payload hash/route mismatch"));
    }
    let mut h: Handoff =
        serde_json::from_str(text).map_err(|_| invalid("invalid prefetched payload"))?;
    let metadata: Value = serde_json::from_str(&prefetched.metadata.to_json())
        .map_err(|_| invalid("invalid prefetched metadata"))?;
    h.assets = serde_json::from_value(metadata[INTERNAL].clone())
        .map_err(|_| invalid("missing prefetched media paths"))?;
    if envelope(&h, false)?.payload_sha256 != prefetched.payload_sha256
        || prefetched.source_reference != h.source_url()
        || prefetched.native_id.as_deref() != Some(h.conversation_id.as_str())
    {
        return Err(invalid("prefetched identity/assets mismatch"));
    }
    Ok(h)
}

impl YuanbaoNativeAdapter {
    pub fn descriptor() -> SourceRouteDescriptor {
        SourceRouteDescriptor {
            id: SourceRouteId(ROUTE.to_owned()),
            provider: "yuanbao".to_owned(),
            status: CapabilityStatus::Enabled,
            activation_phase: "usage-0.2".to_owned(),
        }
    }

    pub fn from_acquisition_handoffs(paths: &[PathBuf]) -> Result<Self, ApplicationError> {
        let mut handoffs = HashMap::new();
        for path in paths {
            let data = fs::read(path).map_err(|_| invalid("unreadable acquisition handoff"))?;
            let h: Handoff = serde_json::from_slice(&data)
                .map_err(|_| invalid("invalid acquisition handoff JSON"))?;
            validate(&h)?;
            if handoffs.insert(h.conversation_id.clone(), h).is_some() {
                return Err(invalid("duplicate conversation handoff"));
            }
        }
        Ok(Self { handoffs })
    }
}

impl SourceAdapterPort for YuanbaoNativeAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        Self::descriptor()
    }

    fn discover(
        &self,
        session: &CollectionSessionId,
        source: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let scope = source
            .strip_prefix("conversations:")
            .or_else(|| source.strip_prefix("conversation:"))
            .unwrap_or(source);
        let mut seen = HashSet::new();
        scope
            .split(',')
            .map(|entry| {
                let id = if entry.starts_with("https://yuanbao.tencent.com/chat/") {
                    entry
                        .rsplit('/')
                        .next()
                        .ok_or_else(|| invalid("invalid conversation URL"))?
                } else {
                    entry
                };
                if !valid_id(id) || !seen.insert(id) {
                    return Err(invalid("invalid/duplicate explicit conversation ID"));
                }
                let h = self.handoffs.get(id).ok_or_else(|| {
                    invalid("explicit scope requires matching native acquisition handoff")
                })?;
                if entry != id && entry != h.source_url() {
                    return Err(invalid("conversation URL does not match handoff"));
                }
                let prefetched = envelope(h, true)?;
                Ok(DiscoveredCandidate {
                    summary: CandidateSummary {
                        candidate_id: format!("yuanbao_{id}"),
                        session_id: session.clone(),
                        route_id: SourceRouteId(ROUTE.to_owned()),
                        source_native_id: Some(id.to_owned()),
                        title: Some(h.title.clone()),
                        source_location: Some(h.source_url()),
                        hierarchy: vec!["Yuanbao".to_owned(), h.title.clone()],
                        content_type: ContentType::Document,
                        source_updated_at: prefetched.common_metadata.source_updated_at.clone(),
                        attachment_available: Some(!h.assets.is_empty()),
                        limitations: self.coverage().limitations,
                        selection_capabilities: vec!["single".to_owned(), "explicit_id".to_owned()],
                        common_metadata: prefetched.common_metadata.clone(),
                    }
                    .with_common_from_legacy(),
                    prefetched: Some(prefetched),
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
        let id = candidate
            .source_native_id
            .as_deref()
            .ok_or_else(|| invalid("missing candidate ID"))?;
        let h = match self.handoffs.get(id) {
            Some(h) => h.clone(),
            None => {
                restore(prefetched.ok_or_else(|| invalid("missing native acquisition handoff"))?)?
            }
        };
        validate(&h)?;
        if h.conversation_id != id
            || candidate.route_id.0 != ROUTE
            || candidate.source_location.as_deref() != Some(h.source_url().as_str())
        {
            return Err(invalid("handoff does not match selected candidate"));
        }
        if !requested_attachments && !h.assets.is_empty() {
            return Err(invalid("native media must be selected"));
        }
        let mut hashes = HashSet::new();
        let assets = h
            .assets
            .iter()
            .filter(|a| hashes.insert(a.sha256.clone()))
            .map(|a| {
                Ok(CaptureImportAsset {
                    path: a.path.clone(),
                    role: AssetRole::Attachment,
                    expected_sha256: Some(Sha256::parse(&a.sha256)?),
                    ..CaptureImportAsset::default()
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        Ok(AcquisitionOutcome::Found {
            candidate: Box::new(envelope(&h, false)?),
            assets,
        })
    }

    fn collect_recollection(
        &self,
        candidate: &CandidateSummary,
        _prefetched: Option<&CandidateEnvelope>,
        attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        if !candidate
            .source_native_id
            .as_ref()
            .is_some_and(|id| self.handoffs.contains_key(id))
        {
            return Ok(AcquisitionOutcome::Inaccessible {reason: "Yuanbao recollection requires a fresh native acquisition handoff; stored snapshots cannot prove live state".to_owned()});
        }
        self.collect(candidate, None, attachments)
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {metadata: true, attachments: true, revisions: true, limitations: vec![
            "Explicit native handoffs only; account enumeration and downloading remain external acquisition".to_owned(),
            "Require complete speechesV2 messages and all typed image/file/audio/video URL dependencies; unproven shapes fail closed".to_owned(),
            "Downloaded size/hash proves the saved platform copy, not original upload-byte fidelity".to_owned(),
            "Fresh acquisition is required for recollection; native metadata/URL changes may create revisions".to_owned(),
        ]}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn handoff() -> Handoff {
        serde_json::from_value(json!({"protocol_version":"1","provider":"yuanbao",
            "conversation_id":"test-chat","agent_id":"test-agent","title":"fixture","has_more":false,
            "messages":[{"id":"test-chat_1","conversationId":"test-chat","speaker":"human",
                "createTime":1_794_140_924,"speechesV2":[{"content":[{"type":"text","msg":"native text"}]}]}],"assets":[]})).unwrap()
    }

    #[test]
    fn incomplete_foreign_duplicate_and_empty_chains_fail_closed() {
        let h = handoff();
        assert!(validate(&h).is_ok());
        let mut bad = h.clone();
        bad.has_more = true;
        assert!(validate(&bad).is_err());
        let mut bad = h.clone();
        bad.messages[0]["conversationId"] = "foreign".into();
        assert!(validate(&bad).is_err());
        let mut bad = h.clone();
        bad.messages.push(h.messages[0].clone());
        assert!(validate(&bad).is_err());
        let mut bad = h.clone();
        bad.messages.clear();
        assert!(validate(&bad).is_err());
        let mut bad = h;
        bad.messages[0]["speechesV2"] = json!([]);
        assert!(validate(&bad).is_err());
    }

    #[test]
    fn native_media_coverage_hash_and_alias_dedup_are_enforced() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("asset.png");
        fs::write(&path, b"image").unwrap();
        let mut h = handoff();
        let url = "https://hunyuan.tencent.com/api/resource/download?resourceId=fixture";
        h.messages[0]["speechesV2"][0]["content"]
            .as_array_mut()
            .unwrap()
            .push(json!({"type":"image","url":url}));
        assert!(validate(&h).is_err());
        h.assets.push(Asset {
            url: url.to_owned(),
            path: path.to_string_lossy().into_owned(),
            sha256: Sha256::of_bytes(b"image").as_str().to_owned(),
            bytes: 5,
        });
        assert!(validate(&h).is_ok());
        h.media_info = json!([{"type":"image","url":"https://example.test/alias"}]);
        let mut alias = h.assets[0].clone();
        alias.url = "https://example.test/alias".to_owned();
        h.assets.push(alias);
        let adapter = YuanbaoNativeAdapter {
            handoffs: HashMap::from([(h.conversation_id.clone(), h.clone())]),
        };
        let discovered = adapter
            .discover(&CollectionSessionId::new(), "test-chat")
            .unwrap()
            .remove(0);
        assert!(adapter.collect(&discovered.summary, None, false).is_err());
        let AcquisitionOutcome::Found { assets, .. } =
            adapter.collect(&discovered.summary, None, true).unwrap()
        else {
            panic!("expected media");
        };
        assert_eq!(assets.len(), 1);
        fs::write(&path, b"wrong").unwrap();
        assert!(validate(&h).is_err());
        fs::remove_file(&path).unwrap();
        assert!(validate(&h).is_err());
    }

    #[test]
    fn explicit_scope_prefetch_integrity_and_fresh_recollection_are_required() {
        let h = handoff();
        let adapter = YuanbaoNativeAdapter {
            handoffs: HashMap::from([(h.conversation_id.clone(), h.clone())]),
        };
        let session = CollectionSessionId::new();
        for invalid in [
            "all",
            "test-chat,test-chat",
            "https://yuanbao.tencent.com/chat/foreign/test-chat",
        ] {
            assert!(adapter.discover(&session, invalid).is_err());
        }
        let discovered = adapter
            .discover(&session, &h.source_url())
            .unwrap()
            .remove(0);
        let empty = YuanbaoNativeAdapter::default();
        assert!(matches!(
            empty
                .collect_recollection(&discovered.summary, discovered.prefetched.as_ref(), true)
                .unwrap(),
            AcquisitionOutcome::Inaccessible { .. }
        ));
        assert!(matches!(
            empty
                .collect(&discovered.summary, discovered.prefetched.as_ref(), true)
                .unwrap(),
            AcquisitionOutcome::Found { .. }
        ));
        let mut p = discovered.prefetched.unwrap();
        p.native_id = Some("foreign".to_owned());
        assert!(empty.collect(&discovered.summary, Some(&p), true).is_err());
        p.native_id = Some(h.conversation_id);
        p.payload_sha256 = Sha256::of_bytes(b"tampered");
        assert!(empty.collect(&discovered.summary, Some(&p), true).is_err());
    }
}

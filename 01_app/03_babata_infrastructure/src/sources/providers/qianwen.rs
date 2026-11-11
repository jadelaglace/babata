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
    CollectionSessionId, ContentType, Metadata, RouteCoverage, Sha256, SourceRouteDescriptor,
    SourceRouteId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const ROUTE: &str = "source.qianwen";
const INTERNAL: &str = "_qianwen_handoff";

#[derive(Debug, Default, Clone)]
pub struct QianwenOpenCliAdapter {
    handoffs: HashMap<String, Handoff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Handoff {
    protocol_version: String,
    provider: String,
    conversation_id: String,
    title: String,
    source_url: String,
    have_next_page: bool,
    messages: Vec<Value>,
    original_keys: Vec<String>,
    excluded_keys: Vec<String>,
    excluded_fields: Vec<String>,
    assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Asset {
    url: String,
    path: String,
    sha256: String,
    bytes: u64,
}

fn invalid(reason: &str) -> ApplicationError {
    ApplicationError::Integrity(format!("Qianwen: {reason}; C0 was not written"))
}

fn row_key(row: &Value) -> Result<String, ApplicationError> {
    let values = ["req_id", "pos", "event"].map(|key| {
        row.get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
    });
    match values {
        [Some(id), Some(pos), Some(event)] => Ok(format!("{id}|{pos}|{event}")),
        _ => Err(invalid("missing native row identity")),
    }
}

// Embedded JSON strings are part of native rich responses, too. Citations are references.
fn media_urls(value: &Value, parent: &str, result: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                media_urls(value, key, result);
            }
        }
        Value::Array(values) => {
            for value in values {
                media_urls(value, parent, result);
            }
        }
        Value::String(s) => {
            if let Ok(nested) = serde_json::from_str::<Value>(s) {
                if nested.is_object() || nested.is_array() {
                    media_urls(&nested, parent, result);
                }
            }
            if (s.starts_with("https://") || s.starts_with("http://"))
                && matches!(
                    parent,
                    "url"
                        | "image_url"
                        | "layout_file_url"
                        | "preview_img_url"
                        | "imagePath"
                        | "previewImages"
                )
                && [
                    "workspace-zb-cdn.",
                    "dl-docs-cdn.",
                    "preview-wenku.",
                    "idocs-zjk.",
                ]
                .iter()
                .any(|host| s.contains(host))
            {
                result.insert(s.clone());
            }
        }
        _ => {}
    }
}

fn validate(h: &Handoff) -> Result<(), ApplicationError> {
    if h.protocol_version != "1"
        || h.provider != "qianwen"
        || h.conversation_id.len() != 32
        || !h.conversation_id.bytes().all(|c| c.is_ascii_hexdigit())
        || h.source_url != format!("https://www.qianwen.com/chat/{}", h.conversation_id)
        || h.title.trim().is_empty()
        || h.have_next_page
        || h.messages.is_empty()
    {
        return Err(invalid(
            "invalid identity, empty messages, or incomplete pagination",
        ));
    }
    let mut retained = HashSet::new();
    let mut required = HashSet::new();
    for row in &h.messages {
        if row.get("session_id").and_then(Value::as_str) != Some(h.conversation_id.as_str())
            || row.get("header").is_some()
            || !retained.insert(row_key(row)?)
        {
            return Err(invalid(
                "foreign or duplicate row, or transport headers present",
            ));
        }
        media_urls(row, "", &mut required);
    }
    let original: HashSet<_> = h.original_keys.iter().cloned().collect();
    let excluded: HashSet<_> = h.excluded_keys.iter().cloned().collect();
    if original.len() != h.original_keys.len()
        || excluded.len() != h.excluded_keys.len()
        || !retained.is_disjoint(&excluded)
        || retained.union(&excluded).cloned().collect::<HashSet<_>>() != original
    {
        return Err(invalid(
            "retained/excluded native positions do not partition the source",
        ));
    }
    let mut supplied = HashSet::new();
    for asset in &h.assets {
        if !supplied.insert(asset.url.clone()) {
            return Err(invalid("duplicate media URL"));
        }
        let bytes = fs::read(&asset.path).map_err(|_| invalid("missing media file"))?;
        if bytes.len() as u64 != asset.bytes
            || bytes.is_empty()
            || Sha256::of_bytes(&bytes).as_str() != asset.sha256
        {
            return Err(invalid("media size/hash mismatch"));
        }
    }
    if supplied != required {
        return Err(invalid(
            "embedded media coverage differs from the retained messages",
        ));
    }
    Ok(())
}

impl QianwenOpenCliAdapter {
    pub fn descriptor() -> SourceRouteDescriptor {
        SourceRouteDescriptor {
            id: SourceRouteId(ROUTE.to_owned()),
            provider: "qianwen".to_owned(),
            status: CapabilityStatus::Enabled,
            activation_phase: "usage-0.2".to_owned(),
        }
    }

    pub fn from_acquisition_handoffs(paths: &[PathBuf]) -> Result<Self, ApplicationError> {
        let mut handoffs = HashMap::new();
        for path in paths {
            let bytes = fs::read(path).map_err(|_| invalid("unreadable acquisition handoff"))?;
            let h: Handoff = serde_json::from_slice(&bytes)
                .map_err(|_| invalid("invalid acquisition handoff JSON"))?;
            validate(&h)?;
            if handoffs.insert(h.conversation_id.clone(), h).is_some() {
                return Err(invalid("duplicate conversation handoff"));
            }
        }
        Ok(Self { handoffs })
    }
}

fn envelope(h: &Handoff, internal: bool) -> Result<CandidateEnvelope, ApplicationError> {
    let payload = serde_json::json!({
        "platform": "qianwen", "conversation_id": h.conversation_id, "title": h.title,
        "messages": h.messages, "have_next_page": false,
        "scope_selection": {"original_keys": h.original_keys, "excluded_keys": h.excluded_keys,
            "excluded_fields": h.excluded_fields,
            "reason": "user-directed exclusion of food ordering, payment and delivery portions"},
        "assets": h.assets.iter().map(|a| serde_json::json!({"url": a.url, "sha256": a.sha256, "bytes": a.bytes})).collect::<Vec<_>>()
    });
    let text = payload.to_string();
    let mut stable = text.clone();
    for asset in &h.assets {
        // Replace transient signed resource URLs, including their JSON-escaped spellings.
        let encoded = serde_json::to_string(&asset.url).map_err(|_| invalid("invalid URL"))?;
        stable = stable.replace(&encoded[1..encoded.len() - 1], &asset.sha256);
    }
    let mut metadata = serde_json::json!({
        "title": h.title, "conversation_id": h.conversation_id, "message_count": h.messages.len(),
        "excluded_message_count": h.excluded_keys.len(), "content_fingerprint": Sha256::of_bytes(stable.as_bytes()).as_str(),
        "adapter_version": "qianwen-native/2", "complete_selected_message_chain": true,
        "native_message_ids_covered": true, "attachments_covered": true, "downloaded_asset_count": h.assets.len()
    });
    if internal {
        metadata[INTERNAL] =
            serde_json::to_value(&h.assets).map_err(|_| invalid("invalid assets"))?;
    }
    Ok(CandidateEnvelope {
        protocol_version: "1".to_owned(),
        route_id: SourceRouteId(ROUTE.to_owned()),
        source_reference: h.source_url.clone(),
        content_type: ContentType::Document,
        payload_sha256: Sha256::of_bytes(text.as_bytes()),
        metadata: Metadata::parse(&metadata.to_string())?,
        payload: CandidatePayload::Text { text },
        context: Some("Qianwen / native message API / selected scope".to_owned()),
        native_id: Some(h.conversation_id.clone()),
        common_metadata: babata_domain::CommonSourceMetadata::default(),
    })
}

fn restore_handoff(p: &CandidateEnvelope) -> Result<Handoff, ApplicationError> {
    let CandidatePayload::Text { text } = &p.payload;
    if Sha256::of_bytes(text.as_bytes()) != p.payload_sha256 {
        return Err(invalid("prefetched payload hash mismatch"));
    }
    let payload: Value =
        serde_json::from_str(text).map_err(|_| invalid("invalid prefetched payload"))?;
    let metadata: Value = serde_json::from_str(&p.metadata.to_json())
        .map_err(|_| invalid("invalid prefetched metadata"))?;
    let selection = &payload["scope_selection"];
    let h: Handoff = serde_json::from_value(serde_json::json!({
        "protocol_version": "1", "provider": payload["platform"],
        "conversation_id": payload["conversation_id"], "title": payload["title"],
        "source_url": p.source_reference, "have_next_page": payload["have_next_page"],
        "messages": payload["messages"], "original_keys": selection["original_keys"],
        "excluded_keys": selection["excluded_keys"], "excluded_fields": selection["excluded_fields"],
        "assets": metadata[INTERNAL]
    })).map_err(|_| invalid("invalid prefetched handoff"))?;
    if envelope(&h, false)?.payload_sha256 != p.payload_sha256 {
        return Err(invalid("prefetched assets differ from native payload"));
    }
    Ok(h)
}

impl SourceAdapterPort for QianwenOpenCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        Self::descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let scope = source_reference
            .strip_prefix("conversations:")
            .unwrap_or(source_reference);
        let ids: Vec<_> = scope.split(',').collect();
        let mut seen = HashSet::new();
        ids.into_iter().map(|id| {
            if !seen.insert(id) { return Err(invalid("duplicate requested conversation")); }
            let h = self.handoffs.get(id).ok_or_else(|| invalid("explicit scope requires a matching native acquisition handoff"))?;
            Ok(DiscoveredCandidate {
                summary: CandidateSummary {
                    candidate_id: format!("qianwen_{id}"), session_id: session_id.clone(), route_id: SourceRouteId(ROUTE.to_owned()),
                    source_native_id: Some(id.to_owned()), title: Some(h.title.clone()), source_location: Some(h.source_url.clone()),
                    hierarchy: vec!["Qianwen".to_owned(), h.title.clone()], content_type: ContentType::Document, source_updated_at: None,
                    attachment_available: Some(!h.assets.is_empty()), limitations: vec!["Only explicitly retained native rows are registered; full acquisition remains external".to_owned()],
                    selection_capabilities: vec!["single".to_owned(), "explicit_id".to_owned()], common_metadata: babata_domain::CommonSourceMetadata::default(),
                }.with_common_from_legacy(), prefetched: Some(envelope(h, true)?),
            })
        }).collect()
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
            .ok_or_else(|| invalid("missing candidate identity"))?;
        let h = if let Some(h) = self.handoffs.get(id) {
            h.clone()
        } else {
            let p = prefetched.ok_or_else(|| invalid("missing native acquisition handoff"))?;
            restore_handoff(p)?
        };
        validate(&h)?;
        if h.conversation_id != id
            || candidate.route_id.0 != ROUTE
            || candidate.source_location.as_deref() != Some(h.source_url.as_str())
        {
            return Err(invalid("handoff does not match selected candidate"));
        }
        if !requested_attachments && !h.assets.is_empty() {
            return Err(invalid("embedded media must be selected"));
        }
        let mut imported_hashes = HashSet::new();
        let assets = h
            .assets
            .iter()
            // Multiple native URLs may resolve to one binary managed asset.
            .filter(|asset| imported_hashes.insert(asset.sha256.clone()))
            .map(|asset| {
                Ok(CaptureImportAsset {
                    path: asset.path.clone(),
                    role: AssetRole::Attachment,
                    expected_sha256: Some(Sha256::parse(&asset.sha256)?),
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
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        if !candidate
            .source_native_id
            .as_ref()
            .is_some_and(|id| self.handoffs.contains_key(id))
        {
            return Ok(AcquisitionOutcome::Inaccessible { reason: "Qianwen recollection requires a freshly acquired native handoff; stored snapshots do not prove live state".to_owned() });
        }
        self.collect(candidate, None, requested_attachments)
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage { metadata: true, attachments: true, revisions: true, limitations: vec![
            "Explicit native API handoffs only; caller must acquire fresh responses for recollection".to_owned(),
            "History's successful empty oldest page has no explicit terminal flag; deleted/unexposed conversations are outside observed scope".to_owned(),
            "Food-ordering portions are excluded at original native row boundaries under explicit user scope".to_owned(),
        ] }
    }
}

#[cfg(test)]
mod tests {
    use super::{Asset, Handoff, QianwenOpenCliAdapter, row_key, validate};
    use babata_application::{AcquisitionOutcome, ports::SourceAdapterPort};
    use babata_domain::{CollectionSessionId, Sha256};

    fn handoff() -> Handoff {
        let id = "0123456789abcdef0123456789abcdef";
        let row = serde_json::json!({"session_id": id, "req_id": "native-request", "pos": "123", "event": "complete", "request_messages": [{"content": "retained"}]});
        Handoff {
            protocol_version: "1".to_owned(),
            provider: "qianwen".to_owned(),
            conversation_id: id.to_owned(),
            title: "fixture".to_owned(),
            source_url: format!("https://www.qianwen.com/chat/{id}"),
            have_next_page: false,
            original_keys: vec![row_key(&row).unwrap()],
            messages: vec![row],
            excluded_keys: vec![],
            excluded_fields: vec![],
            assets: vec![],
        }
    }

    #[test]
    fn incomplete_foreign_duplicate_and_unaccounted_rows_are_rejected() {
        let base = handoff();
        assert!(validate(&base).is_ok());
        let mut h = base.clone();
        h.have_next_page = true;
        assert!(validate(&h).is_err());
        let mut h = base.clone();
        h.messages[0]["session_id"] = "other".into();
        assert!(validate(&h).is_err());
        let mut h = base.clone();
        h.messages.push(h.messages[0].clone());
        assert!(validate(&h).is_err());
        let mut h = base.clone();
        h.original_keys.push("missing|1|complete".to_owned());
        assert!(validate(&h).is_err());
        let mut h = base;
        h.excluded_keys.push(h.original_keys[0].clone());
        assert!(validate(&h).is_err());
    }

    #[test]
    fn embedded_nested_json_media_requires_matching_binary_hash() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("asset");
        std::fs::write(&path, b"image").unwrap();
        let url = "https://workspace-zb-cdn.qianwen.com/fixture";
        let mut h = handoff();
        h.messages[0]["response_messages"] =
            serde_json::json!([{"extra_info": serde_json::json!({"image_url": url}).to_string()}]);
        assert!(validate(&h).is_err());
        h.assets.push(Asset {
            url: url.to_owned(),
            path: path.to_string_lossy().into_owned(),
            sha256: Sha256::of_bytes(b"image").as_str().to_owned(),
            bytes: 5,
        });
        assert!(validate(&h).is_ok());
        let second_url = "https://workspace-zb-cdn.qianwen.com/fixture-alias";
        h.messages[0]["image_url"] = second_url.into();
        let mut alias = h.assets[0].clone();
        alias.url = second_url.to_owned();
        h.assets.push(alias);
        let mut adapter = QianwenOpenCliAdapter::default();
        adapter
            .handoffs
            .insert(h.conversation_id.clone(), h.clone());
        let candidate = adapter
            .discover(&CollectionSessionId::new(), &h.conversation_id)
            .unwrap()
            .remove(0);
        let AcquisitionOutcome::Found { assets, .. } = adapter
            .collect(&candidate.summary, candidate.prefetched.as_ref(), true)
            .unwrap()
        else {
            panic!("expected native acquisition");
        };
        assert_eq!(assets.len(), 1);
        std::fs::write(&path, b"other").unwrap();
        assert!(validate(&h).is_err());
    }

    #[test]
    fn request_continuations_survive_and_stored_snapshot_cannot_prove_recollection() {
        let mut h = handoff();
        let mut continuation = h.messages[0].clone();
        continuation["pos"] = "124".into();
        h.original_keys.push(row_key(&continuation).unwrap());
        h.messages.push(continuation);
        assert!(validate(&h).is_ok());
        let mut adapter = QianwenOpenCliAdapter::default();
        adapter
            .handoffs
            .insert(h.conversation_id.clone(), h.clone());
        let candidate = adapter
            .discover(&CollectionSessionId::new(), &h.conversation_id)
            .unwrap()
            .remove(0);
        let result = QianwenOpenCliAdapter::default()
            .collect_recollection(&candidate.summary, candidate.prefetched.as_ref(), true)
            .unwrap();
        assert!(matches!(result, AcquisitionOutcome::Inaccessible { .. }));
        let saved = adapter
            .collect(&candidate.summary, candidate.prefetched.as_ref(), true)
            .unwrap();
        assert!(matches!(saved, AcquisitionOutcome::Found { .. }));
    }
}

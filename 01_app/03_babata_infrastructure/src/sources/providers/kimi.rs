use std::{collections::HashSet, fs, path::Path, process::Command};

use babata_application::{
    AcquisitionOutcome, ApplicationError, DiscoveredCandidate, ports::SourceAdapterPort,
};
use babata_domain::{
    CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus, CollectionSessionId,
    ContentType, Metadata, RouteCoverage, Sha256, SourceRouteDescriptor, SourceRouteId,
    UtcTimestamp,
};
use serde::Deserialize;
use serde_json::Value;

const ROUTE_ID: &str = "source.kimi";
const ADAPTER_VERSION: &str = "opencli-kimi/2";
const MANIFEST_SCHEMA: &str = "babata.kimi-chat-manifest/v1";
const MANIFEST_METHOD: &str = "ChatService/ListChats";
const RECENT_CONTEXT: &str = "Kimi / Recent conversations";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatManifest {
    schema: String,
    readiness: String,
    account: ManifestAccount,
    enumeration: ManifestEnumeration,
    chats: Vec<ManifestChat>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestAccount {
    display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestEnumeration {
    method: String,
    captured_at: String,
    terminated: bool,
    total_listed: usize,
    pages: Vec<ManifestPage>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestPage {
    page: usize,
    count: usize,
    has_next: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestChat {
    chat_id: String,
    title: String,
    update_time: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct KimiOpenCliAdapter;

impl SourceAdapterPort for KimiOpenCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        SourceRouteDescriptor {
            id: SourceRouteId(ROUTE_ID.to_owned()),
            provider: "kimi".to_owned(),
            status: CapabilityStatus::Disabled,
            activation_phase: "P4".to_owned(),
        }
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        if let Some(path) = source_reference.strip_prefix("manifest:") {
            return manifest_candidates(session_id, Path::new(path));
        }
        let limit = source_reference
            .strip_prefix("recent:")
            .ok_or_else(|| {
                ApplicationError::Conflict(
                    "Kimi scope must be recent:<count> or manifest:<path>; account-wide all is never implicit"
                        .to_owned(),
                )
            })?
            .parse::<usize>()
            .map_err(|_| ApplicationError::Conflict("invalid Kimi recent count".to_owned()))?;
        if !(1..=100).contains(&limit) {
            return Err(ApplicationError::Conflict(
                "Kimi recent count must be between 1 and 100".to_owned(),
            ));
        }
        let output = run_opencli(&[
            "kimi",
            "history",
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
            ApplicationError::Integrity("OpenCLI Kimi history was not an array".to_owned())
        })?;
        rows.iter()
            .map(|row| {
                let chat_id = required_string(row, "ChatId")?;
                let title = required_string(row, "Title")?;
                Ok(DiscoveredCandidate {
                    summary: CandidateSummary {
                        candidate_id: format!("kimi_{chat_id}"),
                        session_id: session_id.clone(),
                        route_id: SourceRouteId(ROUTE_ID.to_owned()),
                        source_native_id: Some(chat_id.clone()),
                        title: Some(title.clone()),
                        source_location: Some(format!("https://www.kimi.com/chat/{chat_id}")),
                        hierarchy: vec![
                            "Kimi".to_owned(),
                            "Recent conversations".to_owned(),
                            title,
                        ],
                        content_type: ContentType::Document,
                        source_updated_at: None,
                        attachment_available: None,
                        limitations: vec![
                            "OpenCLI history exposes the current recent conversation window only"
                                .to_owned(),
                            "history metadata does not expose update time or attachments"
                                .to_owned(),
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

    fn collect(
        &self,
        candidate: &CandidateSummary,
        _prefetched: Option<&CandidateEnvelope>,
        _requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let chat_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Kimi candidate has no chat ID".to_owned())
        })?;
        let output = match run_opencli(&[
            "kimi",
            "detail-full",
            chat_id,
            "--window",
            "background",
            "--site-session",
            "persistent",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ]) {
            Ok(value) => value,
            Err(ApplicationError::NotFound(reason)) => {
                return Ok(AcquisitionOutcome::Removed { reason });
            }
            Err(ApplicationError::Conflict(reason)) => {
                return Ok(AcquisitionOutcome::Inaccessible { reason });
            }
            Err(error) => return Err(error),
        };
        acquisition_from_detail(candidate, chat_id, &output)
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: false,
            revisions: true,
            limitations: vec![
                "the complete ChatService response is preserved, including citations and structured blocks"
                    .to_owned(),
                "attachment metadata is preserved but attachment binary download is not yet covered"
                    .to_owned(),
                "message pages are followed until a token-less response; an unfinished listing is rejected"
                    .to_owned(),
                "citation order inside a message can change between reads and then registers as a new revision"
                    .to_owned(),
                "manifest:<path> accepts only a prepared, terminated account listing; it never enumerates the account itself"
                    .to_owned(),
            ],
        }
    }
}

fn acquisition_from_detail(
    candidate: &CandidateSummary,
    chat_id: &str,
    output: &Value,
) -> Result<AcquisitionOutcome, ApplicationError> {
    let rows = output.as_array().ok_or_else(|| {
        ApplicationError::Integrity("OpenCLI Kimi detail-full was not an array".to_owned())
    })?;
    let Some(row) = rows.first() else {
        return Ok(AcquisitionOutcome::Inaccessible {
            reason: "Kimi returned no structured conversation".to_owned(),
        });
    };
    if rows.len() != 1
        || row.get("ChatId").and_then(Value::as_str) != Some(chat_id)
        || row.pointer("/Chat/id").and_then(Value::as_str) != Some(chat_id)
    {
        return Err(ApplicationError::Integrity(
            "Kimi detail-full must return exactly the requested chat".to_owned(),
        ));
    }
    if row.get("Complete").and_then(Value::as_bool) != Some(true) {
        return Err(ApplicationError::Integrity(
            "Kimi message pagination was incomplete; C0 was not written".to_owned(),
        ));
    }
    let chat = row
        .get("Chat")
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            ApplicationError::Integrity("Kimi detail-full has no Chat object".to_owned())
        })?;
    let messages = row
        .get("Messages")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ApplicationError::Integrity("Kimi detail-full has no Messages array".to_owned())
        })?;
    if row.get("MessageCount").and_then(Value::as_u64) != Some(messages.len() as u64)
        || row.get("MessagePages").and_then(Value::as_u64).unwrap_or(0) == 0
        || !matches!(
            row.get("Termination").and_then(Value::as_str),
            Some("no-token" | "empty-final-page")
        )
    {
        return Err(ApplicationError::Integrity(
            "Kimi detail-full has inconsistent message or termination evidence".to_owned(),
        ));
    }
    let payload = serde_json::to_string_pretty(&serde_json::json!({
        "platform": "kimi",
        "chat_id": chat_id,
        "title": candidate.title,
        "chat": chat,
        "messages": messages,
    }))
    .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let content_fingerprint = Sha256::of_bytes(
        serde_json::json!({
            "chat_id": chat_id,
            "title": candidate.title,
            "messages": messages,
        })
        .to_string()
        .as_bytes(),
    );
    let metadata = Metadata::parse(
        &serde_json::json!({
            "title": candidate.title,
            "chat_id": chat_id,
            "message_count": messages.len(),
            "reference_key_count": row.get("ReferenceKeyCount").and_then(Value::as_u64),
            "attachment_key_count": row.get("AttachmentKeyCount").and_then(Value::as_u64),
            "response_bytes": row.get("ResponseBytes").and_then(Value::as_u64),
            "content_fingerprint": content_fingerprint.as_str(),
            "adapter_version": ADAPTER_VERSION,
            "structured_page_response": true,
            "complete_message_page": true,
            "possibly_truncated": false,
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
                .unwrap_or_else(|| format!("https://www.kimi.com/chat/{chat_id}")),
            content_type: ContentType::Document,
            payload_sha256: Sha256::of_bytes(payload.as_bytes()),
            metadata,
            payload: CandidatePayload::Text { text: payload },
            context: Some(envelope_context(candidate)),
            native_id: Some(chat_id.to_owned()),
            common_metadata: candidate.effective_common_metadata(),
        }),
        assets: Vec::new(),
    })
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
    if output.status.success() {
        return serde_json::from_slice(&output.stdout).map_err(|_| {
            ApplicationError::Integrity("OpenCLI returned a non-JSON response".to_owned())
        });
    }
    Err(classify_failure(&output.stderr))
}

fn classify_failure(stderr: &[u8]) -> ApplicationError {
    let message = failure_message(stderr)
        .unwrap_or_else(|| "OpenCLI returned a non-JSON response".to_owned());
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("not found")
        || normalized.contains("not exist")
        || normalized.contains("http 404")
    {
        ApplicationError::NotFound(message)
    } else if normalized.contains("login")
        || normalized.contains("connect")
        || normalized.contains("permission")
        || normalized.contains("http 401")
        || normalized.contains("http 403")
        || normalized.contains("session token is unavailable")
    {
        ApplicationError::Conflict(message)
    } else if normalized.contains("non-json") {
        ApplicationError::Integrity(message)
    } else {
        ApplicationError::Storage(message)
    }
}

// OpenCLI reports failures as JSON or, observed for HTTP errors, as YAML-like text with the
// reason on a `message:` line.
fn failure_message(stderr: &[u8]) -> Option<String> {
    if let Ok(value) = serde_json::from_slice::<Value>(stderr) {
        return Some(
            value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("OpenCLI Kimi command failed")
                .to_owned(),
        );
    }
    String::from_utf8_lossy(stderr)
        .lines()
        .find_map(|line| line.trim().strip_prefix("message:"))
        .map(|message| message.trim().to_owned())
        .filter(|message| !message.is_empty())
}

fn envelope_context(candidate: &CandidateSummary) -> String {
    let parts: Vec<&str> = candidate
        .hierarchy
        .iter()
        .take(2)
        .map(String::as_str)
        .filter(|part| !part.trim().is_empty())
        .collect();
    if parts.len() == 2 {
        parts.join(" / ")
    } else {
        RECENT_CONTEXT.to_owned()
    }
}

fn is_chat_id(value: &str) -> bool {
    let uuid = value.len() == 36
        && value.char_indices().all(|(index, character)| match index {
            8 | 13 | 18 | 23 => character == '-',
            _ => character.is_ascii_digit() || ('a'..='f').contains(&character),
        });
    let xid = value.len() == 20
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='v').contains(&character));
    uuid || xid
}

fn manifest_candidates(
    session_id: &CollectionSessionId,
    path: &Path,
) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
    let manifest = read_manifest(path)?;
    Ok(manifest
        .chats
        .into_iter()
        .map(|chat| {
            let title = chat.title.trim().to_owned();
            DiscoveredCandidate {
                summary: CandidateSummary {
                    candidate_id: format!("kimi_{}", chat.chat_id),
                    session_id: session_id.clone(),
                    route_id: SourceRouteId(ROUTE_ID.to_owned()),
                    source_native_id: Some(chat.chat_id.clone()),
                    title: Some(title.clone()),
                    source_location: Some(format!("https://www.kimi.com/chat/{}", chat.chat_id)),
                    hierarchy: vec!["Kimi".to_owned(), "Account conversations".to_owned(), title],
                    content_type: ContentType::Document,
                    source_updated_at: UtcTimestamp::parse(&chat.update_time).ok(),
                    attachment_available: None,
                    limitations: vec![
                        "listed by a prepared account manifest; the account may have changed since it was captured"
                            .to_owned(),
                        "list metadata does not expose attachments".to_owned(),
                    ],
                    selection_capabilities: vec!["single".to_owned(), "manifest_listed".to_owned()],
                    common_metadata: babata_domain::CommonSourceMetadata::default(),
                }
                .with_common_from_legacy(),
                prefetched: None,
            }
        })
        .collect())
}

fn read_manifest(path: &Path) -> Result<ChatManifest, ApplicationError> {
    let bytes = fs::read(path).map_err(|error| {
        ApplicationError::Asset(format!(
            "unable to read Kimi chat manifest: {:?}",
            error.kind()
        ))
    })?;
    let manifest: ChatManifest = serde_json::from_slice(&bytes).map_err(|_| {
        ApplicationError::Integrity(
            "Kimi chat manifest is invalid JSON or carries unsupported fields".to_owned(),
        )
    })?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &ChatManifest) -> Result<(), ApplicationError> {
    let integrity =
        |reason: &str| ApplicationError::Integrity(format!("Kimi chat manifest: {reason}"));
    if manifest.schema != MANIFEST_SCHEMA || manifest.readiness != "prepared" {
        return Err(ApplicationError::Conflict(
            "Kimi chat manifest has an unsupported schema or readiness".to_owned(),
        ));
    }
    if manifest.account.display_name.trim().is_empty() {
        return Err(integrity("account identity is missing"));
    }
    let enumeration = &manifest.enumeration;
    if enumeration.method != MANIFEST_METHOD
        || UtcTimestamp::parse(&enumeration.captured_at).is_err()
    {
        return Err(integrity("enumeration method or capture time is invalid"));
    }
    if !enumeration.terminated || enumeration.pages.is_empty() {
        return Err(integrity("listing has no termination evidence"));
    }
    let last = enumeration.pages.len() - 1;
    let mut listed = 0usize;
    for (index, page) in enumeration.pages.iter().enumerate() {
        if page.page != index + 1 || page.has_next != (index != last) {
            return Err(integrity("page chain is not contiguous or not terminated"));
        }
        listed = listed
            .checked_add(page.count)
            .ok_or_else(|| integrity("page counts overflow"))?;
    }
    if manifest.chats.is_empty()
        || listed != enumeration.total_listed
        || manifest.chats.len() != enumeration.total_listed
    {
        return Err(integrity("listed chats do not match the page chain"));
    }
    let mut seen = HashSet::new();
    for chat in &manifest.chats {
        if !is_chat_id(&chat.chat_id) || !seen.insert(chat.chat_id.as_str()) {
            return Err(integrity("chat ID is invalid or duplicated"));
        }
        if chat.title.trim().is_empty() || UtcTimestamp::parse(&chat.update_time).is_err() {
            return Err(integrity("chat title or update time is invalid"));
        }
    }
    Ok(())
}

fn required_string(value: &Value, key: &str) -> Result<String, ApplicationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApplicationError::Integrity(format!("OpenCLI result has no {key}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const UUID_ID: &str = "1a13d403-e462-88e0-8000-099020d9ef29";
    const XID_ID: &str = "d3uc5dhtoom5c4jrpus0";
    const OLD_ID: &str = "cqjkmrsubmsacbq64h5g";

    fn manifest() -> Value {
        serde_json::json!({
            "schema": MANIFEST_SCHEMA,
            "readiness": "prepared",
            "account": {"display_name": "tester"},
            "enumeration": {
                "method": MANIFEST_METHOD,
                "captured_at": "2026-11-02T11:30:21.070Z",
                "terminated": true,
                "total_listed": 3,
                "pages": [
                    {"page": 1, "count": 2, "has_next": true},
                    {"page": 2, "count": 1, "has_next": true},
                    {"page": 3, "count": 0, "has_next": false}
                ]
            },
            "chats": [
                {"chat_id": UUID_ID, "title": "uuid chat", "update_time": "2026-10-15T01:59:12.161471Z"},
                {"chat_id": XID_ID, "title": "xid chat", "update_time": "2025-10-25T08:00:00Z"},
                {"chat_id": OLD_ID, "title": "old chat", "update_time": "2024-08-03T14:16:34.434743Z"}
            ]
        })
    }

    fn write(value: &Value) -> (tempfile::TempDir, String) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        let reference = format!("manifest:{}", path.to_string_lossy());
        (dir, reference)
    }

    fn discover(value: &Value) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let (_dir, reference) = write(value);
        KimiOpenCliAdapter.discover(&CollectionSessionId::new(), &reference)
    }

    #[test]
    fn manifest_lists_both_chat_id_shapes_without_collecting() {
        let candidates = discover(&manifest()).unwrap();
        assert_eq!(candidates.len(), 3);
        let ids: Vec<_> = candidates
            .iter()
            .map(|candidate| candidate.summary.source_native_id.as_deref().unwrap())
            .collect();
        assert_eq!(ids, [UUID_ID, XID_ID, OLD_ID]);
        let first = &candidates[0].summary;
        assert_eq!(first.candidate_id, format!("kimi_{UUID_ID}"));
        assert_eq!(
            first.source_location.as_deref(),
            Some("https://www.kimi.com/chat/1a13d403-e462-88e0-8000-099020d9ef29")
        );
        assert_eq!(envelope_context(first), "Kimi / Account conversations");
        assert!(first.source_updated_at.is_some());
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.prefetched.is_none())
        );
    }

    #[test]
    fn manifest_requires_termination_evidence_and_a_contiguous_chain() {
        let mut open = manifest();
        open["enumeration"]["terminated"] = false.into();
        assert!(discover(&open).is_err());

        let mut dangling = manifest();
        dangling["enumeration"]["pages"][2]["has_next"] = true.into();
        assert!(discover(&dangling).is_err());

        let mut gap = manifest();
        gap["enumeration"]["pages"][1]["page"] = 3.into();
        assert!(discover(&gap).is_err());

        let mut empty = manifest();
        empty["enumeration"]["pages"] = serde_json::json!([]);
        assert!(discover(&empty).is_err());
    }

    #[test]
    fn manifest_counts_must_match_the_listed_chats() {
        let mut total = manifest();
        total["enumeration"]["total_listed"] = 4.into();
        assert!(discover(&total).is_err());

        let mut page = manifest();
        page["enumeration"]["pages"][0]["count"] = 3.into();
        assert!(discover(&page).is_err());

        let mut none = manifest();
        none["chats"] = serde_json::json!([]);
        none["enumeration"]["total_listed"] = 0.into();
        none["enumeration"]["pages"] =
            serde_json::json!([{"page": 1, "count": 0, "has_next": false}]);
        assert!(discover(&none).is_err());
    }

    #[test]
    fn manifest_rejects_bad_identity_and_unsupported_content() {
        let mut duplicate = manifest();
        duplicate["chats"][1]["chat_id"] = UUID_ID.into();
        assert!(discover(&duplicate).is_err());

        for invalid in [
            "",
            "not-a-chat",
            "1A13D403-E462-88E0-8000-099020D9EF29",
            "d3uc5dhtoom5c4jrpus",
        ] {
            let mut bad = manifest();
            bad["chats"][0]["chat_id"] = invalid.into();
            assert!(discover(&bad).is_err(), "{invalid} must be rejected");
        }

        let mut title = manifest();
        title["chats"][0]["title"] = "  ".into();
        assert!(discover(&title).is_err());

        let mut time = manifest();
        time["chats"][0]["update_time"] = "2026-10-15 01:59".into();
        assert!(discover(&time).is_err());

        let mut credential = manifest();
        credential["account"]["access_token"] = "secret".into();
        assert!(matches!(
            discover(&credential),
            Err(ApplicationError::Integrity(_))
        ));

        let mut schema = manifest();
        schema["schema"] = "babata.kimi-chat-manifest/v0".into();
        assert!(matches!(
            discover(&schema),
            Err(ApplicationError::Conflict(_))
        ));

        let mut readiness = manifest();
        readiness["readiness"] = "draft".into();
        assert!(matches!(
            discover(&readiness),
            Err(ApplicationError::Conflict(_))
        ));

        let mut account = manifest();
        account["account"]["display_name"] = " ".into();
        assert!(discover(&account).is_err());
    }

    #[test]
    fn manifest_must_be_a_readable_file() {
        let dir = tempdir().unwrap();
        let missing = format!(
            "manifest:{}",
            dir.path().join("none.json").to_string_lossy()
        );
        assert!(
            KimiOpenCliAdapter
                .discover(&CollectionSessionId::new(), &missing)
                .is_err()
        );
        let directory = format!("manifest:{}", dir.path().to_string_lossy());
        assert!(
            KimiOpenCliAdapter
                .discover(&CollectionSessionId::new(), &directory)
                .is_err()
        );
    }

    #[test]
    fn account_wide_scope_is_never_implicit() {
        for reference in [
            "all",
            "recent:0",
            "recent:101",
            "recent:x",
            "https://www.kimi.com/chat/x",
        ] {
            assert!(
                KimiOpenCliAdapter
                    .discover(&CollectionSessionId::new(), reference)
                    .is_err(),
                "{reference} must be rejected"
            );
        }
    }

    #[test]
    fn chat_id_shapes_are_strict() {
        assert!(is_chat_id(UUID_ID));
        assert!(is_chat_id(XID_ID));
        assert!(is_chat_id(OLD_ID));
        assert!(!is_chat_id("zzzzzzzzzzzzzzzzzzzz"));
        assert!(!is_chat_id("1a13d403e46288e08000099020d9ef29"));
    }

    #[test]
    fn opencli_failures_keep_their_meaning() {
        let yaml = b"ok: false\nerror:\n  code: COMMAND_EXEC\n  message: GetChat returned HTTP 404\n  exitCode: 1\n";
        assert!(matches!(
            classify_failure(yaml),
            ApplicationError::NotFound(_)
        ));

        let json = br#"{"error":{"message":"Please login to Kimi"}}"#;
        assert!(matches!(
            classify_failure(json),
            ApplicationError::Conflict(_)
        ));

        let other = br#"{"error":{"message":"selector timeout"}}"#;
        assert!(matches!(
            classify_failure(other),
            ApplicationError::Storage(_)
        ));

        assert!(matches!(
            classify_failure(b"<html>"),
            ApplicationError::Integrity(_)
        ));
    }

    #[test]
    fn recent_envelopes_keep_their_original_context() {
        let mut recent = discover(&manifest()).unwrap().remove(0).summary;
        recent.hierarchy = vec![
            "Kimi".to_owned(),
            "Recent conversations".to_owned(),
            "t".to_owned(),
        ];
        assert_eq!(envelope_context(&recent), RECENT_CONTEXT);
        recent.hierarchy.clear();
        assert_eq!(envelope_context(&recent), RECENT_CONTEXT);
    }

    #[test]
    fn detail_requires_one_matching_complete_conversation() {
        let candidate = discover(&manifest()).unwrap().remove(0).summary;
        let valid = serde_json::json!([{
            "ChatId": UUID_ID, "Chat": {"id": UUID_ID},
            "Messages": [{"id": "m1", "content": "native text"}],
            "MessageCount": 1, "MessagePages": 1,
            "Complete": true, "Termination": "no-token"
        }]);
        let first = acquisition_from_detail(&candidate, UUID_ID, &valid).unwrap();
        let AcquisitionOutcome::Found {
            candidate: envelope,
            assets,
        } = first
        else {
            panic!("expected found")
        };
        assert!(assets.is_empty());
        assert_eq!(envelope.native_id.as_deref(), Some(UUID_ID));
        let CandidatePayload::Text { text } = &envelope.payload;
        assert_eq!(envelope.payload_sha256, Sha256::of_bytes(text.as_bytes()));
        let restored: Value = serde_json::from_str(text).unwrap();
        assert_eq!(restored["messages"], valid[0]["Messages"]);
        let AcquisitionOutcome::Found {
            candidate: repeated,
            ..
        } = acquisition_from_detail(&candidate, UUID_ID, &valid).unwrap()
        else {
            panic!("expected found")
        };
        assert_eq!(envelope.payload_sha256, repeated.payload_sha256);
        for (key, value) in [
            ("ChatId", Value::from(XID_ID)),
            ("Chat", serde_json::json!({"id": XID_ID})),
            ("Complete", Value::from(false)),
            ("MessageCount", Value::from(2)),
            ("MessagePages", Value::from(0)),
            ("Termination", Value::from("page-limit")),
            ("Messages", Value::Null),
        ] {
            let mut corrupt = valid.clone();
            corrupt[0][key] = value;
            assert!(
                matches!(
                    acquisition_from_detail(&candidate, UUID_ID, &corrupt),
                    Err(ApplicationError::Integrity(_))
                ),
                "{key}"
            );
        }
        let multiple = serde_json::json!([valid[0], valid[0]]);
        assert!(acquisition_from_detail(&candidate, UUID_ID, &multiple).is_err());
        assert!(matches!(
            acquisition_from_detail(&candidate, UUID_ID, &serde_json::json!([])),
            Ok(AcquisitionOutcome::Inaccessible { .. })
        ));
        for message in [
            "Kimi session expired (HTTP 401)",
            "ListMessages HTTP 403",
            "Kimi session token is unavailable in the page",
        ] {
            let stderr = serde_json::json!({"error": {"message": message}}).to_string();
            assert!(matches!(
                classify_failure(stderr.as_bytes()),
                ApplicationError::Conflict(_)
            ));
        }
    }
}

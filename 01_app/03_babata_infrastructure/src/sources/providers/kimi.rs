use std::process::Command;

use babata_application::{
    AcquisitionOutcome, ApplicationError, DiscoveredCandidate, ports::SourceAdapterPort,
};
use babata_domain::{
    CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus, CollectionSessionId,
    ContentType, Metadata, RouteCoverage, Sha256, SourceRouteDescriptor, SourceRouteId,
};
use serde_json::Value;

const ROUTE_ID: &str = "source.kimi";
const ADAPTER_VERSION: &str = "opencli-kimi/2";

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
        let limit = source_reference
            .strip_prefix("recent:")
            .ok_or_else(|| {
                ApplicationError::Conflict(
                    "Kimi scope must be recent:<count>; account-wide all is never implicit"
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
                "conversations exceeding one 100-message response are rejected until pagination is proven"
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
            context: Some("Kimi / Recent conversations".to_owned()),
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
    let bytes = if output.status.success() {
        &output.stdout
    } else {
        &output.stderr
    };
    let value: Value = serde_json::from_slice(bytes).map_err(|_| {
        ApplicationError::Integrity("OpenCLI returned a non-JSON response".to_owned())
    })?;
    if output.status.success() {
        return Ok(value);
    }
    let message = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("OpenCLI Kimi command failed")
        .to_owned();
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("not found") || normalized.contains("not exist") {
        Err(ApplicationError::NotFound(message))
    } else if normalized.contains("login")
        || normalized.contains("connect")
        || normalized.contains("permission")
    {
        Err(ApplicationError::Conflict(message))
    } else {
        Err(ApplicationError::Storage(message))
    }
}

fn required_string(value: &Value, key: &str) -> Result<String, ApplicationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApplicationError::Integrity(format!("OpenCLI result has no {key}")))
}

use std::process::Command;

use babata_application::{
    AcquisitionOutcome, ApplicationError, DiscoveredCandidate, ports::SourceAdapterPort,
};
use babata_domain::{
    CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus, CollectionSessionId,
    ContentType, Metadata, RouteCoverage, Sha256, SourceRouteDescriptor, SourceRouteId,
    UtcTimestamp,
};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const ROUTE_ID: &str = "source.doubao";
const ADAPTER_VERSION: &str = "opencli-doubao/1";

#[derive(Debug, Default, Clone, Copy)]
pub struct DoubaoOpenCliAdapter;

impl SourceAdapterPort for DoubaoOpenCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        SourceRouteDescriptor {
            id: SourceRouteId(ROUTE_ID.to_owned()),
            provider: "doubao".to_owned(),
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
                    "Doubao scope must be recent:<count>; account-wide all is never implicit"
                        .to_owned(),
                )
            })?
            .parse::<usize>()
            .map_err(|_| ApplicationError::Conflict("invalid Doubao recent count".to_owned()))?;
        if !(1..=20).contains(&limit) {
            return Err(ApplicationError::Conflict(
                "Doubao recent count must be between 1 and 20".to_owned(),
            ));
        }
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
                    },
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
        let conversation_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Doubao candidate has no conversation ID".to_owned())
        })?;
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
        let payload = serde_json::to_string_pretty(&serde_json::json!({
            "platform": "doubao",
            "conversation_id": conversation_id,
            "title": candidate.title,
            "conversation_info": info,
            "messages": messages,
        }))
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        let content_fingerprint = Sha256::of_bytes(
            serde_json::json!({
                "conversation_id": conversation_id,
                "title": candidate.title,
                "messages": messages,
            })
            .to_string()
            .as_bytes(),
        );
        let metadata = Metadata::parse(
            &serde_json::json!({
                "title": candidate.title,
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
            candidate: CandidateEnvelope {
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
            },
            assets: Vec::new(),
        })
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: false,
            revisions: true,
            limitations: vec![
                "the complete conversation/info and chain/single structures are preserved"
                    .to_owned(),
                "conversation chains reporting has_more are rejected until pagination is proven"
                    .to_owned(),
                "attachment and media metadata is preserved but binary download is not yet covered"
                    .to_owned(),
            ],
        }
    }
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

use std::process::Command;

use babata_application::{
    AcquisitionOutcome, ApplicationError, DiscoveredCandidate, ports::SourceAdapterPort,
};
use babata_domain::{
    CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus, CollectionSessionId,
    ContentType, Metadata, RouteCoverage, Sha256, SourceRouteDescriptor, SourceRouteId,
};
use serde_json::Value;

const ROUTE_ID: &str = "source.chatgpt";
const ADAPTER_VERSION: &str = "opencli-chatgpt/1";

#[derive(Debug, Default, Clone, Copy)]
pub struct ChatGptOpenCliAdapter;

impl SourceAdapterPort for ChatGptOpenCliAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        SourceRouteDescriptor {
            id: SourceRouteId(ROUTE_ID.to_owned()),
            provider: "chatgpt".to_owned(),
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
                    "ChatGPT scope must be recent:<count>; account-wide all is never implicit"
                        .to_owned(),
                )
            })?
            .parse::<usize>()
            .map_err(|_| ApplicationError::Conflict("invalid ChatGPT recent count".to_owned()))?;
        if !(1..=50).contains(&limit) {
            return Err(ApplicationError::Conflict(
                "ChatGPT recent count must be between 1 and 50".to_owned(),
            ));
        }
        let output = run_opencli(&[
            "chatgpt",
            "history-full",
            "--limit",
            &limit.to_string(),
            "--window",
            "foreground",
            "--site-session",
            "persistent",
            "--keep-tab",
            "false",
            "-f",
            "json",
        ])?;
        let rows = output.as_array().ok_or_else(|| {
            ApplicationError::Integrity("OpenCLI ChatGPT history-full was not an array".to_owned())
        })?;
        rows.iter()
            .map(|row| {
                let id = required_string(row, "Id")?;
                let title = required_string(row, "Title")?;
                let url = required_string(row, "Url")?;
                Ok(DiscoveredCandidate {
                    summary: CandidateSummary {
                        candidate_id: format!("chatgpt_{id}"),
                        session_id: session_id.clone(),
                        route_id: SourceRouteId(ROUTE_ID.to_owned()),
                        source_native_id: Some(id),
                        title: Some(title.clone()),
                        source_location: Some(url),
                        hierarchy: vec![
                            "ChatGPT".to_owned(),
                            "Recent conversations".to_owned(),
                            title,
                        ],
                        content_type: ContentType::Document,
                        source_updated_at: None,
                        attachment_available: None,
                        limitations: vec![
                            "candidate discovery is bounded to the requested recent-chat window"
                                .to_owned(),
                            "the current ChatGPT sidebar requires a foreground tab to mount history links"
                                .to_owned(),
                            "history metadata does not expose update time or attachment inventory"
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
        let conversation_id = candidate.source_native_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("ChatGPT candidate has no conversation ID".to_owned())
        })?;
        let output = match run_opencli(&[
            "chatgpt",
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
        ]) {
            Ok(output) => output,
            Err(ApplicationError::NotFound(reason)) => {
                return Ok(AcquisitionOutcome::Removed { reason });
            }
            Err(ApplicationError::Conflict(reason)) => {
                return Ok(AcquisitionOutcome::Inaccessible { reason });
            }
            Err(error) => return Err(error),
        };
        acquisition_from_detail(candidate, conversation_id, &output)
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: false,
            revisions: true,
            limitations: vec![
                "message roles, complete visible text, citations and attachment references are preserved"
                    .to_owned(),
                "binary conversation attachment download is not yet covered".to_owned(),
                "history discovery currently opens a temporary foreground tab".to_owned(),
            ],
        }
    }
}

fn acquisition_from_detail(
    candidate: &CandidateSummary,
    conversation_id: &str,
    output: &Value,
) -> Result<AcquisitionOutcome, ApplicationError> {
    let rows = output.as_array().ok_or_else(|| {
        ApplicationError::Integrity("OpenCLI ChatGPT detail-full was not an array".to_owned())
    })?;
    let Some(row) = rows.first() else {
        return Ok(AcquisitionOutcome::Inaccessible {
            reason: "ChatGPT returned no structured conversation".to_owned(),
        });
    };
    if row.get("Complete").and_then(Value::as_bool) != Some(true) {
        return Err(ApplicationError::Integrity(
            "ChatGPT conversation was still generating; C0 was not written".to_owned(),
        ));
    }
    let messages = row
        .get("Messages")
        .and_then(Value::as_array)
        .filter(|messages| !messages.is_empty())
        .ok_or_else(|| {
            ApplicationError::Integrity("ChatGPT detail-full has no Messages array".to_owned())
        })?;
    let payload = serde_json::to_string_pretty(&serde_json::json!({
        "platform": "chatgpt",
        "conversation_id": conversation_id,
        "title": candidate.title,
        "messages": messages,
    }))
    .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let content_fingerprint = content_fingerprint(messages);
    let attachment_count = row
        .get("AttachmentCount")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let metadata = Metadata::parse(
        &serde_json::json!({
            "title": candidate.title,
            "conversation_id": conversation_id,
            "message_count": messages.len(),
            "citation_count": row.get("CitationCount").and_then(Value::as_u64),
            "attachment_count": attachment_count,
            "attachments_covered": attachment_count == 0,
            "content_fingerprint": content_fingerprint.as_str(),
            "adapter_version": ADAPTER_VERSION,
            "complete_visible_conversation": true,
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
                .unwrap_or_else(|| format!("https://chatgpt.com/c/{conversation_id}")),
            content_type: ContentType::Document,
            payload_sha256: Sha256::of_bytes(payload.as_bytes()),
            metadata,
            payload: CandidatePayload::Text { text: payload },
            context: Some("ChatGPT / Recent conversations".to_owned()),
            native_id: Some(conversation_id.to_owned()),
            common_metadata: candidate.effective_common_metadata(),
        }),
        assets: Vec::new(),
    })
}

fn content_fingerprint(messages: &[Value]) -> Sha256 {
    let stable = messages
        .iter()
        .map(|message| {
            serde_json::json!({
                "role": message.get("role"),
                "text": message.get("text"),
                "citations": message.get("citations"),
                "attachments": message
                    .get("attachments")
                    .and_then(Value::as_array)
                    .map(|attachments| attachments.iter().map(|attachment| attachment.get("name")).collect::<Vec<_>>()),
            })
        })
        .collect::<Vec<_>>();
    Sha256::of_bytes(
        serde_json::to_string(&stable)
            .unwrap_or_default()
            .as_bytes(),
    )
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
    let value = decode_opencli_output(output.status.success(), &output.stdout, &output.stderr)?;
    if output.status.success() {
        return Ok(value);
    }
    let message = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("OpenCLI ChatGPT command failed")
        .to_owned();
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("not found") || normalized.contains("missing") {
        Err(ApplicationError::NotFound(message))
    } else if normalized.contains("login")
        || normalized.contains("permission")
        || normalized.contains("access")
    {
        Err(ApplicationError::Conflict(message))
    } else {
        Err(ApplicationError::Storage(message))
    }
}

fn decode_opencli_output(
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<Value, ApplicationError> {
    let bytes = if success || stderr.iter().all(u8::is_ascii_whitespace) {
        stdout
    } else {
        stderr
    };
    serde_json::from_slice(bytes).map_err(|_| {
        let response = String::from_utf8_lossy(bytes);
        let readable = response.split_whitespace().collect::<Vec<_>>().join(" ");
        let excerpt = readable.chars().take(240).collect::<String>();
        let detail = if excerpt.is_empty() {
            "empty response".to_owned()
        } else {
            excerpt
        };
        ApplicationError::Storage(format!("OpenCLI returned a non-JSON response: {detail}"))
    })
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
    use super::{content_fingerprint, decode_opencli_output};
    use babata_application::ApplicationError;
    use serde_json::json;

    #[test]
    fn content_fingerprint_ignores_signed_attachment_urls_but_tracks_text() {
        let first = vec![json!({
            "role": "user",
            "text": "stable",
            "citations": [],
            "attachments": [{"name": "report.pdf", "href": "https://signed.example/one"}]
        })];
        let refreshed = vec![json!({
            "role": "user",
            "text": "stable",
            "citations": [],
            "attachments": [{"name": "report.pdf", "href": "https://signed.example/two"}]
        })];
        let changed = vec![json!({
            "role": "user",
            "text": "changed",
            "citations": [],
            "attachments": [{"name": "report.pdf", "href": "https://signed.example/two"}]
        })];
        assert_eq!(content_fingerprint(&first), content_fingerprint(&refreshed));
        assert_ne!(content_fingerprint(&first), content_fingerprint(&changed));
    }

    #[test]
    fn non_json_opencli_response_is_a_readable_storage_failure() {
        let error = decode_opencli_output(true, b"temporary page timeout\nretry later", b"")
            .expect_err("non-JSON output must fail");
        assert!(matches!(error, ApplicationError::Storage(_)));
        assert_eq!(
            error.to_string(),
            "storage failure: OpenCLI returned a non-JSON response: temporary page timeout retry later"
        );
    }

    #[test]
    fn failed_opencli_command_uses_stdout_when_stderr_is_empty() {
        let error = decode_opencli_output(false, b"browser connection unavailable", b"  \n")
            .expect_err("non-JSON failure must fail");
        assert_eq!(
            error.to_string(),
            "storage failure: OpenCLI returned a non-JSON response: browser connection unavailable"
        );
    }
}

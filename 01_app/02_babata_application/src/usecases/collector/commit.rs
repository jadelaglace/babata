use super::{CollectorSessionService, content_fingerprint, require_text};
use crate::{
    ApplicationError, CollectionCommitOperation,
    ports::{AssetStorePort, ClockPort, CollectionRepositoryPort, RawRepositoryPort},
};
use babata_domain::{
    CandidateEnvelope, CandidatePayload, CollectionItemState, CollectionItemStatus,
    CollectionSelection, CollectionSessionState, RawState, Sha256,
};
use serde_json::Value;

const PROTOCOL: &str = "babata.collection.commit/v1";

impl<R, A, C> CollectorSessionService<R, A, C>
where
    R: CollectionRepositoryPort + RawRepositoryPort + Clone,
    A: AssetStorePort + Clone,
    C: ClockPort + Clone,
{
    /// A deliberately narrow sample: one frozen, fully paginated text conversation.
    pub fn commit(
        &self,
        request_key: &str,
        selection: CollectionSelection,
    ) -> Result<CollectionCommitOperation, ApplicationError> {
        validate_key(request_key)?;
        let _guard = self
            .repository
            .acquire_collection_write()
            .map_err(|error| error.with_operation(request_key))?;
        self.commit_locked(request_key, selection)
            .map_err(|error| error.with_operation(request_key))
    }

    pub fn operation(
        &self,
        request_key: &str,
    ) -> Result<CollectionCommitOperation, ApplicationError> {
        validate_key(request_key)?;
        let operation = self
            .repository
            .commit_operation(request_key)?
            .ok_or_else(|| ApplicationError::NotFound("collection commit operation".to_owned()))?;
        let (digest, envelope) = self.frozen_commit_input(&operation.selection)?;
        if operation.protocol_version != PROTOCOL
            || operation.request_key != request_key
            || operation.input_sha256 != digest
        {
            return Err(ApplicationError::Integrity(
                "stored operation binding is invalid".to_owned(),
            ));
        }
        if let Some(result) = operation
            .result
            .as_ref()
            .filter(|item| item.state == CollectionItemState::Saved)
        {
            let verified = self
                .verify_committed(&operation.selection, &envelope)?
                .ok_or_else(|| {
                    ApplicationError::Integrity(
                        "saved operation has no durable observation".to_owned(),
                    )
                })?;
            if result.item_id != verified.item_id || result.revision_id != verified.revision_id {
                return Err(ApplicationError::Integrity(
                    "operation result differs from its durable commit".to_owned(),
                ));
            }
        }
        Ok(operation)
    }

    #[allow(clippy::too_many_lines)]
    fn commit_locked(
        &self,
        key: &str,
        selection: CollectionSelection,
    ) -> Result<CollectionCommitOperation, ApplicationError> {
        let (digest, envelope) = self.frozen_commit_input(&selection)?;
        let proposed = CollectionCommitOperation {
            protocol_version: PROTOCOL.to_owned(),
            request_key: key.to_owned(),
            input_sha256: digest,
            selection: selection.clone(),
            result: None,
        };
        let bound = self.repository.bind_commit_operation(&proposed)?;
        if let Some(result) = &bound.result {
            if result.state == CollectionItemState::Saved {
                let verified = self
                    .verify_committed(&selection, &envelope)?
                    .ok_or_else(|| {
                        ApplicationError::Integrity(
                            "saved operation has no durable observation".to_owned(),
                        )
                    })?;
                if result.item_id != verified.item_id || result.revision_id != verified.revision_id
                {
                    return Err(ApplicationError::Integrity(
                        "saved operation differs from durable commit".to_owned(),
                    ));
                }
            }
            return Ok(bound);
        }
        let candidate_id = &selection.candidate_ids[0];
        if let Some(verified) = self.verify_committed(&selection, &envelope)? {
            let result = self.repository.transition_item(
                &selection.session_id,
                candidate_id,
                CollectionItemState::Saved,
                None,
                false,
                verified.item_id.as_ref(),
                verified.revision_id.as_ref(),
                false,
                &self.clock.now(),
            )?;
            self.finish_session(&selection.session_id)?;
            return self.repository.complete_commit_operation(key, &result);
        }
        let session = self.session(&selection.session_id)?;
        if session.state == CollectionSessionState::Cancelled {
            return Err(ApplicationError::Conflict(
                "collection was cancelled; no uncommitted work is restarted".to_owned(),
            ));
        }
        let current = self
            .status(&selection.session_id)?
            .into_iter()
            .find(|item| item.candidate_id == *candidate_id);
        if let Some(current) = current {
            match current.state {
                CollectionItemState::Saved => {
                    return Err(ApplicationError::Integrity(
                        "saved item has no durable observation".to_owned(),
                    ));
                }
                CollectionItemState::Failed | CollectionItemState::Skipped => {
                    return self.repository.complete_commit_operation(key, &current);
                }
                CollectionItemState::Running => {
                    // Holding the OS lock proves no other Collector can still own this work.
                    self.repository.transition_item(
                        &selection.session_id,
                        candidate_id,
                        CollectionItemState::Queued,
                        Some("recovered before durable capture"),
                        false,
                        None,
                        None,
                        false,
                        &self.clock.now(),
                    )?;
                }
                CollectionItemState::Queued => {}
            }
        }
        if session.state == CollectionSessionState::Running {
            self.repository.update_session_state(
                &selection.session_id,
                CollectionSessionState::AwaitingSelection,
                &self.clock.now(),
            )?;
        }
        let result = self
            .select_locked(selection.clone())?
            .into_iter()
            .find(|item| item.candidate_id == *candidate_id)
            .ok_or_else(|| {
                ApplicationError::Integrity("commit produced no item status".to_owned())
            })?;
        if result.state == CollectionItemState::Saved {
            self.verify_committed(&selection, &envelope)?
                .ok_or_else(|| {
                    ApplicationError::Integrity(
                        "capture returned saved without durable observation".to_owned(),
                    )
                })?;
        }
        self.repository.complete_commit_operation(key, &result)
    }

    fn frozen_commit_input(
        &self,
        selection: &CollectionSelection,
    ) -> Result<(Sha256, CandidateEnvelope), ApplicationError> {
        if !selection.confirmed
            || selection.candidate_ids.len() != 1
            || selection.requested_attachments
        {
            return Err(ApplicationError::Conflict(
                "commit/v1 requires one confirmed frozen text candidate without attachments"
                    .to_owned(),
            ));
        }
        require_text("scope_description", &selection.scope_description)?;
        require_text("authorised_context", &selection.authorised_context)?;
        let session = self.session(&selection.session_id)?;
        if session.route_id.0 != "source.doubao"
            || selection.authorised_context != session.authorisation_id
        {
            return Err(ApplicationError::Conflict(
                "commit/v1 route or authorisation does not match".to_owned(),
            ));
        }
        let (summary, envelope) = self
            .repository
            .candidate(&selection.session_id, &selection.candidate_ids[0])?
            .ok_or_else(|| ApplicationError::NotFound("frozen candidate".to_owned()))?;
        let envelope = envelope.ok_or_else(|| {
            ApplicationError::Conflict(
                "commit/v1 requires a prefetched acquisition, not a live URL".to_owned(),
            )
        })?;
        validate_frozen_text(&envelope)?;
        let metadata: Value = serde_json::from_str(&envelope.metadata.to_json())
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        if metadata
            .get("_doubao_acquisition_route")
            .and_then(Value::as_str)
            != Some("chrome_native")
        {
            return Err(ApplicationError::Conflict(
                "commit/v1 requires a frozen validated handoff".to_owned(),
            ));
        }
        // This route marker guarantees the adapter uses local handoff validation, not a browser fetch.
        self.adapter(&session.route_id)?
            .collect(&summary, Some(&envelope), false)?;
        let bytes = serde_json::to_vec(&(
            PROTOCOL,
            &session.route_id,
            &session.source_reference,
            &session.scope_description,
            &session.authorisation_id,
            selection,
            summary,
            &envelope,
        ))
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        Ok((Sha256::of_bytes(&bytes), envelope))
    }

    fn verify_committed(
        &self,
        selection: &CollectionSelection,
        envelope: &CandidateEnvelope,
    ) -> Result<Option<CollectionItemStatus>, ApplicationError> {
        let candidate = &selection.candidate_ids[0];
        let Some((item, revision)) = self
            .repository
            .committed_candidate(&selection.session_id, candidate)?
        else {
            return Ok(None);
        };
        let detail = self.repository.load_detail(&item)?;
        let stored = detail
            .revisions
            .iter()
            .find(|stored| stored.revision_id == revision)
            .ok_or_else(|| {
                ApplicationError::Integrity("committed revision is missing".to_owned())
            })?;
        let valid_text = stored.raw_text.as_ref().is_some_and(|text| {
            stored.text_sha256.as_deref() == Some(Sha256::of_bytes(text.as_bytes()).as_str())
        });
        if stored.state != RawState::Ready
            || !valid_text
            || detail.provider != "doubao"
            || detail.source_native_id != envelope.native_id
            || detail.source_locator.as_deref() != Some(envelope.source_reference.as_str())
            || content_fingerprint(&stored.metadata) != content_fingerprint(&envelope.metadata)
            || detail
                .assets
                .iter()
                .any(|asset| asset.revision_id == revision)
        {
            return Err(ApplicationError::Integrity(
                "committed text failed identity/hash/readiness readback".to_owned(),
            ));
        }
        Ok(Some(CollectionItemStatus {
            session_id: selection.session_id.clone(),
            candidate_id: candidate.clone(),
            state: CollectionItemState::Saved,
            attempt_count: 0,
            reason: None,
            retryable: false,
            requested_attachments: false,
            item_id: Some(item),
            revision_id: Some(revision),
            updated_at: self.clock.now(),
        }))
    }
}

fn validate_key(key: &str) -> Result<(), ApplicationError> {
    if key.is_empty()
        || key.len() > 128
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
    {
        return Err(ApplicationError::Conflict(
            "request key must be 1..128 ASCII letters/digits or -_.:".to_owned(),
        ));
    }
    Ok(())
}

fn validate_frozen_text(envelope: &CandidateEnvelope) -> Result<(), ApplicationError> {
    let CandidatePayload::Text { text } = &envelope.payload;
    if envelope.protocol_version != "1"
        || envelope.route_id.0 != "source.doubao"
        || Sha256::of_bytes(text.as_bytes()) != envelope.payload_sha256
    {
        return Err(ApplicationError::Integrity(
            "frozen candidate version/route/hash is invalid".to_owned(),
        ));
    }
    let payload: Value = serde_json::from_str(text)
        .map_err(|_| ApplicationError::Integrity("frozen payload is invalid JSON".to_owned()))?;
    let messages = payload
        .get("messages")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
        .ok_or_else(|| ApplicationError::Integrity("frozen payload has no messages".to_owned()))?;
    if payload.get("has_more").and_then(Value::as_bool) != Some(false)
        || content_fingerprint(&envelope.metadata).is_none()
    {
        return Err(ApplicationError::Integrity(
            "frozen payload is incomplete or has no fingerprint".to_owned(),
        ));
    }
    let mut indices = std::collections::HashSet::new();
    for message in messages {
        let index = message.get("index_in_conv").and_then(|value| {
            value
                .as_str()
                .and_then(|value| value.parse::<usize>().ok())
                .or_else(|| value.as_u64().and_then(|value| usize::try_from(value).ok()))
        });
        if !index.is_some_and(|index| index > 0 && index <= messages.len() && indices.insert(index))
        {
            return Err(ApplicationError::Integrity(
                "text sample has missing, duplicate or non-contiguous native message indices"
                    .to_owned(),
            ));
        }
        if contains_media_shape(message) {
            return Err(not_text());
        }
        if message.get("content_type").and_then(Value::as_u64) != Some(9999) {
            return Err(not_text());
        }
        let blocks = message
            .get("content_block")
            .and_then(Value::as_array)
            .filter(|blocks| !blocks.is_empty())
            .ok_or_else(not_text)?;
        for block in blocks {
            if block.get("block_type").and_then(Value::as_u64) != Some(10000) {
                return Err(not_text());
            }
            let content = block
                .get("content")
                .and_then(Value::as_object)
                .ok_or_else(not_text)?;
            if content
                .get("text_block")
                .and_then(|text| text.get("text"))
                .and_then(Value::as_str)
                .is_none()
            {
                return Err(not_text());
            }
            if content
                .keys()
                .any(|key| key != "text_block" && key != "pc_event_block")
            {
                return Err(not_text());
            }
        }
    }
    Ok(())
}
fn contains_media_shape(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            (matches!(
                key.as_str(),
                "image"
                    | "image_ori"
                    | "image_thumb"
                    | "image_preview"
                    | "audio"
                    | "video"
                    | "file"
                    | "attachments"
                    | "attachment_block"
                    | "search_query_result_block"
                    | "sub_message_v2"
            ) && !value.is_null()
                && value != ""
                && value != &serde_json::json!([])
                && value != &serde_json::json!({}))
                || contains_media_shape(value)
        }),
        Value::Array(values) => values.iter().any(contains_media_shape),
        Value::String(text) if text.starts_with('{') || text.starts_with('[') => {
            serde_json::from_str::<Value>(text).is_ok_and(|value| contains_media_shape(&value))
        }
        _ => false,
    }
}
fn not_text() -> ApplicationError {
    ApplicationError::Conflict("commit/v1 supports validated text blocks only; media/unknown shapes require the existing acquisition path".to_owned())
}

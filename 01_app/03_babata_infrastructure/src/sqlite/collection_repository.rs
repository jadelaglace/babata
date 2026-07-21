use babata_application::{
    ApplicationError,
    ports::{CollectionRepositoryPort, NewSourceObservation},
};
use babata_domain::{
    CandidateEnvelope, CandidateSummary, CollectionItemState, CollectionItemStatus,
    CollectionSelection, CollectionSession, CollectionSessionId, CollectionSessionState,
    CommonSourceMetadata, ContentType, ItemId, RecollectionOutcome, RevisionId, SourceRouteId,
    UtcTimestamp,
};
use rusqlite::{OptionalExtension, Row, params};

use super::SqliteRawRepository;

impl CollectionRepositoryPort for SqliteRawRepository {
    fn create_session(&self, session: &CollectionSession) -> Result<(), ApplicationError> {
        self.lock()?
            .execute(
                "INSERT INTO collection_sessions
                 (session_id, route_id, source_reference, scope_description, authorisation_id,
                  state, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    session.session_id.to_string(),
                    session.route_id.0,
                    session.source_reference,
                    session.scope_description,
                    session.authorisation_id,
                    session_state(session.state),
                    session.created_at.as_str(),
                    session.updated_at.as_str(),
                ],
            )
            .map_err(storage)?;
        Ok(())
    }

    fn update_session_state(
        &self,
        session_id: &CollectionSessionId,
        state: CollectionSessionState,
        updated_at: &UtcTimestamp,
    ) -> Result<(), ApplicationError> {
        let changed = self
            .lock()?
            .execute(
                "UPDATE collection_sessions SET state = ?2, updated_at = ?3 WHERE session_id = ?1",
                params![
                    session_id.to_string(),
                    session_state(state),
                    updated_at.as_str()
                ],
            )
            .map_err(storage)?;
        if changed == 0 {
            return Err(ApplicationError::NotFound(format!(
                "collection session: {session_id}"
            )));
        }
        Ok(())
    }

    fn session(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<Option<CollectionSession>, ApplicationError> {
        self.lock()?
            .query_row(
                "SELECT session_id, route_id, source_reference, scope_description,
                        authorisation_id, state, created_at, updated_at
                 FROM collection_sessions WHERE session_id = ?1",
                params![session_id.to_string()],
                session_from_row,
            )
            .optional()
            .map_err(storage)
    }

    fn save_candidates(
        &self,
        candidates: &[(CandidateSummary, Option<CandidateEnvelope>)],
    ) -> Result<(), ApplicationError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction().map_err(storage)?;
        for (candidate, prefetched) in candidates {
            let common_metadata = candidate.effective_common_metadata();
            common_metadata.validate()?;
            let prefetched = prefetched
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(json)?;
            transaction
                .execute(
                    "INSERT INTO collection_candidates
                 (session_id, candidate_id, route_id, source_native_id, title, source_location,
                  hierarchy_json, content_type, source_updated_at, attachment_available,
                  limitations_json, selection_capabilities_json, prefetched_envelope_json,
                  common_metadata_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        candidate.session_id.to_string(),
                        candidate.candidate_id,
                        candidate.route_id.0,
                        candidate.source_native_id,
                        candidate.title,
                        candidate.source_location,
                        serde_json::to_string(&candidate.hierarchy).map_err(json)?,
                        content_type(candidate.content_type),
                        candidate
                            .source_updated_at
                            .as_ref()
                            .map(UtcTimestamp::as_str),
                        candidate.attachment_available.map(i64::from),
                        serde_json::to_string(&candidate.limitations).map_err(json)?,
                        serde_json::to_string(&candidate.selection_capabilities).map_err(json)?,
                        prefetched,
                        serde_json::to_string(&common_metadata).map_err(json)?,
                    ],
                )
                .map_err(storage)?;
        }
        transaction.commit().map_err(storage)
    }

    fn candidates(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<Vec<CandidateSummary>, ApplicationError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT session_id, candidate_id, route_id, source_native_id, title,
                        source_location, hierarchy_json, content_type, source_updated_at,
                        attachment_available, limitations_json, selection_capabilities_json,
                        common_metadata_json
                 FROM collection_candidates WHERE session_id = ?1 ORDER BY candidate_id",
            )
            .map_err(storage)?;
        statement
            .query_map(params![session_id.to_string()], candidate_from_row)
            .map_err(storage)?
            .map(|row| row.map_err(storage))
            .collect()
    }

    fn candidate(
        &self,
        session_id: &CollectionSessionId,
        candidate_id: &str,
    ) -> Result<Option<(CandidateSummary, Option<CandidateEnvelope>)>, ApplicationError> {
        self.lock()?
            .query_row(
                "SELECT session_id, candidate_id, route_id, source_native_id, title,
                        source_location, hierarchy_json, content_type, source_updated_at,
                        attachment_available, limitations_json, selection_capabilities_json,
                        common_metadata_json, prefetched_envelope_json
                 FROM collection_candidates WHERE session_id = ?1 AND candidate_id = ?2",
                params![session_id.to_string(), candidate_id],
                |row| {
                    let candidate = candidate_from_row(row)?;
                    let envelope = row
                        .get::<_, Option<String>>(13)?
                        .map(|value| serde_json::from_str(&value).map_err(to_sql_error))
                        .transpose()?;
                    Ok((candidate, envelope))
                },
            )
            .optional()
            .map_err(storage)
    }

    fn enqueue_selection(
        &self,
        selection: &CollectionSelection,
        now: &UtcTimestamp,
    ) -> Result<Vec<CollectionItemStatus>, ApplicationError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction().map_err(storage)?;
        for candidate_id in &selection.candidate_ids {
            let exists = transaction
                .query_row(
                    "SELECT 1 FROM collection_candidates WHERE session_id = ?1 AND candidate_id = ?2",
                    params![selection.session_id.to_string(), candidate_id],
                    |_| Ok(()),
                )
                .optional()
                .map_err(storage)?
                .is_some();
            if !exists {
                return Err(ApplicationError::NotFound(format!(
                    "candidate in session: {candidate_id}"
                )));
            }
            transaction
                .execute(
                    "INSERT INTO collection_items
                     (session_id, candidate_id, state, attempt_count, reason, retryable,
                      requested_attachments, item_id, revision_id, created_at, updated_at)
                     VALUES (?1, ?2, 'queued', 0, NULL, 0, ?3, NULL, NULL, ?4, ?4)
                     ON CONFLICT(session_id, candidate_id) DO NOTHING",
                    params![
                        selection.session_id.to_string(),
                        candidate_id,
                        i64::from(selection.requested_attachments),
                        now.as_str()
                    ],
                )
                .map_err(storage)?;
        }
        transaction.commit().map_err(storage)?;
        drop(connection);
        self.collection_items(&selection.session_id)
    }

    fn collection_items(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<Vec<CollectionItemStatus>, ApplicationError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT session_id, candidate_id, state, attempt_count, reason, retryable,
                        requested_attachments, item_id, revision_id, updated_at
                 FROM collection_items WHERE session_id = ?1 ORDER BY created_at, candidate_id",
            )
            .map_err(storage)?;
        statement
            .query_map(params![session_id.to_string()], item_status_from_row)
            .map_err(storage)?
            .map(|row| row.map_err(storage))
            .collect()
    }

    fn claim_item(
        &self,
        session_id: &CollectionSessionId,
        candidate_id: &str,
        now: &UtcTimestamp,
    ) -> Result<Option<CollectionItemStatus>, ApplicationError> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE collection_items
                 SET state = 'running', reason = NULL, retryable = 0,
                     attempt_count = attempt_count + 1, updated_at = ?3
                 WHERE session_id = ?1 AND candidate_id = ?2
                   AND state IN ('queued', 'failed')
                   AND EXISTS (
                       SELECT 1 FROM collection_sessions
                       WHERE session_id = ?1 AND state != 'cancelled'
                   )",
                params![session_id.to_string(), candidate_id, now.as_str()],
            )
            .map_err(storage)?;
        if changed == 0 {
            let exists = connection
                .query_row(
                    "SELECT 1 FROM collection_items WHERE session_id = ?1 AND candidate_id = ?2",
                    params![session_id.to_string(), candidate_id],
                    |_| Ok(()),
                )
                .optional()
                .map_err(storage)?
                .is_some();
            if !exists {
                return Err(ApplicationError::NotFound(format!(
                    "collection item: {session_id}/{candidate_id}"
                )));
            }
            return Ok(None);
        }
        connection
            .query_row(
                "SELECT session_id, candidate_id, state, attempt_count, reason, retryable,
                        requested_attachments, item_id, revision_id, updated_at
                 FROM collection_items WHERE session_id = ?1 AND candidate_id = ?2",
                params![session_id.to_string(), candidate_id],
                item_status_from_row,
            )
            .optional()
            .map_err(storage)
    }

    fn cancel_session(
        &self,
        session_id: &CollectionSessionId,
        reason: &str,
        now: &UtcTimestamp,
    ) -> Result<Vec<CollectionItemStatus>, ApplicationError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction().map_err(storage)?;
        let changed = transaction
            .execute(
                "UPDATE collection_sessions
                 SET state = 'cancelled', updated_at = ?2
                 WHERE session_id = ?1",
                params![session_id.to_string(), now.as_str()],
            )
            .map_err(storage)?;
        if changed == 0 {
            return Err(ApplicationError::NotFound(format!(
                "collection session: {session_id}"
            )));
        }
        transaction
            .execute(
                "UPDATE collection_items
                 SET state = 'skipped', reason = ?2, retryable = 0, updated_at = ?3
                 WHERE session_id = ?1 AND state = 'queued'",
                params![session_id.to_string(), reason, now.as_str()],
            )
            .map_err(storage)?;
        transaction.commit().map_err(storage)?;
        drop(connection);
        self.collection_items(session_id)
    }

    fn transition_item(
        &self,
        session_id: &CollectionSessionId,
        candidate_id: &str,
        state: CollectionItemState,
        reason: Option<&str>,
        retryable: bool,
        item_id: Option<&ItemId>,
        revision_id: Option<&RevisionId>,
        increment_attempt: bool,
        now: &UtcTimestamp,
    ) -> Result<CollectionItemStatus, ApplicationError> {
        let changed = self
            .lock()?
            .execute(
                "UPDATE collection_items
             SET state = ?3, reason = ?4, retryable = ?5,
                 item_id = COALESCE(?6, item_id), revision_id = COALESCE(?7, revision_id),
                 attempt_count = attempt_count + ?8, updated_at = ?9
             WHERE session_id = ?1 AND candidate_id = ?2",
                params![
                    session_id.to_string(),
                    candidate_id,
                    item_state(state),
                    reason,
                    i64::from(retryable),
                    item_id.map(ToString::to_string),
                    revision_id.map(ToString::to_string),
                    i64::from(increment_attempt),
                    now.as_str()
                ],
            )
            .map_err(storage)?;
        if changed == 0 {
            return Err(ApplicationError::NotFound(format!(
                "collection item: {session_id}/{candidate_id}"
            )));
        }
        self.lock()?
            .query_row(
                "SELECT session_id, candidate_id, state, attempt_count, reason, retryable,
                        requested_attachments, item_id, revision_id, updated_at
                 FROM collection_items WHERE session_id = ?1 AND candidate_id = ?2",
                params![session_id.to_string(), candidate_id],
                item_status_from_row,
            )
            .map_err(storage)
    }

    fn latest_saved_for_item(
        &self,
        item_id: &ItemId,
    ) -> Result<Option<(CollectionSessionId, String)>, ApplicationError> {
        self.lock()?
            .query_row(
                "SELECT session_id, candidate_id FROM collection_items
                 WHERE item_id = ?1 AND state = 'saved' ORDER BY updated_at DESC LIMIT 1",
                params![item_id.to_string()],
                |row| {
                    Ok((
                        CollectionSessionId::parse(row.get::<_, String>(0)?)
                            .map_err(to_sql_error)?,
                        row.get(1)?,
                    ))
                },
            )
            .optional()
            .map_err(storage)
    }

    fn record_recollection(
        &self,
        outcome: &RecollectionOutcome,
        observation: Option<&NewSourceObservation>,
    ) -> Result<(), ApplicationError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        transaction
            .execute(
                "INSERT INTO collection_recollection_checks
             (check_id, item_id, state, previous_revision_id, new_revision_id, reason, checked_at)
             VALUES ('recollect_' || lower(hex(randomblob(16))), ?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    outcome.item_id.to_string(),
                    recollection_state(outcome.state),
                    outcome.previous_revision_id.to_string(),
                    outcome.new_revision_id.as_ref().map(ToString::to_string),
                    outcome.reason,
                    outcome.checked_at.as_str(),
                ],
            )
            .map_err(storage)?;
        if let Some(observation) = observation {
            if observation.item_id != outcome.item_id
                || observation.revision_id != outcome.previous_revision_id
                || observation.recollection_state != Some(outcome.state)
            {
                return Err(ApplicationError::Integrity(
                    "recollection observation does not match its outcome".to_owned(),
                ));
            }
            super::raw_repository::insert_source_observation(&transaction, observation)?;
        }
        transaction.commit().map_err(storage)
    }
}

fn session_from_row(row: &Row<'_>) -> rusqlite::Result<CollectionSession> {
    Ok(CollectionSession {
        session_id: CollectionSessionId::parse(row.get::<_, String>(0)?).map_err(to_sql_error)?,
        route_id: SourceRouteId(row.get(1)?),
        source_reference: row.get(2)?,
        scope_description: row.get(3)?,
        authorisation_id: row.get(4)?,
        state: parse_session_state(&row.get::<_, String>(5)?).map_err(to_sql_error)?,
        created_at: UtcTimestamp::parse(row.get::<_, String>(6)?).map_err(to_sql_error)?,
        updated_at: UtcTimestamp::parse(row.get::<_, String>(7)?).map_err(to_sql_error)?,
    })
}

fn candidate_from_row(row: &Row<'_>) -> rusqlite::Result<CandidateSummary> {
    Ok(CandidateSummary {
        session_id: CollectionSessionId::parse(row.get::<_, String>(0)?).map_err(to_sql_error)?,
        candidate_id: row.get(1)?,
        route_id: SourceRouteId(row.get(2)?),
        source_native_id: row.get(3)?,
        title: row.get(4)?,
        source_location: row.get(5)?,
        hierarchy: serde_json::from_str(&row.get::<_, String>(6)?).map_err(to_sql_error)?,
        content_type: parse_content_type(&row.get::<_, String>(7)?).map_err(to_sql_error)?,
        source_updated_at: row
            .get::<_, Option<String>>(8)?
            .map(UtcTimestamp::parse)
            .transpose()
            .map_err(to_sql_error)?,
        attachment_available: row.get::<_, Option<i64>>(9)?.map(|value| value != 0),
        limitations: serde_json::from_str(&row.get::<_, String>(10)?).map_err(to_sql_error)?,
        selection_capabilities: serde_json::from_str(&row.get::<_, String>(11)?)
            .map_err(to_sql_error)?,
        common_metadata: serde_json::from_str::<CommonSourceMetadata>(&row.get::<_, String>(12)?)
            .map_err(to_sql_error)?,
    }
    .with_common_from_legacy())
}

fn item_status_from_row(row: &Row<'_>) -> rusqlite::Result<CollectionItemStatus> {
    Ok(CollectionItemStatus {
        session_id: CollectionSessionId::parse(row.get::<_, String>(0)?).map_err(to_sql_error)?,
        candidate_id: row.get(1)?,
        state: parse_item_state(&row.get::<_, String>(2)?).map_err(to_sql_error)?,
        attempt_count: row.get::<_, i64>(3)? as u32,
        reason: row.get(4)?,
        retryable: row.get::<_, i64>(5)? != 0,
        requested_attachments: row.get::<_, i64>(6)? != 0,
        item_id: row
            .get::<_, Option<String>>(7)?
            .map(ItemId::parse)
            .transpose()
            .map_err(to_sql_error)?,
        revision_id: row
            .get::<_, Option<String>>(8)?
            .map(RevisionId::parse)
            .transpose()
            .map_err(to_sql_error)?,
        updated_at: UtcTimestamp::parse(row.get::<_, String>(9)?).map_err(to_sql_error)?,
    })
}

fn session_state(state: CollectionSessionState) -> &'static str {
    match state {
        CollectionSessionState::Discovering => "discovering",
        CollectionSessionState::AwaitingSelection => "awaiting_selection",
        CollectionSessionState::Running => "running",
        CollectionSessionState::Completed => "completed",
        CollectionSessionState::Cancelled => "cancelled",
        CollectionSessionState::Failed => "failed",
    }
}

fn parse_session_state(value: &str) -> Result<CollectionSessionState, ApplicationError> {
    match value {
        "discovering" => Ok(CollectionSessionState::Discovering),
        "awaiting_selection" => Ok(CollectionSessionState::AwaitingSelection),
        "running" => Ok(CollectionSessionState::Running),
        "completed" => Ok(CollectionSessionState::Completed),
        "cancelled" => Ok(CollectionSessionState::Cancelled),
        "failed" => Ok(CollectionSessionState::Failed),
        _ => Err(ApplicationError::Integrity(format!(
            "invalid collection session state: {value}"
        ))),
    }
}

fn item_state(state: CollectionItemState) -> &'static str {
    match state {
        CollectionItemState::Queued => "queued",
        CollectionItemState::Running => "running",
        CollectionItemState::Saved => "saved",
        CollectionItemState::Skipped => "skipped",
        CollectionItemState::Failed => "failed",
    }
}

fn parse_item_state(value: &str) -> Result<CollectionItemState, ApplicationError> {
    match value {
        "queued" => Ok(CollectionItemState::Queued),
        "running" => Ok(CollectionItemState::Running),
        "saved" => Ok(CollectionItemState::Saved),
        "skipped" => Ok(CollectionItemState::Skipped),
        "failed" => Ok(CollectionItemState::Failed),
        _ => Err(ApplicationError::Integrity(format!(
            "invalid collection item state: {value}"
        ))),
    }
}

fn recollection_state(state: babata_domain::RecollectionState) -> &'static str {
    match state {
        babata_domain::RecollectionState::Changed => "changed",
        babata_domain::RecollectionState::Unchanged => "unchanged",
        babata_domain::RecollectionState::Inaccessible => "inaccessible",
        babata_domain::RecollectionState::Removed => "removed",
    }
}

fn content_type(value: ContentType) -> &'static str {
    match value {
        ContentType::Text => "text",
        ContentType::Document => "document",
        ContentType::Image => "image",
        ContentType::Audio => "audio",
        ContentType::Video => "video",
        ContentType::WebPage => "web_page",
        ContentType::Archive => "archive",
        ContentType::Unknown => "unknown",
    }
}

fn parse_content_type(value: &str) -> Result<ContentType, ApplicationError> {
    match value {
        "text" => Ok(ContentType::Text),
        "document" => Ok(ContentType::Document),
        "image" => Ok(ContentType::Image),
        "audio" => Ok(ContentType::Audio),
        "video" => Ok(ContentType::Video),
        "web_page" => Ok(ContentType::WebPage),
        "archive" => Ok(ContentType::Archive),
        "unknown" => Ok(ContentType::Unknown),
        _ => Err(ApplicationError::Integrity(format!(
            "invalid content type: {value}"
        ))),
    }
}

fn storage(error: rusqlite::Error) -> ApplicationError {
    ApplicationError::Storage(error.to_string())
}

fn json(error: serde_json::Error) -> ApplicationError {
    ApplicationError::Integrity(error.to_string())
}

fn to_sql_error(error: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        )),
    )
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Barrier, Mutex},
    };

    use babata_application::{
        AcquisitionOutcome, ApplicationError, CancelCollectionCommand, CaptureImportAsset,
        CollectorSessionService, DiscoveredCandidate, RetryCollectionItemCommand,
        StartCollectionCommand,
        ports::{CollectionRepositoryPort, RawRepositoryPort, SourceAdapterPort},
    };
    use babata_domain::{
        AssetRole, CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus,
        CollectionItemState, CollectionSelection, CollectionSessionId, CollectionSessionState,
        ContentType, ItemId, Metadata, RecollectionState, RouteCoverage, Sha256,
        SourceRouteDescriptor, SourceRouteId, UtcTimestamp,
    };
    use rusqlite::params;
    use tempfile::tempdir;

    use crate::{
        FileAssetStore, SqliteRawRepository, SystemClock,
        paths::{DataPaths, ensure_layout},
    };

    #[derive(Clone)]
    struct FixtureAdapter {
        outcomes: Arc<Mutex<HashMap<String, AcquisitionOutcome>>>,
        attachment_path: Option<String>,
    }

    impl SourceAdapterPort for FixtureAdapter {
        fn describe(&self) -> SourceRouteDescriptor {
            SourceRouteDescriptor {
                id: SourceRouteId("source.kimi".to_owned()),
                provider: "kimi".to_owned(),
                status: CapabilityStatus::Enabled,
                activation_phase: "P4".to_owned(),
            }
        }

        fn discover(
            &self,
            session_id: &CollectionSessionId,
            _source_reference: &str,
        ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
            let mut ids = self
                .outcomes
                .lock()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            ids.sort();
            Ok(ids
                .into_iter()
                .map(|candidate_id| DiscoveredCandidate {
                    summary: CandidateSummary {
                        candidate_id: candidate_id.clone(),
                        session_id: session_id.clone(),
                        route_id: SourceRouteId("source.kimi".to_owned()),
                        source_native_id: Some(candidate_id.clone()),
                        title: Some(candidate_id.clone()),
                        source_location: Some(format!("https://example.test/{candidate_id}")),
                        hierarchy: vec!["Fixture".to_owned(), candidate_id],
                        content_type: ContentType::Document,
                        source_updated_at: None,
                        attachment_available: Some(self.attachment_path.is_some()),
                        limitations: Vec::new(),
                        selection_capabilities: vec!["single".to_owned()],
                        common_metadata: babata_domain::CommonSourceMetadata::default(),
                    },
                    prefetched: None,
                })
                .collect())
        }

        fn collect(
            &self,
            candidate: &CandidateSummary,
            _prefetched: Option<&CandidateEnvelope>,
            requested_attachments: bool,
        ) -> Result<AcquisitionOutcome, ApplicationError> {
            let mut outcome = self
                .outcomes
                .lock()
                .unwrap()
                .get(&candidate.candidate_id)
                .cloned()
                .ok_or_else(|| ApplicationError::NotFound(candidate.candidate_id.clone()))?;
            if requested_attachments
                && let (Some(path), AcquisitionOutcome::Found { assets, .. }) =
                    (&self.attachment_path, &mut outcome)
            {
                assets.push(CaptureImportAsset {
                    path: path.clone(),
                    role: AssetRole::Original,
                });
            }
            Ok(outcome)
        }

        fn coverage(&self) -> RouteCoverage {
            RouteCoverage {
                metadata: true,
                attachments: false,
                revisions: true,
                limitations: vec!["fixture".to_owned()],
            }
        }
    }

    struct BlockingAdapter {
        inner: FixtureAdapter,
        entered: Arc<Barrier>,
        release: Arc<Barrier>,
    }

    impl SourceAdapterPort for BlockingAdapter {
        fn describe(&self) -> SourceRouteDescriptor {
            self.inner.describe()
        }

        fn discover(
            &self,
            session_id: &CollectionSessionId,
            source_reference: &str,
        ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
            self.inner.discover(session_id, source_reference)
        }

        fn collect(
            &self,
            candidate: &CandidateSummary,
            prefetched: Option<&CandidateEnvelope>,
            requested_attachments: bool,
        ) -> Result<AcquisitionOutcome, ApplicationError> {
            if candidate.candidate_id == "a" {
                self.entered.wait();
                self.release.wait();
            }
            self.inner
                .collect(candidate, prefetched, requested_attachments)
        }

        fn coverage(&self) -> RouteCoverage {
            self.inner.coverage()
        }
    }

    #[test]
    fn selection_partial_retry_and_recollection_preserve_c0() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        ensure_layout(&paths).unwrap();
        let repository = crate::sqlite::open_collection_database(&paths, 100).unwrap();
        let outcomes = Arc::new(Mutex::new(HashMap::from([
            (
                "a".to_owned(),
                found_with_common_metadata(
                    "a",
                    "version one",
                    "content-one",
                    "first observed title",
                ),
            ),
            (
                "b".to_owned(),
                AcquisitionOutcome::Inaccessible {
                    reason: "temporary permission failure".to_owned(),
                },
            ),
            (
                "c".to_owned(),
                AcquisitionOutcome::Skipped {
                    reason: "locator only".to_owned(),
                },
            ),
        ])));
        let service = CollectorSessionService::new(
            repository.clone(),
            FileAssetStore::new(paths.clone()),
            SystemClock,
            vec![Box::new(FixtureAdapter {
                outcomes: outcomes.clone(),
                attachment_path: None,
            })],
        );
        let session = service
            .start(StartCollectionCommand {
                route_id: SourceRouteId("source.kimi".to_owned()),
                source_reference: "submitted:fixture".to_owned(),
                scope_description: "three visible fixture candidates".to_owned(),
                authorisation_id: "fixture-authorisation".to_owned(),
            })
            .unwrap();
        let discovered = service.candidates(&session.session_id).unwrap();
        let candidate_a = discovered
            .iter()
            .find(|candidate| candidate.candidate_id == "a")
            .unwrap();
        assert_eq!(candidate_a.common_metadata.title.as_deref(), Some("a"));
        assert_eq!(candidate_a.common_metadata.hierarchy[0].name, "Fixture");
        assert_eq!(table_count(&repository, "items"), 0);
        assert!(
            service
                .select(selection(&session.session_id, false))
                .is_err()
        );
        assert_eq!(table_count(&repository, "items"), 0);

        let mut empty_scope = selection(&session.session_id, true);
        empty_scope.scope_description.clear();
        assert!(service.select(empty_scope).is_err());
        let mut duplicate_scope = selection(&session.session_id, true);
        duplicate_scope.candidate_ids = vec!["a".to_owned(), "a".to_owned()];
        assert!(service.select(duplicate_scope).is_err());
        let mut mismatched_authorisation = selection(&session.session_id, true);
        mismatched_authorisation.authorised_context = "different-authorisation".to_owned();
        assert!(service.select(mismatched_authorisation).is_err());
        assert_eq!(table_count(&repository, "items"), 0);

        let items = service
            .select(selection(&session.session_id, true))
            .unwrap();
        assert_eq!(state(&items, "a"), CollectionItemState::Saved);
        assert_eq!(state(&items, "b"), CollectionItemState::Failed);
        assert_eq!(state(&items, "c"), CollectionItemState::Skipped);
        assert_eq!(table_count(&repository, "items"), 1);

        outcomes
            .lock()
            .unwrap()
            .insert("b".to_owned(), found("b", "retry succeeds"));
        let retried = service
            .retry(RetryCollectionItemCommand {
                session_id: session.session_id.clone(),
                candidate_id: "b".to_owned(),
            })
            .unwrap();
        assert_eq!(retried.state, CollectionItemState::Saved);
        assert_eq!(retried.attempt_count, 2);
        assert_eq!(table_count(&repository, "items"), 2);

        let item_id = items
            .iter()
            .find(|item| item.candidate_id == "a")
            .and_then(|item| item.item_id.clone())
            .unwrap();
        assert_recollection_transitions(&service, &repository, &outcomes, &item_id);
    }

    #[test]
    fn cancellation_keeps_started_success_and_skips_every_queued_item() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        ensure_layout(&paths).unwrap();
        let repository = crate::sqlite::open_collection_database(&paths, 100).unwrap();
        let outcomes = Arc::new(Mutex::new(HashMap::from([
            ("a".to_owned(), found("a", "already running")),
            ("b".to_owned(), found("b", "still queued")),
            ("c".to_owned(), found("c", "still queued")),
        ])));
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let selecting = CollectorSessionService::new(
            repository.clone(),
            FileAssetStore::new(paths.clone()),
            SystemClock,
            vec![Box::new(BlockingAdapter {
                inner: FixtureAdapter {
                    outcomes: outcomes.clone(),
                    attachment_path: None,
                },
                entered: entered.clone(),
                release: release.clone(),
            })],
        );
        let session = selecting
            .start(StartCollectionCommand {
                route_id: SourceRouteId("source.kimi".to_owned()),
                source_reference: "submitted:cancellation".to_owned(),
                scope_description: "three cancellable candidates".to_owned(),
                authorisation_id: "fixture-authorisation".to_owned(),
            })
            .unwrap();
        let session_id = session.session_id.clone();
        let selection = selection(&session_id, true);
        let handle = std::thread::spawn(move || selecting.select(selection));

        entered.wait();
        let cancelling = CollectorSessionService::new(
            repository.clone(),
            FileAssetStore::new(paths),
            SystemClock,
            vec![Box::new(FixtureAdapter {
                outcomes,
                attachment_path: None,
            })],
        );
        let during_cancel = cancelling
            .cancel(CancelCollectionCommand {
                session_id: session_id.clone(),
                reason: "user cancelled remaining scope".to_owned(),
            })
            .unwrap();
        assert_eq!(state(&during_cancel, "a"), CollectionItemState::Running);
        assert_eq!(state(&during_cancel, "b"), CollectionItemState::Skipped);
        assert_eq!(state(&during_cancel, "c"), CollectionItemState::Skipped);
        assert!(
            repository
                .claim_item(
                    &session_id,
                    "b",
                    &UtcTimestamp::parse("2026-07-18T00:00:00Z").unwrap()
                )
                .unwrap()
                .is_none()
        );

        release.wait();
        let after_cancel = handle.join().unwrap().unwrap();
        assert_eq!(state(&after_cancel, "a"), CollectionItemState::Saved);
        assert_eq!(state(&after_cancel, "b"), CollectionItemState::Skipped);
        assert_eq!(state(&after_cancel, "c"), CollectionItemState::Skipped);
        assert_eq!(
            cancelling.session(&session_id).unwrap().state,
            CollectionSessionState::Cancelled
        );
        assert_eq!(table_count(&repository, "items"), 1);

        drop(cancelling);
        drop(repository);
        let reopened = crate::sqlite::open_collection_database(
            &DataPaths::new(temporary.path().to_path_buf()),
            100,
        )
        .unwrap();
        assert_eq!(
            reopened.session(&session_id).unwrap().unwrap().state,
            CollectionSessionState::Cancelled
        );
        let durable_items = reopened.collection_items(&session_id).unwrap();
        assert_eq!(state(&durable_items, "a"), CollectionItemState::Saved);
        assert_eq!(state(&durable_items, "b"), CollectionItemState::Skipped);
        assert_eq!(state(&durable_items, "c"), CollectionItemState::Skipped);
    }

    type FixtureCollector =
        CollectorSessionService<SqliteRawRepository, FileAssetStore, SystemClock>;

    fn assert_recollection_transitions(
        service: &FixtureCollector,
        repository: &SqliteRawRepository,
        outcomes: &Arc<Mutex<HashMap<String, AcquisitionOutcome>>>,
        item_id: &ItemId,
    ) {
        outcomes.lock().unwrap().insert(
            "a".to_owned(),
            found_with_common_metadata("a", "version two", "content-two", "later observed title"),
        );
        assert_eq!(
            service.recollect(item_id).unwrap().state,
            RecollectionState::Changed
        );
        assert_eq!(table_count(repository, "revisions"), 3);
        outcomes.lock().unwrap().insert(
            "a".to_owned(),
            found_with_fingerprint("a", "version two with volatile URL", "content-two"),
        );
        assert_eq!(
            service.recollect(item_id).unwrap().state,
            RecollectionState::Unchanged
        );
        outcomes.lock().unwrap().insert(
            "a".to_owned(),
            AcquisitionOutcome::Inaccessible {
                reason: "permission removed".to_owned(),
            },
        );
        assert_eq!(
            service.recollect(item_id).unwrap().state,
            RecollectionState::Inaccessible
        );
        outcomes.lock().unwrap().insert(
            "a".to_owned(),
            AcquisitionOutcome::Removed {
                reason: "source deleted".to_owned(),
            },
        );
        assert_eq!(
            service.recollect(item_id).unwrap().state,
            RecollectionState::Removed
        );
        assert_eq!(table_count(repository, "revisions"), 3);
        assert_eq!(table_count(repository, "collection_recollection_checks"), 4);
        assert_eq!(table_count(repository, "source_observations"), 6);
        assert_eq!(
            table_count(repository, "capture_operations"),
            table_count(repository, "source_observations") - 3
        );
        assert_eq!(
            recollection_states(repository),
            ["changed", "unchanged", "inaccessible", "removed"]
        );
        let detail = repository.load_detail(item_id).unwrap();
        assert_eq!(
            detail.common_metadata.title.as_deref(),
            Some("first observed title")
        );
        assert_eq!(detail.source_observations.len(), 5);
        assert!(detail.source_observations.iter().any(|observation| {
            observation.common_metadata.title.as_deref() == Some("later observed title")
        }));
        assert_eq!(
            detail
                .source_observations
                .iter()
                .filter(|observation| observation.recollection_state.is_some())
                .count(),
            3
        );
        let connection = repository.lock().unwrap();
        assert!(
            connection
                .execute(
                    "UPDATE source_observations SET reason = 'tampered' WHERE item_id = ?1",
                    params![item_id.to_string()],
                )
                .is_err()
        );
        assert!(
            connection
                .execute(
                    "DELETE FROM source_observations WHERE item_id = ?1",
                    params![item_id.to_string()],
                )
                .is_err()
        );
    }

    #[test]
    fn attachment_selection_persists_and_unchanged_recollection_does_not_duplicate_asset() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        ensure_layout(&paths).unwrap();
        let attachment = temporary.path().join("original.mp4");
        std::fs::write(&attachment, b"real attachment bytes").unwrap();
        let repository = crate::sqlite::open_collection_database(&paths, 100).unwrap();
        let outcomes = Arc::new(Mutex::new(HashMap::from([(
            "asset".to_owned(),
            found_with_fingerprint("asset", "stable body", "stable-content"),
        )])));
        let adapter = FixtureAdapter {
            outcomes,
            attachment_path: Some(attachment.to_string_lossy().into_owned()),
        };
        let service = CollectorSessionService::new(
            repository.clone(),
            FileAssetStore::new(paths.clone()),
            SystemClock,
            vec![Box::new(adapter.clone())],
        );
        let session = service
            .start(StartCollectionCommand {
                route_id: SourceRouteId("source.kimi".to_owned()),
                source_reference: "submitted:asset".to_owned(),
                scope_description: "one candidate with its attachment".to_owned(),
                authorisation_id: "fixture-authorisation".to_owned(),
            })
            .unwrap();
        let selected = service
            .select(CollectionSelection {
                session_id: session.session_id.clone(),
                candidate_ids: vec!["asset".to_owned()],
                scope_description: "the displayed candidate and attachment".to_owned(),
                confirmed: true,
                authorised_context: "fixture-authorisation".to_owned(),
                requested_attachments: true,
            })
            .unwrap();
        let item_id = selected[0].item_id.clone().unwrap();
        assert!(selected[0].requested_attachments);
        assert_eq!(table_count(&repository, "items"), 1);
        assert_eq!(table_count(&repository, "revisions"), 1);
        assert_eq!(table_count(&repository, "assets"), 1);

        drop(service);
        drop(repository);
        let reopened_repository = crate::sqlite::open_collection_database(&paths, 100).unwrap();
        let reopened = CollectorSessionService::new(
            reopened_repository.clone(),
            FileAssetStore::new(paths),
            SystemClock,
            vec![Box::new(adapter)],
        );
        assert!(reopened.status(&session.session_id).unwrap()[0].requested_attachments);
        assert_eq!(
            reopened.recollect(&item_id).unwrap().state,
            RecollectionState::Unchanged
        );
        assert_eq!(table_count(&reopened_repository, "revisions"), 1);
        assert_eq!(table_count(&reopened_repository, "assets"), 1);
        assert_eq!(
            table_count(&reopened_repository, "collection_recollection_checks"),
            1
        );
    }

    fn selection(session_id: &CollectionSessionId, confirmed: bool) -> CollectionSelection {
        CollectionSelection {
            session_id: session_id.clone(),
            candidate_ids: vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
            scope_description: "the three displayed candidates".to_owned(),
            confirmed,
            authorised_context: "fixture-authorisation".to_owned(),
            requested_attachments: false,
        }
    }

    fn found(native_id: &str, text: &str) -> AcquisitionOutcome {
        found_with_metadata(native_id, text, &format!("{{\"title\":\"{native_id}\"}}"))
    }

    fn found_with_fingerprint(
        native_id: &str,
        text: &str,
        content_fingerprint: &str,
    ) -> AcquisitionOutcome {
        found_with_metadata(
            native_id,
            text,
            &format!(
                "{{\"title\":\"{native_id}\",\"content_fingerprint\":\"{content_fingerprint}\"}}"
            ),
        )
    }

    fn found_with_common_metadata(
        native_id: &str,
        text: &str,
        content_fingerprint: &str,
        title: &str,
    ) -> AcquisitionOutcome {
        let mut outcome = found_with_fingerprint(native_id, text, content_fingerprint);
        let AcquisitionOutcome::Found { candidate, .. } = &mut outcome else {
            unreachable!("fixture always returns a found acquisition");
        };
        candidate.common_metadata.title = Some(title.to_owned());
        outcome
    }

    fn found_with_metadata(native_id: &str, text: &str, metadata: &str) -> AcquisitionOutcome {
        AcquisitionOutcome::Found {
            candidate: Box::new(CandidateEnvelope {
                protocol_version: "1".to_owned(),
                route_id: SourceRouteId("source.kimi".to_owned()),
                source_reference: format!("https://example.test/{native_id}"),
                content_type: ContentType::Document,
                payload_sha256: Sha256::of_bytes(text.as_bytes()),
                metadata: Metadata::parse(metadata).unwrap(),
                payload: CandidatePayload::Text {
                    text: text.to_owned(),
                },
                context: Some("fixture".to_owned()),
                native_id: Some(native_id.to_owned()),
                common_metadata: babata_domain::CommonSourceMetadata::default(),
            }),
            assets: Vec::new(),
        }
    }

    fn state(items: &[babata_domain::CollectionItemStatus], id: &str) -> CollectionItemState {
        items
            .iter()
            .find(|item| item.candidate_id == id)
            .unwrap()
            .state
    }

    fn table_count(repository: &SqliteRawRepository, table: &str) -> i64 {
        assert!(matches!(
            table,
            "items"
                | "revisions"
                | "assets"
                | "capture_operations"
                | "source_observations"
                | "collection_recollection_checks"
        ));
        repository
            .lock()
            .unwrap()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn recollection_states(repository: &SqliteRawRepository) -> Vec<String> {
        let connection = repository.lock().unwrap();
        let mut statement = connection
            .prepare("SELECT state FROM collection_recollection_checks ORDER BY rowid")
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }
}

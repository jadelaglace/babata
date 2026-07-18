use std::collections::HashSet;

use babata_domain::{
    CandidatePayload, CandidateSummary, CollectionItemState, CollectionItemStatus,
    CollectionSelection, CollectionSession, CollectionSessionId, CollectionSessionState, ItemId,
    RawState, RecollectionOutcome, RecollectionState, SourceRouteId,
};

use crate::{
    AcquisitionOutcome, ApplicationError, CancelCollectionCommand, CandidateCaptureCommand,
    CaptureService, RetryCollectionItemCommand, RouteEvidenceCommand, StartCollectionCommand,
    ports::{
        AssetStorePort, ClockPort, CollectionRepositoryPort, RawRepositoryPort, SourceAdapterPort,
    },
};

pub struct CollectorSessionService<R, A, C> {
    repository: R,
    assets: A,
    clock: C,
    adapters: Vec<Box<dyn SourceAdapterPort>>,
}

impl<R, A, C> CollectorSessionService<R, A, C>
where
    R: CollectionRepositoryPort + RawRepositoryPort + Clone,
    A: AssetStorePort + Clone,
    C: ClockPort + Clone,
{
    pub fn new(
        repository: R,
        assets: A,
        clock: C,
        adapters: Vec<Box<dyn SourceAdapterPort>>,
    ) -> Self {
        Self {
            repository,
            assets,
            clock,
            adapters,
        }
    }

    pub fn start(
        &self,
        command: StartCollectionCommand,
    ) -> Result<CollectionSession, ApplicationError> {
        require_text("source_reference", &command.source_reference)?;
        require_text("scope_description", &command.scope_description)?;
        require_text("authorisation_id", &command.authorisation_id)?;
        let adapter = self.adapter(&command.route_id)?;
        let now = self.clock.now();
        let session = CollectionSession {
            session_id: CollectionSessionId::new(),
            route_id: command.route_id,
            source_reference: command.source_reference,
            scope_description: command.scope_description,
            authorisation_id: command.authorisation_id,
            state: CollectionSessionState::Discovering,
            created_at: now.clone(),
            updated_at: now,
        };
        self.repository.create_session(&session)?;
        let discovered = match adapter.discover(&session.session_id, &session.source_reference) {
            Ok(discovered) => discovered,
            Err(error) => {
                self.repository.update_session_state(
                    &session.session_id,
                    CollectionSessionState::Failed,
                    &self.clock.now(),
                )?;
                return Err(error);
            }
        };
        validate_discovery(&session, &discovered)?;
        self.repository.save_candidates(
            &discovered
                .into_iter()
                .map(|candidate| (candidate.summary, candidate.prefetched))
                .collect::<Vec<_>>(),
        )?;
        self.repository.update_session_state(
            &session.session_id,
            CollectionSessionState::AwaitingSelection,
            &self.clock.now(),
        )?;
        self.session(&session.session_id)
    }

    pub fn session(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<CollectionSession, ApplicationError> {
        self.repository
            .session(session_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("collection session: {session_id}")))
    }

    pub fn candidates(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<Vec<CandidateSummary>, ApplicationError> {
        self.session(session_id)?;
        self.repository.candidates(session_id)
    }

    pub fn select(
        &self,
        selection: CollectionSelection,
    ) -> Result<Vec<CollectionItemStatus>, ApplicationError> {
        if !selection.confirmed {
            return Err(ApplicationError::Conflict(
                "collection scope was not confirmed; C0 remains unchanged".to_owned(),
            ));
        }
        require_text("scope_description", &selection.scope_description)?;
        require_text("authorised_context", &selection.authorised_context)?;
        if selection.candidate_ids.is_empty() {
            return Err(ApplicationError::Conflict(
                "at least one candidate must be selected explicitly".to_owned(),
            ));
        }
        if selection.candidate_ids.iter().collect::<HashSet<_>>().len()
            != selection.candidate_ids.len()
        {
            return Err(ApplicationError::Conflict(
                "candidate selection contains duplicates".to_owned(),
            ));
        }
        let session = self.session(&selection.session_id)?;
        if session.state != CollectionSessionState::AwaitingSelection
            && session.state != CollectionSessionState::Completed
        {
            return Err(ApplicationError::Conflict(format!(
                "collection session cannot accept a selection in state {:?}",
                session.state
            )));
        }
        if selection.authorised_context != session.authorisation_id {
            return Err(ApplicationError::Conflict(
                "selection authorisation does not match the discovered session".to_owned(),
            ));
        }
        self.repository
            .enqueue_selection(&selection, &self.clock.now())?;
        self.repository.update_session_state(
            &selection.session_id,
            CollectionSessionState::Running,
            &self.clock.now(),
        )?;
        for candidate_id in &selection.candidate_ids {
            self.run_item(&selection.session_id, candidate_id)?;
        }
        self.finish_session(&selection.session_id)?;
        self.status(&selection.session_id)
    }

    pub fn status(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<Vec<CollectionItemStatus>, ApplicationError> {
        self.session(session_id)?;
        self.repository.collection_items(session_id)
    }

    pub fn retry(
        &self,
        command: RetryCollectionItemCommand,
    ) -> Result<CollectionItemStatus, ApplicationError> {
        let current = self
            .status(&command.session_id)?
            .into_iter()
            .find(|item| item.candidate_id == command.candidate_id)
            .ok_or_else(|| ApplicationError::NotFound("collection item".to_owned()))?;
        if current.state != CollectionItemState::Failed || !current.retryable {
            return Err(ApplicationError::Conflict(
                "only retryable failed items can be retried".to_owned(),
            ));
        }
        self.repository.update_session_state(
            &command.session_id,
            CollectionSessionState::Running,
            &self.clock.now(),
        )?;
        let result = self.run_item(&command.session_id, &command.candidate_id)?;
        self.finish_session(&command.session_id)?;
        Ok(result)
    }

    pub fn cancel(
        &self,
        command: CancelCollectionCommand,
    ) -> Result<Vec<CollectionItemStatus>, ApplicationError> {
        require_text("reason", &command.reason)?;
        for item in self.status(&command.session_id)? {
            if item.state == CollectionItemState::Queued {
                self.repository.transition_item(
                    &command.session_id,
                    &item.candidate_id,
                    CollectionItemState::Skipped,
                    Some(&command.reason),
                    false,
                    None,
                    None,
                    false,
                    &self.clock.now(),
                )?;
            }
        }
        self.repository.update_session_state(
            &command.session_id,
            CollectionSessionState::Cancelled,
            &self.clock.now(),
        )?;
        self.status(&command.session_id)
    }

    pub fn recollect(&self, item_id: &ItemId) -> Result<RecollectionOutcome, ApplicationError> {
        let (session_id, candidate_id) = self
            .repository
            .latest_saved_for_item(item_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("collected item: {item_id}")))?;
        let session = self.session(&session_id)?;
        let adapter = self.adapter(&session.route_id)?;
        let (candidate, prefetched) = self
            .repository
            .candidate(&session_id, &candidate_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("candidate: {candidate_id}")))?;
        let requested_attachments = self
            .status(&session_id)?
            .into_iter()
            .find(|item| item.candidate_id == candidate_id)
            .is_some_and(|item| item.requested_attachments);
        let detail = self.repository.load_detail(item_id)?;
        let previous = detail
            .revisions
            .iter()
            .filter(|revision| revision.state == RawState::Ready)
            .max_by_key(|revision| revision.ordinal)
            .ok_or_else(|| {
                ApplicationError::Integrity("collected item has no ready revision".to_owned())
            })?;
        let checked_at = self.clock.now();
        let outcome =
            match adapter.collect(&candidate, prefetched.as_ref(), requested_attachments)? {
                AcquisitionOutcome::Found { candidate, assets } => {
                    let envelope = candidate;
                    let CandidatePayload::Text { ref text } = envelope.payload;
                    let hash = babata_domain::Sha256::of_bytes(text.as_bytes());
                    let content_unchanged = match (
                        content_fingerprint(&previous.metadata),
                        content_fingerprint(&envelope.metadata),
                    ) {
                        (Some(previous), Some(current)) => previous == current,
                        _ => previous.text_sha256.as_deref() == Some(hash.as_str()),
                    };
                    if content_unchanged {
                        RecollectionOutcome {
                            item_id: item_id.clone(),
                            state: RecollectionState::Unchanged,
                            previous_revision_id: previous.revision_id.clone(),
                            new_revision_id: None,
                            reason: None,
                            checked_at,
                        }
                    } else {
                        let capture =
                            self.capture_candidate(&session, adapter, envelope, assets)?;
                        RecollectionOutcome {
                            item_id: item_id.clone(),
                            state: RecollectionState::Changed,
                            previous_revision_id: previous.revision_id.clone(),
                            new_revision_id: Some(capture.revision_id),
                            reason: None,
                            checked_at,
                        }
                    }
                }
                AcquisitionOutcome::Inaccessible { reason }
                | AcquisitionOutcome::Skipped { reason } => RecollectionOutcome {
                    item_id: item_id.clone(),
                    state: RecollectionState::Inaccessible,
                    previous_revision_id: previous.revision_id.clone(),
                    new_revision_id: None,
                    reason: Some(reason),
                    checked_at,
                },
                AcquisitionOutcome::Removed { reason } => RecollectionOutcome {
                    item_id: item_id.clone(),
                    state: RecollectionState::Removed,
                    previous_revision_id: previous.revision_id.clone(),
                    new_revision_id: None,
                    reason: Some(reason),
                    checked_at,
                },
            };
        self.repository.record_recollection(&outcome)?;
        Ok(outcome)
    }

    fn run_item(
        &self,
        session_id: &CollectionSessionId,
        candidate_id: &str,
    ) -> Result<CollectionItemStatus, ApplicationError> {
        let session = self.session(session_id)?;
        let adapter = self.adapter(&session.route_id)?;
        let (candidate, prefetched) = self
            .repository
            .candidate(session_id, candidate_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("candidate: {candidate_id}")))?;
        let requested_attachments = self
            .status(session_id)?
            .into_iter()
            .find(|item| item.candidate_id == candidate_id)
            .is_some_and(|item| item.requested_attachments);
        self.repository.transition_item(
            session_id,
            candidate_id,
            CollectionItemState::Running,
            None,
            false,
            None,
            None,
            true,
            &self.clock.now(),
        )?;
        let acquisition =
            match adapter.collect(&candidate, prefetched.as_ref(), requested_attachments) {
                Ok(outcome) => outcome,
                Err(error) => {
                    return self.repository.transition_item(
                        session_id,
                        candidate_id,
                        CollectionItemState::Failed,
                        Some(&error.to_string()),
                        true,
                        None,
                        None,
                        false,
                        &self.clock.now(),
                    );
                }
            };
        match acquisition {
            AcquisitionOutcome::Found { candidate, assets } => {
                match self.capture_candidate(&session, adapter, candidate, assets) {
                    Ok(outcome) => self.repository.transition_item(
                        session_id,
                        candidate_id,
                        CollectionItemState::Saved,
                        None,
                        false,
                        Some(&outcome.item_id),
                        Some(&outcome.revision_id),
                        false,
                        &self.clock.now(),
                    ),
                    Err(error) => self.repository.transition_item(
                        session_id,
                        candidate_id,
                        CollectionItemState::Failed,
                        Some(&error.to_string()),
                        true,
                        None,
                        None,
                        false,
                        &self.clock.now(),
                    ),
                }
            }
            AcquisitionOutcome::Skipped { reason } | AcquisitionOutcome::Removed { reason } => {
                self.repository.transition_item(
                    session_id,
                    candidate_id,
                    CollectionItemState::Skipped,
                    Some(&reason),
                    false,
                    None,
                    None,
                    false,
                    &self.clock.now(),
                )
            }
            AcquisitionOutcome::Inaccessible { reason } => self.repository.transition_item(
                session_id,
                candidate_id,
                CollectionItemState::Failed,
                Some(&reason),
                true,
                None,
                None,
                false,
                &self.clock.now(),
            ),
        }
    }

    fn capture_candidate(
        &self,
        session: &CollectionSession,
        adapter: &dyn SourceAdapterPort,
        candidate: babata_domain::CandidateEnvelope,
        assets: Vec<crate::CaptureImportAsset>,
    ) -> Result<crate::CaptureOutcome, ApplicationError> {
        let coverage = adapter.coverage();
        CaptureService::new(
            self.repository.clone(),
            self.assets.clone(),
            self.clock.clone(),
        )
        .capture_candidate(CandidateCaptureCommand {
            route_evidence: Some(RouteEvidenceCommand {
                route_id: session.route_id.clone(),
                authorization_id: session.authorisation_id.clone(),
                source_reference: candidate.source_reference.clone(),
                coverage,
            }),
            candidate,
            assets,
        })
    }

    fn finish_session(&self, session_id: &CollectionSessionId) -> Result<(), ApplicationError> {
        let items = self.repository.collection_items(session_id)?;
        if items.iter().all(|item| {
            matches!(
                item.state,
                CollectionItemState::Saved
                    | CollectionItemState::Skipped
                    | CollectionItemState::Failed
            )
        }) {
            self.repository.update_session_state(
                session_id,
                CollectionSessionState::Completed,
                &self.clock.now(),
            )?;
        }
        Ok(())
    }

    fn adapter(
        &self,
        route_id: &SourceRouteId,
    ) -> Result<&dyn SourceAdapterPort, ApplicationError> {
        self.adapters
            .iter()
            .find(|adapter| adapter.describe().id == *route_id)
            .map(Box::as_ref)
            .ok_or_else(|| ApplicationError::capability_unavailable(route_id.0.clone(), "P4"))
    }
}

fn content_fingerprint(metadata: &babata_domain::Metadata) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(&metadata.to_json())
        .ok()?
        .get("content_fingerprint")?
        .as_str()
        .map(str::to_owned)
}

fn require_text(field: &'static str, value: &str) -> Result<(), ApplicationError> {
    if value.trim().is_empty() {
        return Err(babata_domain::DomainError::Empty { field }.into());
    }
    Ok(())
}

fn validate_discovery(
    session: &CollectionSession,
    discovered: &[crate::DiscoveredCandidate],
) -> Result<(), ApplicationError> {
    let mut ids = HashSet::new();
    for candidate in discovered {
        if candidate.summary.session_id != session.session_id
            || candidate.summary.route_id != session.route_id
        {
            return Err(ApplicationError::Integrity(
                "adapter returned a candidate outside the active session or route".to_owned(),
            ));
        }
        require_text("candidate_id", &candidate.summary.candidate_id)?;
        if !ids.insert(&candidate.summary.candidate_id) {
            return Err(ApplicationError::Integrity(
                "adapter returned duplicate candidate IDs".to_owned(),
            ));
        }
    }
    Ok(())
}

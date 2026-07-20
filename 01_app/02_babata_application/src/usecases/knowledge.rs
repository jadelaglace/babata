use babata_domain::{
    DerivativeRef, ItemId, KnowledgeId, KnowledgeKind, KnowledgeRecord, KnowledgeVersion, Metadata,
    ProcessingState, RevisionId, Sha256,
};

use crate::{
    ApplicationError, CreateKnowledgeCommand, CreateNoteCommand, KnowledgeDetail,
    KnowledgeReviewContext, ReviseCommand, ReviseKnowledgeCommand, ShowProcessRunOutcome,
    WorkspaceService,
    ports::{
        AssetStorePort, ClockPort, DerivedRepositoryPort, KnowledgeRepositoryPort,
        RawRepositoryPort,
    },
};

pub struct KnowledgeService<R, D, A, C> {
    raw: R,
    derived: D,
    assets: A,
    clock: C,
}

impl<R, D, A, C> KnowledgeService<R, D, A, C>
where
    R: RawRepositoryPort + KnowledgeRepositoryPort + Clone,
    D: DerivedRepositoryPort,
    A: AssetStorePort + Clone,
    C: ClockPort + Clone,
{
    pub fn new(raw: R, derived: D, assets: A, clock: C) -> Self {
        Self {
            raw,
            derived,
            assets,
            clock,
        }
    }

    pub fn review(
        &self,
        item_id: &ItemId,
        revision_id: &RevisionId,
    ) -> Result<KnowledgeReviewContext, ApplicationError> {
        let revision = self.ready_revision(revision_id)?;
        if revision.item_id != *item_id {
            return Err(ApplicationError::Integrity(format!(
                "revision {revision_id} belongs to item {}, not {item_id}",
                revision.item_id
            )));
        }
        let target = self.raw.load_detail(item_id)?;
        let process_runs = self
            .derived
            .list_runs_for_revision(revision_id)?
            .into_iter()
            .map(|run| {
                let derivatives = self.derived.list_derivatives(&run.id)?;
                self.validate_process_evidence(&revision, &target, &run, &derivatives)?;
                Ok(ShowProcessRunOutcome { run, derivatives })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        let knowledge_records = self.raw.list_knowledge_for_source_revision(revision_id)?;
        Ok(KnowledgeReviewContext {
            target,
            target_revision_id: revision_id.clone(),
            process_runs,
            knowledge_records,
        })
    }

    pub fn create(
        &self,
        command: CreateKnowledgeCommand,
    ) -> Result<KnowledgeDetail, ApplicationError> {
        validate_label("knowledge title", &command.title)?;
        validate_label("knowledge author", &command.author)?;
        let source = self.ready_revision(&command.source_revision_id)?;
        let knowledge_id = KnowledgeId::new();
        let metadata =
            knowledge_metadata(&knowledge_id, &command.source_revision_id, &command.title)?;
        let first_party =
            WorkspaceService::new(self.raw.clone(), self.assets.clone(), self.clock.clone())
                .create(CreateNoteCommand {
                    text: command.body,
                    path: None,
                    context: Some("knowledge".to_owned()),
                    metadata,
                })?;
        let first_party_revision = self.ready_revision(&first_party.revision_id)?;
        let record = KnowledgeRecord {
            id: knowledge_id,
            kind: KnowledgeKind::Knowledge,
            author: command.author,
            first_party_item_id: first_party.item_id,
            source_item_id: source.item_id,
            source_revision_id: command.source_revision_id,
            created_at: first_party_revision.captured_at.clone(),
            versions: vec![KnowledgeVersion {
                ordinal: 1,
                first_party_revision_id: first_party_revision.id,
                title: command.title,
                created_at: first_party_revision.captured_at,
            }],
        };
        self.raw.create_knowledge(&record)?;
        self.detail(record)
    }

    pub fn revise(
        &self,
        command: ReviseKnowledgeCommand,
    ) -> Result<KnowledgeDetail, ApplicationError> {
        validate_label("knowledge title", &command.title)?;
        let record = self
            .raw
            .get_knowledge(&command.knowledge_id)?
            .ok_or_else(|| ApplicationError::NotFound(command.knowledge_id.to_string()))?;
        let latest = record.versions.last().ok_or_else(|| {
            ApplicationError::Integrity(format!(
                "knowledge {} has no first-party version",
                record.id
            ))
        })?;
        let metadata = knowledge_metadata(&record.id, &record.source_revision_id, &command.title)?;
        let outcome =
            WorkspaceService::new(self.raw.clone(), self.assets.clone(), self.clock.clone())
                .revise(ReviseCommand {
                    parent: latest.first_party_revision_id.clone(),
                    text: command.body,
                    path: None,
                    note: command.note,
                    metadata,
                })?;
        let revision = self.ready_revision(&outcome.revision_id)?;
        let version = KnowledgeVersion {
            ordinal: latest.ordinal + 1,
            first_party_revision_id: revision.id,
            title: command.title,
            created_at: revision.captured_at,
        };
        self.raw.append_knowledge_version(&record.id, &version)?;
        self.show(&record.id)
    }

    pub fn show(&self, knowledge_id: &KnowledgeId) -> Result<KnowledgeDetail, ApplicationError> {
        let record = self
            .raw
            .get_knowledge(knowledge_id)?
            .ok_or_else(|| ApplicationError::NotFound(knowledge_id.to_string()))?;
        self.detail(record)
    }

    fn ready_revision(
        &self,
        revision_id: &RevisionId,
    ) -> Result<crate::ports::NewRevision, ApplicationError> {
        let revision = self
            .raw
            .find_revision(revision_id)?
            .ok_or_else(|| ApplicationError::NotFound(revision_id.to_string()))?;
        if self.raw.find_revision_state(revision_id)? != Some(babata_domain::RawState::Ready) {
            return Err(ApplicationError::Conflict(format!(
                "revision {revision_id} is not ready"
            )));
        }
        Ok(revision)
    }

    fn detail(&self, record: KnowledgeRecord) -> Result<KnowledgeDetail, ApplicationError> {
        let first_party = self.raw.load_detail(&record.first_party_item_id)?;
        Ok(KnowledgeDetail {
            record,
            first_party,
        })
    }

    fn validate_process_evidence(
        &self,
        revision: &crate::ports::NewRevision,
        target: &crate::RecordDetail,
        run: &babata_domain::ProcessRun,
        derivatives: &[DerivativeRef],
    ) -> Result<(), ApplicationError> {
        if run.invalidated_at.is_some() {
            return Ok(());
        }
        if run.input_revision_id != revision.id
            || run.input_item_id.as_ref() != Some(&revision.item_id)
        {
            return Err(ApplicationError::Integrity(format!(
                "active C1 run {} does not resolve to target {}/{}",
                run.id, revision.item_id, revision.id
            )));
        }
        let expected_input_hash = match &run.input_asset_id {
            Some(asset_id) => {
                let asset = target
                    .assets
                    .iter()
                    .find(|asset| asset.asset_id == *asset_id && asset.revision_id == revision.id)
                    .ok_or_else(|| {
                        ApplicationError::Integrity(format!(
                            "active C1 run {} input asset is not bound to target revision",
                            run.id
                        ))
                    })?;
                Sha256::parse(&asset.sha256).map_err(ApplicationError::from)?
            }
            None => revision.text_sha256.clone().ok_or_else(|| {
                ApplicationError::Integrity(format!(
                    "active C1 run {} has neither a bound asset nor C0 text",
                    run.id
                ))
            })?,
        };
        if run.input_sha256 != expected_input_hash {
            return Err(ApplicationError::Integrity(format!(
                "active C1 run {} input hash no longer matches C0",
                run.id
            )));
        }
        if run.state == ProcessingState::Succeeded && derivatives.is_empty() {
            return Err(ApplicationError::Integrity(format!(
                "active succeeded C1 run {} has no derivative",
                run.id
            )));
        }
        for derivative in derivatives {
            if derivative.input_asset_id != run.input_asset_id {
                return Err(ApplicationError::Integrity(format!(
                    "derivative {} input identity disagrees with run {}",
                    derivative.id, run.id
                )));
            }
            self.validate_derivative_output(derivative)?;
        }
        Ok(())
    }

    fn validate_derivative_output(
        &self,
        derivative: &DerivativeRef,
    ) -> Result<(), ApplicationError> {
        let expected = derivative.output_sha256.as_ref().ok_or_else(|| {
            ApplicationError::Integrity(format!(
                "active derivative {} has no output hash",
                derivative.id
            ))
        })?;
        let mut actual = Vec::new();
        if let Some(text) = &derivative.content_text {
            actual.push(Sha256::of_bytes(text.as_bytes()));
        }
        if let Some(json) = &derivative.content_json {
            actual.push(Sha256::of_bytes(json.as_bytes()));
        }
        if let Some(path) = &derivative.logical_path {
            actual.push(self.assets.hash_logical(path)?);
        }
        if actual.is_empty() || actual.iter().any(|hash| hash != expected) {
            return Err(ApplicationError::Integrity(format!(
                "active derivative {} output no longer matches its hash",
                derivative.id
            )));
        }
        Ok(())
    }
}

fn validate_label(field: &'static str, value: &str) -> Result<(), ApplicationError> {
    if value.trim().is_empty() {
        return Err(ApplicationError::Integrity(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn knowledge_metadata(
    knowledge_id: &KnowledgeId,
    source_revision_id: &RevisionId,
    title: &str,
) -> Result<Metadata, ApplicationError> {
    Metadata::parse(
        &serde_json::json!({
            "babata_semantic_kind": "knowledge",
            "knowledge_id": knowledge_id,
            "source_revision_id": source_revision_id,
            "title": title,
        })
        .to_string(),
    )
    .map_err(ApplicationError::from)
}

use babata_domain::{
    DerivativeId, DerivativeKind, DerivativeRef, ItemId, ProcessingState, RevisionId, Sha256,
};

use crate::{
    ApplicationError, ChangeMapNodeTagCommand, ChangeMapParentCommand,
    ChangeSemanticMapAssignmentCommand, CreateMapNodeCommand, CreateScoreProfileCommand,
    EvolveMapNodeCommand, FirstPartySemanticOutcome, IngestSemanticCandidateCommand,
    KnowledgeReviewContext, MapNodeDetail, RecordRelevanceScoreCommand,
    RecordSuggestionReviewCommand, RegisterFirstPartySemanticCommand, RelevanceScoreDetail,
    SemanticCoreSnapshot, SemanticEntryDetail, SemanticIngestOutcome, ShowProcessRunOutcome,
    ports::{
        AssetStorePort, DerivedRepositoryPort, KnowledgeCoreRepositoryPort, RawRepositoryPort,
    },
};

pub struct KnowledgeService<R, D, A> {
    raw: R,
    derived: D,
    assets: A,
}

impl<R, D, A> KnowledgeService<R, D, A>
where
    R: RawRepositoryPort + KnowledgeCoreRepositoryPort,
    D: DerivedRepositoryPort,
    A: AssetStorePort,
{
    pub fn new(raw: R, derived: D, assets: A) -> Self {
        Self {
            raw,
            derived,
            assets,
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
        Ok(KnowledgeReviewContext {
            target,
            target_revision_id: revision_id.clone(),
            process_runs,
        })
    }

    pub fn ingest_derivative(
        &self,
        derivative_id: &DerivativeId,
    ) -> Result<SemanticIngestOutcome, ApplicationError> {
        let derivative = self
            .derived
            .get_derivative(derivative_id)?
            .ok_or_else(|| ApplicationError::NotFound(derivative_id.to_string()))?;
        if derivative.kind != DerivativeKind::StructuredResult {
            return Err(ApplicationError::Conflict(format!(
                "derivative {derivative_id} is not a structured semantic package"
            )));
        }
        let package_json = derivative.content_json.as_deref().ok_or_else(|| {
            ApplicationError::Integrity(format!(
                "semantic derivative {derivative_id} has no JSON content"
            ))
        })?;
        let output_sha256 = derivative.output_sha256.clone().ok_or_else(|| {
            ApplicationError::Integrity(format!(
                "semantic derivative {derivative_id} has no output hash"
            ))
        })?;
        if Sha256::of_bytes(package_json.as_bytes()) != output_sha256 {
            return Err(ApplicationError::Integrity(format!(
                "semantic derivative {derivative_id} output no longer matches its hash"
            )));
        }
        let package: babata_domain::SemanticCandidatePackage = serde_json::from_str(package_json)
            .map_err(|error| {
            ApplicationError::Integrity(format!(
                "semantic derivative {derivative_id} is not a candidate package: {error}"
            ))
        })?;
        package.validate().map_err(ApplicationError::from)?;
        let review_snapshot = self.review(&package.source_item_id, &package.source_revision_id)?;
        let visible_derivatives = review_snapshot
            .process_runs
            .iter()
            .flat_map(|run| run.derivatives.iter())
            .map(|candidate| &candidate.id)
            .collect::<std::collections::HashSet<_>>();
        if !visible_derivatives.contains(derivative_id)
            || package.evidence_derivatives.iter().any(|evidence| {
                review_snapshot
                    .process_runs
                    .iter()
                    .flat_map(|run| run.derivatives.iter())
                    .find(|candidate| candidate.id == evidence.derivative_id)
                    .is_none_or(|candidate| {
                        candidate.output_sha256.as_ref() != Some(&evidence.output_sha256)
                    })
            })
        {
            return Err(ApplicationError::Integrity(
                "semantic package cites C1 evidence outside its ready C0 review context".to_owned(),
            ));
        }
        self.raw
            .ingest_machine_candidate(&IngestSemanticCandidateCommand {
                source_derivative_id: derivative_id.clone(),
                source_output_sha256: output_sha256,
                package,
            })
    }

    pub fn show_semantic(
        &self,
        suggestion_id: &str,
    ) -> Result<SemanticCoreSnapshot, ApplicationError> {
        self.raw.load_semantic_snapshot(suggestion_id)
    }

    pub fn review_suggestion(
        &self,
        command: &RecordSuggestionReviewCommand,
    ) -> Result<SemanticCoreSnapshot, ApplicationError> {
        self.raw.record_suggestion_review(command)?;
        self.raw.load_semantic_snapshot(&command.suggestion_id)
    }

    pub fn create_score_profile(
        &self,
        command: &CreateScoreProfileCommand,
    ) -> Result<Vec<babata_domain::ScoreProfile>, ApplicationError> {
        self.raw.create_score_profile(command)?;
        self.raw.list_score_profiles()
    }

    pub fn score_profiles(&self) -> Result<Vec<babata_domain::ScoreProfile>, ApplicationError> {
        self.raw.list_score_profiles()
    }

    pub fn register_first_party(
        &self,
        command: &RegisterFirstPartySemanticCommand,
    ) -> Result<FirstPartySemanticOutcome, ApplicationError> {
        self.raw.register_first_party_semantic(command)
    }

    pub fn show_entry(&self, semantic_id: &str) -> Result<SemanticEntryDetail, ApplicationError> {
        self.raw.load_semantic_entry(semantic_id)
    }

    pub fn create_map_node(
        &self,
        command: &CreateMapNodeCommand,
    ) -> Result<MapNodeDetail, ApplicationError> {
        self.raw.create_map_node(command)
    }

    pub fn evolve_map_node(
        &self,
        command: &EvolveMapNodeCommand,
    ) -> Result<MapNodeDetail, ApplicationError> {
        self.raw.evolve_map_node(command)
    }

    pub fn change_map_parent(
        &self,
        command: &ChangeMapParentCommand,
    ) -> Result<MapNodeDetail, ApplicationError> {
        self.raw.change_map_parent(command)
    }

    pub fn change_map_assignment(
        &self,
        command: &ChangeSemanticMapAssignmentCommand,
    ) -> Result<MapNodeDetail, ApplicationError> {
        self.raw.change_semantic_map_assignment(command)
    }

    pub fn change_map_tag(
        &self,
        command: &ChangeMapNodeTagCommand,
    ) -> Result<MapNodeDetail, ApplicationError> {
        self.raw.change_map_node_tag(command)
    }

    pub fn show_map_node(&self, map_node_id: &str) -> Result<MapNodeDetail, ApplicationError> {
        self.raw.load_map_node(map_node_id)
    }

    pub fn record_score(
        &self,
        command: &RecordRelevanceScoreCommand,
    ) -> Result<RelevanceScoreDetail, ApplicationError> {
        self.raw.record_relevance_score(command)
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

use babata_domain::{DerivativeRef, ItemId, ProcessingState, RevisionId, Sha256};

use crate::{
    ApplicationError, KnowledgeReviewContext, ShowProcessRunOutcome,
    ports::{AssetStorePort, DerivedRepositoryPort, RawRepositoryPort},
};

pub struct KnowledgeService<R, D, A> {
    raw: R,
    derived: D,
    assets: A,
}

impl<R, D, A> KnowledgeService<R, D, A>
where
    R: RawRepositoryPort,
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

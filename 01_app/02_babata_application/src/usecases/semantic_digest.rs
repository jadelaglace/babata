use babata_domain::{
    DerivativeEvidence, DerivativeId, DerivativeKind, DerivativeRef, ItemId, Metadata, PipelineId,
    ProcessRun, ProcessingState, RevisionId, RunId, Sha256,
};

use crate::{
    ApplicationError, KnowledgeService, SemanticDigestAndIngestOutcome, ShowProcessRunOutcome,
    ports::{
        AssetStorePort, ClockPort, DerivedRepositoryPort, KnowledgeCoreRepositoryPort,
        ProcessCommit, RawRepositoryPort, SemanticDigestProviderPort, SemanticDigestRequest,
    },
};

pub struct SemanticDigestService<R, D, A, P, C> {
    raw: R,
    derived: D,
    assets: A,
    provider: P,
    clock: C,
}

impl<R, D, A, P, C> SemanticDigestService<R, D, A, P, C>
where
    R: RawRepositoryPort + KnowledgeCoreRepositoryPort + Clone,
    D: DerivedRepositoryPort + Clone,
    A: AssetStorePort + Clone,
    P: SemanticDigestProviderPort,
    C: ClockPort,
{
    pub fn new(raw: R, derived: D, assets: A, provider: P, clock: C) -> Self {
        Self {
            raw,
            derived,
            assets,
            provider,
            clock,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn digest(
        &self,
        item_id: &ItemId,
        revision_id: &RevisionId,
    ) -> Result<SemanticDigestAndIngestOutcome, ApplicationError> {
        let knowledge =
            KnowledgeService::new(self.raw.clone(), self.derived.clone(), self.assets.clone());
        let context = knowledge.review(item_id, revision_id)?;
        let revision = context
            .target
            .revisions
            .iter()
            .find(|candidate| candidate.revision_id == *revision_id)
            .ok_or_else(|| ApplicationError::NotFound(revision_id.to_string()))?;
        let source_input_sha256 = revision
            .text_sha256
            .as_deref()
            .map(Sha256::parse)
            .transpose()
            .map_err(ApplicationError::from)?
            .ok_or_else(|| {
                ApplicationError::Conflict(
                    "P6 semantic digest currently requires ready C0 text".to_owned(),
                )
            })?;
        let evidence = collect_evidence(&context.process_runs)?;
        let review_context = build_review_context(&context, revision_id)?;
        let generated_at = self.clock.now();
        let outcome = self.provider.execute(&SemanticDigestRequest {
            source_item_id: item_id.clone(),
            source_revision_id: revision_id.clone(),
            source_input_sha256: source_input_sha256.clone(),
            evidence: evidence.clone(),
            review_context: review_context.clone(),
            generated_at: generated_at.clone(),
        })?;
        outcome.package.validate().map_err(ApplicationError::from)?;
        if outcome.package.source_item_id != *item_id
            || outcome.package.source_revision_id != *revision_id
            || outcome.package.evidence_derivatives != evidence
        {
            return Err(ApplicationError::Integrity(
                "semantic provider changed core-owned source or evidence identity".to_owned(),
            ));
        }
        let content_json = serde_json::to_string(&outcome.package).map_err(|error| {
            ApplicationError::Integrity(format!("semantic package serialization failed: {error}"))
        })?;
        let output_sha256 = Sha256::of_bytes(content_json.as_bytes());
        let run_id = RunId::new();
        let derivative_id = DerivativeId::new();
        let params = Metadata::parse(
            &serde_json::json!({
                "prompt_version": outcome.package.prompt_version,
                "provider_task_id": outcome.provider_task_id,
                "review_context_sha256": Sha256::of_bytes(review_context.as_bytes()),
                "evidence": evidence,
            })
            .to_string(),
        )
        .map_err(ApplicationError::from)?;
        let run = ProcessRun {
            id: run_id.clone(),
            pipeline_id: PipelineId::new("p6_semantic_digest"),
            input_revision_id: revision_id.clone(),
            input_item_id: Some(item_id.clone()),
            input_sha256: source_input_sha256,
            target_kind: Some(DerivativeKind::StructuredResult),
            input_asset_id: None,
            state: ProcessingState::Succeeded,
            provider: outcome.package.provider.clone(),
            tool_or_model: Some(outcome.package.model.clone()),
            tool_version: Some(outcome.package.model_version.clone()),
            attempt: 1,
            retry_of_run_id: None,
            error_code: None,
            error_message: None,
            params,
            usage: outcome.usage,
            loss_notes: Some(
                "Machine semantic interpretation may omit context or contain uncertain classifications; retain C0/C1 evidence."
                    .to_owned(),
            ),
            created_at: generated_at.clone(),
            started_at: Some(generated_at.clone()),
            finished_at: Some(generated_at.clone()),
            invalidated_at: None,
            invalidation_reason: None,
        };
        let derivative = DerivativeRef {
            id: derivative_id.clone(),
            run_id: run_id.clone(),
            kind: DerivativeKind::StructuredResult,
            output_sha256: Some(output_sha256),
            content_text: None,
            content_json: Some(content_json),
            logical_path: None,
            media_type: Some("application/vnd.babata.p6-semantic+json".to_owned()),
            language: Some("zh".to_owned()),
            input_asset_id: None,
            loss_notes: Some(
                "Normalized machine candidate; it is not a user judgment or confirmed fact."
                    .to_owned(),
            ),
            metadata: Metadata::parse(
                &serde_json::json!({"schema_version": babata_domain::SEMANTIC_CANDIDATE_SCHEMA_V1})
                    .to_string(),
            )
            .map_err(ApplicationError::from)?,
            created_at: generated_at,
        };
        self.derived
            .commit_run(&ProcessCommit::new(run).with_derivative(derivative))?;
        let ingest = knowledge.ingest_derivative(&derivative_id)?;
        Ok(SemanticDigestAndIngestOutcome {
            run_id,
            derivative_id,
            ingest,
        })
    }
}

fn collect_evidence(
    runs: &[ShowProcessRunOutcome],
) -> Result<Vec<DerivativeEvidence>, ApplicationError> {
    let mut evidence = runs
        .iter()
        .filter(|run| {
            run.run.invalidated_at.is_none() && run.run.state == ProcessingState::Succeeded
        })
        .flat_map(|run| run.derivatives.iter())
        .map(|derivative| {
            Ok(DerivativeEvidence {
                derivative_id: derivative.id.clone(),
                output_sha256: derivative.output_sha256.clone().ok_or_else(|| {
                    ApplicationError::Integrity(format!(
                        "active derivative {} has no output hash",
                        derivative.id
                    ))
                })?,
            })
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    evidence.sort_by(|left, right| {
        left.derivative_id
            .as_str()
            .cmp(right.derivative_id.as_str())
    });
    Ok(evidence)
}

fn build_review_context(
    context: &crate::KnowledgeReviewContext,
    revision_id: &RevisionId,
) -> Result<String, ApplicationError> {
    let revision = context
        .target
        .revisions
        .iter()
        .find(|candidate| candidate.revision_id == *revision_id)
        .ok_or_else(|| ApplicationError::NotFound(revision_id.to_string()))?;
    let derivatives = context
        .process_runs
        .iter()
        .filter(|run| {
            run.run.invalidated_at.is_none() && run.run.state == ProcessingState::Succeeded
        })
        .flat_map(|run| run.derivatives.iter())
        .map(|derivative| {
            serde_json::json!({
                "derivative_id": derivative.id,
                "kind": derivative.kind,
                "output_sha256": derivative.output_sha256,
                "content_text": derivative.content_text,
                "content_json": derivative.content_json,
                "loss_notes": derivative.loss_notes,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&serde_json::json!({
        "source": {
            "item_id": context.target.item_id,
            "revision_id": revision_id,
            "provider": context.target.provider,
            "source_locator": context.target.source_locator,
            "source_published_at": context.target.source_published_at,
            "metadata": context.target.metadata,
            "raw_text": revision.raw_text,
        },
        "c1_derivatives": derivatives,
    }))
    .map_err(|error| ApplicationError::Integrity(error.to_string()))
}

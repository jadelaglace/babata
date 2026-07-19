use babata_domain::{
    AssetId, DerivativeId, DerivativeKind, DerivativeRef, JobId, Metadata, PipelineId, ProcessRun,
    ProcessingState, RevisionId, RunId, Sha256,
};

use crate::{
    ApplicationError, EnqueueProcessCommand, ProcessJobOutcome, RegisterDerivativeCommand,
    RegisterDerivativeOutcome, RegisterFailureCommand, ShowProcessRunOutcome,
    ports::{
        AssetStorePort, ClockPort, DerivedRepositoryPort, FinalizeAssetOutcome, NewAsset,
        ProcessCommit, RawRepositoryPort, StagedAsset,
    },
};

const PIPELINES: &[&str] = &[
    "local_extract_text",
    "bailian_ocr",
    "bailian_transcript",
    "bailian_summary",
    "bailian_visual_description",
    "agent_import",
];

struct RetryIdentity<'a> {
    pipeline_id: &'a PipelineId,
    revision_id: &'a RevisionId,
    item_id: Option<&'a babata_domain::ItemId>,
    input_sha256: &'a Sha256,
    kind: DerivativeKind,
    input_asset_id: Option<&'a AssetId>,
}

pub struct ProcessService<D, R, A, C> {
    repository: D,
    raw: R,
    assets: A,
    clock: C,
}

impl<D, R, A, C> ProcessService<D, R, A, C>
where
    D: DerivedRepositoryPort,
    R: RawRepositoryPort,
    A: AssetStorePort,
    C: ClockPort,
{
    pub fn new(repository: D, raw: R, assets: A, clock: C) -> Self {
        Self {
            repository,
            raw,
            assets,
            clock,
        }
    }

    pub fn list_pipelines(&self) -> Result<Vec<PipelineId>, ApplicationError> {
        Ok(PIPELINES
            .iter()
            .map(|id| PipelineId::new((*id).to_owned()))
            .collect())
    }

    /// Register a completed cleaning result as C1. Never mutates C0.
    /// The run must prove its input: revision exists and is ready, the
    /// declared input hash matches the revision text hash or a bound asset
    /// hash, and any input asset belongs to that same revision.
    /// Retries always create a new `process_run` row chained to the parent.
    pub fn register_derivative(
        &self,
        mut command: RegisterDerivativeCommand,
    ) -> Result<RegisterDerivativeOutcome, ApplicationError> {
        ensure_known_pipeline(&command.pipeline_id)?;
        ensure_pipeline_kind(&command.pipeline_id, command.kind)?;
        ensure_run_metadata(
            &command.provider,
            command.tool_or_model.as_deref(),
            command.tool_version.as_deref(),
        )?;
        self.validate_input(
            command.kind,
            &command.revision_id,
            command.item_id.as_ref(),
            &command.input_sha256,
            command.input_asset_id.as_ref(),
        )?;
        let attempt = if let Some(retry_of) = &command.retry_of_run_id {
            self.retry_attempt(retry_of, &command)?
        } else {
            1
        };
        let run_id = RunId::new();
        let operation_id = format!("c1_{run_id}");
        let staged = self.prepare_output(&mut command, &operation_id)?;
        let output_sha256 = command.output_sha256.clone().ok_or_else(|| {
            ApplicationError::Integrity("derivative output hash could not be derived".to_owned())
        })?;
        let now = self.clock.now();
        let run = ProcessRun {
            id: run_id,
            pipeline_id: command.pipeline_id.clone(),
            input_revision_id: command.revision_id.clone(),
            input_item_id: command.item_id.clone(),
            input_sha256: command.input_sha256.clone(),
            target_kind: Some(command.kind),
            input_asset_id: command.input_asset_id.clone(),
            state: ProcessingState::Succeeded,
            provider: command.provider.clone(),
            tool_or_model: command.tool_or_model.clone(),
            tool_version: command.tool_version.clone(),
            attempt,
            retry_of_run_id: command.retry_of_run_id.clone(),
            error_code: None,
            error_message: None,
            params: command.params.clone(),
            usage: command.usage.clone(),
            loss_notes: command.loss_notes.clone(),
            created_at: now.clone(),
            started_at: Some(now.clone()),
            finished_at: Some(now.clone()),
            invalidated_at: None,
            invalidation_reason: None,
        };
        let derivative = DerivativeRef {
            id: DerivativeId::new(),
            run_id: run.id.clone(),
            kind: command.kind,
            output_sha256: Some(output_sha256),
            content_text: command.content_text,
            content_json: command.content_json,
            logical_path: command.logical_path,
            media_type: command.media_type,
            language: command.language,
            input_asset_id: command.input_asset_id,
            loss_notes: command.derivative_loss_notes,
            metadata: command.derivative_metadata,
            created_at: now,
        };
        let commit = ProcessCommit::new(run).with_derivative(derivative);
        let warnings = self.commit_output(&commit, staged.as_ref(), &operation_id)?;

        Ok(RegisterDerivativeOutcome {
            run_id: commit.run.id,
            derivative_id: Some(commit.derivatives[0].id.clone()),
            pipeline_id: commit.run.pipeline_id,
            kind: Some(commit.derivatives[0].kind),
            state: commit.run.state,
            warnings,
        })
    }

    /// Record a failed processing attempt so retries are honest: the parent
    /// failed run stays in C1 history and a later success chains to it.
    pub fn register_failure(
        &self,
        command: RegisterFailureCommand,
    ) -> Result<RegisterDerivativeOutcome, ApplicationError> {
        ensure_known_pipeline(&command.pipeline_id)?;
        ensure_pipeline_kind(&command.pipeline_id, command.kind)?;
        ensure_run_metadata(
            &command.provider,
            command.tool_or_model.as_deref(),
            command.tool_version.as_deref(),
        )?;
        self.validate_input(
            command.kind,
            &command.revision_id,
            command.item_id.as_ref(),
            &command.input_sha256,
            command.input_asset_id.as_ref(),
        )?;
        if command.error_code.trim().is_empty() {
            return Err(ApplicationError::Integrity(
                "failed run needs a non-empty error_code".to_owned(),
            ));
        }

        let now = self.clock.now();
        let attempt = if let Some(retry_of) = &command.retry_of_run_id {
            self.retry_identity(
                retry_of,
                RetryIdentity {
                    pipeline_id: &command.pipeline_id,
                    revision_id: &command.revision_id,
                    item_id: command.item_id.as_ref(),
                    input_sha256: &command.input_sha256,
                    kind: command.kind,
                    input_asset_id: command.input_asset_id.as_ref(),
                },
            )?
        } else {
            1
        };
        let target_kind = command.kind;

        let run = ProcessRun {
            id: RunId::new(),
            pipeline_id: command.pipeline_id.clone(),
            input_revision_id: command.revision_id,
            input_item_id: command.item_id,
            input_sha256: command.input_sha256,
            target_kind: Some(target_kind),
            input_asset_id: command.input_asset_id,
            state: ProcessingState::Failed,
            provider: command.provider,
            tool_or_model: command.tool_or_model,
            tool_version: command.tool_version,
            attempt,
            retry_of_run_id: command.retry_of_run_id,
            error_code: Some(command.error_code),
            error_message: command.error_message,
            params: command.params,
            usage: Metadata::empty(),
            loss_notes: command.loss_notes,
            created_at: now.clone(),
            started_at: Some(now.clone()),
            finished_at: Some(now),
            invalidated_at: None,
            invalidation_reason: None,
        };
        let commit = ProcessCommit::new(run);
        self.repository.commit_run(&commit)?;

        Ok(RegisterDerivativeOutcome {
            run_id: commit.run.id,
            derivative_id: None,
            pipeline_id: commit.run.pipeline_id,
            kind: Some(target_kind),
            state: commit.run.state,
            warnings: Vec::new(),
        })
    }

    pub fn show_run(&self, run_id: &RunId) -> Result<ShowProcessRunOutcome, ApplicationError> {
        let run = self
            .repository
            .get_run(run_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("run {run_id}")))?;
        let derivatives = self.repository.list_derivatives(run_id)?;
        Ok(ShowProcessRunOutcome { run, derivatives })
    }

    pub fn list_runs_for_revision(
        &self,
        revision_id: &RevisionId,
    ) -> Result<Vec<ProcessRun>, ApplicationError> {
        self.repository.list_runs_for_revision(revision_id)
    }

    /// Logically delete a completed C1 result without erasing its audit trail
    /// or touching the C0 input. A rebuild is a separate process run.
    pub fn delete_result(
        &self,
        run_id: &RunId,
        reason: &str,
    ) -> Result<ShowProcessRunOutcome, ApplicationError> {
        if reason.trim().is_empty() {
            return Err(ApplicationError::Integrity(
                "C1 result deletion needs a non-empty reason".to_owned(),
            ));
        }
        let mut run = self.parent_run(run_id)?;
        if run.state != ProcessingState::Succeeded {
            return Err(ApplicationError::Integrity(format!(
                "only succeeded C1 results can be deleted; run {run_id} is {:?}",
                run.state
            )));
        }
        let derivatives = self.repository.list_derivatives(run_id)?;
        if derivatives.is_empty() {
            return Err(ApplicationError::Integrity(format!(
                "succeeded run {run_id} has no derivative to delete"
            )));
        }
        if run.invalidated_at.is_none() {
            run.invalidated_at = Some(self.clock.now());
            run.invalidation_reason = Some(reason.trim().to_owned());
            self.repository.update_run(&run)?;
        }
        Ok(ShowProcessRunOutcome { run, derivatives })
    }

    pub fn enqueue(
        &self,
        _command: EnqueueProcessCommand,
    ) -> Result<ProcessJobOutcome, ApplicationError> {
        Err(ApplicationError::capability_unavailable(
            "processing.enqueue",
            "P5+",
        ))
    }

    pub fn run_once(&self) -> Result<ProcessJobOutcome, ApplicationError> {
        Err(ApplicationError::capability_unavailable(
            "processing.run_once",
            "P5+",
        ))
    }

    pub fn status(&self, _job_id: &JobId) -> Result<ProcessJobOutcome, ApplicationError> {
        Err(ApplicationError::capability_unavailable(
            "processing.job_queue",
            "P5+",
        ))
    }

    pub fn retry(&self, _job_id: &JobId) -> Result<ProcessJobOutcome, ApplicationError> {
        Err(ApplicationError::capability_unavailable(
            "processing.job_queue",
            "P5+",
        ))
    }

    pub fn cancel(&self, _job_id: &JobId) -> Result<ProcessJobOutcome, ApplicationError> {
        Err(ApplicationError::capability_unavailable(
            "processing.job_queue",
            "P5+",
        ))
    }

    fn validate_input(
        &self,
        kind: DerivativeKind,
        revision_id: &RevisionId,
        item_id: Option<&babata_domain::ItemId>,
        input_sha256: &Sha256,
        input_asset_id: Option<&AssetId>,
    ) -> Result<(), ApplicationError> {
        if kind_requires_asset(kind) && input_asset_id.is_none() {
            return Err(ApplicationError::Integrity(format!(
                "{} derivatives require --input-asset-id",
                derivative_kind_name(kind)
            )));
        }
        let bound_asset = input_asset_id
            .map(|asset_id| self.asset_for_revision(asset_id, revision_id))
            .transpose()?;
        if let Some(asset) = &bound_asset {
            if asset.sha256 != *input_sha256 {
                return Err(ApplicationError::Integrity(format!(
                    "input_sha256 {input_sha256} does not match asset {} hash {}",
                    asset.id, asset.sha256
                )));
            }
            ensure_asset_kind(kind, asset)?;
        }
        self.validate_revision_binding(revision_id, item_id, input_sha256, bound_asset.as_ref())
    }

    fn prepare_output(
        &self,
        command: &mut RegisterDerivativeCommand,
        operation_id: &str,
    ) -> Result<Option<StagedAsset>, ApplicationError> {
        if command.source_file.is_some() && command.logical_path.is_some() {
            return Err(ApplicationError::Integrity(
                "use either source_file or logical_path, not both".to_owned(),
            ));
        }
        let staged = command
            .source_file
            .take()
            .map(|source| self.assets.stage_derived_file(&source, operation_id))
            .transpose()
            .map_err(|error| error.with_operation(operation_id.to_owned()))?;
        if let Some(staged) = &staged {
            command.logical_path = Some(staged.logical_path.clone());
            if command.media_type.is_none() {
                command.media_type.clone_from(&staged.media_type);
            }
        }
        if let Err(error) = self.validate_output_representations(command, staged.as_ref()) {
            if let Some(staged) = &staged {
                self.discard_invalid_output(staged, operation_id)?;
            }
            return Err(error);
        }
        Ok(staged)
    }

    fn validate_output_representations(
        &self,
        command: &mut RegisterDerivativeCommand,
        staged: Option<&StagedAsset>,
    ) -> Result<(), ApplicationError> {
        let mut hashes = Vec::new();
        if let Some(text) = &command.content_text {
            if text.trim().is_empty() {
                return Err(ApplicationError::Integrity(
                    "content_text must not be empty".to_owned(),
                ));
            }
            hashes.push(("content_text", Sha256::of_bytes(text.as_bytes())));
        }
        if let Some(json) = &command.content_json {
            if json.trim().is_empty() {
                return Err(ApplicationError::Integrity(
                    "content_json must not be empty".to_owned(),
                ));
            }
            serde_json::from_str::<serde_json::Value>(json).map_err(|error| {
                ApplicationError::Integrity(format!("content_json is invalid JSON: {error}"))
            })?;
            hashes.push(("content_json", Sha256::of_bytes(json.as_bytes())));
        }
        if let Some(path) = &command.logical_path {
            ensure_managed_c1_prefix(path)?;
            let hash = match staged {
                Some(staged) => staged.sha256.clone(),
                None => self.assets.hash_logical(path)?,
            };
            ensure_managed_c1_path(path, &hash)?;
            hashes.push(("logical_path", hash));
        }
        let Some((first_name, first_hash)) = hashes.first() else {
            return Err(ApplicationError::Integrity(
                "derivative needs content_text, content_json, logical_path, or source_file"
                    .to_owned(),
            ));
        };
        for (name, hash) in hashes.iter().skip(1) {
            if hash != first_hash {
                return Err(ApplicationError::Integrity(format!(
                    "output representations disagree: {first_name} hashes to {first_hash}, but {name} hashes to {hash}"
                )));
            }
        }
        if let Some(declared) = &command.output_sha256 {
            if declared != first_hash {
                return Err(ApplicationError::Integrity(format!(
                    "output_sha256 {declared} does not match output bytes {first_hash}"
                )));
            }
        } else {
            command.output_sha256 = Some(first_hash.clone());
        }
        Ok(())
    }

    fn discard_invalid_output(
        &self,
        staged: &StagedAsset,
        operation_id: &str,
    ) -> Result<(), ApplicationError> {
        self.assets
            .discard_stage(staged)
            .and_then(|()| self.assets.complete_operation(operation_id))
            .map_err(|error| error.with_operation(operation_id.to_owned()))
    }

    fn commit_output(
        &self,
        commit: &ProcessCommit,
        staged: Option<&StagedAsset>,
        operation_id: &str,
    ) -> Result<Vec<String>, ApplicationError> {
        let Some(staged) = staged else {
            self.repository.commit_run(commit)?;
            return Ok(Vec::new());
        };
        let finalization = match self.assets.finalize(staged) {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = self.assets.preserve_operation(
                    operation_id,
                    &commit.run.input_revision_id.to_string(),
                    "c1_finalize_failed",
                );
                return Err(error.with_operation(operation_id.to_owned()));
            }
        };
        match self.assets.verify(staged) {
            Ok(true) => {}
            Ok(false) => {
                let error = ApplicationError::Integrity(
                    "finalized C1 bytes failed hash verification".to_owned(),
                );
                self.preserve_uncommitted_output(
                    staged,
                    operation_id,
                    finalization,
                    &commit.run.input_revision_id,
                    &error,
                )?;
                return Err(error.with_operation(operation_id.to_owned()));
            }
            Err(error) => {
                self.preserve_uncommitted_output(
                    staged,
                    operation_id,
                    finalization,
                    &commit.run.input_revision_id,
                    &error,
                )?;
                return Err(error.with_operation(operation_id.to_owned()));
            }
        }
        if let Err(error) = self.repository.commit_run(commit) {
            self.preserve_uncommitted_output(
                staged,
                operation_id,
                finalization,
                &commit.run.input_revision_id,
                &error,
            )?;
            return Err(error.with_operation(operation_id.to_owned()));
        }
        let mut warnings = Vec::new();
        if self.assets.complete_operation(operation_id).is_err() {
            warnings.push("C1 committed; recovery journal cleanup is pending".to_owned());
        }
        Ok(warnings)
    }

    fn preserve_uncommitted_output(
        &self,
        staged: &StagedAsset,
        operation_id: &str,
        finalization: FinalizeAssetOutcome,
        revision_id: &RevisionId,
        original: &ApplicationError,
    ) -> Result<(), ApplicationError> {
        self.assets
            .quarantine_finalized(staged, operation_id, finalization)
            .and_then(|()| {
                self.assets.preserve_operation(
                    operation_id,
                    &revision_id.to_string(),
                    "c1_database_commit_failed",
                )
            })
            .map_err(|recovery| {
                ApplicationError::Integrity(format!(
                    "C1 commit failed ({original}); recovery evidence also failed ({recovery})"
                ))
                .with_operation(operation_id.to_owned())
            })
    }

    fn asset_for_revision(
        &self,
        asset_id: &babata_domain::AssetId,
        revision_id: &RevisionId,
    ) -> Result<NewAsset, ApplicationError> {
        let asset = self
            .raw
            .find_asset(asset_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("asset {asset_id}")))?;
        if asset.revision_id != *revision_id {
            return Err(ApplicationError::Integrity(format!(
                "asset {asset_id} belongs to revision {}, not {revision_id}",
                asset.revision_id
            )));
        }
        Ok(asset)
    }

    fn validate_revision_binding(
        &self,
        revision_id: &RevisionId,
        item_id: Option<&babata_domain::ItemId>,
        input_sha256: &Sha256,
        bound_asset: Option<&NewAsset>,
    ) -> Result<(), ApplicationError> {
        let revision = self
            .raw
            .find_revision(revision_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("revision {revision_id}")))?;
        let state = self
            .raw
            .find_revision_state(revision_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("revision {revision_id}")))?;
        if state != babata_domain::RawState::Ready {
            return Err(ApplicationError::Integrity(format!(
                "revision {revision_id} is {state:?}, only ready revisions accept C1 derivatives"
            )));
        }
        if let Some(item_id) = item_id {
            if revision.item_id != *item_id {
                return Err(ApplicationError::Integrity(format!(
                    "revision {revision_id} belongs to item {}, not {item_id}",
                    revision.item_id
                )));
            }
        }
        let text_match = revision.text_sha256.as_ref() == Some(input_sha256);
        let asset_match = bound_asset.is_some_and(|asset| asset.sha256 == *input_sha256);
        if !text_match && !asset_match {
            return Err(ApplicationError::Integrity(format!(
                "input_sha256 {input_sha256} matches neither the text hash of revision {revision_id} nor the bound asset hash"
            )));
        }
        Ok(())
    }

    fn parent_run(&self, run_id: &RunId) -> Result<ProcessRun, ApplicationError> {
        self.repository
            .get_run(run_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("run {run_id}")))
    }

    fn retry_attempt(
        &self,
        retry_of: &RunId,
        command: &RegisterDerivativeCommand,
    ) -> Result<u32, ApplicationError> {
        self.retry_identity(
            retry_of,
            RetryIdentity {
                pipeline_id: &command.pipeline_id,
                revision_id: &command.revision_id,
                item_id: command.item_id.as_ref(),
                input_sha256: &command.input_sha256,
                kind: command.kind,
                input_asset_id: command.input_asset_id.as_ref(),
            },
        )
    }

    fn retry_identity(
        &self,
        retry_of: &RunId,
        identity: RetryIdentity<'_>,
    ) -> Result<u32, ApplicationError> {
        let parent = self.parent_run(retry_of)?;
        if parent.state != ProcessingState::Failed {
            return Err(ApplicationError::Integrity(format!(
                "retry parent {retry_of} is not failed"
            )));
        }
        if parent.input_revision_id != *identity.revision_id {
            return Err(ApplicationError::Integrity(format!(
                "retry parent {retry_of} belongs to revision {}, not {}",
                parent.input_revision_id, identity.revision_id
            )));
        }
        if parent.input_item_id.as_ref() != identity.item_id {
            return Err(ApplicationError::Integrity(
                "retry parent item identity does not match this attempt".to_owned(),
            ));
        }
        if parent.input_sha256 != *identity.input_sha256 {
            return Err(ApplicationError::Integrity(
                "retry parent input hash does not match this attempt".to_owned(),
            ));
        }
        if parent.pipeline_id != *identity.pipeline_id {
            return Err(ApplicationError::Integrity(
                "retry parent pipeline does not match this attempt".to_owned(),
            ));
        }
        if parent.target_kind != Some(identity.kind) {
            return Err(ApplicationError::Integrity(
                "retry parent target kind does not match this attempt".to_owned(),
            ));
        }
        if parent.input_asset_id.as_ref() != identity.input_asset_id {
            return Err(ApplicationError::Integrity(
                "retry parent input asset does not match this attempt".to_owned(),
            ));
        }
        Ok(parent.attempt.saturating_add(1))
    }
}

fn ensure_known_pipeline(pipeline_id: &PipelineId) -> Result<(), ApplicationError> {
    if !PIPELINES.contains(&pipeline_id.as_str()) {
        return Err(ApplicationError::Integrity(format!(
            "unknown pipeline: {}",
            pipeline_id.as_str()
        )));
    }
    Ok(())
}

fn ensure_pipeline_kind(
    pipeline_id: &PipelineId,
    kind: DerivativeKind,
) -> Result<(), ApplicationError> {
    let compatible = match pipeline_id.as_str() {
        "local_extract_text" => kind == DerivativeKind::ExtractedText,
        "bailian_ocr" => kind == DerivativeKind::OcrText,
        "bailian_transcript" => {
            matches!(kind, DerivativeKind::Transcript | DerivativeKind::Subtitle)
        }
        "bailian_summary" => kind == DerivativeKind::Summary,
        "bailian_visual_description" => {
            matches!(
                kind,
                DerivativeKind::VisualDescription | DerivativeKind::KeyFrame
            )
        }
        "agent_import" => true,
        _ => false,
    };
    if !compatible {
        return Err(ApplicationError::Integrity(format!(
            "pipeline {} cannot produce {}",
            pipeline_id.as_str(),
            derivative_kind_name(kind)
        )));
    }
    Ok(())
}

fn ensure_run_metadata(
    provider: &str,
    tool_or_model: Option<&str>,
    tool_version: Option<&str>,
) -> Result<(), ApplicationError> {
    if provider.trim().is_empty() {
        return Err(ApplicationError::Integrity(
            "provider must not be empty".to_owned(),
        ));
    }
    if tool_or_model.is_none_or(|value| value.trim().is_empty()) {
        return Err(ApplicationError::Integrity(
            "tool_or_model must not be empty".to_owned(),
        ));
    }
    if tool_version.is_none_or(|value| value.trim().is_empty()) {
        return Err(ApplicationError::Integrity(
            "tool_version must not be empty".to_owned(),
        ));
    }
    Ok(())
}

fn kind_requires_asset(kind: DerivativeKind) -> bool {
    matches!(
        kind,
        DerivativeKind::ExtractedText
            | DerivativeKind::OcrText
            | DerivativeKind::Transcript
            | DerivativeKind::Subtitle
            | DerivativeKind::VisualDescription
            | DerivativeKind::KeyFrame
            | DerivativeKind::MediaMetadata
    )
}

fn ensure_asset_kind(kind: DerivativeKind, asset: &NewAsset) -> Result<(), ApplicationError> {
    let media_type = asset.media_type.as_deref().ok_or_else(|| {
        ApplicationError::Integrity(format!(
            "asset {} has no media_type for {}",
            asset.id,
            derivative_kind_name(kind)
        ))
    })?;
    let compatible = match kind {
        DerivativeKind::ExtractedText => {
            media_type.starts_with("text/") || media_type.starts_with("application/")
        }
        DerivativeKind::OcrText | DerivativeKind::VisualDescription | DerivativeKind::KeyFrame => {
            media_type.starts_with("image/")
                || media_type.starts_with("video/")
                || media_type == "application/pdf"
        }
        DerivativeKind::Transcript | DerivativeKind::Subtitle => {
            media_type.starts_with("audio/") || media_type.starts_with("video/")
        }
        DerivativeKind::MediaMetadata
        | DerivativeKind::Summary
        | DerivativeKind::Tags
        | DerivativeKind::StructuredResult => true,
    };
    if !compatible {
        return Err(ApplicationError::Integrity(format!(
            "asset {} media_type {media_type} is incompatible with {}",
            asset.id,
            derivative_kind_name(kind)
        )));
    }
    Ok(())
}

fn ensure_managed_c1_prefix(path: &babata_domain::LogicalPath) -> Result<(), ApplicationError> {
    if !path.as_str().starts_with("02_derived/files/sha256/") {
        return Err(ApplicationError::Integrity(format!(
            "C1 logical_path must be under 02_derived/files/sha256, got {}",
            path.as_str()
        )));
    }
    Ok(())
}

fn ensure_managed_c1_path(
    path: &babata_domain::LogicalPath,
    hash: &Sha256,
) -> Result<(), ApplicationError> {
    let expected = format!("02_derived/files/sha256/{}/{}", &hash.as_str()[..2], hash);
    if path.as_str() != expected {
        return Err(ApplicationError::Integrity(format!(
            "C1 logical_path {} is not the content-addressed path for {hash}",
            path.as_str()
        )));
    }
    Ok(())
}

fn derivative_kind_name(kind: DerivativeKind) -> &'static str {
    match kind {
        DerivativeKind::ExtractedText => "extracted_text",
        DerivativeKind::OcrText => "ocr_text",
        DerivativeKind::Transcript => "transcript",
        DerivativeKind::Subtitle => "subtitle",
        DerivativeKind::Summary => "summary",
        DerivativeKind::VisualDescription => "visual_description",
        DerivativeKind::KeyFrame => "key_frame",
        DerivativeKind::Tags => "tags",
        DerivativeKind::StructuredResult => "structured_result",
        DerivativeKind::MediaMetadata => "media_metadata",
    }
}

use babata_domain::{
    DerivativeId, DerivativeRef, JobId, Metadata, PipelineId, ProcessRun, ProcessingState,
    RevisionId, RunId, Sha256,
};

use crate::{
    ApplicationError, EnqueueProcessCommand, ProcessJobOutcome, RegisterDerivativeCommand,
    RegisterDerivativeOutcome, RegisterFailureCommand, ShowProcessRunOutcome,
    ports::{
        AssetStorePort, ClockPort, DerivedRepositoryPort, NewAsset, ProcessCommit,
        RawRepositoryPort,
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
    /// Retries always create a new process_run row chained to the parent.
    pub fn register_derivative(
        &self,
        mut command: RegisterDerivativeCommand,
    ) -> Result<RegisterDerivativeOutcome, ApplicationError> {
        ensure_known_pipeline(&command.pipeline_id)?;
        if command.content_text.is_none()
            && command.content_json.is_none()
            && command.logical_path.is_none()
        {
            return Err(ApplicationError::Integrity(
                "derivative needs content_text, content_json, or logical_path".to_owned(),
            ));
        }
        if let Some(asset_id) = &command.input_asset_id {
            let asset = self.asset_for_revision(asset_id, &command.revision_id)?;
            self.bind_input_to_asset(&mut command, &asset)?;
        } else {
            self.validate_revision_binding(
                &command.revision_id,
                command.item_id.as_ref(),
                &command.input_sha256,
                None,
            )?;
        }
        if let Some(logical_path) = &command.logical_path {
            let actual = self.assets.hash_logical(logical_path)?;
            if let Some(declared) = &command.output_sha256 {
                if declared != &actual {
                    return Err(ApplicationError::Integrity(format!(
                        "output_sha256 {declared} does not match stored bytes {actual} at {}",
                        logical_path.as_str()
                    )));
                }
            } else {
                command.output_sha256 = Some(actual);
            }
        }
        if command.output_sha256.is_none() {
            command.output_sha256 = command
                .content_text
                .as_ref()
                .map(|text| Sha256::of_bytes(text.as_bytes()))
                .or_else(|| {
                    command
                        .content_json
                        .as_ref()
                        .map(|text| Sha256::of_bytes(text.as_bytes()))
                });
        }
        let output_sha256 = command.output_sha256.clone().ok_or_else(|| {
            ApplicationError::Integrity("derivative output hash could not be derived".to_owned())
        })?;

        let now = self.clock.now();
        let attempt = if let Some(retry_of) = &command.retry_of_run_id {
            self.retry_attempt(retry_of, &command)?
        } else {
            1
        };

        let run = ProcessRun {
            id: RunId::new(),
            pipeline_id: command.pipeline_id.clone(),
            input_revision_id: command.revision_id.clone(),
            input_item_id: command.item_id.clone(),
            input_sha256: command.input_sha256.clone(),
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
        self.repository.commit_run(&commit)?;

        Ok(RegisterDerivativeOutcome {
            run_id: commit.run.id,
            derivative_id: Some(commit.derivatives[0].id.clone()),
            pipeline_id: commit.run.pipeline_id,
            kind: Some(commit.derivatives[0].kind),
            state: commit.run.state,
        })
    }

    /// Record a failed processing attempt so retries are honest: the parent
    /// failed run stays in C1 history and a later success chains to it.
    pub fn register_failure(
        &self,
        command: RegisterFailureCommand,
    ) -> Result<RegisterDerivativeOutcome, ApplicationError> {
        ensure_known_pipeline(&command.pipeline_id)?;
        let bound_asset = match &command.input_asset_id {
            Some(asset_id) => Some(self.asset_for_revision(asset_id, &command.revision_id)?),
            None => None,
        };
        self.validate_revision_binding(
            &command.revision_id,
            command.item_id.as_ref(),
            &command.input_sha256,
            bound_asset.as_ref(),
        )?;
        if command.error_code.trim().is_empty() {
            return Err(ApplicationError::Integrity(
                "failed run needs a non-empty error_code".to_owned(),
            ));
        }

        let now = self.clock.now();
        let attempt = if let Some(retry_of) = &command.retry_of_run_id {
            let parent = self.parent_run(retry_of)?;
            if parent.input_revision_id != command.revision_id {
                return Err(ApplicationError::Integrity(format!(
                    "retry parent {retry_of} belongs to revision {}, not {}",
                    parent.input_revision_id, command.revision_id
                )));
            }
            if parent.input_sha256 != command.input_sha256 {
                return Err(ApplicationError::Integrity(
                    "retry parent input hash does not match this attempt".to_owned(),
                ));
            }
            if parent.pipeline_id != command.pipeline_id {
                return Err(ApplicationError::Integrity(
                    "retry parent pipeline does not match this attempt".to_owned(),
                ));
            }
            parent.attempt.saturating_add(1)
        } else {
            1
        };

        let run = ProcessRun {
            id: RunId::new(),
            pipeline_id: command.pipeline_id.clone(),
            input_revision_id: command.revision_id,
            input_item_id: command.item_id,
            input_sha256: command.input_sha256,
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
        };
        let commit = ProcessCommit::new(run);
        self.repository.commit_run(&commit)?;

        Ok(RegisterDerivativeOutcome {
            run_id: commit.run.id,
            derivative_id: None,
            pipeline_id: commit.run.pipeline_id,
            kind: None,
            state: commit.run.state,
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

    fn bind_input_to_asset(
        &self,
        command: &mut RegisterDerivativeCommand,
        asset: &NewAsset,
    ) -> Result<(), ApplicationError> {
        self.validate_revision_binding(
            &command.revision_id,
            command.item_id.as_ref(),
            &command.input_sha256,
            Some(asset),
        )?;
        if asset.sha256 != command.input_sha256 {
            return Err(ApplicationError::Integrity(format!(
                "input_sha256 {} does not match asset {} hash {}",
                command.input_sha256, asset.id, asset.sha256
            )));
        }
        if command.media_type.is_none() {
            command.media_type = asset.media_type.clone();
        }
        Ok(())
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
        let parent = self.parent_run(retry_of)?;
        if parent.input_revision_id != command.revision_id {
            return Err(ApplicationError::Integrity(format!(
                "retry parent {retry_of} belongs to revision {}, not {}",
                parent.input_revision_id, command.revision_id
            )));
        }
        if parent.input_sha256 != command.input_sha256 {
            return Err(ApplicationError::Integrity(
                "retry parent input hash does not match this attempt".to_owned(),
            ));
        }
        if parent.pipeline_id != command.pipeline_id {
            return Err(ApplicationError::Integrity(
                "retry parent pipeline does not match this attempt".to_owned(),
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

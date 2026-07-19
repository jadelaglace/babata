use babata_domain::{
    DerivativeId, DerivativeRef, JobId, Metadata, PipelineId, ProcessRun, ProcessingState,
    RevisionId, RunId, Sha256,
};

use crate::{
    ApplicationError, EnqueueProcessCommand, ProcessJobOutcome, RegisterDerivativeCommand,
    RegisterDerivativeOutcome, ShowProcessRunOutcome,
    ports::{ClockPort, DerivedRepositoryPort},
};

const PIPELINES: &[&str] = &[
    "local_extract_text",
    "bailian_ocr",
    "bailian_transcript",
    "bailian_summary",
    "bailian_visual_description",
    "agent_import",
];

pub struct ProcessService<R, C> {
    repository: R,
    clock: C,
}

impl<R, C> ProcessService<R, C>
where
    R: DerivedRepositoryPort,
    C: ClockPort,
{
    pub fn new(repository: R, clock: C) -> Self {
        Self { repository, clock }
    }

    pub fn list_pipelines(&self) -> Result<Vec<PipelineId>, ApplicationError> {
        Ok(PIPELINES
            .iter()
            .map(|id| PipelineId::new((*id).to_owned()))
            .collect())
    }

    /// Register a completed cleaning result as C1. Does not mutate C0.
    /// Retries always create a new process_run row.
    pub fn register_derivative(
        &self,
        command: RegisterDerivativeCommand,
    ) -> Result<RegisterDerivativeOutcome, ApplicationError> {
        if !PIPELINES.contains(&command.pipeline_id.as_str()) {
            return Err(ApplicationError::Integrity(format!(
                "unknown pipeline: {}",
                command.pipeline_id.as_str()
            )));
        }
        if command.content_text.is_none()
            && command.content_json.is_none()
            && command.logical_path.is_none()
        {
            return Err(ApplicationError::Integrity(
                "derivative needs content_text, content_json, or logical_path".to_owned(),
            ));
        }

        let now = self.clock.now();
        let attempt = if let Some(retry_of) = &command.retry_of_run_id {
            let parent = self
                .repository
                .get_run(retry_of)?
                .ok_or_else(|| ApplicationError::NotFound(format!("run {retry_of}")))?;
            parent.attempt.saturating_add(1)
        } else {
            1
        };

        let output_sha256 = command
            .content_text
            .as_ref()
            .map(|text| Sha256::of_bytes(text.as_bytes()))
            .or_else(|| {
                command
                    .content_json
                    .as_ref()
                    .map(|text| Sha256::of_bytes(text.as_bytes()))
            })
            .or(command.output_sha256.clone());

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
        self.repository.create_run(&run)?;

        let derivative = DerivativeRef {
            id: DerivativeId::new(),
            run_id: run.id.clone(),
            kind: command.kind,
            output_sha256,
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
        self.repository.add_derivative(&derivative)?;

        Ok(RegisterDerivativeOutcome {
            run_id: run.id,
            derivative_id: derivative.id,
            pipeline_id: run.pipeline_id,
            kind: derivative.kind,
            state: run.state,
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
}

// keep Metadata in scope for callers via command
#[allow(dead_code)]
type _Meta = Metadata;

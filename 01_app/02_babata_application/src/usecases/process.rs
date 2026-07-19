use babata_domain::{
    AssetId, CapabilityStatus, DerivativeId, DerivativeKind, DerivativeRef, JobId, LogicalPath,
    Metadata, PipelineId, ProcessJob, ProcessJobState, ProcessRun, ProcessingState, RevisionId,
    RunId, Sha256,
};

use crate::{
    ApplicationError, EnqueueProcessCommand, ProcessJobOutcome, RegisterDerivativeCommand,
    RegisterDerivativeOutcome, RegisterFailureCommand, ShowProcessRunOutcome,
    ports::{
        AssetStorePort, ClockPort, DerivedRepositoryPort, FinalizeAssetOutcome, JobRepositoryPort,
        NewAsset, ProcessCommit, ProcessProviderPort, ProviderExecutionOutcome,
        ProviderExecutionRequest, RawRepositoryPort, StagedAsset,
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

const JOB_LEASE_SECONDS: u32 = 300;

pub struct ProcessService<D, R, A, C, J = (), P = ()> {
    repository: D,
    raw: R,
    assets: A,
    clock: C,
    jobs: J,
    providers: P,
}

impl<D, R, A, C> ProcessService<D, R, A, C, (), ()>
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
            jobs: (),
            providers: (),
        }
    }
}

impl<D, R, A, C, J, P> ProcessService<D, R, A, C, J, P> {
    pub fn with_runtime<J2, P2>(
        self,
        jobs: J2,
        providers: P2,
    ) -> ProcessService<D, R, A, C, J2, P2> {
        ProcessService {
            repository: self.repository,
            raw: self.raw,
            assets: self.assets,
            clock: self.clock,
            jobs,
            providers,
        }
    }
}

impl<D, R, A, C, J, P> ProcessService<D, R, A, C, J, P>
where
    D: DerivedRepositoryPort,
    R: RawRepositoryPort,
    A: AssetStorePort,
    C: ClockPort,
    J: JobRepositoryPort,
    P: ProcessProviderPort,
{
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
        command: EnqueueProcessCommand,
    ) -> Result<ProcessJobOutcome, ApplicationError> {
        ensure_known_pipeline(&command.pipeline_id)?;
        let descriptor = self.providers.describe(&command.pipeline_id);
        if descriptor.status != CapabilityStatus::Enabled {
            return Err(ApplicationError::capability_unavailable(
                descriptor.id.0,
                descriptor.activation_phase,
            ));
        }
        let identity = self.providers.identity(&command.pipeline_id)?;
        ensure_pipeline_kind(&command.pipeline_id, identity.kind)?;
        ensure_run_metadata(
            &identity.provider,
            Some(&identity.tool_or_model),
            Some(&identity.tool_version),
        )?;
        let revision = self
            .raw
            .find_revision(&command.revision_id)?
            .ok_or_else(|| {
                ApplicationError::NotFound(format!("revision {}", command.revision_id))
            })?;
        let (input_sha256, input_asset_id) =
            self.select_queued_input(&command.pipeline_id, &revision)?;
        self.validate_input(
            identity.kind,
            &revision.id,
            Some(&revision.item_id),
            &input_sha256,
            input_asset_id.as_ref(),
        )?;
        let job = ProcessJob {
            id: JobId::new(),
            pipeline_id: command.pipeline_id,
            input_revision_id: revision.id,
            input_item_id: Some(revision.item_id),
            input_sha256,
            target_kind: identity.kind,
            input_asset_id,
            state: ProcessJobState::Queued,
            provider: identity.provider,
            tool_or_model: identity.tool_or_model,
            tool_version: identity.tool_version,
            attempt: 1,
            retry_of_job_id: None,
            worker_id: None,
            lease_expires_at: None,
            provider_task: None,
            error_code: None,
            error_message: None,
            result_run_id: None,
            cancel_requested: false,
            params: Metadata::empty(),
            created_at: self.clock.now(),
            started_at: None,
            heartbeat_at: None,
            finished_at: None,
        };
        self.jobs.enqueue(&job)?;
        Ok(job_outcome(job))
    }

    pub fn run_once(&self) -> Result<ProcessJobOutcome, ApplicationError> {
        let now = self.clock.now();
        let worker_id = format!("process-worker-{}", JobId::new());
        let Some(job) = self.jobs.claim(&worker_id, &now, JOB_LEASE_SECONDS)? else {
            return Ok(ProcessJobOutcome {
                status: "idle".to_owned(),
                job: None,
            });
        };
        let request = match self.execution_request(&job) {
            Ok(request) => request,
            Err(error) => return self.fail_job(&job, &worker_id, None, error),
        };
        let execution = match self.providers.execute(&request) {
            Ok(execution) => execution,
            Err(error) => return self.fail_job(&job, &worker_id, None, error),
        };
        let current = self
            .jobs
            .get(&job.id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("job {}", job.id)))?;
        if current.state == ProcessJobState::Cancelled || current.cancel_requested {
            self.providers.cancel(&execution.task)?;
            return Ok(job_outcome(current));
        }
        if let Err(error) = validate_provider_outcome(&job, &execution) {
            return self.fail_job(&job, &worker_id, Some(&execution.task), error);
        }
        let retry_of_run_id = self.retry_run_id(&job)?;
        let traced_params = match execution_trace_params(execution.params, &job, &execution.task) {
            Ok(params) => params,
            Err(error) => {
                return self.fail_job(&job, &worker_id, Some(&execution.task), error);
            }
        };
        let register = RegisterDerivativeCommand {
            pipeline_id: job.pipeline_id.clone(),
            revision_id: job.input_revision_id.clone(),
            item_id: job.input_item_id.clone(),
            input_sha256: job.input_sha256.clone(),
            kind: execution.kind,
            provider: execution.provider,
            tool_or_model: Some(execution.tool_or_model),
            tool_version: Some(execution.tool_version),
            retry_of_run_id,
            params: traced_params,
            usage: execution.usage,
            loss_notes: execution.loss_notes.clone(),
            content_text: execution.content_text,
            content_json: execution.content_json,
            logical_path: None,
            source_file: None,
            media_type: execution.media_type,
            language: execution.language,
            input_asset_id: job.input_asset_id.clone(),
            output_sha256: None,
            derivative_loss_notes: execution.loss_notes,
            derivative_metadata: Metadata::empty(),
        };
        let task = execution.task;
        let registered = match self.register_derivative(register) {
            Ok(outcome) => outcome,
            Err(error) => return self.fail_job(&job, &worker_id, Some(&task), error),
        };
        let completed = self.jobs.complete(
            &job.id,
            &worker_id,
            &registered.run_id,
            &task,
            &self.clock.now(),
        )?;
        Ok(job_outcome(completed))
    }

    pub fn status(&self, job_id: &JobId) -> Result<ProcessJobOutcome, ApplicationError> {
        let job = self
            .jobs
            .get(job_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("job {job_id}")))?;
        Ok(job_outcome(self.reconcile_success(job)?))
    }

    pub fn retry(&self, job_id: &JobId) -> Result<ProcessJobOutcome, ApplicationError> {
        let parent = self
            .jobs
            .get(job_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("job {job_id}")))?;
        let parent = self.reconcile_success(parent)?;
        if parent.state != ProcessJobState::Failed {
            return Err(ApplicationError::Conflict(format!(
                "job {job_id} is not failed"
            )));
        }
        let parent = self.ensure_failure_run(parent)?;
        let retry = ProcessJob {
            id: JobId::new(),
            state: ProcessJobState::Queued,
            attempt: parent.attempt.saturating_add(1),
            retry_of_job_id: Some(parent.id.clone()),
            worker_id: None,
            lease_expires_at: None,
            provider_task: None,
            error_code: None,
            error_message: None,
            result_run_id: None,
            cancel_requested: false,
            created_at: self.clock.now(),
            started_at: None,
            heartbeat_at: None,
            finished_at: None,
            ..parent.clone()
        };
        self.jobs.retry(job_id, &retry)?;
        Ok(job_outcome(retry))
    }

    pub fn cancel(&self, job_id: &JobId) -> Result<ProcessJobOutcome, ApplicationError> {
        let current = self
            .jobs
            .get(job_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("job {job_id}")))?;
        let current = self.reconcile_success(current)?;
        if let Some(task) = &current.provider_task {
            self.providers.cancel(task)?;
        }
        Ok(job_outcome(self.jobs.cancel(job_id, &self.clock.now())?))
    }

    fn select_queued_input(
        &self,
        pipeline_id: &PipelineId,
        revision: &crate::ports::NewRevision,
    ) -> Result<(Sha256, Option<AssetId>), ApplicationError> {
        match pipeline_id.as_str() {
            "local_extract_text" => {
                let asset = self
                    .raw
                    .list_assets_for_revision(&revision.id)?
                    .into_iter()
                    .find(is_text_asset);
                let asset = asset.ok_or_else(|| {
                    ApplicationError::Integrity(format!(
                        "revision {} has no UTF-8 text-like C0 asset for local extraction",
                        revision.id
                    ))
                })?;
                let logical_path = LogicalPath::parse(asset.logical_path.clone())
                    .map_err(ApplicationError::from)?;
                let bytes = self.assets.open(&logical_path)?;
                std::str::from_utf8(&bytes).map_err(|_| {
                    ApplicationError::Integrity(format!(
                        "asset {} is not valid UTF-8 text",
                        asset.id
                    ))
                })?;
                if Sha256::of_bytes(&bytes) != asset.sha256 {
                    return Err(ApplicationError::Integrity(format!(
                        "asset {} bytes no longer match C0 hash {}",
                        asset.id, asset.sha256
                    )));
                }
                Ok((asset.sha256, Some(asset.id)))
            }
            "bailian_summary" => {
                let text = revision.raw_text.as_deref().ok_or_else(|| {
                    ApplicationError::Integrity(format!(
                        "revision {} has no C0 text for Bailian summary",
                        revision.id
                    ))
                })?;
                if text.trim().is_empty() {
                    return Err(ApplicationError::Integrity(
                        "Bailian summary input text is empty".to_owned(),
                    ));
                }
                let hash = revision.text_sha256.clone().ok_or_else(|| {
                    ApplicationError::Integrity("C0 text has no recorded hash".to_owned())
                })?;
                if Sha256::of_bytes(text.as_bytes()) != hash {
                    return Err(ApplicationError::Integrity(
                        "C0 text bytes do not match the recorded hash".to_owned(),
                    ));
                }
                Ok((hash, None))
            }
            other => Err(ApplicationError::capability_unavailable(
                format!("processing.{other}"),
                "P5+",
            )),
        }
    }

    fn execution_request(
        &self,
        job: &ProcessJob,
    ) -> Result<ProviderExecutionRequest, ApplicationError> {
        let input_text = if let Some(asset_id) = &job.input_asset_id {
            let asset = self.asset_for_revision(asset_id, &job.input_revision_id)?;
            if asset.sha256 != job.input_sha256 {
                return Err(ApplicationError::Integrity(
                    "queued asset hash no longer matches C0".to_owned(),
                ));
            }
            let path = LogicalPath::parse(asset.logical_path).map_err(ApplicationError::from)?;
            let bytes = self.assets.open(&path)?;
            if Sha256::of_bytes(&bytes) != job.input_sha256 {
                return Err(ApplicationError::Integrity(
                    "queued C0 asset bytes changed after enqueue".to_owned(),
                ));
            }
            String::from_utf8(bytes).map_err(|_| {
                ApplicationError::Integrity("queued C0 asset is not valid UTF-8".to_owned())
            })?
        } else {
            let revision = self
                .raw
                .find_revision(&job.input_revision_id)?
                .ok_or_else(|| {
                    ApplicationError::NotFound(format!("revision {}", job.input_revision_id))
                })?;
            let text = revision.raw_text.ok_or_else(|| {
                ApplicationError::Integrity("queued revision has no C0 text".to_owned())
            })?;
            if Sha256::of_bytes(text.as_bytes()) != job.input_sha256 {
                return Err(ApplicationError::Integrity(
                    "queued C0 text changed after enqueue".to_owned(),
                ));
            }
            text
        };
        Ok(ProviderExecutionRequest {
            job_id: job.id.clone(),
            pipeline_id: job.pipeline_id.clone(),
            revision_id: job.input_revision_id.clone(),
            input_sha256: job.input_sha256.clone(),
            input_text,
        })
    }

    fn retry_run_id(&self, job: &ProcessJob) -> Result<Option<RunId>, ApplicationError> {
        let Some(parent_id) = &job.retry_of_job_id else {
            return Ok(None);
        };
        let parent = self
            .jobs
            .get(parent_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("retry parent job {parent_id}")))?;
        parent
            .result_run_id
            .ok_or_else(|| {
                ApplicationError::Integrity(format!(
                    "failed retry parent job {parent_id} has no C1 failure run"
                ))
            })
            .map(Some)
    }

    fn ensure_failure_run(&self, parent: ProcessJob) -> Result<ProcessJob, ApplicationError> {
        if parent.result_run_id.is_some() {
            return Ok(parent);
        }
        let retry_of_run_id = if let Some(ancestor_id) = &parent.retry_of_job_id {
            let ancestor = self
                .jobs
                .get(ancestor_id)?
                .ok_or_else(|| ApplicationError::NotFound(format!("job {ancestor_id}")))?;
            self.ensure_failure_run(ancestor)?.result_run_id
        } else {
            None
        };
        let failure = self.register_failure(RegisterFailureCommand {
            pipeline_id: parent.pipeline_id.clone(),
            revision_id: parent.input_revision_id.clone(),
            item_id: parent.input_item_id.clone(),
            input_sha256: parent.input_sha256.clone(),
            kind: parent.target_kind,
            provider: parent.provider.clone(),
            tool_or_model: Some(parent.tool_or_model.clone()),
            tool_version: Some(parent.tool_version.clone()),
            retry_of_run_id,
            params: parent.params.clone(),
            error_code: parent
                .error_code
                .clone()
                .unwrap_or_else(|| "job_failed".to_owned()),
            error_message: parent.error_message.clone(),
            loss_notes: Some(
                "Runtime failure occurred before a C1 derivative was committed.".to_owned(),
            ),
            input_asset_id: parent.input_asset_id.clone(),
        })?;
        let worker_id = parent.worker_id.as_deref().ok_or_else(|| {
            ApplicationError::Integrity(format!(
                "failed job {} has no worker identity for C1 failure reconciliation",
                parent.id
            ))
        })?;
        self.jobs.fail(
            &parent.id,
            worker_id,
            parent.error_code.as_deref().unwrap_or("job_failed"),
            parent
                .error_message
                .as_deref()
                .unwrap_or("process job failed"),
            Some(&failure.run_id),
            parent.provider_task.as_ref(),
            &self.clock.now(),
        )
    }

    fn reconcile_success(&self, job: ProcessJob) -> Result<ProcessJob, ApplicationError> {
        if job.result_run_id.is_some()
            || !matches!(
                job.state,
                ProcessJobState::Running | ProcessJobState::Failed
            )
        {
            return Ok(job);
        }
        let Some(worker_id) = job.worker_id.as_deref() else {
            return Ok(job);
        };
        let matching = self
            .repository
            .list_runs_for_revision(&job.input_revision_id)?
            .into_iter()
            .find_map(|run| queued_task_for_run(&run, &job.id).map(|task| (run, task)));
        let Some((run, task)) = matching else {
            return Ok(job);
        };
        self.jobs
            .complete(&job.id, worker_id, &run.id, &task, &self.clock.now())
    }

    fn fail_job(
        &self,
        job: &ProcessJob,
        worker_id: &str,
        task: Option<&babata_domain::ProviderTaskRef>,
        error: ApplicationError,
    ) -> Result<ProcessJobOutcome, ApplicationError> {
        let current = self
            .jobs
            .get(&job.id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("job {}", job.id)))?;
        if current.state == ProcessJobState::Cancelled || current.cancel_requested {
            return Ok(job_outcome(current));
        }
        let message: String = error.to_string().chars().take(1000).collect();
        let params = task
            .and_then(|task| execution_trace_params(job.params.clone(), job, task).ok())
            .unwrap_or_else(|| job.params.clone());
        let failure = RegisterFailureCommand {
            pipeline_id: job.pipeline_id.clone(),
            revision_id: job.input_revision_id.clone(),
            item_id: job.input_item_id.clone(),
            input_sha256: job.input_sha256.clone(),
            kind: job.target_kind,
            provider: job.provider.clone(),
            tool_or_model: Some(job.tool_or_model.clone()),
            tool_version: Some(job.tool_version.clone()),
            retry_of_run_id: self.retry_run_id(job)?,
            params,
            error_code: error.code().to_owned(),
            error_message: Some(message.clone()),
            loss_notes: Some("Processing failed before a C1 derivative was committed.".to_owned()),
            input_asset_id: job.input_asset_id.clone(),
        };
        let run_id = self
            .register_failure(failure)
            .ok()
            .map(|outcome| outcome.run_id);
        let failed = self.jobs.fail(
            &job.id,
            worker_id,
            error.code(),
            &message,
            run_id.as_ref(),
            task,
            &self.clock.now(),
        )?;
        Ok(job_outcome(failed))
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

fn job_outcome(job: ProcessJob) -> ProcessJobOutcome {
    let status = match job.state {
        ProcessJobState::Queued => "queued",
        ProcessJobState::Running => "running",
        ProcessJobState::Succeeded => "succeeded",
        ProcessJobState::Failed => "failed",
        ProcessJobState::Cancelled => "cancelled",
    };
    ProcessJobOutcome {
        status: status.to_owned(),
        job: Some(job),
    }
}

fn is_text_asset(asset: &NewAsset) -> bool {
    asset.media_type.as_deref().is_some_and(|media_type| {
        media_type.starts_with("text/")
            || matches!(
                media_type,
                "application/json"
                    | "application/xml"
                    | "application/xhtml+xml"
                    | "application/yaml"
            )
    })
}

fn validate_provider_outcome(
    job: &ProcessJob,
    outcome: &ProviderExecutionOutcome,
) -> Result<(), ApplicationError> {
    if outcome.kind != job.target_kind
        || outcome.provider != job.provider
        || outcome.tool_or_model != job.tool_or_model
        || outcome.tool_version != job.tool_version
        || outcome.task.provider != job.provider
    {
        return Err(ApplicationError::Integrity(
            "provider outcome identity does not match the queued job".to_owned(),
        ));
    }
    Ok(())
}

fn execution_trace_params(
    params: Metadata,
    job: &ProcessJob,
    task: &babata_domain::ProviderTaskRef,
) -> Result<Metadata, ApplicationError> {
    let mut value: serde_json::Value =
        serde_json::from_str(&params.to_json()).map_err(|error| {
            ApplicationError::Integrity(format!("provider params are invalid JSON: {error}"))
        })?;
    let object = value.as_object_mut().ok_or_else(|| {
        ApplicationError::Integrity("provider params must be a JSON object".to_owned())
    })?;
    object.insert(
        "queue_job_id".to_owned(),
        serde_json::Value::String(job.id.to_string()),
    );
    object.insert(
        "queue_attempt".to_owned(),
        serde_json::Value::from(job.attempt),
    );
    object.insert(
        "provider_task_provider".to_owned(),
        serde_json::Value::String(task.provider.clone()),
    );
    object.insert(
        "provider_task_id".to_owned(),
        serde_json::Value::String(task.task_id.clone()),
    );
    Metadata::parse(&value.to_string()).map_err(ApplicationError::from)
}

fn queued_task_for_run(run: &ProcessRun, job_id: &JobId) -> Option<babata_domain::ProviderTaskRef> {
    if run.state != ProcessingState::Succeeded || run.invalidated_at.is_some() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(&run.params.to_json()).ok()?;
    if value.get("queue_job_id")?.as_str()? != job_id.to_string() {
        return None;
    }
    Some(babata_domain::ProviderTaskRef {
        provider: value.get("provider_task_provider")?.as_str()?.to_owned(),
        task_id: value.get("provider_task_id")?.as_str()?.to_owned(),
    })
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

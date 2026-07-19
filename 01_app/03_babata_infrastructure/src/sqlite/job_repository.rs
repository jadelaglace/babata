use std::sync::{Arc, Mutex};

use babata_application::{ApplicationError, ports::JobRepositoryPort};
use babata_domain::{
    AssetId, DerivativeKind, ItemId, JobId, Metadata, PipelineId, ProcessJob, ProcessJobState,
    ProviderTaskRef, RevisionId, RunId, Sha256, UtcTimestamp,
};
use rusqlite::{Connection, OptionalExtension, params};

const MIGRATIONS: &[(&str, &str)] = &[(
    "0001_process_jobs.sql",
    include_str!("../../../../03_migrations/03_runtime/0001_process_jobs.sql"),
)];

const JOB_COLUMNS: &str = "job_id, pipeline_id, input_revision_id, input_item_id, input_sha256,
    target_kind, input_asset_id, state, provider, tool_or_model, tool_version,
    attempt, retry_of_job_id, worker_id,
    lease_expires_at, provider_task_provider, provider_task_id, error_code, error_message,
    result_run_id, cancel_requested, params_json, created_at, started_at, heartbeat_at, finished_at";

#[derive(Clone)]
pub struct SqliteJobRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteJobRepository {
    pub(crate) fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ApplicationError> {
        self.connection
            .lock()
            .map_err(|_| ApplicationError::Storage("SQLite connection lock poisoned".to_owned()))
    }
}

pub(crate) fn migrate_runtime(connection: &Connection) -> Result<(), ApplicationError> {
    let migration_table_exists = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = 'runtime_schema_migrations'
            )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(storage)?;
    let existing_version = if migration_table_exists {
        connection
            .query_row(
                "SELECT MAX(version) FROM runtime_schema_migrations",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage)?
    } else {
        None
    };
    if existing_version.is_some_and(|version| version > MIGRATIONS.len() as i64) {
        return Err(ApplicationError::Integrity(format!(
            "runtime schema version {} is newer than this binary supports ({})",
            existing_version.unwrap_or_default(),
            MIGRATIONS.len()
        )));
    }

    for (index, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        let checksum = super::migration_checksum(sql);
        let existing = if existing_version.is_some() {
            connection
                .query_row(
                    "SELECT checksum_sha256 FROM runtime_schema_migrations WHERE version = ?1",
                    params![version],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(storage)?
        } else {
            None
        };
        if let Some(existing) = existing {
            if !super::migration_checksum_matches(&existing, sql) {
                return Err(ApplicationError::Integrity(format!(
                    "runtime migration checksum changed: {name}"
                )));
            }
            continue;
        }
        let transaction = connection.unchecked_transaction().map_err(storage)?;
        transaction.execute_batch(sql).map_err(storage)?;
        transaction
            .execute(
                "INSERT INTO runtime_schema_migrations
                 (version, name, applied_at, checksum_sha256)
                 VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?3)",
                params![version, name, checksum],
            )
            .map_err(storage)?;
        transaction.commit().map_err(storage)?;
    }
    Ok(())
}

impl JobRepositoryPort for SqliteJobRepository {
    fn enqueue(&self, job: &ProcessJob) -> Result<(), ApplicationError> {
        let connection = self.lock()?;
        insert_job(&connection, job)
    }

    fn get(&self, job_id: &JobId) -> Result<Option<ProcessJob>, ApplicationError> {
        let connection = self.lock()?;
        load_job(&connection, job_id)
    }

    fn claim(
        &self,
        worker_id: &str,
        at: &UtcTimestamp,
        lease_seconds: u32,
    ) -> Result<Option<ProcessJob>, ApplicationError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        transaction
            .execute(
                "UPDATE process_jobs
                 SET state = 'failed', error_code = 'lease_expired',
                     error_message = 'worker lease expired before completion', finished_at = ?1
                 WHERE state = 'running'
                   AND julianday(lease_expires_at) < julianday(?1)",
                params![at.as_str()],
            )
            .map_err(storage)?;
        let next = transaction
            .query_row(
                "SELECT job_id FROM process_jobs
                 WHERE state = 'queued' AND cancel_requested = 0
                 ORDER BY created_at ASC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage)?;
        let Some(next) = next else {
            transaction.commit().map_err(storage)?;
            return Ok(None);
        };
        let job_id = JobId::parse(next).map_err(ApplicationError::from)?;
        let changed = transaction
            .execute(
                "UPDATE process_jobs
                 SET state = 'running', worker_id = ?2,
                     lease_expires_at = strftime('%Y-%m-%dT%H:%M:%fZ', julianday(?3) + (?4 / 86400.0)),
                     started_at = COALESCE(started_at, ?3), heartbeat_at = ?3
                 WHERE job_id = ?1 AND state = 'queued' AND cancel_requested = 0",
                params![
                    job_id.to_string(),
                    worker_id,
                    at.as_str(),
                    i64::from(lease_seconds)
                ],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(ApplicationError::Conflict(format!(
                "job {job_id} could not be claimed"
            )));
        }
        let job = load_job(&transaction, &job_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("job {job_id} after claim")))?;
        transaction.commit().map_err(storage)?;
        Ok(Some(job))
    }

    fn heartbeat(
        &self,
        job_id: &JobId,
        worker_id: &str,
        at: &UtcTimestamp,
        lease_seconds: u32,
    ) -> Result<ProcessJob, ApplicationError> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE process_jobs
                 SET heartbeat_at = ?3,
                     lease_expires_at = strftime('%Y-%m-%dT%H:%M:%fZ', julianday(?3) + (?4 / 86400.0))
                 WHERE job_id = ?1 AND state = 'running' AND worker_id = ?2",
                params![
                    job_id.to_string(),
                    worker_id,
                    at.as_str(),
                    i64::from(lease_seconds)
                ],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(ApplicationError::Conflict(format!(
                "job {job_id} heartbeat owner/state mismatch"
            )));
        }
        load_job(&connection, job_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("job {job_id}")))
    }

    fn complete(
        &self,
        job_id: &JobId,
        worker_id: &str,
        run_id: &RunId,
        task: &ProviderTaskRef,
        at: &UtcTimestamp,
    ) -> Result<ProcessJob, ApplicationError> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE process_jobs
                 SET state = 'succeeded', result_run_id = ?3,
                     provider_task_provider = ?4, provider_task_id = ?5,
                     lease_expires_at = NULL, heartbeat_at = ?6, finished_at = ?6
                 WHERE job_id = ?1 AND worker_id = ?2 AND cancel_requested = 0
                   AND (
                       state = 'running'
                       OR (state = 'failed' AND result_run_id IS NULL)
                   )",
                params![
                    job_id.to_string(),
                    worker_id,
                    run_id.to_string(),
                    task.provider,
                    task.task_id,
                    at.as_str()
                ],
            )
            .map_err(storage)?;
        transition_result(&connection, job_id, changed, "complete")
    }

    fn fail(
        &self,
        job_id: &JobId,
        worker_id: &str,
        error_code: &str,
        error_message: &str,
        run_id: Option<&RunId>,
        task: Option<&ProviderTaskRef>,
        at: &UtcTimestamp,
    ) -> Result<ProcessJob, ApplicationError> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE process_jobs
                 SET state = 'failed', error_code = ?3, error_message = ?4,
                     result_run_id = ?5, provider_task_provider = ?6, provider_task_id = ?7,
                     lease_expires_at = NULL, heartbeat_at = ?8, finished_at = ?8
                 WHERE job_id = ?1 AND worker_id = ?2
                   AND (
                       state = 'running'
                       OR (state = 'failed' AND result_run_id IS NULL AND ?5 IS NOT NULL)
                   )",
                params![
                    job_id.to_string(),
                    worker_id,
                    error_code,
                    error_message,
                    run_id.map(ToString::to_string),
                    task.map(|task| task.provider.clone()),
                    task.map(|task| task.task_id.clone()),
                    at.as_str()
                ],
            )
            .map_err(storage)?;
        transition_result(&connection, job_id, changed, "fail")
    }

    fn retry(&self, parent_id: &JobId, retry: &ProcessJob) -> Result<(), ApplicationError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        let parent = load_job(&transaction, parent_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("job {parent_id}")))?;
        if parent.state != ProcessJobState::Failed {
            return Err(ApplicationError::Conflict(format!(
                "job {parent_id} is not failed"
            )));
        }
        if retry.retry_of_job_id.as_ref() != Some(parent_id)
            || retry.attempt != parent.attempt.saturating_add(1)
            || retry.pipeline_id != parent.pipeline_id
            || retry.input_revision_id != parent.input_revision_id
            || retry.input_item_id != parent.input_item_id
            || retry.input_sha256 != parent.input_sha256
            || retry.target_kind != parent.target_kind
            || retry.input_asset_id != parent.input_asset_id
            || retry.provider != parent.provider
            || retry.tool_or_model != parent.tool_or_model
            || retry.tool_version != parent.tool_version
        {
            return Err(ApplicationError::Integrity(
                "retry job identity does not match failed parent".to_owned(),
            ));
        }
        insert_job(&transaction, retry)?;
        transaction.commit().map_err(storage)
    }

    fn cancel(&self, job_id: &JobId, at: &UtcTimestamp) -> Result<ProcessJob, ApplicationError> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE process_jobs
                 SET state = 'cancelled', cancel_requested = 1,
                     lease_expires_at = NULL, finished_at = ?2
                 WHERE job_id = ?1 AND state IN ('queued', 'running')",
                params![job_id.to_string(), at.as_str()],
            )
            .map_err(storage)?;
        transition_result(&connection, job_id, changed, "cancel")
    }
}

fn transition_result(
    connection: &Connection,
    job_id: &JobId,
    changed: usize,
    transition: &str,
) -> Result<ProcessJob, ApplicationError> {
    let job = load_job(connection, job_id)?
        .ok_or_else(|| ApplicationError::NotFound(format!("job {job_id}")))?;
    if changed == 0 && job.state != ProcessJobState::Cancelled {
        return Err(ApplicationError::Conflict(format!(
            "job {job_id} cannot {transition} from {:?}",
            job.state
        )));
    }
    Ok(job)
}

fn insert_job(connection: &Connection, job: &ProcessJob) -> Result<(), ApplicationError> {
    connection
        .execute(
            "INSERT INTO process_jobs (
                job_id, pipeline_id, input_revision_id, input_item_id, input_sha256,
                target_kind, input_asset_id, state, provider, tool_or_model, tool_version,
                attempt, retry_of_job_id,
                worker_id, lease_expires_at, provider_task_provider, provider_task_id,
                error_code, error_message, result_run_id, cancel_requested, params_json,
                created_at, started_at, heartbeat_at, finished_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
            params![
                job.id.to_string(),
                job.pipeline_id.as_str(),
                job.input_revision_id.to_string(),
                job.input_item_id.as_ref().map(ToString::to_string),
                job.input_sha256.as_str(),
                derivative_kind(job.target_kind),
                job.input_asset_id.as_ref().map(ToString::to_string),
                job_state(job.state),
                job.provider,
                job.tool_or_model,
                job.tool_version,
                i64::from(job.attempt),
                job.retry_of_job_id.as_ref().map(ToString::to_string),
                job.worker_id,
                job.lease_expires_at.as_ref().map(|value| value.as_str().to_owned()),
                job.provider_task.as_ref().map(|task| task.provider.clone()),
                job.provider_task.as_ref().map(|task| task.task_id.clone()),
                job.error_code,
                job.error_message,
                job.result_run_id.as_ref().map(ToString::to_string),
                i64::from(job.cancel_requested),
                job.params.to_json(),
                job.created_at.as_str(),
                job.started_at.as_ref().map(|value| value.as_str().to_owned()),
                job.heartbeat_at.as_ref().map(|value| value.as_str().to_owned()),
                job.finished_at.as_ref().map(|value| value.as_str().to_owned()),
            ],
        )
        .map_err(storage)?;
    Ok(())
}

fn load_job(
    connection: &Connection,
    job_id: &JobId,
) -> Result<Option<ProcessJob>, ApplicationError> {
    connection
        .query_row(
            &format!("SELECT {JOB_COLUMNS} FROM process_jobs WHERE job_id = ?1"),
            params![job_id.to_string()],
            job_from_row,
        )
        .optional()
        .map_err(storage)
}

fn job_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProcessJob> {
    let task_provider = row.get::<_, Option<String>>(15)?;
    let task_id = row.get::<_, Option<String>>(16)?;
    let provider_task = match (task_provider, task_id) {
        (Some(provider), Some(task_id)) => Some(ProviderTaskRef { provider, task_id }),
        (None, None) => None,
        _ => return Err(to_sql("provider task identity is incomplete")),
    };
    Ok(ProcessJob {
        id: JobId::parse(row.get::<_, String>(0)?).map_err(to_sql)?,
        pipeline_id: PipelineId::new(row.get::<_, String>(1)?),
        input_revision_id: RevisionId::parse(row.get::<_, String>(2)?).map_err(to_sql)?,
        input_item_id: row
            .get::<_, Option<String>>(3)?
            .map(ItemId::parse)
            .transpose()
            .map_err(to_sql)?,
        input_sha256: Sha256::parse(row.get::<_, String>(4)?).map_err(to_sql)?,
        target_kind: parse_derivative_kind(&row.get::<_, String>(5)?).map_err(to_sql)?,
        input_asset_id: row
            .get::<_, Option<String>>(6)?
            .map(AssetId::parse)
            .transpose()
            .map_err(to_sql)?,
        state: parse_job_state(&row.get::<_, String>(7)?).map_err(to_sql)?,
        provider: row.get(8)?,
        tool_or_model: row.get(9)?,
        tool_version: row.get(10)?,
        attempt: u32::try_from(row.get::<_, i64>(11)?).map_err(to_sql)?,
        retry_of_job_id: row
            .get::<_, Option<String>>(12)?
            .map(JobId::parse)
            .transpose()
            .map_err(to_sql)?,
        worker_id: row.get(13)?,
        lease_expires_at: row
            .get::<_, Option<String>>(14)?
            .map(UtcTimestamp::parse)
            .transpose()
            .map_err(to_sql)?,
        provider_task,
        error_code: row.get(17)?,
        error_message: row.get(18)?,
        result_run_id: row
            .get::<_, Option<String>>(19)?
            .map(RunId::parse)
            .transpose()
            .map_err(to_sql)?,
        cancel_requested: row.get::<_, i64>(20)? != 0,
        params: Metadata::parse(&row.get::<_, String>(21)?).map_err(to_sql)?,
        created_at: UtcTimestamp::parse(row.get::<_, String>(22)?).map_err(to_sql)?,
        started_at: row
            .get::<_, Option<String>>(23)?
            .map(UtcTimestamp::parse)
            .transpose()
            .map_err(to_sql)?,
        heartbeat_at: row
            .get::<_, Option<String>>(24)?
            .map(UtcTimestamp::parse)
            .transpose()
            .map_err(to_sql)?,
        finished_at: row
            .get::<_, Option<String>>(25)?
            .map(UtcTimestamp::parse)
            .transpose()
            .map_err(to_sql)?,
    })
}

fn job_state(state: ProcessJobState) -> &'static str {
    match state {
        ProcessJobState::Queued => "queued",
        ProcessJobState::Running => "running",
        ProcessJobState::Succeeded => "succeeded",
        ProcessJobState::Failed => "failed",
        ProcessJobState::Cancelled => "cancelled",
    }
}

fn parse_job_state(value: &str) -> Result<ProcessJobState, ApplicationError> {
    match value {
        "queued" => Ok(ProcessJobState::Queued),
        "running" => Ok(ProcessJobState::Running),
        "succeeded" => Ok(ProcessJobState::Succeeded),
        "failed" => Ok(ProcessJobState::Failed),
        "cancelled" => Ok(ProcessJobState::Cancelled),
        other => Err(ApplicationError::Integrity(format!(
            "unknown process job state: {other}"
        ))),
    }
}

fn derivative_kind(kind: DerivativeKind) -> &'static str {
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

fn parse_derivative_kind(value: &str) -> Result<DerivativeKind, ApplicationError> {
    match value {
        "extracted_text" => Ok(DerivativeKind::ExtractedText),
        "ocr_text" => Ok(DerivativeKind::OcrText),
        "transcript" => Ok(DerivativeKind::Transcript),
        "subtitle" => Ok(DerivativeKind::Subtitle),
        "summary" => Ok(DerivativeKind::Summary),
        "visual_description" => Ok(DerivativeKind::VisualDescription),
        "key_frame" => Ok(DerivativeKind::KeyFrame),
        "tags" => Ok(DerivativeKind::Tags),
        "structured_result" => Ok(DerivativeKind::StructuredResult),
        "media_metadata" => Ok(DerivativeKind::MediaMetadata),
        other => Err(ApplicationError::Integrity(format!(
            "unknown derivative kind: {other}"
        ))),
    }
}

fn storage(error: rusqlite::Error) -> ApplicationError {
    ApplicationError::Storage(error.to_string())
}

fn to_sql(error: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        error.to_string(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(attempt: u32, retry_of_job_id: Option<JobId>) -> ProcessJob {
        ProcessJob {
            id: JobId::new(),
            pipeline_id: PipelineId::new("local_extract_text"),
            input_revision_id: RevisionId::new(),
            input_item_id: None,
            input_sha256: Sha256::of_bytes(b"input"),
            target_kind: DerivativeKind::ExtractedText,
            input_asset_id: None,
            state: ProcessJobState::Queued,
            provider: "local_extract".to_owned(),
            tool_or_model: "identity_text_extract".to_owned(),
            tool_version: "0.1.0".to_owned(),
            attempt,
            retry_of_job_id,
            worker_id: None,
            lease_expires_at: None,
            provider_task: None,
            error_code: None,
            error_message: None,
            result_run_id: None,
            cancel_requested: false,
            params: Metadata::empty(),
            created_at: UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap(),
            started_at: None,
            heartbeat_at: None,
            finished_at: None,
        }
    }

    fn repository() -> SqliteJobRepository {
        let connection = Connection::open_in_memory().unwrap();
        migrate_runtime(&connection).unwrap();
        SqliteJobRepository::new(Arc::new(Mutex::new(connection)))
    }

    #[test]
    fn runtime_migration_is_idempotent_and_changed_history_fails_closed() {
        let connection = Connection::open_in_memory().unwrap();
        migrate_runtime(&connection).unwrap();
        migrate_runtime(&connection).unwrap();
        connection
            .execute(
                "UPDATE runtime_schema_migrations SET checksum_sha256 = 'changed' WHERE version = 1",
                [],
            )
            .unwrap();
        assert!(matches!(
            migrate_runtime(&connection),
            Err(ApplicationError::Integrity(_))
        ));
    }

    #[test]
    fn runtime_schema_newer_than_the_binary_is_rejected() {
        let connection = Connection::open_in_memory().unwrap();
        migrate_runtime(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO runtime_schema_migrations
                 (version, name, applied_at, checksum_sha256)
                 VALUES (2, 'future.sql', '2026-01-01T00:00:00Z', 'future')",
                [],
            )
            .unwrap();
        assert!(matches!(
            migrate_runtime(&connection),
            Err(ApplicationError::Integrity(_))
        ));
    }

    #[test]
    fn queue_claim_fail_retry_and_cancel_preserve_attempts() {
        let repository = repository();
        let first = job(1, None);
        repository.enqueue(&first).unwrap();
        let at = UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap();
        let claimed = repository.claim("worker-1", &at, 60).unwrap().unwrap();
        assert_eq!(claimed.id, first.id);
        assert_eq!(claimed.state, ProcessJobState::Running);
        let failed = repository
            .fail(
                &first.id,
                "worker-1",
                "provider_failed",
                "intentional",
                None,
                None,
                &at,
            )
            .unwrap();
        assert_eq!(failed.state, ProcessJobState::Failed);

        let retry = job(2, Some(first.id.clone()));
        let retry = ProcessJob {
            pipeline_id: first.pipeline_id.clone(),
            input_revision_id: first.input_revision_id.clone(),
            input_sha256: first.input_sha256.clone(),
            target_kind: first.target_kind,
            provider: first.provider.clone(),
            ..retry
        };
        repository.retry(&first.id, &retry).unwrap();
        assert_eq!(repository.get(&first.id).unwrap().unwrap().attempt, 1);
        assert_eq!(repository.get(&retry.id).unwrap().unwrap().attempt, 2);
        let cancelled = repository.cancel(&retry.id, &at).unwrap();
        assert_eq!(cancelled.state, ProcessJobState::Cancelled);
    }

    #[test]
    fn worker_ownership_is_enforced_and_expired_leases_become_retryable_failures() {
        let repository = repository();
        let first = job(1, None);
        repository.enqueue(&first).unwrap();
        let started = UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap();
        repository.claim("worker-1", &started, 1).unwrap().unwrap();

        let wrong_owner = repository
            .heartbeat(&first.id, "worker-2", &started, 30)
            .unwrap_err();
        assert!(matches!(wrong_owner, ApplicationError::Conflict(_)));

        let after_expiry = UtcTimestamp::parse("2026-01-01T00:00:02Z").unwrap();
        assert!(
            repository
                .claim("worker-2", &after_expiry, 30)
                .unwrap()
                .is_none()
        );
        let expired = repository.get(&first.id).unwrap().unwrap();
        assert_eq!(expired.state, ProcessJobState::Failed);
        assert_eq!(expired.error_code.as_deref(), Some("lease_expired"));
        let failure_run = RunId::new();
        let reconciled = repository
            .fail(
                &first.id,
                "worker-1",
                "lease_expired",
                "worker lease expired before completion",
                Some(&failure_run),
                None,
                &after_expiry,
            )
            .unwrap();
        assert_eq!(reconciled.result_run_id.as_ref(), Some(&failure_run));

        let retry = ProcessJob {
            id: JobId::new(),
            state: ProcessJobState::Queued,
            attempt: 2,
            retry_of_job_id: Some(first.id.clone()),
            worker_id: None,
            lease_expires_at: None,
            provider_task: None,
            error_code: None,
            error_message: None,
            result_run_id: None,
            cancel_requested: false,
            created_at: after_expiry.clone(),
            started_at: None,
            heartbeat_at: None,
            finished_at: None,
            ..first.clone()
        };
        repository.retry(&first.id, &retry).unwrap();
        assert_eq!(repository.get(&retry.id).unwrap().unwrap().attempt, 2);

        let interrupted = job(1, None);
        repository.enqueue(&interrupted).unwrap();
        repository.claim("worker-3", &started, 1).unwrap().unwrap();
        repository.claim("worker-4", &after_expiry, 30).unwrap();
        let recovered_run = RunId::new();
        let task = ProviderTaskRef {
            provider: "local_extract".to_owned(),
            task_id: "local:recovered".to_owned(),
        };
        let recovered = repository
            .complete(
                &interrupted.id,
                "worker-3",
                &recovered_run,
                &task,
                &after_expiry,
            )
            .unwrap();
        assert_eq!(recovered.state, ProcessJobState::Succeeded);
        assert_eq!(recovered.result_run_id.as_ref(), Some(&recovered_run));
    }
}

use std::sync::{Arc, Mutex};

use babata_application::{
    ApplicationError,
    ports::{DerivedRepositoryPort, ProcessCommit},
};
use babata_domain::{
    AssetId, DerivativeId, DerivativeKind, DerivativeRef, ItemId, LogicalPath, Metadata,
    PipelineId, ProcessRun, ProcessingState, RevisionId, RunId, Sha256, UtcTimestamp,
};
use rusqlite::{Connection, OptionalExtension, params};

#[derive(Clone)]
pub struct SqliteDerivedRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteDerivedRepository {
    pub(crate) fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ApplicationError> {
        self.connection
            .lock()
            .map_err(|_| ApplicationError::Storage("SQLite connection lock poisoned".to_owned()))
    }

    #[cfg(feature = "test-support")]
    fn commit_fault() -> bool {
        std::env::var("BABATA_TEST_DERIVED_FAULT").is_ok_and(|value| value == "commit")
    }

    #[cfg(not(feature = "test-support"))]
    fn commit_fault() -> bool {
        false
    }
}

impl DerivedRepositoryPort for SqliteDerivedRepository {
    fn create_run(&self, run: &ProcessRun) -> Result<(), ApplicationError> {
        let connection = self.lock()?;
        insert_run(&connection, run)
    }

    fn update_run(&self, run: &ProcessRun) -> Result<(), ApplicationError> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE process_runs SET
                    pipeline_id = ?2,
                    input_revision_id = ?3,
                    input_item_id = ?4,
                    input_sha256 = ?5,
                    target_kind = ?6,
                    input_asset_id = ?7,
                    state = ?8,
                    provider = ?9,
                    tool_or_model = ?10,
                    tool_version = ?11,
                    attempt = ?12,
                    retry_of_run_id = ?13,
                    error_code = ?14,
                    error_message = ?15,
                    params_json = ?16,
                    usage_json = ?17,
                    loss_notes = ?18,
                    started_at = ?19,
                    finished_at = ?20
                 WHERE run_id = ?1",
                params![
                    run.id.to_string(),
                    run.pipeline_id.as_str(),
                    run.input_revision_id.to_string(),
                    run.input_item_id.as_ref().map(ToString::to_string),
                    run.input_sha256.as_str(),
                    run.target_kind.map(derivative_kind),
                    run.input_asset_id.as_ref().map(ToString::to_string),
                    processing_state(run.state),
                    run.provider,
                    run.tool_or_model,
                    run.tool_version,
                    i64::from(run.attempt),
                    run.retry_of_run_id.as_ref().map(ToString::to_string),
                    run.error_code,
                    run.error_message,
                    run.params.to_json(),
                    run.usage.to_json(),
                    run.loss_notes,
                    run.started_at.as_ref().map(|t| t.as_str().to_owned()),
                    run.finished_at.as_ref().map(|t| t.as_str().to_owned()),
                ],
            )
            .map_err(storage)?;
        if changed == 0 {
            return Err(ApplicationError::NotFound(format!("run {}", run.id)));
        }
        Ok(())
    }

    fn get_run(&self, run_id: &RunId) -> Result<Option<ProcessRun>, ApplicationError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT run_id, pipeline_id, input_revision_id, input_item_id, input_sha256,
                        target_kind, input_asset_id, state,
                        provider, tool_or_model, tool_version, attempt, retry_of_run_id,
                        error_code, error_message, params_json, usage_json, loss_notes,
                        created_at, started_at, finished_at
                 FROM process_runs WHERE run_id = ?1",
                params![run_id.to_string()],
                run_from_row,
            )
            .optional()
            .map_err(storage)
    }

    fn list_runs_for_revision(
        &self,
        revision_id: &RevisionId,
    ) -> Result<Vec<ProcessRun>, ApplicationError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT run_id, pipeline_id, input_revision_id, input_item_id, input_sha256,
                        target_kind, input_asset_id, state,
                        provider, tool_or_model, tool_version, attempt, retry_of_run_id,
                        error_code, error_message, params_json, usage_json, loss_notes,
                        created_at, started_at, finished_at
                 FROM process_runs
                 WHERE input_revision_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(params![revision_id.to_string()], run_from_row)
            .map_err(storage)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(storage)?);
        }
        Ok(out)
    }

    fn add_derivative(&self, derivative: &DerivativeRef) -> Result<(), ApplicationError> {
        let connection = self.lock()?;
        insert_derivative(&connection, derivative)
    }

    fn get_derivative(
        &self,
        derivative_id: &DerivativeId,
    ) -> Result<Option<DerivativeRef>, ApplicationError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT derivative_id, run_id, kind, output_sha256, content_text, content_json,
                        logical_path, media_type, language, input_asset_id, loss_notes,
                        metadata_json, created_at
                 FROM derivatives WHERE derivative_id = ?1",
                params![derivative_id.to_string()],
                derivative_from_row,
            )
            .optional()
            .map_err(storage)
    }

    fn list_derivatives(&self, run_id: &RunId) -> Result<Vec<DerivativeRef>, ApplicationError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT derivative_id, run_id, kind, output_sha256, content_text, content_json,
                        logical_path, media_type, language, input_asset_id, loss_notes,
                        metadata_json, created_at
                 FROM derivatives WHERE run_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(params![run_id.to_string()], derivative_from_row)
            .map_err(storage)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(storage)?);
        }
        Ok(out)
    }

    fn commit_run(&self, commit: &ProcessCommit) -> Result<(), ApplicationError> {
        if Self::commit_fault() {
            return Err(ApplicationError::Storage(
                "injected derived commit failure".to_owned(),
            ));
        }
        let mut connection = self.lock()?;
        let transaction = connection.transaction().map_err(storage)?;
        insert_run(&transaction, &commit.run)?;
        for derivative in &commit.derivatives {
            insert_derivative(&transaction, derivative)?;
        }
        transaction.commit().map_err(storage)?;
        Ok(())
    }
}

fn insert_run(connection: &Connection, run: &ProcessRun) -> Result<(), ApplicationError> {
    connection
        .execute(
            "INSERT INTO process_runs (
                run_id, pipeline_id, input_revision_id, input_item_id, input_sha256,
                target_kind, input_asset_id, state,
                provider, tool_or_model, tool_version, attempt, retry_of_run_id,
                error_code, error_message, params_json, usage_json, loss_notes,
                created_at, started_at, finished_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
            params![
                run.id.to_string(),
                run.pipeline_id.as_str(),
                run.input_revision_id.to_string(),
                run.input_item_id.as_ref().map(ToString::to_string),
                run.input_sha256.as_str(),
                run.target_kind.map(derivative_kind),
                run.input_asset_id.as_ref().map(ToString::to_string),
                processing_state(run.state),
                run.provider,
                run.tool_or_model,
                run.tool_version,
                i64::from(run.attempt),
                run.retry_of_run_id.as_ref().map(ToString::to_string),
                run.error_code,
                run.error_message,
                run.params.to_json(),
                run.usage.to_json(),
                run.loss_notes,
                run.created_at.as_str(),
                run.started_at.as_ref().map(|t| t.as_str().to_owned()),
                run.finished_at.as_ref().map(|t| t.as_str().to_owned()),
            ],
        )
        .map_err(storage)?;
    Ok(())
}

fn insert_derivative(
    connection: &Connection,
    derivative: &DerivativeRef,
) -> Result<(), ApplicationError> {
    connection
        .execute(
            "INSERT INTO derivatives (
                derivative_id, run_id, kind, output_sha256, content_text, content_json,
                logical_path, media_type, language, input_asset_id, loss_notes,
                metadata_json, created_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                derivative.id.to_string(),
                derivative.run_id.to_string(),
                derivative_kind(derivative.kind),
                derivative
                    .output_sha256
                    .as_ref()
                    .map(|hash| hash.as_str().to_owned()),
                derivative.content_text,
                derivative.content_json,
                derivative
                    .logical_path
                    .as_ref()
                    .map(|path| path.as_str().to_owned()),
                derivative.media_type,
                derivative.language,
                derivative.input_asset_id.as_ref().map(ToString::to_string),
                derivative.loss_notes,
                derivative.metadata.to_json(),
                derivative.created_at.as_str(),
            ],
        )
        .map_err(storage)?;
    Ok(())
}

fn run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProcessRun> {
    Ok(ProcessRun {
        id: RunId::parse(row.get::<_, String>(0)?).map_err(to_sql)?,
        pipeline_id: PipelineId::new(row.get::<_, String>(1)?),
        input_revision_id: RevisionId::parse(row.get::<_, String>(2)?).map_err(to_sql)?,
        input_item_id: row
            .get::<_, Option<String>>(3)?
            .map(ItemId::parse)
            .transpose()
            .map_err(to_sql)?,
        input_sha256: Sha256::parse(row.get::<_, String>(4)?).map_err(to_sql)?,
        target_kind: row
            .get::<_, Option<String>>(5)?
            .map(|kind| parse_derivative_kind(&kind))
            .transpose()
            .map_err(to_sql)?,
        input_asset_id: row
            .get::<_, Option<String>>(6)?
            .map(AssetId::parse)
            .transpose()
            .map_err(to_sql)?,
        state: parse_processing_state(&row.get::<_, String>(7)?).map_err(to_sql)?,
        provider: row.get(8)?,
        tool_or_model: row.get(9)?,
        tool_version: row.get(10)?,
        attempt: row.get::<_, i64>(11)? as u32,
        retry_of_run_id: row
            .get::<_, Option<String>>(12)?
            .map(RunId::parse)
            .transpose()
            .map_err(to_sql)?,
        error_code: row.get(13)?,
        error_message: row.get(14)?,
        params: Metadata::parse(&row.get::<_, String>(15)?).map_err(to_sql)?,
        usage: Metadata::parse(&row.get::<_, String>(16)?).map_err(to_sql)?,
        loss_notes: row.get(17)?,
        created_at: UtcTimestamp::parse(row.get::<_, String>(18)?).map_err(to_sql)?,
        started_at: row
            .get::<_, Option<String>>(19)?
            .map(UtcTimestamp::parse)
            .transpose()
            .map_err(to_sql)?,
        finished_at: row
            .get::<_, Option<String>>(20)?
            .map(UtcTimestamp::parse)
            .transpose()
            .map_err(to_sql)?,
    })
}

fn derivative_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DerivativeRef> {
    Ok(DerivativeRef {
        id: DerivativeId::parse(row.get::<_, String>(0)?).map_err(to_sql)?,
        run_id: RunId::parse(row.get::<_, String>(1)?).map_err(to_sql)?,
        kind: parse_derivative_kind(&row.get::<_, String>(2)?).map_err(to_sql)?,
        output_sha256: row
            .get::<_, Option<String>>(3)?
            .map(Sha256::parse)
            .transpose()
            .map_err(to_sql)?,
        content_text: row.get(4)?,
        content_json: row.get(5)?,
        logical_path: row
            .get::<_, Option<String>>(6)?
            .map(LogicalPath::parse)
            .transpose()
            .map_err(to_sql)?,
        media_type: row.get(7)?,
        language: row.get(8)?,
        input_asset_id: row
            .get::<_, Option<String>>(9)?
            .map(AssetId::parse)
            .transpose()
            .map_err(to_sql)?,
        loss_notes: row.get(10)?,
        metadata: Metadata::parse(&row.get::<_, String>(11)?).map_err(to_sql)?,
        created_at: UtcTimestamp::parse(row.get::<_, String>(12)?).map_err(to_sql)?,
    })
}

fn processing_state(state: ProcessingState) -> &'static str {
    match state {
        ProcessingState::Pending => "pending",
        ProcessingState::Running => "running",
        ProcessingState::Succeeded => "succeeded",
        ProcessingState::Failed => "failed",
        ProcessingState::Cancelled => "cancelled",
    }
}

fn parse_processing_state(value: &str) -> Result<ProcessingState, ApplicationError> {
    match value {
        "pending" => Ok(ProcessingState::Pending),
        "running" => Ok(ProcessingState::Running),
        "succeeded" => Ok(ProcessingState::Succeeded),
        "failed" => Ok(ProcessingState::Failed),
        "cancelled" => Ok(ProcessingState::Cancelled),
        other => Err(ApplicationError::Integrity(format!(
            "unknown processing state: {other}"
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

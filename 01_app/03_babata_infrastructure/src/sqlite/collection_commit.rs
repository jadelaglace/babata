use super::SqliteRawRepository;
use babata_application::{
    ApplicationError, CollectionCommitOperation,
    ports::{CollectionRepositoryPort, collection_repository::CollectionWriteGuard},
};
use babata_domain::{CollectionItemStatus, CollectionSessionId, ItemId, RevisionId};
use fs2::FileExt;
use rusqlite::{OptionalExtension, params};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

struct WriteGuard {
    _file: Option<File>,
}
impl CollectionWriteGuard for WriteGuard {}

impl SqliteRawRepository {
    pub(super) fn acquire_collection_write_impl(
        &self,
    ) -> Result<Box<dyn CollectionWriteGuard>, ApplicationError> {
        let connection = self.lock()?;
        let file = connection.path().filter(|path| !path.is_empty()).map(|path| {
            let path = Path::new(path).with_extension("collector.lock");
            let file = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)
                .map_err(|error| ApplicationError::Storage(error.to_string()))?;
            file.try_lock_exclusive().map_err(|_| ApplicationError::Conflict("collector_busy: another collection writer owns this data root; query status and retry the same key".to_owned()))?;
            Ok::<_, ApplicationError>(file)
        }).transpose()?;
        Ok(Box::new(WriteGuard { _file: file }))
    }

    pub(super) fn bind_commit_operation_impl(
        &self,
        operation: &CollectionCommitOperation,
    ) -> Result<CollectionCommitOperation, ApplicationError> {
        let candidate_id = &operation.selection.candidate_ids[0];
        {
            let connection = self.lock()?;
            connection.execute(
                "INSERT INTO collection_commit_operations (request_key, input_sha256, session_id, candidate_id, operation_json)
                 VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(request_key) DO NOTHING",
                params![operation.request_key, operation.input_sha256.as_str(), operation.selection.session_id.to_string(), candidate_id, serde_json::to_string(operation).map_err(json_error)?],
            ).map_err(|error| if matches!(&error, rusqlite::Error::SqliteFailure(code, _) if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE) {
                ApplicationError::Conflict("candidate already belongs to another request key".to_owned())
            } else { storage(error) })?;
        }
        let bound = self
            .commit_operation(&operation.request_key)?
            .ok_or_else(|| ApplicationError::Integrity("commit binding disappeared".to_owned()))?;
        if bound.protocol_version != operation.protocol_version
            || bound.request_key != operation.request_key
            || bound.input_sha256 != operation.input_sha256
            || bound.selection != operation.selection
        {
            return Err(ApplicationError::Conflict(
                "request key is bound to different frozen input or selection".to_owned(),
            ));
        }
        Ok(bound)
    }

    pub(super) fn commit_operation_impl(
        &self,
        key: &str,
    ) -> Result<Option<CollectionCommitOperation>, ApplicationError> {
        let json = self
            .lock()?
            .query_row(
                "SELECT operation_json FROM collection_commit_operations WHERE request_key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage)?;
        json.map(|value| serde_json::from_str(&value).map_err(json_error))
            .transpose()
    }

    pub(super) fn complete_commit_operation_impl(
        &self,
        key: &str,
        result: &CollectionItemStatus,
    ) -> Result<CollectionCommitOperation, ApplicationError> {
        let mut operation = self
            .commit_operation(key)?
            .ok_or_else(|| ApplicationError::NotFound("collection commit operation".to_owned()))?;
        if operation.selection.session_id != result.session_id
            || operation.selection.candidate_ids != [result.candidate_id.clone()]
        {
            return Err(ApplicationError::Integrity(
                "commit result has a different candidate identity".to_owned(),
            ));
        }
        operation.result = Some(result.clone());
        self.lock()?.execute("UPDATE collection_commit_operations SET operation_json = ?2 WHERE request_key = ?1", params![key, serde_json::to_string(&operation).map_err(json_error)?]).map_err(storage)?;
        Ok(operation)
    }

    pub(super) fn committed_candidate_impl(
        &self,
        session: &CollectionSessionId,
        candidate: &str,
    ) -> Result<Option<(ItemId, RevisionId)>, ApplicationError> {
        let connection = self.lock()?;
        // Later recollections legitimately retain the same session/candidate provenance.
        // Replay belongs to its first durable observation, not the most recent source revision.
        let mut statement = connection.prepare("SELECT observation.item_id, observation.revision_id, revision.state FROM source_observations observation JOIN revisions revision ON revision.revision_id = observation.revision_id WHERE observation.collection_session_id = ?1 AND observation.candidate_id = ?2 ORDER BY observation.rowid LIMIT 1").map_err(storage)?;
        let rows = statement
            .query_map(params![session.to_string(), candidate], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        match rows.as_slice() {
            [] => Ok(None),
            [(item, revision, state)] if state == "ready" => Ok(Some((
                ItemId::parse(item.clone())?,
                RevisionId::parse(revision.clone())?,
            ))),
            _ => Err(ApplicationError::Integrity(
                "previous candidate commit is non-ready or ambiguous; reconcile before retry"
                    .to_owned(),
            )),
        }
    }
}
fn storage(error: rusqlite::Error) -> ApplicationError {
    ApplicationError::Storage(error.to_string())
}
fn json_error(error: serde_json::Error) -> ApplicationError {
    ApplicationError::Integrity(error.to_string())
}

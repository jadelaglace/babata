pub mod derived_repository;
pub mod job_repository;
mod migrate;
mod raw_repository;
pub mod read_projection;

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use babata_application::ApplicationError;
use rusqlite::Connection;

pub use migrate::migrate_raw;
pub use raw_repository::SqliteRawRepository;
pub use read_projection::SqliteReadProjection;

#[derive(Debug, Clone, Copy)]
pub struct RawStatus {
    pub reachable: bool,
    pub schema_version: u32,
    pub pending_journals: usize,
    pub orphans: usize,
    pub quarantined_revisions: usize,
}

pub(crate) fn open_connection(
    path: &Path,
    busy_timeout_ms: u64,
) -> Result<Connection, ApplicationError> {
    let connection =
        Connection::open(path).map_err(|error| ApplicationError::Storage(error.to_string()))?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    connection
        .busy_timeout(std::time::Duration::from_millis(busy_timeout_ms))
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    Ok(connection)
}

pub fn open_raw_database(
    paths: &crate::paths::DataPaths,
    busy_timeout_ms: u64,
) -> Result<SqliteRawRepository, ApplicationError> {
    let connection = open_connection(&paths.raw_database(), busy_timeout_ms)?;
    migrate_raw(&connection)?;
    Ok(SqliteRawRepository::new(Arc::new(Mutex::new(connection))))
}

pub fn raw_status(
    paths: &crate::paths::DataPaths,
    busy_timeout_ms: u64,
) -> Result<RawStatus, ApplicationError> {
    let database = paths.raw_database();
    let pending_journals = std::fs::read_dir(paths.journal())
        .map_err(|error| ApplicationError::Storage(error.to_string()))?
        .count();
    let orphans = std::fs::read_dir(paths.orphan())
        .map_err(|error| ApplicationError::Storage(error.to_string()))?
        .count();
    if !database.exists() {
        return Ok(RawStatus {
            reachable: false,
            schema_version: 0,
            pending_journals,
            orphans,
            quarantined_revisions: 0,
        });
    }
    let connection = open_connection(&database, busy_timeout_ms)?;
    let version = connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    let quarantined_revisions = connection
        .query_row(
            "SELECT COUNT(*) FROM revisions WHERE state = 'quarantined'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    Ok(RawStatus {
        reachable: true,
        schema_version: version as u32,
        pending_journals,
        orphans,
        quarantined_revisions: quarantined_revisions as usize,
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    #[test]
    fn raw_status_reports_schema_and_recovery_artifacts() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        std::fs::write(paths.journal().join("pending.json"), "{}").unwrap();
        std::fs::write(paths.orphan().join("orphan.bin"), "orphan").unwrap();
        let before = super::raw_status(&paths, 100).unwrap();
        assert!(!before.reachable);
        assert_eq!(before.pending_journals, 1);
        assert_eq!(before.orphans, 1);
        super::open_raw_database(&paths, 100).unwrap();
        let after = super::raw_status(&paths, 100).unwrap();
        assert!(after.reachable);
        assert_eq!(after.schema_version, 3);
        assert_eq!(after.pending_journals, 1);
        assert_eq!(after.orphans, 1);
        assert_eq!(after.quarantined_revisions, 0);
    }
}

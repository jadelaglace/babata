pub mod derived_migrate;
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
use sha2::{Digest, Sha256};

pub use collection_migrate::migrate_collection;
pub use derived_migrate::migrate_derived;
pub use derived_repository::SqliteDerivedRepository;
pub use job_repository::SqliteJobRepository;
pub use migrate::{migrate_knowledge, migrate_raw};
pub use raw_repository::SqliteRawRepository;
pub use read_projection::SqliteReadProjection;

mod collection_migrate;
mod collection_repository;

fn migration_checksum(sql: &str) -> String {
    // Existing v3 repair history uses the repository's CRLF checksum. Keep
    // that canonical while accepting LF checkouts in the matcher below.
    let normalized = sql.replace("\r\n", "\n").replace('\r', "\n");
    let canonical_crlf = normalized.replace('\n', "\r\n");
    format!("{:x}", Sha256::digest(canonical_crlf.as_bytes()))
}

fn migration_checksum_matches(recorded: &str, sql: &str) -> bool {
    if recorded == migration_checksum(sql) {
        return true;
    }
    let normalized = sql.replace("\r\n", "\n").replace('\r', "\n");
    recorded == format!("{:x}", Sha256::digest(normalized.as_bytes()))
        || recorded == format!("{:x}", Sha256::digest(sql.as_bytes()))
}

#[derive(Debug, Clone, Copy)]
pub struct RawStatus {
    pub reachable: bool,
    pub schema_version: u32,
    pub pending_journals: usize,
    pub orphans: usize,
    pub quarantined_revisions: usize,
    pub pending_operations: usize,
    pub quarantined_operations: usize,
}

pub(crate) fn open_connection(
    path: &Path,
    busy_timeout_ms: u64,
) -> Result<Connection, ApplicationError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    }
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

pub fn open_collection_database(
    paths: &crate::paths::DataPaths,
    busy_timeout_ms: u64,
) -> Result<SqliteRawRepository, ApplicationError> {
    let repository = open_raw_database(paths, busy_timeout_ms)?;
    {
        let connection = repository.lock()?;
        migrate_collection(&connection)?;
    }
    Ok(repository)
}

pub fn open_knowledge_review_database(
    paths: &crate::paths::DataPaths,
    busy_timeout_ms: u64,
) -> Result<SqliteRawRepository, ApplicationError> {
    let repository = open_raw_database(paths, busy_timeout_ms)?;
    {
        let connection = repository.lock()?;
        migrate_knowledge(&connection)?;
    }
    Ok(repository)
}

pub fn open_derived_database(
    paths: &crate::paths::DataPaths,
    busy_timeout_ms: u64,
) -> Result<SqliteDerivedRepository, ApplicationError> {
    crate::paths::ensure_layout(paths)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    let connection = open_connection(&paths.derived_database(), busy_timeout_ms)?;
    migrate_derived(&connection)?;
    Ok(SqliteDerivedRepository::new(Arc::new(Mutex::new(
        connection,
    ))))
}

pub fn open_job_database(
    paths: &crate::paths::DataPaths,
    busy_timeout_ms: u64,
) -> Result<SqliteJobRepository, ApplicationError> {
    let connection = open_connection(&paths.runtime_database(), busy_timeout_ms)?;
    job_repository::migrate_runtime(&connection)?;
    Ok(SqliteJobRepository::new(Arc::new(Mutex::new(connection))))
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
            pending_operations: 0,
            quarantined_operations: 0,
        });
    }
    let connection = open_connection(&database, busy_timeout_ms)?;
    let schema_version = connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as u32;
    let quarantined_revisions = connection
        .query_row(
            "SELECT COUNT(*) FROM revisions WHERE state = 'quarantined'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize;
    let pending_operations = connection
        .query_row(
            "SELECT COUNT(*) FROM capture_operations WHERE state = 'pending'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize;
    let quarantined_operations = connection
        .query_row(
            "SELECT COUNT(*) FROM capture_operations WHERE state = 'quarantined'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize;
    Ok(RawStatus {
        reachable: true,
        schema_version,
        pending_journals,
        orphans,
        quarantined_revisions,
        pending_operations,
        quarantined_operations,
    })
}

#[cfg(feature = "test-support")]
pub mod test_support {
    use std::path::Path;

    use rusqlite::{OptionalExtension, params};

    use crate::paths::DataPaths;

    use super::open_connection;

    #[derive(Debug, Clone)]
    pub struct CaptureOperationSnapshot {
        pub operation_state: String,
        pub revision_state: String,
        pub asset_state: Option<String>,
        pub logical_path: Option<String>,
        pub failure_code: Option<String>,
        pub revision_id: String,
    }

    pub fn inject_ready_failure(data_root: &Path) -> Result<(), String> {
        let connection = open(data_root)?;
        connection
            .execute_batch(
                "CREATE TRIGGER fail_ready BEFORE UPDATE OF state ON revisions
                 WHEN NEW.state = 'ready'
                 BEGIN SELECT RAISE(ABORT, 'injected ready failure'); END;",
            )
            .map_err(|error| error.to_string())
    }

    pub fn inject_graph_failure(data_root: &Path) -> Result<(), String> {
        let connection = open(data_root)?;
        connection
            .execute_batch(
                "CREATE TRIGGER fail_graph BEFORE INSERT ON revisions
                 BEGIN SELECT RAISE(ABORT, 'injected graph failure'); END;",
            )
            .map_err(|error| error.to_string())
    }

    pub fn inject_post_ready_readback_failure(data_root: &Path) -> Result<(), String> {
        let connection = open(data_root)?;
        connection
            .execute_batch(
                "CREATE TRIGGER corrupt_ready_readback AFTER UPDATE OF state ON revisions
                 WHEN NEW.state = 'ready'
                 BEGIN
                   UPDATE items SET metadata_json = 'not-json' WHERE item_id = NEW.item_id;
                 END;",
            )
            .map_err(|error| error.to_string())
    }

    pub fn capture_operation_snapshot(
        data_root: &Path,
        operation_id: &str,
    ) -> Result<Option<CaptureOperationSnapshot>, String> {
        let connection = open(data_root)?;
        connection
            .query_row(
                "SELECT o.state, r.state, a.state, a.logical_path, o.failure_code, r.revision_id
                 FROM capture_operations o
                 JOIN revisions r ON r.revision_id = o.revision_id
                 LEFT JOIN assets a ON a.revision_id = r.revision_id
                 WHERE o.operation_id = ?1",
                params![operation_id],
                |row| {
                    Ok(CaptureOperationSnapshot {
                        operation_state: row.get(0)?,
                        revision_state: row.get(1)?,
                        asset_state: row.get(2)?,
                        logical_path: row.get(3)?,
                        failure_code: row.get(4)?,
                        revision_id: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    fn open(data_root: &Path) -> Result<rusqlite::Connection, String> {
        open_connection(&DataPaths::new(data_root.to_path_buf()).raw_database(), 100)
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use sha2::Digest;
    use tempfile::tempdir;

    #[test]
    fn migration_checksums_are_stable_across_line_endings() {
        let lf = "CREATE TABLE example (id INTEGER);\nSELECT 1;\n";
        let crlf = lf.replace('\n', "\r\n");

        assert_eq!(
            super::migration_checksum(lf),
            super::migration_checksum(&crlf)
        );
        let legacy_lf = format!("{:x}", sha2::Sha256::digest(lf.as_bytes()));
        assert!(super::migration_checksum_matches(&legacy_lf, &crlf));
    }

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
        assert_eq!(after.schema_version, 4);
        assert_eq!(after.pending_journals, 1);
        assert_eq!(after.orphans, 1);
        assert_eq!(after.quarantined_revisions, 0);
        assert_eq!(after.pending_operations, 0);
        assert_eq!(after.quarantined_operations, 0);
    }

    #[test]
    fn open_derived_database_migrates() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let repo = super::open_derived_database(&paths, 100).unwrap();
        drop(repo);
        assert!(paths.derived_database().exists());
    }

    #[test]
    fn open_knowledge_review_database_migrates_compatibility_state_in_raw_authority() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let repo = super::open_knowledge_review_database(&paths, 100).unwrap();
        drop(repo);
        let connection = rusqlite::Connection::open(paths.raw_database()).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT MAX(version) FROM knowledge_schema_migrations",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
    }

    #[test]
    fn raw_status_reads_a_pre_operation_schema_without_migrating_it() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let connection = rusqlite::Connection::open(paths.raw_database()).unwrap();
        for (version, name, sql) in [
            (
                1,
                "0001_raw_schema.sql",
                include_str!("../../../../03_migrations/01_raw/0001_raw_schema.sql"),
            ),
            (
                2,
                "0002_raw_indexes.sql",
                include_str!("../../../../03_migrations/01_raw/0002_raw_indexes.sql"),
            ),
            (
                3,
                "0003_raw_fts.sql",
                include_str!("../../../../03_migrations/01_raw/0003_raw_fts.sql"),
            ),
        ] {
            connection.execute_batch(sql).unwrap();
            connection
                .execute(
                    "INSERT INTO schema_migrations (version, name, applied_at, checksum_sha256) VALUES (?1, ?2, '2026-01-01T00:00:00Z', ?3)",
                    params![version, name, format!("fixture-{version}")],
                )
                .unwrap();
        }
        drop(connection);
        let status = super::raw_status(&paths, 100).unwrap();
        assert_eq!(status.schema_version, 3);
        assert_eq!(status.pending_operations, 0);
        assert_eq!(status.quarantined_operations, 0);
    }
}

use std::{path::Path, time::Duration};

use babata_application::ApplicationError;
use rusqlite::{Connection, OpenFlags, backup::Backup};

pub fn snapshot_database(
    source: &Path,
    destination_path: &Path,
    busy_timeout_ms: u64,
) -> Result<(), ApplicationError> {
    if let Some(parent) = destination_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    }
    let source = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    source
        .busy_timeout(Duration::from_millis(busy_timeout_ms))
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    let mut destination = Connection::open(destination_path)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    let backup = Backup::new(&source, &mut destination)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    backup
        .run_to_completion(128, Duration::from_millis(10), None)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    drop(backup);
    destination
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    drop(destination);
    verify_database(destination_path)?;
    Ok(())
}

pub fn verify_database(path: &Path) -> Result<(), ApplicationError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    let quick_check = connection
        .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    if quick_check != "ok" {
        return Err(ApplicationError::Integrity(format!(
            "SQLite quick_check failed for {}: {quick_check}",
            path.display()
        )));
    }
    let foreign_key_violations = connection
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get::<_, u64>(0)
        })
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    if foreign_key_violations != 0 {
        return Err(ApplicationError::Integrity(format!(
            "SQLite foreign key violations for {}: {foreign_key_violations}",
            path.display()
        )));
    }
    Ok(())
}

use std::collections::BTreeMap;

use babata_application::ApplicationError;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_raw_schema.sql",
        include_str!("../../../../03_migrations/01_raw/0001_raw_schema.sql"),
    ),
    (
        "0002_raw_indexes.sql",
        include_str!("../../../../03_migrations/01_raw/0002_raw_indexes.sql"),
    ),
    (
        "0003_raw_fts.sql",
        include_str!("../../../../03_migrations/01_raw/0003_raw_fts.sql"),
    ),
    (
        "0004_route_evidence.sql",
        include_str!("../../../../03_migrations/01_raw/0004_route_evidence.sql"),
    ),
];

pub fn migrate_raw(connection: &Connection) -> Result<(), ApplicationError> {
    let mut recorded = BTreeMap::new();
    let table_exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |_| Ok(()),
        )
        .is_ok();
    if table_exists {
        let mut statement = connection
            .prepare("SELECT version, checksum_sha256 FROM schema_migrations")
            .map_err(storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage)?;
        for row in rows {
            let (version, checksum) = row.map_err(storage)?;
            recorded.insert(version, checksum);
        }
    }
    for (index, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        let checksum = format!("{:x}", Sha256::digest(sql.as_bytes()));
        if let Some(existing) = recorded.get(&version) {
            if existing != &checksum {
                return Err(ApplicationError::Integrity(format!(
                    "migration checksum changed: {name}"
                )));
            }
            continue;
        }
        let transaction = connection.unchecked_transaction().map_err(storage)?;
        transaction.execute_batch(sql).map_err(storage)?;
        transaction.execute("INSERT INTO schema_migrations (version, name, applied_at, checksum_sha256) VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?3)", params![version, name, checksum]).map_err(storage)?;
        transaction.commit().map_err(storage)?;
    }
    Ok(())
}

fn storage(error: rusqlite::Error) -> ApplicationError {
    ApplicationError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrates_an_empty_database() {
        let connection = Connection::open_in_memory().unwrap();
        migrate_raw(&connection).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            4
        );
    }

    #[test]
    fn migration_is_idempotent() {
        let connection = Connection::open_in_memory().unwrap();
        migrate_raw(&connection).unwrap();
        migrate_raw(&connection).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            4
        );
    }

    #[test]
    fn changed_recorded_checksum_is_rejected() {
        let connection = Connection::open_in_memory().unwrap();
        migrate_raw(&connection).unwrap();
        connection
            .execute(
                "UPDATE schema_migrations SET checksum_sha256 = 'tampered' WHERE version = 1",
                [],
            )
            .unwrap();
        assert!(matches!(
            migrate_raw(&connection),
            Err(ApplicationError::Integrity(_))
        ));
    }

    #[test]
    fn foreign_keys_reject_an_invalid_asset() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        migrate_raw(&connection).unwrap();
        assert!(connection.execute("INSERT INTO assets (asset_id, revision_id, asset_role, logical_path, sha256, byte_size, state, created_at) VALUES ('asset_01KXGDJP1ENK14ADJVT7RS6JDH', 'rev_01KXGDJP1ENK14ADJVT7RS6JDH', 'original', '01_raw/a', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 1, 'pending', '2026-01-01T00:00:00Z')", []).is_err());
    }
}

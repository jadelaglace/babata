use std::collections::BTreeMap;

use babata_application::ApplicationError;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_route_evidence.sql",
        include_str!("../../../../03_migrations/02_collection/0001_route_evidence.sql"),
    ),
    (
        "0002_collection_sessions.sql",
        include_str!("../../../../03_migrations/02_collection/0002_collection_sessions.sql"),
    ),
    (
        "0003_collection_item_options.sql",
        include_str!("../../../../03_migrations/02_collection/0003_collection_item_options.sql"),
    ),
];

pub fn migrate_collection(connection: &Connection) -> Result<(), ApplicationError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS collection_schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL,
                checksum_sha256 TEXT NOT NULL
            );",
        )
        .map_err(storage)?;
    let mut recorded = BTreeMap::new();
    let mut statement = connection
        .prepare("SELECT version, checksum_sha256 FROM collection_schema_migrations")
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
    drop(statement);
    for (index, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        let checksum = format!("{:x}", Sha256::digest(sql.as_bytes()));
        if let Some(existing) = recorded.get(&version) {
            if existing != &checksum {
                return Err(ApplicationError::Integrity(format!(
                    "collection migration checksum changed: {name}"
                )));
            }
            continue;
        }
        let transaction = connection.unchecked_transaction().map_err(storage)?;
        transaction.execute_batch(sql).map_err(storage)?;
        transaction
            .execute(
                "INSERT INTO collection_schema_migrations
                 (version, name, applied_at, checksum_sha256)
                 VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?3)",
                params![version, name, checksum],
            )
            .map_err(storage)?;
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

    #[test]
    fn collection_migration_is_explicit_and_idempotent() {
        let connection = Connection::open_in_memory().unwrap();
        crate::sqlite::migrate_raw(&connection).unwrap();
        migrate_collection(&connection).unwrap();
        migrate_collection(&connection).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM collection_schema_migrations",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            3
        );
    }
}

use std::collections::BTreeMap;

use babata_application::ApplicationError;
use rusqlite::{Connection, params};

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
        "0004_capture_operations.sql",
        include_str!("../../../../03_migrations/01_raw/0004_capture_operations.sql"),
    ),
];

const INTEGRITY_MIGRATIONS: &[(&str, &str)] = &[(
    "0001_raw_reference_bindings.sql",
    include_str!("../../../../03_migrations/04_integrity/0001_raw_reference_bindings.sql"),
)];

const KNOWLEDGE_MIGRATIONS: &[(&str, &str)] = &[(
    "0001_knowledge_records.sql",
    include_str!("../../../../03_migrations/05_knowledge/0001_knowledge_records.sql"),
)];

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
    reject_newer_schema(
        recorded.last_key_value().map(|(version, _)| *version),
        MIGRATIONS.len(),
    )?;
    for (index, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        let checksum = super::migration_checksum(sql);
        if let Some(existing) = recorded.get(&version) {
            if !super::migration_checksum_matches(existing, sql) {
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
    migrate_raw_integrity(connection)
}

pub fn migrate_knowledge(connection: &Connection) -> Result<(), ApplicationError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS knowledge_schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                applied_at TEXT NOT NULL,
                checksum_sha256 TEXT NOT NULL
            );",
        )
        .map_err(storage)?;
    let mut recorded = BTreeMap::new();
    let mut statement = connection
        .prepare("SELECT version, checksum_sha256 FROM knowledge_schema_migrations")
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
    if recorded
        .last_key_value()
        .is_some_and(|(version, _)| *version > KNOWLEDGE_MIGRATIONS.len() as i64)
    {
        return Err(ApplicationError::Integrity(format!(
            "knowledge schema version {} is newer than this binary supports ({})",
            recorded.last_key_value().map_or(0, |(version, _)| *version),
            KNOWLEDGE_MIGRATIONS.len()
        )));
    }
    for (index, (name, sql)) in KNOWLEDGE_MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        let checksum = super::migration_checksum(sql);
        if let Some(existing) = recorded.get(&version) {
            if !super::migration_checksum_matches(existing, sql) {
                return Err(ApplicationError::Integrity(format!(
                    "knowledge migration checksum changed: {name}"
                )));
            }
            continue;
        }
        let transaction = connection.unchecked_transaction().map_err(storage)?;
        transaction.execute_batch(sql).map_err(storage)?;
        transaction
            .execute(
                "INSERT INTO knowledge_schema_migrations
                 (version, name, applied_at, checksum_sha256)
                 VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?3)",
                params![version, name, checksum],
            )
            .map_err(storage)?;
        transaction.commit().map_err(storage)?;
    }
    Ok(())
}

fn migrate_raw_integrity(connection: &Connection) -> Result<(), ApplicationError> {
    ensure_reference_integrity(connection)?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS raw_integrity_schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                applied_at TEXT NOT NULL,
                checksum_sha256 TEXT NOT NULL
            );",
        )
        .map_err(storage)?;
    let mut recorded = BTreeMap::new();
    let mut statement = connection
        .prepare("SELECT version, checksum_sha256 FROM raw_integrity_schema_migrations")
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
    reject_newer_schema(
        recorded.last_key_value().map(|(version, _)| *version),
        INTEGRITY_MIGRATIONS.len(),
    )?;
    for (index, (name, sql)) in INTEGRITY_MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        let checksum = super::migration_checksum(sql);
        if let Some(existing) = recorded.get(&version) {
            if !super::migration_checksum_matches(existing, sql) {
                return Err(ApplicationError::Integrity(format!(
                    "raw integrity migration checksum changed: {name}"
                )));
            }
            continue;
        }
        let transaction = connection.unchecked_transaction().map_err(storage)?;
        transaction.execute_batch(sql).map_err(storage)?;
        transaction
            .execute(
                "INSERT INTO raw_integrity_schema_migrations
                 (version, name, applied_at, checksum_sha256)
                 VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?3)",
                params![version, name, checksum],
            )
            .map_err(storage)?;
        transaction.commit().map_err(storage)?;
    }
    Ok(())
}

fn reject_newer_schema(recorded: Option<i64>, supported: usize) -> Result<(), ApplicationError> {
    if recorded.is_some_and(|version| version > supported as i64) {
        return Err(ApplicationError::Integrity(format!(
            "raw schema version {} is newer than this binary supports ({supported})",
            recorded.unwrap_or_default()
        )));
    }
    Ok(())
}

fn ensure_reference_integrity(connection: &Connection) -> Result<(), ApplicationError> {
    let anomaly_count = connection
        .query_row(
            "SELECT
                (SELECT COUNT(*) FROM revisions child JOIN revisions parent
                 ON parent.revision_id = child.parent_revision_id
                 WHERE parent.item_id <> child.item_id OR parent.ordinal >= child.ordinal)
              + (SELECT COUNT(*) FROM capture_operations operation JOIN revisions revision
                 ON revision.revision_id = operation.revision_id
                 WHERE revision.item_id <> operation.item_id)
              + (SELECT COUNT(*) FROM relations relation JOIN revisions revision
                 ON revision.revision_id = relation.from_revision_id
                 WHERE relation.from_revision_id IS NOT NULL
                   AND revision.item_id <> relation.from_item_id)
              + (SELECT COUNT(*) FROM relations relation JOIN revisions revision
                 ON revision.revision_id = relation.to_revision_id
                 WHERE relation.to_revision_id IS NOT NULL
                   AND revision.item_id <> relation.to_item_id)",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage)?;
    if anomaly_count != 0 {
        return Err(ApplicationError::Integrity(format!(
            "raw reference audit found {anomaly_count} item/revision mismatches"
        )));
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
        assert!(
            connection
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'route_evidence'",
                    [],
                    |_| Ok(()),
                )
                .is_err()
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
    fn newer_raw_schema_is_rejected() {
        let connection = Connection::open_in_memory().unwrap();
        migrate_raw(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations
                 (version, name, applied_at, checksum_sha256)
                 VALUES (5, 'future.sql', '2026-01-01T00:00:00Z', 'future')",
                [],
            )
            .unwrap();
        assert!(
            migrate_raw(&connection)
                .unwrap_err()
                .to_string()
                .contains("newer than this binary supports")
        );
    }

    #[test]
    fn knowledge_migration_is_explicit_and_idempotent() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        migrate_raw(&connection).unwrap();
        migrate_knowledge(&connection).unwrap();
        migrate_knowledge(&connection).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM knowledge_schema_migrations",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn knowledge_bindings_and_versions_fail_closed() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        migrate_raw(&connection).unwrap();
        migrate_knowledge(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO sources (source_id, source_kind, provider, created_at) VALUES
                    ('source_external', 'external', 'fixture', '2026-01-01T00:00:00Z'),
                    ('source_first_party', 'first_party', 'babata', '2026-01-01T00:00:00Z');
                 INSERT INTO items
                    (item_id, source_id, content_type, first_captured_at, created_at) VALUES
                    ('item_source', 'source_external', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
                    ('item_other', 'source_external', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
                    ('item_knowledge', 'source_first_party', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
                    ('item_other_first_party', 'source_first_party', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                 INSERT INTO revisions
                    (revision_id, item_id, revision_kind, ordinal, captured_at, raw_text,
                     text_sha256, state, created_at) VALUES
                    ('revision_source', 'item_source', 'capture', 1, '2026-01-01T00:00:00Z', 'source',
                     'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'ready', '2026-01-01T00:00:00Z'),
                    ('revision_knowledge_1', 'item_knowledge', 'authored', 1, '2026-01-01T00:00:00Z', 'v1',
                     'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', 'ready', '2026-01-01T00:00:00Z'),
                    ('revision_knowledge_2', 'item_knowledge', 'edit', 2, '2026-01-02T00:00:00Z', 'v2',
                     'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc', 'ready', '2026-01-02T00:00:00Z'),
                    ('revision_wrong_item', 'item_other_first_party', 'edit', 1, '2026-01-02T00:00:00Z', 'wrong',
                     'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd', 'ready', '2026-01-02T00:00:00Z');",
            )
            .unwrap();
        assert!(connection.execute("INSERT INTO knowledge_records (knowledge_id, semantic_kind, author, first_party_item_id, source_item_id, source_revision_id, created_at) VALUES ('knowledge_bad_source', 'knowledge', 'user', 'item_knowledge', 'item_other', 'revision_source', '2026-01-01T00:00:00Z')", []).is_err());
        assert!(connection.execute("INSERT INTO knowledge_records (knowledge_id, semantic_kind, author, first_party_item_id, source_item_id, source_revision_id, created_at) VALUES ('knowledge_bad_authority', 'knowledge', 'user', 'item_other', 'item_source', 'revision_source', '2026-01-01T00:00:00Z')", []).is_err());
        connection.execute("INSERT INTO knowledge_records (knowledge_id, semantic_kind, author, first_party_item_id, source_item_id, source_revision_id, created_at) VALUES ('knowledge_valid', 'knowledge', 'user', 'item_knowledge', 'item_source', 'revision_source', '2026-01-01T00:00:00Z')", []).unwrap();
        assert!(connection.execute("INSERT INTO knowledge_versions (knowledge_id, ordinal, first_party_revision_id, title, created_at) VALUES ('knowledge_valid', 2, 'revision_knowledge_2', 'v2', '2026-01-02T00:00:00Z')", []).is_err());
        assert!(connection.execute("INSERT INTO knowledge_versions (knowledge_id, ordinal, first_party_revision_id, title, created_at) VALUES ('knowledge_valid', 1, 'revision_wrong_item', 'wrong', '2026-01-01T00:00:00Z')", []).is_err());
        connection.execute("INSERT INTO knowledge_versions (knowledge_id, ordinal, first_party_revision_id, title, created_at) VALUES ('knowledge_valid', 1, 'revision_knowledge_1', 'v1', '2026-01-01T00:00:00Z')", []).unwrap();
        assert!(connection.execute("UPDATE knowledge_records SET author = 'other' WHERE knowledge_id = 'knowledge_valid'", []).is_err());
        assert!(
            connection
                .execute(
                    "DELETE FROM knowledge_versions WHERE knowledge_id = 'knowledge_valid'",
                    []
                )
                .is_err()
        );
    }

    #[test]
    fn raw_reference_trigger_rejects_cross_item_revision() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        migrate_raw(&connection).unwrap();
        connection.execute_batch("INSERT INTO sources (source_id, source_kind, provider, created_at) VALUES ('source_a', 'external', 'fixture', '2026-01-01T00:00:00Z');
            INSERT INTO items (item_id, source_id, content_type, first_captured_at, created_at) VALUES
                ('item_a', 'source_a', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
                ('item_b', 'source_a', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
            INSERT INTO revisions (revision_id, item_id, revision_kind, ordinal, captured_at, raw_text, text_sha256, state, created_at) VALUES
                ('revision_a', 'item_a', 'capture', 1, '2026-01-01T00:00:00Z', 'a', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'ready', '2026-01-01T00:00:00Z');").unwrap();
        assert!(
            connection
                .execute("INSERT INTO capture_operations (operation_id, item_id, revision_id, state, started_at) VALUES ('operation_bad', 'item_b', 'revision_a', 'ready', '2026-01-01T00:00:00Z')", [])
                .is_err()
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

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
    (
        "0005_asset_attachment_operations.sql",
        include_str!("../../../../03_migrations/01_raw/0005_asset_attachment_operations.sql"),
    ),
    (
        "0006_source_observations.sql",
        include_str!("../../../../03_migrations/01_raw/0006_source_observations.sql"),
    ),
];

const INTEGRITY_MIGRATIONS: &[(&str, &str)] = &[(
    "0001_raw_reference_bindings.sql",
    include_str!("../../../../03_migrations/04_integrity/0001_raw_reference_bindings.sql"),
)];

const KNOWLEDGE_MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_knowledge_records.sql",
        include_str!("../../../../03_migrations/05_knowledge/0001_knowledge_records.sql"),
    ),
    (
        "0002_deprecate_manual_knowledge_loop.sql",
        include_str!(
            "../../../../03_migrations/05_knowledge/0002_deprecate_manual_knowledge_loop.sql"
        ),
    ),
    (
        "0003_p6_semantic_core.sql",
        include_str!("../../../../03_migrations/05_knowledge/0003_p6_semantic_core.sql"),
    ),
    (
        "0004_p6_map_evolution.sql",
        include_str!("../../../../03_migrations/05_knowledge/0004_p6_map_evolution.sql"),
    ),
    (
        "0005_lock_baseline_foundation_transition.sql",
        include_str!(
            "../../../../03_migrations/05_knowledge/0005_lock_baseline_foundation_transition.sql"
        ),
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
            6
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
            6
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
                 VALUES (7, 'future.sql', '2026-01-01T00:00:00Z', 'future')",
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
    fn raw_v5_to_v6_preserves_existing_rows_and_backfills_common_metadata() {
        let connection = Connection::open_in_memory().unwrap();
        for (index, (name, sql)) in MIGRATIONS.iter().take(5).enumerate() {
            connection.execute_batch(sql).unwrap();
            connection
                .execute(
                    "INSERT INTO schema_migrations
                     (version, name, applied_at, checksum_sha256)
                     VALUES (?1, ?2, '2026-07-21T00:00:00Z', ?3)",
                    params![
                        (index + 1) as i64,
                        name,
                        super::super::migration_checksum(sql)
                    ],
                )
                .unwrap();
        }
        connection
            .execute_batch(
                "INSERT INTO sources
                    (source_id, source_kind, provider, created_at)
                 VALUES ('source_existing', 'external', 'fixture', '2026-07-21T00:00:00Z');
                 INSERT INTO items
                    (item_id, source_id, source_native_id, content_type,
                     first_captured_at, metadata_json, created_at)
                 VALUES ('item_existing', 'source_existing', 'native-1', 'document',
                         '2026-07-21T00:00:00Z', '{\"provider_unknown\":true}',
                         '2026-07-21T00:00:00Z');
                 INSERT INTO revisions
                    (revision_id, item_id, revision_kind, ordinal, captured_at,
                     raw_text, text_sha256, metadata_json, state, created_at)
                 VALUES ('revision_existing', 'item_existing', 'capture', 1,
                         '2026-07-21T00:00:00Z', 'existing',
                         'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
                         '{\"provider_unknown\":true}', 'ready', '2026-07-21T00:00:00Z');",
            )
            .unwrap();

        migrate_raw(&connection).unwrap();
        migrate_raw(&connection).unwrap();

        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM items", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM revisions", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT metadata_json FROM items WHERE item_id = 'item_existing'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "{\"provider_unknown\":true}"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT metadata_json FROM revisions
                     WHERE revision_id = 'revision_existing'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "{\"provider_unknown\":true}"
        );
        let common: String = connection
            .query_row(
                "SELECT common_metadata_json FROM items WHERE item_id = 'item_existing'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&common).unwrap()["schema"],
            "babata.c0.common/v1"
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM source_observations", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            0
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
            5
        );
    }

    #[test]
    fn map_evolution_migration_backfills_history_and_locks_foundations() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        migrate_raw(&connection).unwrap();
        migrate_knowledge(&connection).unwrap();

        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM knowledge_map_node_events
                     WHERE event_kind = 'created' AND provenance_kind = 'system'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            4
        );
        assert!(
            connection
                .execute(
                    "UPDATE knowledge_map_nodes SET name = 'changed'
                     WHERE map_node_id = 'mapnode_p6_time'",
                    [],
                )
                .unwrap_err()
                .to_string()
                .contains("foundation nodes are immutable")
        );
        assert!(
            connection
                .execute(
                    "INSERT INTO knowledge_map_nodes
                     (map_node_id, map_version_id, node_level, canonical_key, name,
                      provenance_kind, suggestion_id, created_at, lifecycle_state)
                     VALUES ('mapnode_extra_foundation', 'map_version_p6_baseline', 'foundation',
                             'foundation:extra', 'extra', 'first_party', NULL,
                             '2026-07-21T00:00:00Z', 'active')",
                    [],
                )
                .unwrap_err()
                .to_string()
                .contains("foundation nodes are fixed")
        );

        connection
            .execute(
                "INSERT INTO worldview_map_versions
                 (map_version_id, ordinal, rationale, author_kind, author, created_at)
                 VALUES ('map_version_transition_probe', 2, 'negative migration test',
                         'system', 'test', '2026-07-21T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO knowledge_map_nodes
                 (map_node_id, map_version_id, node_level, canonical_key, name,
                  provenance_kind, suggestion_id, created_at, lifecycle_state)
                 VALUES ('mapnode_transition_probe', 'map_version_transition_probe',
                         'foundation', 'foundation:transition-probe', 'transition probe',
                         'system', NULL, '2026-07-21T00:00:00Z', 'active')",
                [],
            )
            .unwrap();
        assert!(
            connection
                .execute(
                    "UPDATE knowledge_map_nodes SET map_version_id = 'map_version_p6_baseline'
                     WHERE map_node_id = 'mapnode_transition_probe'",
                    [],
                )
                .unwrap_err()
                .to_string()
                .contains("foundation nodes are immutable")
        );
    }

    #[test]
    fn superseded_manual_knowledge_rows_are_preserved_but_isolated() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        migrate_raw(&connection).unwrap();
        connection.execute_batch(KNOWLEDGE_MIGRATIONS[0].1).unwrap();
        connection
            .execute_batch(
                "INSERT INTO sources (source_id, source_kind, provider, created_at) VALUES
                    ('source_external', 'external', 'fixture', '2026-01-01T00:00:00Z'),
                    ('source_first_party', 'first_party', 'babata', '2026-01-01T00:00:00Z');
                 INSERT INTO items
                    (item_id, source_id, content_type, first_captured_at, created_at) VALUES
                    ('item_source', 'source_external', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
                    ('item_knowledge', 'source_first_party', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                 INSERT INTO revisions
                    (revision_id, item_id, revision_kind, ordinal, captured_at, raw_text,
                     text_sha256, state, created_at) VALUES
                    ('revision_source', 'item_source', 'capture', 1, '2026-01-01T00:00:00Z', 'source',
                     'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'ready', '2026-01-01T00:00:00Z'),
                    ('revision_knowledge', 'item_knowledge', 'authored', 1, '2026-01-01T00:00:00Z', 'old model',
                     'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', 'ready', '2026-01-01T00:00:00Z');
                 INSERT INTO knowledge_records
                    (knowledge_id, semantic_kind, author, first_party_item_id, source_item_id,
                     source_revision_id, created_at)
                 VALUES ('knowledge_old', 'knowledge', 'user', 'item_knowledge', 'item_source',
                         'revision_source', '2026-01-01T00:00:00Z');
                 INSERT INTO knowledge_versions
                    (knowledge_id, ordinal, first_party_revision_id, title, created_at)
                 VALUES ('knowledge_old', 1, 'revision_knowledge', 'old model',
                         '2026-01-01T00:00:00Z');",
            )
            .unwrap();

        connection.execute_batch(KNOWLEDGE_MIGRATIONS[1].1).unwrap();

        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM deprecated_manual_knowledge_records",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM deprecated_manual_knowledge_versions",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert!(
            connection
                .query_row(
                    "SELECT 1 FROM sqlite_master
                     WHERE type = 'table' AND name = 'knowledge_records'",
                    [],
                    |_| Ok(())
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
    fn asset_attachment_trigger_rejects_cross_revision_membership() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        migrate_raw(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO sources
                    (source_id, source_kind, provider, created_at)
                 VALUES ('source_a', 'external', 'fixture', '2026-01-01T00:00:00Z');
                 INSERT INTO items
                    (item_id, source_id, content_type, first_captured_at, created_at)
                 VALUES
                    ('item_a', 'source_a', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
                    ('item_b', 'source_a', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                 INSERT INTO revisions
                    (revision_id, item_id, revision_kind, ordinal, captured_at,
                     state, created_at)
                 VALUES
                    ('revision_a', 'item_a', 'capture', 1, '2026-01-01T00:00:00Z',
                     'ready', '2026-01-01T00:00:00Z'),
                    ('revision_b', 'item_b', 'capture', 1, '2026-01-01T00:00:00Z',
                     'ready', '2026-01-01T00:00:00Z');
                 INSERT INTO assets
                    (asset_id, revision_id, asset_role, logical_path, sha256,
                     byte_size, state, created_at)
                 VALUES
                    ('asset_b', 'revision_b', 'original', '01_raw/asset-b',
                     'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
                     1, 'pending', '2026-01-01T00:00:00Z');
                 INSERT INTO asset_attachment_operations
                    (operation_id, revision_id, reason, state, started_at)
                 VALUES
                    ('asset_attachment_a', 'revision_a', 'fixture', 'pending',
                     '2026-01-01T00:00:00Z');",
            )
            .unwrap();

        assert!(
            connection
                .execute(
                    "INSERT INTO asset_attachment_members (operation_id, asset_id)
                     VALUES ('asset_attachment_a', 'asset_b')",
                    [],
                )
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

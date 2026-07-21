use std::collections::BTreeMap;

use babata_application::ApplicationError;
use rusqlite::{Connection, params};

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
    (
        "0004_reference_bindings.sql",
        include_str!("../../../../03_migrations/02_collection/0004_reference_bindings.sql"),
    ),
    (
        "0005_candidate_common_metadata.sql",
        include_str!("../../../../03_migrations/02_collection/0005_candidate_common_metadata.sql"),
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
    if recorded
        .last_key_value()
        .is_some_and(|(version, _)| *version > MIGRATIONS.len() as i64)
    {
        return Err(ApplicationError::Integrity(format!(
            "collection schema version {} is newer than this binary supports ({})",
            recorded.last_key_value().map_or(0, |(version, _)| *version),
            MIGRATIONS.len()
        )));
    }
    ensure_collection_reference_integrity(connection)?;
    for (index, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        let checksum = super::migration_checksum(sql);
        if let Some(existing) = recorded.get(&version) {
            if !super::migration_checksum_matches(existing, sql) {
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

fn ensure_collection_reference_integrity(connection: &Connection) -> Result<(), ApplicationError> {
    let tables_ready = [
        "route_evidence",
        "collection_items",
        "collection_recollection_checks",
    ]
    .into_iter()
    .all(|table| {
        connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |_| Ok(()),
            )
            .is_ok()
    });
    if !tables_ready {
        return Ok(());
    }
    let anomaly_count = connection
        .query_row(
            "SELECT
                (SELECT COUNT(*) FROM route_evidence evidence JOIN revisions revision
                 ON revision.revision_id = evidence.revision_id
                 WHERE revision.item_id <> evidence.item_id)
              + (SELECT COUNT(*) FROM collection_items item JOIN revisions revision
                 ON revision.revision_id = item.revision_id
                 WHERE item.item_id IS NOT NULL AND item.revision_id IS NOT NULL
                   AND revision.item_id <> item.item_id)
              + (SELECT COUNT(*) FROM collection_recollection_checks check_row
                 JOIN revisions revision ON revision.revision_id = check_row.previous_revision_id
                 WHERE revision.item_id <> check_row.item_id)
              + (SELECT COUNT(*) FROM collection_recollection_checks check_row
                 JOIN revisions revision ON revision.revision_id = check_row.new_revision_id
                 WHERE check_row.new_revision_id IS NOT NULL
                   AND revision.item_id <> check_row.item_id)",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage)?;
    if anomaly_count != 0 {
        return Err(ApplicationError::Integrity(format!(
            "collection reference audit found {anomaly_count} item/revision mismatches"
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
            5
        );
    }

    #[test]
    fn newer_collection_schema_is_rejected() {
        let connection = Connection::open_in_memory().unwrap();
        crate::sqlite::migrate_raw(&connection).unwrap();
        migrate_collection(&connection).unwrap();
        connection.execute("INSERT INTO collection_schema_migrations (version, name, applied_at, checksum_sha256) VALUES (6, 'future.sql', '2026-01-01T00:00:00Z', 'future')", []).unwrap();
        assert!(
            migrate_collection(&connection)
                .unwrap_err()
                .to_string()
                .contains("newer than this binary supports")
        );
    }

    #[test]
    fn collection_v4_to_v5_preserves_candidates_and_backfills_common_metadata() {
        let connection = Connection::open_in_memory().unwrap();
        crate::sqlite::migrate_raw(&connection).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE collection_schema_migrations (
                    version INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    applied_at TEXT NOT NULL,
                    checksum_sha256 TEXT NOT NULL
                );",
            )
            .unwrap();
        for (index, (name, sql)) in MIGRATIONS.iter().take(4).enumerate() {
            connection.execute_batch(sql).unwrap();
            connection
                .execute(
                    "INSERT INTO collection_schema_migrations
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
                "INSERT INTO collection_sessions
                    (session_id, route_id, source_reference, scope_description,
                     authorisation_id, state, created_at, updated_at)
                 VALUES ('session_existing', 'source.fixture', 'fixture', 'fixture scope',
                         'authorised', 'awaiting_selection', '2026-07-21T00:00:00Z',
                         '2026-07-21T00:00:00Z');
                 INSERT INTO collection_candidates
                    (session_id, candidate_id, route_id, title, hierarchy_json,
                     content_type, limitations_json, selection_capabilities_json)
                 VALUES ('session_existing', 'candidate_existing', 'source.fixture',
                         'Existing title', '[\"Folder\"]', 'document', '[]', '[\"single\"]');",
            )
            .unwrap();

        migrate_collection(&connection).unwrap();
        migrate_collection(&connection).unwrap();

        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM collection_candidates", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT title FROM collection_candidates
                     WHERE candidate_id = 'candidate_existing'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "Existing title"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT hierarchy_json FROM collection_candidates
                     WHERE candidate_id = 'candidate_existing'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "[\"Folder\"]"
        );
        let common: String = connection
            .query_row(
                "SELECT common_metadata_json FROM collection_candidates
                 WHERE candidate_id = 'candidate_existing'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&common).unwrap()["schema"],
            "babata.c0.common/v1"
        );
    }

    #[test]
    fn changed_collection_migration_checksum_is_rejected() {
        let connection = Connection::open_in_memory().unwrap();
        crate::sqlite::migrate_raw(&connection).unwrap();
        migrate_collection(&connection).unwrap();
        connection
            .execute(
                "UPDATE collection_schema_migrations
                 SET checksum_sha256 = 'tampered' WHERE version = 5",
                [],
            )
            .unwrap();
        assert!(
            migrate_collection(&connection)
                .unwrap_err()
                .to_string()
                .contains("migration checksum changed")
        );
    }

    #[test]
    fn collection_reference_trigger_rejects_cross_item_revision() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        crate::sqlite::migrate_raw(&connection).unwrap();
        migrate_collection(&connection).unwrap();
        connection.execute_batch("INSERT INTO sources (source_id, source_kind, provider, created_at) VALUES ('source_a', 'external', 'fixture', '2026-01-01T00:00:00Z');
            INSERT INTO items (item_id, source_id, content_type, first_captured_at, created_at) VALUES
                ('item_a', 'source_a', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
                ('item_b', 'source_a', 'text', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
            INSERT INTO revisions (revision_id, item_id, revision_kind, ordinal, captured_at, raw_text, text_sha256, state, created_at) VALUES
                ('revision_a', 'item_a', 'capture', 1, '2026-01-01T00:00:00Z', 'a', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'ready', '2026-01-01T00:00:00Z');").unwrap();
        assert!(connection.execute("INSERT INTO route_evidence (evidence_id, route_id, authorization_id, source_reference, item_id, revision_id, metadata_covered, attachments_covered, revisions_covered, limitations_json, reimported, recorded_at) VALUES ('evidence_bad', 'fixture', 'authorised', 'fixture', 'item_b', 'revision_a', 1, 1, 1, '[]', 0, '2026-01-01T00:00:00Z')", []).is_err());
    }
}

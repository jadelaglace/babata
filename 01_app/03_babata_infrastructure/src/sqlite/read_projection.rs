use std::{path::Path, str::FromStr};

use babata_application::{
    ApplicationError, ProjectionOperationOutcome, SearchPage, SearchQuery, SurfaceQuery,
    ports::ReadProjectionPort,
};
use babata_domain::{
    DerivativeId, ItemId, JudgmentStatus, LogicalPath, PageCursor, ProjectionStatus, RecordSummary,
    RevisionId, SearchAssetRef, SearchDerivativeRef, SearchMapRef, SearchRecordDetail,
    SearchRecordMarker, SearchRelationRef, SearchRevisionRef, SearchScoreRef, SearchSort, SourceId,
    SurfacingReason, SurfacingReasonKind, UtcTimestamp,
};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, params, params_from_iter, types::Value,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::paths::{DataPaths, ensure_layout};

const PROJECTION_SCHEMA_VERSION: u32 = 1;
const DEFAULT_PROFILE_ID: &str = "score_profile_p6_default";
const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 200;
const PROJECTION_MIGRATION: &str =
    include_str!("../../../../03_migrations/06_projection/0001_search_projection.sql");

#[derive(Debug, Clone)]
pub struct SqliteReadProjection {
    paths: DataPaths,
    busy_timeout_ms: u64,
}

impl SqliteReadProjection {
    pub fn new(paths: DataPaths, busy_timeout_ms: u64) -> Self {
        Self {
            paths,
            busy_timeout_ms,
        }
    }

    fn open(&self) -> Result<Connection, ApplicationError> {
        super::open_connection(
            &self.paths.search_projection_database(),
            self.busy_timeout_ms,
        )
    }

    fn open_existing(&self) -> Result<Connection, ApplicationError> {
        let path = self.paths.search_projection_database();
        if !path.exists() {
            return Err(ApplicationError::NotFound(
                "search projection has not been built".to_owned(),
            ));
        }
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(storage)?;
        connection
            .busy_timeout(std::time::Duration::from_millis(self.busy_timeout_ms))
            .map_err(storage)?;
        Ok(connection)
    }
}

impl ReadProjectionPort for SqliteReadProjection {
    fn rebuild(&self) -> Result<ProjectionOperationOutcome, ApplicationError> {
        ensure_layout(&self.paths).map_err(storage)?;
        // Apply authoritative migrations through their owning repositories before
        // taking the read transaction used to rebuild this disposable projection.
        drop(super::open_knowledge_review_database(
            &self.paths,
            self.busy_timeout_ms,
        )?);
        drop(super::open_derived_database(
            &self.paths,
            self.busy_timeout_ms,
        )?);

        let mut connection = self.open()?;
        migrate_projection(&connection)?;
        attach_authorities(&connection, &self.paths)?;
        let transaction = connection.transaction().map_err(storage)?;
        clear_projection(&transaction)?;
        populate_records(&transaction)?;
        populate_facets(&transaction)?;
        mark_missing_files(&transaction, &self.paths)?;
        populate_fts(&transaction)?;
        write_metadata(&transaction)?;
        transaction.commit().map_err(storage)?;
        drop(connection);

        Ok(ProjectionOperationOutcome {
            operation: "rebuilt".to_owned(),
            status: self.status()?,
        })
    }

    fn delete(&self) -> Result<ProjectionOperationOutcome, ApplicationError> {
        let database = self.paths.search_projection_database();
        for candidate in [
            database.clone(),
            database.with_extension("sqlite-wal"),
            database.with_extension("sqlite-shm"),
        ] {
            match std::fs::remove_file(&candidate) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(storage(error)),
            }
        }
        Ok(ProjectionOperationOutcome {
            operation: "deleted".to_owned(),
            status: ProjectionStatus {
                state: "missing".to_owned(),
                schema_version: 0,
                built_at: None,
                raw_items: 0,
                semantic_entries: 0,
                relations: 0,
                source_fingerprint: None,
            },
        })
    }

    fn search(&self, query: SearchQuery) -> Result<SearchPage, ApplicationError> {
        let connection = self.open_existing()?;
        search_connection(&connection, &query, false)
    }

    fn surface(&self, query: SurfaceQuery) -> Result<SearchPage, ApplicationError> {
        let filter = babata_domain::QueryFilter {
            captured_from: query.since.clone(),
            map_node: query.map_node.clone(),
            related_to: query.related_to.clone(),
            profile_id: query.profile_id.clone(),
            min_weighted_score: Some(0),
            sort: SearchSort::WeightedScore,
            limit: query.limit,
            ..babata_domain::QueryFilter::default()
        };
        let connection = self.open_existing()?;
        let mut page = search_connection(
            &connection,
            &SearchQuery {
                filter,
                cursor: None,
            },
            true,
        )?;
        for record in &mut page.records {
            let relation_count: u64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM search_relations WHERE record_id = ?1",
                    [&record.record_id],
                    |row| row.get(0),
                )
                .map_err(storage)?;
            record.reasons = surfacing_reasons(record, relation_count, &query);
        }
        Ok(page)
    }

    fn show(&self, record_id: &str) -> Result<SearchRecordDetail, ApplicationError> {
        let connection = self.open_existing()?;
        load_detail(&connection, record_id)
    }

    fn traverse(&self, record_id: &str) -> Result<Vec<SearchRecordDetail>, ApplicationError> {
        let connection = self.open_existing()?;
        ensure_record_exists(&connection, record_id)?;
        let mut statement = connection
            .prepare(
                "SELECT DISTINCT related_record_id FROM search_relations
                 WHERE record_id = ?1 AND related_record_id IS NOT NULL AND broken = 0
                 ORDER BY related_record_id",
            )
            .map_err(storage)?;
        let ids = statement
            .query_map([record_id], |row| row.get::<_, String>(0))
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        drop(statement);
        ids.iter().map(|id| load_detail(&connection, id)).collect()
    }

    fn status(&self) -> Result<ProjectionStatus, ApplicationError> {
        if !self.paths.search_projection_database().exists() {
            return Ok(ProjectionStatus {
                state: "missing".to_owned(),
                schema_version: 0,
                built_at: None,
                raw_items: 0,
                semantic_entries: 0,
                relations: 0,
                source_fingerprint: None,
            });
        }
        let connection = self.open_existing()?;
        let schema_version = connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM projection_schema_migrations",
                [],
                |row| row.get::<_, u32>(0),
            )
            .map_err(storage)?;
        let metadata = connection
            .query_row(
                "SELECT built_at, raw_items, semantic_entries, relations, source_fingerprint
                 FROM projection_metadata WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, u64>(1)?,
                        row.get::<_, u64>(2)?,
                        row.get::<_, u64>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(storage)?;
        match metadata {
            Some((built_at, raw_items, semantic_entries, relations, fingerprint)) => {
                Ok(ProjectionStatus {
                    state: "ready".to_owned(),
                    schema_version,
                    built_at: Some(parse_timestamp(&built_at)?),
                    raw_items,
                    semantic_entries,
                    relations,
                    source_fingerprint: Some(fingerprint),
                })
            }
            None => Ok(ProjectionStatus {
                state: "unbuilt".to_owned(),
                schema_version,
                built_at: None,
                raw_items: 0,
                semantic_entries: 0,
                relations: 0,
                source_fingerprint: None,
            }),
        }
    }
}

fn migrate_projection(connection: &Connection) -> Result<(), ApplicationError> {
    let version = connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM projection_schema_migrations",
            [],
            |row| row.get::<_, u32>(0),
        )
        .unwrap_or(0);
    if version == 0 {
        connection
            .execute_batch(PROJECTION_MIGRATION)
            .map_err(storage)?;
        connection
            .execute(
                "INSERT INTO projection_schema_migrations
                 (version, name, applied_at, checksum_sha256)
                 VALUES (1, '0001_search_projection.sql', ?1, ?2)",
                params![now()?, super::migration_checksum(PROJECTION_MIGRATION)],
            )
            .map_err(storage)?;
    } else if version > PROJECTION_SCHEMA_VERSION {
        return Err(ApplicationError::Integrity(format!(
            "unsupported search projection schema version {version}"
        )));
    } else {
        let recorded = connection
            .query_row(
                "SELECT checksum_sha256 FROM projection_schema_migrations WHERE version = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .map_err(storage)?;
        if !super::migration_checksum_matches(&recorded, PROJECTION_MIGRATION) {
            return Err(ApplicationError::Integrity(
                "search projection migration checksum changed: 0001_search_projection.sql"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn attach_authorities(connection: &Connection, paths: &DataPaths) -> Result<(), ApplicationError> {
    connection
        .execute(
            "ATTACH DATABASE ?1 AS raw",
            [paths.raw_database().to_string_lossy().as_ref()],
        )
        .map_err(storage)?;
    connection
        .execute(
            "ATTACH DATABASE ?1 AS derived",
            [paths.derived_database().to_string_lossy().as_ref()],
        )
        .map_err(storage)?;
    Ok(())
}

fn clear_projection(transaction: &Transaction<'_>) -> Result<(), ApplicationError> {
    transaction
        .execute_batch(
            "DELETE FROM search_records_fts;
             DELETE FROM search_relations;
             DELETE FROM search_derivatives;
             DELETE FROM search_assets;
             DELETE FROM search_revisions;
             DELETE FROM search_scores;
             DELETE FROM search_tags;
             DELETE FROM search_maps;
             DELETE FROM search_people;
             DELETE FROM search_records;
             DELETE FROM projection_metadata;",
        )
        .map_err(storage)
}

#[allow(clippy::too_many_lines)]
fn populate_records(transaction: &Transaction<'_>) -> Result<(), ApplicationError> {
    transaction
        .execute_batch(
            "INSERT INTO search_records
                (record_id, record_kind, item_id, revision_id, semantic_id, source_id,
                 source_kind, provider, content_type, semantic_kind, realm, title,
                 body_text, state, processing_state, origin_kind, review_state, event_at,
                 restricted, missing, media_only, attachment_only, human_judgment,
                 confirmed_fact, metadata_json)
             SELECT
                'item:' || item.item_id,
                'raw_item', item.item_id,
                (SELECT revision_id FROM raw.revisions revision
                 WHERE revision.item_id = item.item_id
                 ORDER BY ordinal DESC LIMIT 1),
                NULL, source.source_id, source.source_kind, source.provider,
                item.content_type, NULL, NULL,
                COALESCE(json_extract(item.common_metadata_json, '$.title'),
                         source.display_name, source.provider || ' / ' || item.item_id),
                trim(
                    COALESCE((SELECT group_concat(raw_text, char(10)) FROM
                        (SELECT raw_text FROM raw.revisions revision
                         WHERE revision.item_id = item.item_id
                           AND revision.state = 'ready' AND revision.raw_text IS NOT NULL
                         ORDER BY revision.ordinal)), '') || char(10) ||
                    COALESCE((SELECT group_concat(content, char(10)) FROM
                        (SELECT COALESCE(derivative.content_text, derivative.content_json, '') AS content
                         FROM derived.process_runs run
                         JOIN derived.derivatives derivative ON derivative.run_id = run.run_id
                         WHERE run.invalidated_at IS NULL
                           AND (run.input_item_id = item.item_id OR run.input_revision_id IN
                                (SELECT revision_id FROM raw.revisions source_revision
                                 WHERE source_revision.item_id = item.item_id))
                         ORDER BY derivative.created_at, derivative.derivative_id)), '')
                ),
                COALESCE((SELECT state FROM raw.revisions revision
                          WHERE revision.item_id = item.item_id
                          ORDER BY ordinal DESC LIMIT 1), 'missing'),
                COALESCE((SELECT CASE WHEN run.invalidated_at IS NULL THEN run.state
                                      ELSE 'invalidated' END
                          FROM derived.process_runs run
                          WHERE run.input_item_id = item.item_id OR run.input_revision_id IN
                                (SELECT revision_id FROM raw.revisions source_revision
                                 WHERE source_revision.item_id = item.item_id)
                          ORDER BY run.created_at DESC, run.run_id DESC LIMIT 1),
                         'not_processed'),
                source.source_kind, NULL,
                COALESCE(item.source_published_at, item.source_updated_at,
                         item.first_captured_at),
                CASE WHEN COALESCE(
                    (SELECT CASE WHEN observation.recollection_state IN ('inaccessible', 'removed')
                                 THEN observation.recollection_state
                                 ELSE json_extract(observation.common_metadata_json,
                                                   '$.access_state') END
                     FROM raw.source_observations observation
                     WHERE observation.item_id = item.item_id
                     ORDER BY observation.observed_at DESC, observation.observation_id DESC LIMIT 1),
                    json_extract(item.common_metadata_json, '$.access_state'), 'unknown')
                    IN ('restricted', 'inaccessible') THEN 1 ELSE 0 END,
                CASE WHEN NOT EXISTS (SELECT 1 FROM raw.revisions revision
                                      WHERE revision.item_id = item.item_id) THEN 1 ELSE 0 END,
                CASE WHEN NOT EXISTS (SELECT 1 FROM raw.revisions revision
                                      WHERE revision.item_id = item.item_id
                                        AND revision.state = 'ready'
                                        AND revision.raw_text IS NOT NULL
                                        AND length(trim(revision.raw_text)) > 0)
                          AND (item.content_type IN ('image', 'audio', 'video')
                               OR COALESCE(json_array_length(
                                   item.common_metadata_json, '$.media.entries'), 0) > 0)
                     THEN 1 ELSE 0 END,
                CASE WHEN NOT EXISTS (SELECT 1 FROM raw.revisions revision
                                      WHERE revision.item_id = item.item_id
                                        AND revision.state = 'ready'
                                        AND revision.raw_text IS NOT NULL
                                        AND length(trim(revision.raw_text)) > 0)
                          AND EXISTS (SELECT 1 FROM raw.assets asset
                                      JOIN raw.revisions revision
                                        ON revision.revision_id = asset.revision_id
                                      WHERE revision.item_id = item.item_id
                                        AND asset.asset_role = 'attachment')
                     THEN 1 ELSE 0 END,
                CASE WHEN source.source_kind = 'first_party' THEN 1 ELSE 0 END,
                0,
                json_object('common', json(item.common_metadata_json),
                            'current_common', COALESCE(
                                (SELECT json(observation.common_metadata_json)
                                 FROM raw.source_observations observation
                                 WHERE observation.item_id = item.item_id
                                 ORDER BY observation.observed_at DESC,
                                          observation.observation_id DESC LIMIT 1),
                                json(item.common_metadata_json)),
                            'item', json(item.metadata_json),
                            'source_locator', item.source_locator,
                            'source_native_id', item.source_native_id,
                            'observation_reason',
                                (SELECT observation.reason
                                 FROM raw.source_observations observation
                                 WHERE observation.item_id = item.item_id
                                 ORDER BY observation.observed_at DESC,
                                          observation.observation_id DESC LIMIT 1),
                            'access_state', COALESCE(
                                (SELECT CASE WHEN observation.recollection_state
                                                   IN ('inaccessible', 'removed')
                                             THEN observation.recollection_state
                                             ELSE json_extract(
                                                 observation.common_metadata_json,
                                                 '$.access_state') END
                                 FROM raw.source_observations observation
                                 WHERE observation.item_id = item.item_id
                                 ORDER BY observation.observed_at DESC,
                                          observation.observation_id DESC LIMIT 1),
                                json_extract(item.common_metadata_json, '$.access_state'),
                                'unknown'))
             FROM raw.items item
             JOIN raw.sources source ON source.source_id = item.source_id;

             INSERT INTO search_records
                (record_id, record_kind, item_id, revision_id, semantic_id, source_id,
                 source_kind, provider, content_type, semantic_kind, realm, title,
                 body_text, state, processing_state, origin_kind, review_state, event_at,
                 restricted, missing, media_only, attachment_only, human_judgment,
                 confirmed_fact, metadata_json)
             SELECT
                'semantic:' || entry.semantic_id,
                'semantic_entry', item.item_id,
                COALESCE(entry.source_revision_id, entry.first_party_revision_id),
                entry.semantic_id, source.source_id, source.source_kind, source.provider,
                item.content_type, entry.semantic_kind, entry.realm, entry.title,
                trim(entry.payload_json || char(10) ||
                    COALESCE((SELECT group_concat(expression.content_text, char(10))
                              FROM raw.dense_expressions expression
                              WHERE expression.semantic_id = entry.semantic_id), '')),
                COALESCE(revision.state, 'missing'),
                CASE WHEN entry.origin_kind = 'first_party' THEN 'not_processed'
                     ELSE COALESCE((SELECT CASE WHEN run.invalidated_at IS NULL THEN run.state
                                               ELSE 'invalidated' END
                                    FROM raw.model_suggestions suggestion
                                    JOIN derived.derivatives derivative
                                      ON derivative.derivative_id = suggestion.source_derivative_id
                                    JOIN derived.process_runs run ON run.run_id = derivative.run_id
                                    WHERE suggestion.suggestion_id = entry.suggestion_id
                                    ORDER BY run.created_at DESC LIMIT 1), 'missing') END,
                entry.origin_kind,
                CASE WHEN entry.origin_kind = 'first_party' THEN 'first_party'
                     ELSE COALESCE((SELECT review.decision FROM raw.suggestion_reviews review
                                    WHERE review.suggestion_id = entry.suggestion_id
                                    ORDER BY review.created_at DESC, review.review_id DESC LIMIT 1),
                                   'unreviewed') END,
                entry.created_at,
                CASE WHEN COALESCE(
                    (SELECT CASE WHEN observation.recollection_state IN ('inaccessible', 'removed')
                                 THEN observation.recollection_state
                                 ELSE json_extract(observation.common_metadata_json,
                                                   '$.access_state') END
                     FROM raw.source_observations observation
                     WHERE observation.item_id = item.item_id
                     ORDER BY observation.observed_at DESC, observation.observation_id DESC LIMIT 1),
                    json_extract(item.common_metadata_json, '$.access_state'), 'unknown')
                    IN ('restricted', 'inaccessible') THEN 1 ELSE 0 END,
                CASE WHEN revision.revision_id IS NULL THEN 1
                     WHEN entry.origin_kind = 'machine' AND NOT EXISTS (
                         SELECT 1 FROM raw.model_suggestions suggestion
                         JOIN derived.derivatives derivative
                           ON derivative.derivative_id = suggestion.source_derivative_id
                         JOIN derived.process_runs run ON run.run_id = derivative.run_id
                         WHERE suggestion.suggestion_id = entry.suggestion_id
                           AND run.invalidated_at IS NULL)
                     THEN 1 ELSE 0 END,
                0, 0,
                CASE WHEN entry.origin_kind = 'first_party' THEN 1 ELSE 0 END,
                0,
                json_object('payload', json(entry.payload_json),
                            'current_common', COALESCE(
                                (SELECT json(observation.common_metadata_json)
                                 FROM raw.source_observations observation
                                 WHERE observation.item_id = item.item_id
                                 ORDER BY observation.observed_at DESC,
                                          observation.observation_id DESC LIMIT 1),
                                json(item.common_metadata_json)),
                            'suggestion_id', entry.suggestion_id,
                            'author', entry.author,
                            'source_locator', item.source_locator,
                            'source_native_id', item.source_native_id,
                            'observation_reason',
                                (SELECT observation.reason
                                 FROM raw.source_observations observation
                                 WHERE observation.item_id = item.item_id
                                 ORDER BY observation.observed_at DESC,
                                          observation.observation_id DESC LIMIT 1),
                            'access_state', COALESCE(
                                (SELECT CASE WHEN observation.recollection_state
                                                   IN ('inaccessible', 'removed')
                                             THEN observation.recollection_state
                                             ELSE json_extract(
                                                 observation.common_metadata_json,
                                                 '$.access_state') END
                                 FROM raw.source_observations observation
                                 WHERE observation.item_id = item.item_id
                                 ORDER BY observation.observed_at DESC,
                                          observation.observation_id DESC LIMIT 1),
                                json_extract(item.common_metadata_json, '$.access_state'),
                                'unknown'))
             FROM raw.semantic_entries entry
             JOIN raw.items item
               ON item.item_id = COALESCE(entry.source_item_id, entry.first_party_item_id)
             JOIN raw.sources source ON source.source_id = item.source_id
             LEFT JOIN raw.revisions revision
               ON revision.revision_id = COALESCE(entry.source_revision_id,
                                                  entry.first_party_revision_id);",
        )
        .map_err(storage)
}

#[allow(clippy::too_many_lines)]
fn populate_facets(transaction: &Transaction<'_>) -> Result<(), ApplicationError> {
    transaction
        .execute_batch(
            "INSERT OR IGNORE INTO search_people(record_id, person)
             SELECT record.record_id, json_extract(author.value, '$.display_name')
             FROM search_records record
             JOIN raw.items item ON item.item_id = record.item_id
             JOIN json_each(item.common_metadata_json, '$.authors') author
             WHERE length(trim(json_extract(author.value, '$.display_name'))) > 0;

             INSERT OR IGNORE INTO search_people(record_id, person)
             SELECT 'semantic:' || semantic_id, author FROM raw.semantic_entries;

             INSERT OR IGNORE INTO search_maps(record_id, map_node_id, name, level, lifecycle)
             SELECT 'semantic:' || assignment.semantic_id, node.map_node_id, node.name,
                    node.node_level, node.lifecycle_state
             FROM raw.semantic_map_assignments assignment
             JOIN raw.knowledge_map_nodes node ON node.map_node_id = assignment.map_node_id;

             INSERT OR IGNORE INTO search_maps(record_id, map_node_id, name, level, lifecycle)
             SELECT 'item:' || COALESCE(entry.source_item_id, entry.first_party_item_id),
                    node.map_node_id, node.name, node.node_level, node.lifecycle_state
             FROM raw.semantic_map_assignments assignment
             JOIN raw.semantic_entries entry ON entry.semantic_id = assignment.semantic_id
             JOIN raw.knowledge_map_nodes node ON node.map_node_id = assignment.map_node_id;

             INSERT OR IGNORE INTO search_tags(record_id, tag)
             SELECT 'semantic:' || assignment.semantic_id, tag.display_name
             FROM raw.semantic_tag_assignments assignment
             JOIN raw.semantic_tags tag ON tag.tag_id = assignment.tag_id;

             INSERT OR IGNORE INTO search_tags(record_id, tag)
             SELECT 'item:' || COALESCE(entry.source_item_id, entry.first_party_item_id),
                    tag.display_name
             FROM raw.semantic_tag_assignments assignment
             JOIN raw.semantic_entries entry ON entry.semantic_id = assignment.semantic_id
             JOIN raw.semantic_tags tag ON tag.tag_id = assignment.tag_id;

             INSERT OR IGNORE INTO search_scores
                (record_id, score_id, target_id, profile_id, profile_ordinal,
                 interest_weight, strategy_weight, consensus_weight,
                 interest, strategy, consensus, weighted_score, rationale,
                 provenance_kind, author, created_at, eligible_for_surface)
             SELECT 'semantic:' || score.target_id, score.score_id, score.target_id,
                    score.profile_id, profile.ordinal, profile.interest_weight,
                    profile.strategy_weight, profile.consensus_weight, score.interest,
                    score.strategy, score.consensus, score.weighted_score, score.rationale,
                    score.provenance_kind, score.author, score.created_at,
                    CASE WHEN entry.origin_kind = 'first_party' OR COALESCE(
                        (SELECT review.decision FROM raw.suggestion_reviews review
                         WHERE review.suggestion_id = entry.suggestion_id
                         ORDER BY review.created_at DESC, review.review_id DESC LIMIT 1),
                        'unreviewed') IN ('unreviewed', 'accepted') THEN 1 ELSE 0 END
             FROM raw.relevance_scores score
             JOIN raw.semantic_entries entry ON entry.semantic_id = score.target_id
             JOIN raw.score_profiles profile ON profile.profile_id = score.profile_id
             WHERE score.target_kind = 'semantic';

             INSERT OR IGNORE INTO search_scores
                (record_id, score_id, target_id, profile_id, profile_ordinal,
                 interest_weight, strategy_weight, consensus_weight,
                 interest, strategy, consensus, weighted_score, rationale,
                 provenance_kind, author, created_at, eligible_for_surface)
             SELECT 'item:' || COALESCE(entry.source_item_id, entry.first_party_item_id),
                    score.score_id, score.target_id, score.profile_id, profile.ordinal,
                    profile.interest_weight, profile.strategy_weight,
                    profile.consensus_weight, score.interest, score.strategy,
                    score.consensus, score.weighted_score, score.rationale,
                    score.provenance_kind, score.author, score.created_at,
                    CASE WHEN entry.origin_kind = 'first_party' OR COALESCE(
                        (SELECT review.decision FROM raw.suggestion_reviews review
                         WHERE review.suggestion_id = entry.suggestion_id
                         ORDER BY review.created_at DESC, review.review_id DESC LIMIT 1),
                        'unreviewed') IN ('unreviewed', 'accepted') THEN 1 ELSE 0 END
             FROM raw.relevance_scores score
             JOIN raw.semantic_entries entry ON entry.semantic_id = score.target_id
             JOIN raw.score_profiles profile ON profile.profile_id = score.profile_id
             WHERE score.target_kind = 'semantic';

             INSERT OR IGNORE INTO search_revisions
                (record_id, revision_id, parent_revision_id, ordinal, kind, state,
                 captured_at, authored_at, text_sha256)
             SELECT record.record_id, revision.revision_id, revision.parent_revision_id,
                    revision.ordinal, revision.revision_kind, revision.state,
                    revision.captured_at, revision.authored_at, revision.text_sha256
             FROM search_records record
             JOIN raw.revisions revision ON revision.item_id = record.item_id;

             INSERT OR IGNORE INTO search_assets
                (record_id, asset_id, revision_id, role, logical_path, media_type,
                 state, missing)
             SELECT record.record_id, asset.asset_id, asset.revision_id, asset.asset_role,
                    asset.logical_path, asset.media_type, asset.state, 0
             FROM search_records record
             JOIN raw.revisions revision ON revision.item_id = record.item_id
             JOIN raw.assets asset ON asset.revision_id = revision.revision_id;

             INSERT OR IGNORE INTO search_derivatives
                (record_id, derivative_id, run_id, revision_id, kind, processing_state,
                 output_sha256, logical_path, media_type, invalidated, missing, created_at)
             SELECT record.record_id, derivative.derivative_id, run.run_id,
                    run.input_revision_id, derivative.kind, run.state,
                    derivative.output_sha256, derivative.logical_path,
                    derivative.media_type, CASE WHEN run.invalidated_at IS NULL THEN 0 ELSE 1 END,
                    0, derivative.created_at
             FROM search_records record
             JOIN derived.process_runs run
               ON run.input_item_id = record.item_id OR run.input_revision_id IN
                  (SELECT revision_id FROM raw.revisions source_revision
                   WHERE source_revision.item_id = record.item_id)
             JOIN derived.derivatives derivative ON derivative.run_id = run.run_id;

             INSERT OR IGNORE INTO search_relations
                (relation_key, record_id, direction, relation_kind, related_record_id,
                 related_entity_id, related_title, evidence, broken)
             SELECT 'raw:out:' || relation.relation_id, 'item:' || relation.from_item_id,
                    'outgoing', relation.relation_kind, 'item:' || relation.to_item_id,
                    relation.to_item_id, NULL, relation.metadata_json, 0
             FROM raw.relations relation;

             INSERT OR IGNORE INTO search_relations
                (relation_key, record_id, direction, relation_kind, related_record_id,
                 related_entity_id, related_title, evidence, broken)
             SELECT 'raw:in:' || relation.relation_id, 'item:' || relation.to_item_id,
                    'incoming', relation.relation_kind, 'item:' || relation.from_item_id,
                    relation.from_item_id, NULL, relation.metadata_json, 0
             FROM raw.relations relation;

             INSERT OR IGNORE INTO search_relations
                (relation_key, record_id, direction, relation_kind, related_record_id,
                 related_entity_id, related_title, evidence, broken)
             SELECT 'semantic:out:' || relation.semantic_relation_id,
                    'semantic:' || relation.from_semantic_id, 'outgoing',
                    relation.relation_kind, 'semantic:' || relation.to_semantic_id,
                    relation.to_semantic_id, NULL, relation.evidence, 0
             FROM raw.semantic_relations relation;

             INSERT OR IGNORE INTO search_relations
                (relation_key, record_id, direction, relation_kind, related_record_id,
                 related_entity_id, related_title, evidence, broken)
             SELECT 'semantic:in:' || relation.semantic_relation_id,
                    'semantic:' || relation.to_semantic_id, 'incoming',
                    relation.relation_kind, 'semantic:' || relation.from_semantic_id,
                    relation.from_semantic_id, NULL, relation.evidence, 0
             FROM raw.semantic_relations relation;

             INSERT OR IGNORE INTO search_relations
                (relation_key, record_id, direction, relation_kind, related_record_id,
                 related_entity_id, related_title, evidence, broken)
             SELECT 'source:out:' || semantic_id, 'semantic:' || semantic_id,
                    'outgoing', 'source_record',
                    'item:' || COALESCE(source_item_id, first_party_item_id),
                    COALESCE(source_item_id, first_party_item_id), NULL,
                    'semantic entry source record', 0
             FROM raw.semantic_entries;

             INSERT OR IGNORE INTO search_relations
                (relation_key, record_id, direction, relation_kind, related_record_id,
                 related_entity_id, related_title, evidence, broken)
             SELECT 'source:in:' || semantic_id,
                    'item:' || COALESCE(source_item_id, first_party_item_id),
                    'incoming', 'semantic_entry', 'semantic:' || semantic_id,
                    semantic_id, title, 'semantic entry derived from this record', 0
             FROM raw.semantic_entries;

             UPDATE search_relations
             SET related_title = (SELECT title FROM search_records related
                                  WHERE related.record_id = search_relations.related_record_id),
                 broken = CASE WHEN related_record_id IS NULL OR NOT EXISTS (
                                   SELECT 1 FROM search_records related
                                   WHERE related.record_id = search_relations.related_record_id)
                               THEN 1 ELSE 0 END;",
        )
        .map_err(storage)
}

fn mark_missing_files(
    transaction: &Transaction<'_>,
    paths: &DataPaths,
) -> Result<(), ApplicationError> {
    let assets = {
        let mut statement = transaction
            .prepare("SELECT record_id, asset_id, logical_path FROM search_assets")
            .map_err(storage)?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?
    };
    for (record_id, asset_id, logical_path) in assets {
        if !logical_file_exists(paths.root(), &logical_path) {
            transaction
                .execute(
                    "UPDATE search_assets SET missing = 1
                     WHERE record_id = ?1 AND asset_id = ?2",
                    params![record_id, asset_id],
                )
                .map_err(storage)?;
        }
    }

    let derivatives = {
        let mut statement = transaction
            .prepare(
                "SELECT record_id, derivative_id, logical_path FROM search_derivatives
                 WHERE logical_path IS NOT NULL",
            )
            .map_err(storage)?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?
    };
    for (record_id, derivative_id, logical_path) in derivatives {
        if !logical_file_exists(paths.root(), &logical_path) {
            transaction
                .execute(
                    "UPDATE search_derivatives SET missing = 1
                     WHERE record_id = ?1 AND derivative_id = ?2",
                    params![record_id, derivative_id],
                )
                .map_err(storage)?;
        }
    }
    transaction
        .execute(
            "UPDATE search_records SET missing = 1
             WHERE EXISTS (SELECT 1 FROM search_assets asset
                           WHERE asset.record_id = search_records.record_id
                             AND asset.missing = 1)
                OR EXISTS (SELECT 1 FROM search_derivatives derivative
                           WHERE derivative.record_id = search_records.record_id
                             AND derivative.missing = 1)",
            [],
        )
        .map_err(storage)?;
    Ok(())
}

fn logical_file_exists(root: &Path, logical_path: &str) -> bool {
    LogicalPath::parse(logical_path).is_ok_and(|logical| {
        DataPaths::new(root.to_path_buf())
            .resolve_logical(&logical)
            .is_ok_and(|resolved| resolved.is_file())
    })
}

fn populate_fts(transaction: &Transaction<'_>) -> Result<(), ApplicationError> {
    transaction
        .execute(
            "INSERT INTO search_records_fts(record_id, title, body_text, facets)
             SELECT record.record_id, record.title, record.body_text,
                    trim(record.provider || ' ' || record.source_kind || ' ' ||
                         record.content_type || ' ' || COALESCE(record.semantic_kind, '') || ' ' ||
                         COALESCE(record.realm, '') || ' ' ||
                         COALESCE((SELECT group_concat(person, ' ') FROM search_people person
                                   WHERE person.record_id = record.record_id), '') || ' ' ||
                         COALESCE((SELECT group_concat(name, ' ') FROM search_maps map
                                   WHERE map.record_id = record.record_id), '') || ' ' ||
                         COALESCE((SELECT group_concat(tag, ' ') FROM search_tags tag
                                   WHERE tag.record_id = record.record_id), ''))
             FROM search_records record",
            [],
        )
        .map_err(storage)?;
    Ok(())
}

fn write_metadata(transaction: &Transaction<'_>) -> Result<(), ApplicationError> {
    let (raw_items, semantic_entries, _raw_relations, _semantic_relations, _derivatives): (
        u64,
        u64,
        u64,
        u64,
        u64,
    ) = transaction
        .query_row(
            "SELECT (SELECT COUNT(*) FROM raw.items),
                    (SELECT COUNT(*) FROM raw.semantic_entries),
                    (SELECT COUNT(*) FROM raw.relations),
                    (SELECT COUNT(*) FROM raw.semantic_relations),
                    (SELECT COUNT(*) FROM derived.derivatives)",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(storage)?;
    let fingerprint = source_fingerprint(transaction)?;
    let projected_relations: u64 = transaction
        .query_row("SELECT COUNT(*) FROM search_relations", [], |row| {
            row.get(0)
        })
        .map_err(storage)?;
    transaction
        .execute(
            "INSERT INTO projection_metadata
             (singleton, built_at, raw_items, semantic_entries, relations, source_fingerprint)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![
                now()?,
                raw_items,
                semantic_entries,
                projected_relations,
                fingerprint
            ],
        )
        .map_err(storage)?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn source_fingerprint(transaction: &Transaction<'_>) -> Result<String, ApplicationError> {
    let mut statement = transaction
        .prepare(
            "SELECT kind, identity, value_a, value_b, value_c FROM (
                 SELECT 'source' AS kind, source_id AS identity,
                        source_kind AS value_a, provider AS value_b,
                        json_array(display_name, account_or_workspace, base_locator,
                                   metadata_json, created_at) AS value_c
                 FROM raw.sources
                 UNION ALL
                 SELECT 'item' AS kind, item_id AS identity,
                        common_metadata_json AS value_a, metadata_json AS value_b,
                        json_array(source_id, source_native_id, source_locator,
                                   source_identity_key, content_type, source_published_at,
                                   source_updated_at, first_captured_at) AS value_c
                 FROM raw.items
                 UNION ALL
                 SELECT 'revision', revision_id, COALESCE(text_sha256, raw_text, ''),
                        json_array(item_id, parent_revision_id, revision_kind, ordinal,
                                   captured_at, authored_at, state), metadata_json
                        FROM raw.revisions
                 UNION ALL
                 SELECT 'asset', asset_id, sha256,
                        json_array(revision_id, asset_role, logical_path, byte_size,
                                   media_type, original_filename, state), created_at
                        FROM raw.assets
                 UNION ALL
                 SELECT 'observation', observation_id, common_metadata_json,
                        provider_metadata_json,
                        json_array(item_id, revision_id, observation_kind,
                                   recollection_state, source_native_id, source_locator,
                                   context, reason, observed_at)
                        FROM raw.source_observations
                 UNION ALL
                 SELECT 'semantic', semantic_id, payload_json, title,
                        json_array(semantic_kind, realm, origin_kind, author,
                                   source_item_id, source_revision_id, first_party_item_id,
                                   first_party_revision_id, suggestion_id, created_at)
                        FROM raw.semantic_entries
                 UNION ALL
                 SELECT 'map', map_node_id, name, lifecycle_state, canonical_key
                        FROM raw.knowledge_map_nodes
                 UNION ALL
                 SELECT 'map_edge', parent_node_id || ':' || child_node_id,
                        map_version_id, provenance_kind,
                        json_array(suggestion_id, created_at)
                        FROM raw.knowledge_map_edges
                 UNION ALL
                 SELECT 'map_assignment', semantic_id || ':' || map_node_id,
                        provenance_kind, COALESCE(suggestion_id, ''), created_at
                        FROM raw.semantic_map_assignments
                 UNION ALL
                 SELECT 'tag', tag_id, canonical_name, display_name, created_at
                        FROM raw.semantic_tags
                 UNION ALL
                 SELECT 'tag_assignment', semantic_id || ':' || tag_id,
                        provenance_kind, COALESCE(suggestion_id, ''), created_at
                        FROM raw.semantic_tag_assignments
                 UNION ALL
                 SELECT 'dense_expression', expression_id, expression_kind, content_text,
                        json_array(semantic_id, provenance_kind, suggestion_id, created_at)
                        FROM raw.dense_expressions
                 UNION ALL
                 SELECT 'semantic_relation', semantic_relation_id, relation_kind,
                        from_semantic_id || ':' || to_semantic_id,
                        json_array(evidence, provenance_kind, suggestion_id, created_at)
                        FROM raw.semantic_relations
                 UNION ALL
                 SELECT 'score', score_id, profile_id,
                        json_array(target_kind, target_id, interest, strategy, consensus,
                                   weighted_score),
                        json_array(rationale, provenance_kind, author, suggestion_id,
                                   created_at) FROM raw.relevance_scores
                 UNION ALL
                 SELECT 'profile', profile_id,
                        json_array(ordinal, interest_weight, strategy_weight,
                                   consensus_weight), rationale,
                        json_array(author_kind, author, created_at)
                        FROM raw.score_profiles
                 UNION ALL
                 SELECT 'review', review_id, suggestion_id, decision,
                        json_array(reason, first_party_item_id, first_party_revision_id,
                                   reviewer, created_at)
                        FROM raw.suggestion_reviews
                 UNION ALL
                 SELECT 'raw_relation', relation_id, relation_kind,
                        json_array(from_item_id, from_revision_id, to_item_id, to_revision_id),
                        metadata_json FROM raw.relations
                 UNION ALL
                 SELECT 'run', run_id, state,
                        json_array(input_item_id, input_revision_id, input_asset_id,
                                   input_sha256, provider, tool_or_model, created_at),
                        json_array(invalidated_at, invalidation_reason)
                        FROM derived.process_runs
                 UNION ALL
                 SELECT 'derivative', derivative_id, COALESCE(output_sha256, ''),
                        COALESCE(content_text, content_json, ''),
                        json_array(run_id, kind, logical_path, media_type, created_at)
                        FROM derived.derivatives
             ) ORDER BY kind, identity",
        )
        .map_err(storage)?;
    let mut rows = statement.query([]).map_err(storage)?;
    let mut hasher = Sha256::new();
    while let Some(row) = rows.next().map_err(storage)? {
        for column in 0..5 {
            let value = row.get::<_, String>(column).map_err(storage)?;
            hasher.update(value.len().to_le_bytes());
            hasher.update(value.as_bytes());
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
struct FlatRecord {
    record_id: String,
    record_kind: String,
    item_id: Option<String>,
    revision_id: Option<String>,
    semantic_id: Option<String>,
    title: String,
    excerpt: Option<String>,
    source_kind: String,
    provider: String,
    content_type: String,
    semantic_kind: Option<String>,
    realm: Option<String>,
    state: String,
    processing_state: String,
    origin_kind: String,
    review_state: Option<String>,
    event_at: String,
    restricted: bool,
    missing: bool,
    media_only: bool,
    attachment_only: bool,
    human_judgment: bool,
    confirmed_fact: bool,
    access_state: String,
    source_id: String,
    metadata_json: String,
    score: Option<SearchScoreRef>,
}

#[allow(clippy::too_many_lines)]
fn search_connection(
    connection: &Connection,
    query: &SearchQuery,
    surface_only: bool,
) -> Result<SearchPage, ApplicationError> {
    let filter = &query.filter;
    validate_filter(filter)?;
    let limit = if filter.limit == 0 {
        DEFAULT_LIMIT
    } else {
        filter.limit.min(MAX_LIMIT)
    };
    let offset = query.cursor.as_ref().map_or(Ok(0_u64), |cursor| {
        u64::from_str(&cursor.0).map_err(|_| {
            ApplicationError::Integrity("search cursor must be a non-negative offset".to_owned())
        })
    })?;
    let profile_id = filter.profile_id.as_deref().unwrap_or(DEFAULT_PROFILE_ID);
    let text_query = filter
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(fts_query);

    let mut values = vec![Value::Text(profile_id.to_owned())];
    let mut joins = String::new();
    let mut conditions = Vec::new();
    if let Some(text) = &text_query {
        joins.push_str(
            " JOIN search_records_fts ON search_records_fts.record_id = record.record_id ",
        );
        conditions.push("search_records_fts MATCH ?".to_owned());
        values.push(Value::Text(text.clone()));
    }
    macro_rules! exact {
        ($value:expr, $sql:expr) => {
            if let Some(value) = $value {
                conditions.push($sql.to_owned());
                values.push(Value::Text(value));
            }
        };
    }
    exact!(filter.source_kind.map(wire), "record.source_kind = ?");
    exact!(
        filter.provider.clone(),
        "record.provider = ? COLLATE NOCASE"
    );
    exact!(filter.content_type.map(wire), "record.content_type = ?");
    exact!(filter.semantic_kind.map(wire), "record.semantic_kind = ?");
    exact!(filter.realm.map(wire), "record.realm = ?");
    exact!(filter.state.clone(), "record.state = ?");
    exact!(
        filter.access_state.clone(),
        "json_extract(record.metadata_json, '$.access_state') = ?"
    );
    exact!(
        filter.processing_state.clone(),
        "record.processing_state = ?"
    );
    exact!(filter.origin_kind.clone(), "record.origin_kind = ?");
    exact!(filter.review_state.clone(), "record.review_state = ?");
    if filter.profile_id.is_some() {
        conditions.push("score.profile_id IS NOT NULL".to_owned());
    }
    if let Some(from) = &filter.captured_from {
        conditions.push("record.event_at >= ?".to_owned());
        values.push(Value::Text(from.as_str().to_owned()));
    }
    if let Some(to) = &filter.captured_to {
        conditions.push("record.event_at <= ?".to_owned());
        values.push(Value::Text(to.as_str().to_owned()));
    }
    if let Some(person) = &filter.person {
        conditions.push(
            "EXISTS (SELECT 1 FROM search_people person
                     WHERE person.record_id = record.record_id
                       AND person.person LIKE ? ESCAPE '\\' COLLATE NOCASE)"
                .to_owned(),
        );
        values.push(Value::Text(format!("%{}%", escape_like(person))));
    }
    if let Some(map_node) = &filter.map_node {
        conditions.push(
            "EXISTS (SELECT 1 FROM search_maps map
                     WHERE map.record_id = record.record_id
                       AND (map.map_node_id = ? OR map.name = ? COLLATE NOCASE))"
                .to_owned(),
        );
        values.push(Value::Text(map_node.clone()));
        values.push(Value::Text(map_node.clone()));
    }
    if let Some(tag) = &filter.tag {
        conditions.push(
            "EXISTS (SELECT 1 FROM search_tags tag
                     WHERE tag.record_id = record.record_id
                       AND tag.tag = ? COLLATE NOCASE)"
                .to_owned(),
        );
        values.push(Value::Text(tag.clone()));
    }
    if let Some(kind) = &filter.relation_kind {
        conditions.push(
            "EXISTS (SELECT 1 FROM search_relations relation
                     WHERE relation.record_id = record.record_id
                       AND relation.relation_kind = ? COLLATE NOCASE)"
                .to_owned(),
        );
        values.push(Value::Text(kind.clone()));
    }
    if let Some(related_to) = &filter.related_to {
        conditions.push(
            "EXISTS (SELECT 1 FROM search_relations relation
                     WHERE relation.record_id = record.record_id
                       AND (relation.related_record_id = ? OR relation.related_entity_id = ?))"
                .to_owned(),
        );
        values.push(Value::Text(related_to.clone()));
        values.push(Value::Text(related_to.clone()));
    }
    for (value, column) in [
        (filter.restricted, "record.restricted"),
        (filter.missing, "record.missing"),
        (filter.media_only, "record.media_only"),
        (filter.attachment_only, "record.attachment_only"),
    ] {
        if let Some(value) = value {
            conditions.push(format!("{column} = ?"));
            values.push(Value::Integer(i64::from(value)));
        }
    }
    if surface_only {
        conditions.push("record.record_kind = 'semantic_entry'".to_owned());
        conditions.push("score.eligible_for_surface = 1".to_owned());
    }
    for (value, column) in [
        (filter.min_interest.map(u16::from), "score.interest"),
        (filter.min_strategy.map(u16::from), "score.strategy"),
        (filter.min_consensus.map(u16::from), "score.consensus"),
        (filter.min_weighted_score, "score.weighted_score"),
    ] {
        if let Some(value) = value {
            conditions.push(format!("{column} >= ?"));
            values.push(Value::Integer(i64::from(value)));
        }
    }

    let excerpt = if text_query.is_some() {
        "snippet(search_records_fts, 2, '', '', ' ... ', 24)"
    } else {
        "CASE WHEN length(record.body_text) > 240
              THEN substr(record.body_text, 1, 240) || ' ...'
              ELSE NULLIF(record.body_text, '') END"
    };
    let order = match filter.sort {
        SearchSort::Relevance if text_query.is_some() => {
            "bm25(search_records_fts), COALESCE(score.weighted_score, -1) DESC, record.event_at DESC, record.record_id"
        }
        SearchSort::Newest => "record.event_at DESC, record.record_id",
        SearchSort::Interest => {
            "COALESCE(score.interest, -1) DESC, record.event_at DESC, record.record_id"
        }
        SearchSort::Strategy => {
            "COALESCE(score.strategy, -1) DESC, record.event_at DESC, record.record_id"
        }
        SearchSort::Consensus => {
            "COALESCE(score.consensus, -1) DESC, record.event_at DESC, record.record_id"
        }
        SearchSort::Relevance | SearchSort::WeightedScore => {
            "COALESCE(score.weighted_score, -1) DESC, record.event_at DESC, record.record_id"
        }
    };
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };
    let score_eligibility = if surface_only {
        " AND candidate.eligible_for_surface = 1"
    } else {
        ""
    };
    let sql = format!(
        "SELECT record.record_id, record.record_kind, record.item_id, record.revision_id,
                record.semantic_id, record.title, {excerpt}, record.source_kind,
                record.provider, record.content_type, record.semantic_kind, record.realm,
                record.state, record.processing_state, record.origin_kind, record.review_state,
                record.event_at, record.restricted, record.missing, record.media_only,
                record.attachment_only, record.human_judgment, record.confirmed_fact,
                json_extract(record.metadata_json, '$.access_state'),
                score.score_id, score.profile_id, score.profile_ordinal,
                score.interest_weight, score.strategy_weight, score.consensus_weight,
                score.interest, score.strategy, score.consensus, score.weighted_score,
                score.rationale, score.provenance_kind, score.author, score.created_at,
                score.eligible_for_surface, record.source_id, record.metadata_json
         FROM search_records record
         LEFT JOIN search_scores score ON score.record_id = record.record_id
             AND score.score_id = (
             SELECT candidate.score_id FROM search_scores candidate
             WHERE candidate.record_id = record.record_id AND candidate.profile_id = ?
                   {score_eligibility}
             ORDER BY candidate.created_at DESC, candidate.score_id DESC LIMIT 1)
         {joins}{where_clause}
         ORDER BY {order} LIMIT ? OFFSET ?"
    );
    values.push(Value::Integer(i64::from(limit + 1)));
    values.push(Value::Integer(i64::try_from(offset).map_err(|_| {
        ApplicationError::Integrity("search cursor is too large".to_owned())
    })?));

    let mut statement = connection.prepare(&sql).map_err(storage)?;
    let mut flat = statement
        .query_map(params_from_iter(values), flat_record)
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    drop(statement);
    let has_more = flat.len() > limit as usize;
    flat.truncate(limit as usize);
    let mut records = Vec::with_capacity(flat.len());
    for record in flat {
        records.push(hydrate_summary(connection, record, filter)?);
    }
    Ok(SearchPage {
        records,
        next_cursor: has_more.then(|| PageCursor((offset + u64::from(limit)).to_string())),
    })
}

fn flat_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<FlatRecord> {
    let score_id = row.get::<_, Option<String>>(24)?;
    let score = score_id
        .map(|score_id| -> rusqlite::Result<SearchScoreRef> {
            Ok(SearchScoreRef {
                score_id,
                profile_id: row.get(25)?,
                profile_ordinal: row.get(26)?,
                interest_weight: row.get(27)?,
                strategy_weight: row.get(28)?,
                consensus_weight: row.get(29)?,
                interest: row.get(30)?,
                strategy: row.get(31)?,
                consensus: row.get(32)?,
                weighted_score: row.get(33)?,
                rationale: row.get(34)?,
                provenance_kind: row.get(35)?,
                author: row.get(36)?,
                created_at: parse_timestamp_sql(row.get::<_, String>(37)?)?,
                eligible_for_surface: row.get(38)?,
            })
        })
        .transpose()?;
    Ok(FlatRecord {
        record_id: row.get(0)?,
        record_kind: row.get(1)?,
        item_id: row.get(2)?,
        revision_id: row.get(3)?,
        semantic_id: row.get(4)?,
        title: row.get(5)?,
        excerpt: row.get(6)?,
        source_kind: row.get(7)?,
        provider: row.get(8)?,
        content_type: row.get(9)?,
        semantic_kind: row.get(10)?,
        realm: row.get(11)?,
        state: row.get(12)?,
        processing_state: row.get(13)?,
        origin_kind: row.get(14)?,
        review_state: row.get(15)?,
        event_at: row.get(16)?,
        restricted: row.get(17)?,
        missing: row.get(18)?,
        media_only: row.get(19)?,
        attachment_only: row.get(20)?,
        human_judgment: row.get(21)?,
        confirmed_fact: row.get(22)?,
        access_state: row.get(23)?,
        source_id: row.get(39)?,
        metadata_json: row.get(40)?,
        score,
    })
}

fn hydrate_summary(
    connection: &Connection,
    flat: FlatRecord,
    filter: &babata_domain::QueryFilter,
) -> Result<RecordSummary, ApplicationError> {
    let people = string_column(
        connection,
        "SELECT person FROM search_people WHERE record_id = ?1 ORDER BY person",
        &flat.record_id,
    )?;
    let tags = string_column(
        connection,
        "SELECT tag FROM search_tags WHERE record_id = ?1 ORDER BY tag",
        &flat.record_id,
    )?;
    let navigation = source_navigation_metadata(&flat.metadata_json)?;
    let mut markers = Vec::new();
    for (present, marker) in [
        (flat.restricted, SearchRecordMarker::Restricted),
        (flat.missing, SearchRecordMarker::Missing),
        (flat.media_only, SearchRecordMarker::MediaOnly),
        (flat.attachment_only, SearchRecordMarker::AttachmentOnly),
    ] {
        if present {
            markers.push(marker);
        }
    }
    let map_nodes = load_maps(connection, &flat.record_id)?;
    let mut reasons = Vec::new();
    if let Some(text) = &filter.text {
        reasons.push(SurfacingReason {
            kind: SurfacingReasonKind::TextMatch,
            explanation: format!("matched search text: {}", text.trim()),
            evidence: vec![flat.excerpt.clone().unwrap_or_default()],
        });
    }
    if structured_filter_count(filter) > 0 {
        reasons.push(SurfacingReason {
            kind: SurfacingReasonKind::FilterMatch,
            explanation: "matched all requested structured filters".to_owned(),
            evidence: vec![format!(
                "{} structured conditions",
                structured_filter_count(filter)
            )],
        });
    }
    Ok(RecordSummary {
        record_id: flat.record_id,
        record_kind: parse_wire(&flat.record_kind)?,
        item_id: flat.item_id.map(ItemId::parse).transpose()?,
        revision_id: flat.revision_id.map(RevisionId::parse).transpose()?,
        semantic_id: flat.semantic_id,
        source_id: SourceId::parse(flat.source_id)?,
        source_locator: navigation.source_locator,
        source_native_id: navigation.source_native_id,
        title: flat.title,
        excerpt: flat.excerpt,
        source_kind: parse_wire(&flat.source_kind)?,
        provider: flat.provider,
        content_type: parse_wire(&flat.content_type)?,
        semantic_kind: flat.semantic_kind.as_deref().map(parse_wire).transpose()?,
        realm: flat.realm.as_deref().map(parse_wire).transpose()?,
        state: flat.state,
        processing_state: flat.processing_state,
        origin_kind: flat.origin_kind,
        review_state: flat.review_state,
        access_state: flat.access_state,
        judgment: JudgmentStatus {
            human_judgment: flat.human_judgment,
            confirmed_fact: flat.confirmed_fact,
        },
        event_at: parse_timestamp(&flat.event_at)?,
        markers,
        limitations: navigation.limitations,
        people,
        map_nodes,
        tags,
        score: flat.score,
        reasons,
    })
}

fn load_detail(
    connection: &Connection,
    record_id: &str,
) -> Result<SearchRecordDetail, ApplicationError> {
    let query = SearchQuery {
        filter: babata_domain::QueryFilter {
            limit: 1,
            ..babata_domain::QueryFilter::default()
        },
        cursor: None,
    };
    let flat = load_flat_by_id(connection, record_id)?;
    let record = hydrate_summary(connection, flat, &query.filter)?;
    Ok(SearchRecordDetail {
        revisions: load_revisions(connection, record_id)?,
        assets: load_assets(connection, record_id)?,
        derivatives: load_derivatives(connection, record_id)?,
        relations: load_relations(connection, record_id)?,
        score_history: load_scores(connection, record_id)?,
        record,
    })
}

fn load_flat_by_id(
    connection: &Connection,
    record_id: &str,
) -> Result<FlatRecord, ApplicationError> {
    connection
        .query_row(
            "SELECT record.record_id, record.record_kind, record.item_id, record.revision_id,
                    record.semantic_id, record.title, NULLIF(record.body_text, ''),
                    record.source_kind, record.provider, record.content_type,
                    record.semantic_kind, record.realm, record.state, record.processing_state,
                    record.origin_kind, record.review_state, record.event_at,
                    record.restricted, record.missing, record.media_only,
                    record.attachment_only, record.human_judgment, record.confirmed_fact,
                    json_extract(record.metadata_json, '$.access_state'),
                    score.score_id, score.profile_id, score.profile_ordinal,
                    score.interest_weight, score.strategy_weight, score.consensus_weight,
                    score.interest, score.strategy, score.consensus, score.weighted_score,
                    score.rationale, score.provenance_kind, score.author, score.created_at,
                    score.eligible_for_surface, record.source_id, record.metadata_json
             FROM search_records record
             LEFT JOIN search_scores score ON score.record_id = record.record_id
                 AND score.score_id = (
                 SELECT candidate.score_id FROM search_scores candidate
                 WHERE candidate.record_id = record.record_id
                   AND candidate.profile_id = ?2
                 ORDER BY candidate.created_at DESC, candidate.score_id DESC LIMIT 1)
             WHERE record.record_id = ?1",
            params![record_id, DEFAULT_PROFILE_ID],
            flat_record,
        )
        .optional()
        .map_err(storage)?
        .ok_or_else(|| ApplicationError::NotFound(format!("search record {record_id}")))
}

fn ensure_record_exists(connection: &Connection, record_id: &str) -> Result<(), ApplicationError> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM search_records WHERE record_id = ?1",
            [record_id],
            |_| Ok(()),
        )
        .optional()
        .map_err(storage)?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(ApplicationError::NotFound(format!(
            "search record {record_id}"
        )))
    }
}

fn load_maps(
    connection: &Connection,
    record_id: &str,
) -> Result<Vec<SearchMapRef>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT map_node_id, name, level, lifecycle FROM search_maps
             WHERE record_id = ?1 ORDER BY level, name",
        )
        .map_err(storage)?;
    statement
        .query_map([record_id], |row| {
            Ok(SearchMapRef {
                map_node_id: row.get(0)?,
                name: row.get(1)?,
                level: row.get(2)?,
                lifecycle: row.get(3)?,
            })
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)
}

fn load_revisions(
    connection: &Connection,
    record_id: &str,
) -> Result<Vec<SearchRevisionRef>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT revision_id, parent_revision_id, ordinal, kind, state, captured_at,
                    authored_at, text_sha256 FROM search_revisions
             WHERE record_id = ?1 ORDER BY ordinal",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map([record_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    rows.into_iter()
        .map(|row| {
            Ok(SearchRevisionRef {
                revision_id: RevisionId::parse(row.0)?,
                parent_revision_id: row.1.map(RevisionId::parse).transpose()?,
                ordinal: row.2,
                kind: row.3,
                state: row.4,
                captured_at: parse_timestamp(&row.5)?,
                authored_at: row.6.as_deref().map(parse_timestamp).transpose()?,
                text_sha256: row.7,
            })
        })
        .collect()
}

fn load_assets(
    connection: &Connection,
    record_id: &str,
) -> Result<Vec<SearchAssetRef>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT asset_id, revision_id, role, logical_path, media_type, state, missing
             FROM search_assets WHERE record_id = ?1 ORDER BY asset_id",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map([record_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, bool>(6)?,
            ))
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    rows.into_iter()
        .map(|row| {
            Ok(SearchAssetRef {
                asset_id: row.0,
                revision_id: RevisionId::parse(row.1)?,
                role: row.2,
                logical_path: row.3,
                media_type: row.4,
                state: row.5,
                missing: row.6,
            })
        })
        .collect()
}

fn load_derivatives(
    connection: &Connection,
    record_id: &str,
) -> Result<Vec<SearchDerivativeRef>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT derivative_id, run_id, revision_id, kind, processing_state,
                    output_sha256, logical_path, media_type, invalidated, missing, created_at
             FROM search_derivatives WHERE record_id = ?1
             ORDER BY created_at, derivative_id",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map([record_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, bool>(8)?,
                row.get::<_, bool>(9)?,
                row.get::<_, String>(10)?,
            ))
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    rows.into_iter()
        .map(|row| {
            Ok(SearchDerivativeRef {
                derivative_id: DerivativeId::parse(row.0)?,
                run_id: row.1,
                revision_id: RevisionId::parse(row.2)?,
                kind: row.3,
                processing_state: row.4,
                output_sha256: row.5,
                logical_path: row.6,
                media_type: row.7,
                invalidated: row.8,
                missing: row.9,
                created_at: parse_timestamp(&row.10)?,
            })
        })
        .collect()
}

fn load_relations(
    connection: &Connection,
    record_id: &str,
) -> Result<Vec<SearchRelationRef>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT direction, relation_kind, related_record_id, related_entity_id,
                    related_title, evidence, broken
             FROM search_relations WHERE record_id = ?1
             ORDER BY relation_kind, direction, related_entity_id",
        )
        .map_err(storage)?;
    statement
        .query_map([record_id], |row| {
            Ok(SearchRelationRef {
                direction: row.get(0)?,
                relation_kind: row.get(1)?,
                related_record_id: row.get(2)?,
                related_entity_id: row.get(3)?,
                related_title: row.get(4)?,
                evidence: row.get(5)?,
                broken: row.get(6)?,
            })
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)
}

fn load_scores(
    connection: &Connection,
    record_id: &str,
) -> Result<Vec<SearchScoreRef>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT score_id, profile_id, profile_ordinal, interest_weight,
                    strategy_weight, consensus_weight, interest, strategy, consensus,
                    weighted_score, rationale, provenance_kind, author, created_at,
                    eligible_for_surface
             FROM search_scores WHERE record_id = ?1
             ORDER BY profile_ordinal, created_at, score_id",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map([record_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, u8>(3)?,
                row.get::<_, u8>(4)?,
                row.get::<_, u8>(5)?,
                row.get::<_, u8>(6)?,
                row.get::<_, u8>(7)?,
                row.get::<_, u8>(8)?,
                row.get::<_, u16>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, bool>(14)?,
            ))
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    rows.into_iter()
        .map(|row| {
            Ok(SearchScoreRef {
                score_id: row.0,
                profile_id: row.1,
                profile_ordinal: row.2,
                interest_weight: row.3,
                strategy_weight: row.4,
                consensus_weight: row.5,
                interest: row.6,
                strategy: row.7,
                consensus: row.8,
                weighted_score: row.9,
                rationale: row.10,
                provenance_kind: row.11,
                author: row.12,
                created_at: parse_timestamp(&row.13)?,
                eligible_for_surface: row.14,
            })
        })
        .collect()
}

fn string_column(
    connection: &Connection,
    sql: &str,
    record_id: &str,
) -> Result<Vec<String>, ApplicationError> {
    let mut statement = connection.prepare(sql).map_err(storage)?;
    statement
        .query_map([record_id], |row| row.get(0))
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)
}

struct SourceNavigationMetadata {
    source_locator: Option<String>,
    source_native_id: Option<String>,
    limitations: Vec<String>,
}

fn source_navigation_metadata(
    metadata_json: &str,
) -> Result<SourceNavigationMetadata, ApplicationError> {
    let metadata: serde_json::Value = serde_json::from_str(metadata_json).map_err(|error| {
        ApplicationError::Integrity(format!("invalid projection metadata: {error}"))
    })?;
    let source_locator = metadata
        .get("source_locator")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let source_native_id = metadata
        .get("source_native_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let mut limitations = metadata
        .pointer("/current_common/limitations")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|limitation| {
            if let Some(detail) = limitation.as_str() {
                return Some(detail.to_owned());
            }
            let code = limitation.get("code").and_then(serde_json::Value::as_str);
            let detail = limitation.get("detail").and_then(serde_json::Value::as_str);
            match (code, detail) {
                (Some(code), Some(detail)) => Some(format!("{code}: {detail}")),
                (None, Some(detail)) => Some(detail.to_owned()),
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    if let Some(reason) = metadata
        .get("observation_reason")
        .and_then(serde_json::Value::as_str)
        .filter(|reason| !reason.trim().is_empty())
    {
        limitations.push(reason.to_owned());
    }
    Ok(SourceNavigationMetadata {
        source_locator,
        source_native_id,
        limitations,
    })
}

fn surfacing_reasons(
    record: &RecordSummary,
    relation_count: u64,
    query: &SurfaceQuery,
) -> Vec<SurfacingReason> {
    let score = record
        .score
        .as_ref()
        .expect("surface filters require a score");
    let direction = record
        .map_nodes
        .iter()
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    vec![
        SurfacingReason {
            kind: SurfacingReasonKind::Direction,
            explanation: format!(
                "profile {} v{} weights interest/strategy/consensus at {}/{}/{}",
                score.profile_id,
                score.profile_ordinal,
                score.interest_weight,
                score.strategy_weight,
                score.consensus_weight
            ),
            evidence: if direction.is_empty() {
                vec!["no map assignment; profile direction only".to_owned()]
            } else {
                direction
            },
        },
        SurfacingReason {
            kind: SurfacingReasonKind::Relevance,
            explanation: format!(
                "weighted score {} from interest/strategy/consensus {}/{}/{}",
                score.weighted_score, score.interest, score.strategy, score.consensus
            ),
            evidence: vec![
                score.rationale.clone(),
                format!("score_id={}", score.score_id),
            ],
        },
        SurfacingReason {
            kind: SurfacingReasonKind::Time,
            explanation: format!("record time is {}", record.event_at.as_str()),
            evidence: query.since.as_ref().map_or_else(
                || vec!["ranked by recorded time as a tie-breaker".to_owned()],
                |since| vec![format!("on or after {}", since.as_str())],
            ),
        },
        SurfacingReason {
            kind: SurfacingReasonKind::Relation,
            explanation: if relation_count == 0 {
                "no explicit relation boost; surfaced independently".to_owned()
            } else {
                format!("{relation_count} navigable relation(s) support this result")
            },
            evidence: query.related_to.as_ref().map_or_else(
                || vec![format!("relation_count={relation_count}")],
                |related| vec![format!("related_to={related}")],
            ),
        },
    ]
}

fn structured_filter_count(filter: &babata_domain::QueryFilter) -> usize {
    [
        filter.source_kind.is_some(),
        filter.provider.is_some(),
        filter.content_type.is_some(),
        filter.captured_from.is_some(),
        filter.captured_to.is_some(),
        filter.semantic_kind.is_some(),
        filter.realm.is_some(),
        filter.state.is_some(),
        filter.access_state.is_some(),
        filter.person.is_some(),
        filter.map_node.is_some(),
        filter.tag.is_some(),
        filter.relation_kind.is_some(),
        filter.related_to.is_some(),
        filter.processing_state.is_some(),
        filter.origin_kind.is_some(),
        filter.review_state.is_some(),
        filter.restricted.is_some(),
        filter.missing.is_some(),
        filter.media_only.is_some(),
        filter.attachment_only.is_some(),
        filter.min_interest.is_some(),
        filter.min_strategy.is_some(),
        filter.min_consensus.is_some(),
        filter.min_weighted_score.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count()
}

fn validate_filter(filter: &babata_domain::QueryFilter) -> Result<(), ApplicationError> {
    for (name, value) in [
        ("min_interest", filter.min_interest),
        ("min_strategy", filter.min_strategy),
        ("min_consensus", filter.min_consensus),
    ] {
        if value.is_some_and(|value| value > 100) {
            return Err(ApplicationError::Integrity(format!(
                "{name} must be between 0 and 100"
            )));
        }
    }
    if filter
        .min_weighted_score
        .is_some_and(|value| value > 10_000)
    {
        return Err(ApplicationError::Integrity(
            "min_weighted_score must be between 0 and 10000".to_owned(),
        ));
    }
    if let (Some(from), Some(to)) = (&filter.captured_from, &filter.captured_to)
        && from.as_str() > to.as_str()
    {
        return Err(ApplicationError::Integrity(
            "captured_from must not be later than captured_to".to_owned(),
        ));
    }
    Ok(())
}

fn fts_query(text: &str) -> String {
    text.split_whitespace()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn wire<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .expect("wire enums serialize as strings")
}

fn parse_wire<T: DeserializeOwned>(value: &str) -> Result<T, ApplicationError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(|error| {
        ApplicationError::Integrity(format!("invalid projection wire value {value}: {error}"))
    })
}

fn parse_timestamp(value: &str) -> Result<UtcTimestamp, ApplicationError> {
    UtcTimestamp::parse(value).map_err(ApplicationError::from)
}

fn parse_timestamp_sql(value: String) -> rusqlite::Result<UtcTimestamp> {
    UtcTimestamp::parse(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn now() -> Result<String, ApplicationError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))
}

fn storage(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use babata_application::{SearchQuery, SurfaceQuery, ports::ReadProjectionPort};
    use babata_domain::{
        AssetId, DerivativeId, ItemId, QueryFilter, RelationId, RevisionId, RunId, ScoreId,
        SearchRecordMarker, SemanticId, SourceId, SourceObservationId, SuggestionId,
        SurfacingReasonKind,
    };
    use rusqlite::{Connection, params};
    use sha2::Digest;
    use tempfile::tempdir;

    use super::{DataPaths, SqliteReadProjection};

    struct Fixture {
        paths: DataPaths,
        text_item: String,
        media_item: String,
        attachment_item: String,
        knowledge_semantic: String,
        case_semantic: String,
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_item(
        connection: &Connection,
        item_id: &str,
        revision_id: &str,
        source_id: &str,
        content_type: &str,
        title: &str,
        author: &str,
        access_state: &str,
        media_kind: Option<&str>,
        raw_text: Option<&str>,
    ) {
        let common = serde_json::json!({
            "schema": "babata.c0.common/v1",
            "title": title,
            "authors": [{"display_name": author}],
            "hierarchy": [{"kind": "folder", "name": "Research"}],
            "limitations": if access_state == "restricted" {
                vec![serde_json::json!({"code": "login_required", "detail": "source login required"})]
            } else {
                Vec::new()
            },
            "access_state": access_state,
            "media": {
                "schema": "babata.c0.media/v1",
                "entries": media_kind.map_or_else(Vec::new, |kind| vec![serde_json::json!({"kind": kind})])
            }
        });
        connection
            .execute(
                "INSERT INTO items
                 (item_id, source_id, content_type, source_published_at, first_captured_at,
                  metadata_json, common_metadata_json, created_at)
                 VALUES (?1, ?2, ?3, '2026-07-01T00:00:00Z',
                         '2026-07-02T00:00:00Z', '{}', ?4, '2026-07-02T00:00:00Z')",
                params![item_id, source_id, content_type, common.to_string()],
            )
            .unwrap();
        let hash = raw_text.map(|text| format!("{:x}", sha2::Sha256::digest(text.as_bytes())));
        connection
            .execute(
                "INSERT INTO revisions
                 (revision_id, item_id, revision_kind, ordinal, captured_at, raw_text,
                  text_sha256, metadata_json, state, created_at)
                 VALUES (?1, ?2, 'capture', 1, '2026-07-02T00:00:00Z', ?3, ?4,
                         '{}', 'ready', '2026-07-02T00:00:00Z')",
                params![revision_id, item_id, raw_text, hash],
            )
            .unwrap();
    }

    fn insert_derivative(
        connection: &Connection,
        run_id: &str,
        derivative_id: &str,
        item_id: &str,
        revision_id: &str,
        kind: &str,
        text: &str,
    ) {
        let input_hash = format!("{:x}", sha2::Sha256::digest(revision_id.as_bytes()));
        let output_hash = format!("{:x}", sha2::Sha256::digest(text.as_bytes()));
        connection
            .execute(
                "INSERT INTO process_runs
                 (run_id, pipeline_id, input_revision_id, input_item_id, input_sha256,
                  state, provider, tool_or_model, attempt, params_json, usage_json,
                  created_at, started_at, finished_at)
                 VALUES (?1, 'fixture-pipeline', ?2, ?3, ?4, 'succeeded', 'fixture',
                         'fixture-model', 1, '{}', '{}', '2026-07-03T00:00:00Z',
                         '2026-07-03T00:00:00Z', '2026-07-03T00:01:00Z')",
                params![run_id, revision_id, item_id, input_hash],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO derivatives
                 (derivative_id, run_id, kind, output_sha256, content_text,
                  metadata_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, '{}', '2026-07-03T00:01:00Z')",
                params![derivative_id, run_id, kind, output_hash, text],
            )
            .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_machine_semantic(
        connection: &Connection,
        suggestion_id: &str,
        semantic_id: &str,
        derivative_id: &str,
        item_id: &str,
        revision_id: &str,
        kind: &str,
        realm: &str,
        title: &str,
        review: Option<&str>,
    ) {
        let output_hash = format!("{:x}", sha2::Sha256::digest(title.as_bytes()));
        connection
            .execute(
                "INSERT INTO model_suggestions
                 (suggestion_id, suggestion_kind, source_item_id, source_revision_id,
                  source_derivative_id, source_output_sha256, provider, model,
                  model_version, prompt_version, generated_at, evidence_derivatives_json,
                  limitations_json, created_at)
                 VALUES (?1, 'semantic_package', ?2, ?3, ?4, ?5, 'fixture',
                         'fixture-model', '1', 'p6-fixture/v1', '2026-07-04T00:00:00Z',
                         '[]', '[]', '2026-07-04T00:00:00Z')",
                params![
                    suggestion_id,
                    item_id,
                    revision_id,
                    derivative_id,
                    output_hash
                ],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO semantic_entries
                 (semantic_id, semantic_kind, realm, origin_kind, author, title,
                  payload_json, source_item_id, source_revision_id, suggestion_id, created_at)
                 VALUES (?1, ?2, ?3, 'machine', 'fixture-model', ?4,
                         json_object('kind', ?2, 'body', ?4), ?5, ?6, ?7,
                         '2026-07-04T00:00:00Z')",
                params![
                    semantic_id,
                    kind,
                    realm,
                    title,
                    item_id,
                    revision_id,
                    suggestion_id
                ],
            )
            .unwrap();
        if let Some(decision) = review {
            connection
                .execute(
                    "INSERT INTO suggestion_reviews
                     (review_id, suggestion_id, decision, reviewer, created_at)
                     VALUES (?1, ?2, ?3, 'fixture-reviewer', '2026-07-05T00:00:00Z')",
                    params![format!("review_{suggestion_id}"), suggestion_id, decision],
                )
                .unwrap();
        }
    }

    #[allow(clippy::too_many_lines)]
    fn fixture() -> Fixture {
        let temporary = tempdir().unwrap().keep();
        let paths = DataPaths::new(temporary);
        super::super::open_knowledge_review_database(&paths, 100).unwrap();
        super::super::open_derived_database(&paths, 100).unwrap();

        let source_id = SourceId::new().to_string();
        let first_party_source_id = SourceId::new().to_string();
        let text_item = ItemId::new().to_string();
        let media_item = ItemId::new().to_string();
        let attachment_item = ItemId::new().to_string();
        let first_party_item = ItemId::new().to_string();
        let text_revision = RevisionId::new().to_string();
        let media_revision = RevisionId::new().to_string();
        let attachment_revision = RevisionId::new().to_string();
        let first_party_revision = RevisionId::new().to_string();
        let raw = Connection::open(paths.raw_database()).unwrap();
        raw.pragma_update(None, "foreign_keys", "ON").unwrap();
        raw.execute(
            "INSERT INTO sources
             (source_id, source_kind, provider, display_name, metadata_json, created_at)
             VALUES (?1, 'external', 'fixture', 'Fixture source', '{}',
                     '2026-07-01T00:00:00Z')",
            [&source_id],
        )
        .unwrap();
        raw.execute(
            "INSERT INTO sources
             (source_id, source_kind, provider, display_name, metadata_json, created_at)
             VALUES (?1, 'first_party', 'babata', 'First-party', '{}',
                     '2026-07-01T00:00:00Z')",
            [&first_party_source_id],
        )
        .unwrap();
        insert_item(
            &raw,
            &text_item,
            &text_revision,
            &source_id,
            "text",
            "Quantum strategy note",
            "Ada",
            "accessible",
            None,
            Some("quantum systems and long-term strategy"),
        );
        insert_item(
            &raw,
            &media_item,
            &media_revision,
            &source_id,
            "video",
            "Restricted quantum lecture",
            "Lin",
            "restricted",
            Some("video"),
            None,
        );
        insert_item(
            &raw,
            &first_party_item,
            &first_party_revision,
            &first_party_source_id,
            "text",
            "Personal strategy insight",
            "fixture-user",
            "accessible",
            None,
            Some("my own strategic insight"),
        );
        insert_item(
            &raw,
            &attachment_item,
            &attachment_revision,
            &source_id,
            "document",
            "Attachment-only brief",
            "Mira",
            "accessible",
            None,
            None,
        );
        raw.execute(
            "INSERT INTO source_observations
             (observation_id, item_id, revision_id, observation_kind,
              recollection_state, common_metadata_json, provider_metadata_json,
              reason, observed_at)
             VALUES (?1, ?2, ?3, 'recollection', 'removed',
                     '{\"schema\":\"babata.c0.common/v1\",\"access_state\":\"removed\",\"media\":{\"schema\":\"babata.c0.media/v1\",\"entries\":[]}}',
                     '{}', 'removed at source but retained locally',
                     '2026-07-06T00:00:00Z')",
            params![
                SourceObservationId::new().to_string(),
                attachment_item,
                attachment_revision
            ],
        )
        .unwrap();

        let missing_asset = AssetId::new().to_string();
        raw.execute(
            "INSERT INTO assets
             (asset_id, revision_id, asset_role, logical_path, sha256, byte_size,
              media_type, state, created_at)
             VALUES (?1, ?2, 'original', '01_raw/assets/sha256/missing/video.mp4',
                     ?3, 10, 'video/mp4', 'ready', '2026-07-02T00:00:00Z')",
            params![missing_asset, media_revision, "a".repeat(64)],
        )
        .unwrap();
        let attachment_asset = AssetId::new().to_string();
        let attachment_path = "01_raw/assets/sha256/fixture/brief.pdf";
        let physical = paths.root().join(attachment_path);
        std::fs::create_dir_all(physical.parent().unwrap()).unwrap();
        std::fs::write(&physical, b"fixture attachment").unwrap();
        raw.execute(
            "INSERT INTO assets
             (asset_id, revision_id, asset_role, logical_path, sha256, byte_size,
              media_type, state, created_at)
             VALUES (?1, ?2, 'attachment', ?3, ?4, 18, 'application/pdf',
                     'ready', '2026-07-02T00:00:00Z')",
            params![
                attachment_asset,
                attachment_revision,
                attachment_path,
                "b".repeat(64)
            ],
        )
        .unwrap();
        raw.execute(
            "INSERT INTO relations
             (relation_id, from_item_id, relation_kind, to_item_id, metadata_json, created_at)
             VALUES (?1, ?2, 'related_to', ?3, '{\"reason\":\"fixture\"}',
                     '2026-07-02T00:00:00Z')",
            params![RelationId::new().to_string(), text_item, media_item],
        )
        .unwrap();

        let derivative_ids = (0..3)
            .map(|_| DerivativeId::new().to_string())
            .collect::<Vec<_>>();
        let derived = Connection::open(paths.derived_database()).unwrap();
        for (index, derivative_id) in derivative_ids.iter().enumerate() {
            insert_derivative(
                &derived,
                &RunId::new().to_string(),
                derivative_id,
                &text_item,
                &text_revision,
                "structured_result",
                &format!("semantic candidate {index} about quantum strategy"),
            );
        }
        insert_derivative(
            &derived,
            &RunId::new().to_string(),
            &DerivativeId::new().to_string(),
            &media_item,
            &media_revision,
            "visual_description",
            "quantum lecture visual frames",
        );

        let knowledge_semantic = SemanticId::new().to_string();
        let case_semantic = SemanticId::new().to_string();
        let accepted_semantic = SemanticId::new().to_string();
        let suggestions = (0..3)
            .map(|_| SuggestionId::new().to_string())
            .collect::<Vec<_>>();
        insert_machine_semantic(
            &raw,
            &suggestions[0],
            &knowledge_semantic,
            &derivative_ids[0],
            &text_item,
            &text_revision,
            "knowledge",
            "knowledge_and_cases",
            "Quantum knowledge model",
            None,
        );
        insert_machine_semantic(
            &raw,
            &suggestions[1],
            &case_semantic,
            &derivative_ids[1],
            &text_item,
            &text_revision,
            "case",
            "knowledge_and_cases",
            "Quantum case evidence",
            Some("rejected"),
        );
        insert_machine_semantic(
            &raw,
            &suggestions[2],
            &accepted_semantic,
            &derivative_ids[2],
            &text_item,
            &text_revision,
            "map_direction",
            "knowledge_map",
            "Quantum direction",
            Some("accepted"),
        );
        let first_party_semantic = SemanticId::new().to_string();
        raw.execute(
            "INSERT INTO semantic_entries
             (semantic_id, semantic_kind, realm, origin_kind, author, title, payload_json,
              first_party_item_id, first_party_revision_id, created_at)
             VALUES (?1, 'insight', 'cognitive_trail', 'first_party', 'fixture-user',
                     'Personal strategy insight',
                     '{\"kind\":\"insight\",\"maturity\":\"spark\",\"body\":\"my own strategic insight\"}',
                     ?2, ?3, '2026-07-04T00:00:00Z')",
            params![first_party_semantic, first_party_item, first_party_revision],
        )
        .unwrap();
        raw.execute(
            "INSERT INTO semantic_map_assignments
             (semantic_id, map_node_id, provenance_kind, suggestion_id, created_at)
             VALUES (?1, 'mapnode_p6_time', 'machine', ?2, '2026-07-04T00:00:00Z')",
            params![knowledge_semantic, suggestions[0]],
        )
        .unwrap();
        raw.execute(
            "INSERT INTO semantic_tags
             (tag_id, canonical_name, display_name, created_at)
             VALUES ('tag_fixture_quantum', 'quantum', 'Quantum', '2026-07-04T00:00:00Z')",
            [],
        )
        .unwrap();
        raw.execute(
            "INSERT INTO semantic_tag_assignments
             (semantic_id, tag_id, provenance_kind, suggestion_id, created_at)
             VALUES (?1, 'tag_fixture_quantum', 'machine', ?2,
                     '2026-07-04T00:00:00Z')",
            params![knowledge_semantic, suggestions[0]],
        )
        .unwrap();
        raw.execute(
            "INSERT INTO semantic_relations
             (semantic_relation_id, from_semantic_id, relation_kind, to_semantic_id,
              evidence, provenance_kind, suggestion_id, created_at)
             VALUES (?1, ?2, 'supports', ?3, 'case supports knowledge', 'machine', ?4,
                     '2026-07-04T00:00:00Z')",
            params![
                format!("semantic_relation_{}", RelationId::new()),
                knowledge_semantic,
                case_semantic,
                suggestions[0]
            ],
        )
        .unwrap();
        raw.execute(
            "INSERT INTO score_profiles
             (profile_id, ordinal, interest_weight, strategy_weight, consensus_weight,
              rationale, author_kind, author, created_at)
             VALUES ('profile_fixture_strategy', 2, 10, 80, 10, 'fixture strategy profile',
                     'first_party', 'fixture-user', '2026-07-05T00:00:00Z')",
            [],
        )
        .unwrap();
        for (score_id, profile, interest, strategy, consensus, weighted) in [
            (
                ScoreId::new().to_string(),
                "score_profile_p6_default",
                70,
                80,
                50,
                6950,
            ),
            (
                ScoreId::new().to_string(),
                "profile_fixture_strategy",
                20,
                95,
                30,
                8100,
            ),
        ] {
            raw.execute(
                "INSERT INTO relevance_scores
                 (score_id, target_kind, target_id, profile_id, interest, strategy,
                  consensus, weighted_score, rationale, provenance_kind, author,
                  suggestion_id, created_at)
                 VALUES (?1, 'semantic', ?2, ?3, ?4, ?5, ?6, ?7,
                         'fixture score rationale', 'machine', 'fixture-model', ?8,
                         '2026-07-05T00:00:00Z')",
                params![
                    score_id,
                    knowledge_semantic,
                    profile,
                    interest,
                    strategy,
                    consensus,
                    weighted,
                    suggestions[0]
                ],
            )
            .unwrap();
        }
        raw.execute(
            "INSERT INTO relevance_scores
             (score_id, target_kind, target_id, profile_id, interest, strategy,
              consensus, weighted_score, rationale, provenance_kind, author, created_at)
             VALUES (?1, 'semantic', ?2, 'score_profile_p6_default', 80, 90, 20,
                     6950, 'first-party insight score', 'first_party', 'fixture-user',
                     '2026-07-05T00:00:00Z')",
            params![ScoreId::new().to_string(), first_party_semantic],
        )
        .unwrap();
        raw.execute(
            "INSERT INTO relevance_scores
             (score_id, target_kind, target_id, profile_id, interest, strategy,
              consensus, weighted_score, rationale, provenance_kind, author,
              suggestion_id, created_at)
             VALUES (?1, 'semantic', ?2, 'score_profile_p6_default', 90, 90, 90,
                     9000, 'rejected score remains searchable', 'machine',
                     'fixture-model', ?3, '2026-07-06T00:00:00Z')",
            params![ScoreId::new().to_string(), case_semantic, suggestions[1]],
        )
        .unwrap();

        Fixture {
            paths,
            text_item,
            media_item,
            attachment_item,
            knowledge_semantic,
            case_semantic,
        }
    }

    fn query(filter: QueryFilter) -> SearchQuery {
        SearchQuery {
            filter,
            cursor: None,
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn projection_covers_discovery_ranking_navigation_and_rebuild_invariants() {
        let fixture = fixture();
        let projection = SqliteReadProjection::new(fixture.paths.clone(), 100);
        let before = {
            let raw = Connection::open(fixture.paths.raw_database()).unwrap();
            raw.query_row(
                "SELECT (SELECT COUNT(*) FROM items),
                        (SELECT COUNT(*) FROM revisions),
                        (SELECT COUNT(*) FROM semantic_entries),
                        (SELECT COUNT(*) FROM relevance_scores)",
                [],
                |row| {
                    Ok((
                        row.get::<_, u64>(0)?,
                        row.get::<_, u64>(1)?,
                        row.get::<_, u64>(2)?,
                        row.get::<_, u64>(3)?,
                    ))
                },
            )
            .unwrap()
        };
        let built = projection.rebuild().unwrap();
        assert_eq!(built.status.raw_items, 4);
        assert_eq!(built.status.semantic_entries, 4);
        let fingerprint = built.status.source_fingerprint.clone();

        let text = projection
            .search(query(QueryFilter {
                text: Some("quantum".to_owned()),
                limit: 20,
                ..QueryFilter::default()
            }))
            .unwrap();
        assert!(text.records.len() >= 4);
        assert!(text.records.iter().all(|record| !record.reasons.is_empty()));

        let restricted_media = projection
            .search(query(QueryFilter {
                provider: Some("fixture".to_owned()),
                restricted: Some(true),
                media_only: Some(true),
                limit: 20,
                ..QueryFilter::default()
            }))
            .unwrap();
        assert_eq!(restricted_media.records.len(), 1);
        assert_eq!(
            restricted_media.records[0].record_id,
            format!("item:{}", fixture.media_item)
        );
        assert!(
            restricted_media.records[0]
                .markers
                .contains(&SearchRecordMarker::Restricted)
        );
        assert!(
            restricted_media.records[0]
                .markers
                .contains(&SearchRecordMarker::Missing)
        );
        assert!(
            restricted_media.records[0]
                .markers
                .contains(&SearchRecordMarker::MediaOnly)
        );

        let attachment = projection
            .search(query(QueryFilter {
                attachment_only: Some(true),
                missing: Some(false),
                limit: 20,
                ..QueryFilter::default()
            }))
            .unwrap();
        assert_eq!(attachment.records.len(), 1);
        assert_eq!(
            attachment.records[0].record_id,
            format!("item:{}", fixture.attachment_item)
        );
        assert_eq!(attachment.records[0].access_state, "removed");
        let removed = projection
            .search(query(QueryFilter {
                access_state: Some("removed".to_owned()),
                limit: 20,
                ..QueryFilter::default()
            }))
            .unwrap();
        assert!(
            removed
                .records
                .iter()
                .any(|record| { record.record_id == format!("item:{}", fixture.attachment_item) })
        );

        for state in ["unreviewed", "rejected", "accepted"] {
            let results = projection
                .search(query(QueryFilter {
                    review_state: Some(state.to_owned()),
                    limit: 20,
                    ..QueryFilter::default()
                }))
                .unwrap();
            assert!(!results.records.is_empty(), "review state {state}");
            assert!(
                results
                    .records
                    .iter()
                    .all(|record| { record.review_state.as_deref() == Some(state) })
            );
        }
        let first_party = projection
            .search(query(QueryFilter {
                review_state: Some("first_party".to_owned()),
                limit: 20,
                ..QueryFilter::default()
            }))
            .unwrap();
        assert_eq!(first_party.records.len(), 1);
        assert!(first_party.records[0].judgment.human_judgment);
        assert!(!first_party.records[0].judgment.confirmed_fact);
        let unreviewed = projection
            .search(query(QueryFilter {
                review_state: Some("unreviewed".to_owned()),
                limit: 20,
                ..QueryFilter::default()
            }))
            .unwrap();
        assert!(
            unreviewed.records.iter().all(|record| {
                !record.judgment.human_judgment && !record.judgment.confirmed_fact
            })
        );

        let strategy = projection
            .search(query(QueryFilter {
                profile_id: Some("profile_fixture_strategy".to_owned()),
                min_strategy: Some(90),
                limit: 20,
                ..QueryFilter::default()
            }))
            .unwrap();
        assert_eq!(
            strategy.records.len(),
            2,
            "strategy records: {:?}",
            strategy
                .records
                .iter()
                .map(|record| (&record.record_id, &record.score))
                .collect::<Vec<_>>()
        );
        assert!(strategy.records.iter().all(|record| {
            record
                .score
                .as_ref()
                .is_some_and(|score| score.profile_id == "profile_fixture_strategy")
        }));

        let semantic_record = format!("semantic:{}", fixture.knowledge_semantic);
        let detail = projection.show(&semantic_record).unwrap();
        assert_eq!(detail.score_history.len(), 2);
        assert!(!detail.revisions.is_empty());
        assert!(!detail.derivatives.is_empty());
        assert_eq!(detail.record.map_nodes[0].name, "时间");
        assert!(detail.relations.iter().any(|relation| {
            relation.related_record_id == Some(format!("semantic:{}", fixture.case_semantic))
        }));
        let traversed = projection.traverse(&semantic_record).unwrap();
        assert!(traversed.iter().any(|related| {
            related.record.record_id == format!("semantic:{}", fixture.case_semantic)
        }));
        assert!(
            traversed.iter().any(|related| {
                related.record.record_id == format!("item:{}", fixture.text_item)
            })
        );

        let surfaced = projection
            .surface(SurfaceQuery {
                profile_id: Some("profile_fixture_strategy".to_owned()),
                map_node: Some("mapnode_p6_time".to_owned()),
                related_to: None,
                since: None,
                limit: 10,
            })
            .unwrap();
        assert_eq!(surfaced.records.len(), 1);
        for record in &surfaced.records {
            for kind in [
                SurfacingReasonKind::Direction,
                SurfacingReasonKind::Relevance,
                SurfacingReasonKind::Time,
                SurfacingReasonKind::Relation,
            ] {
                assert!(record.reasons.iter().any(|reason| reason.kind == kind));
            }
        }
        let general_surface = projection
            .surface(SurfaceQuery {
                profile_id: Some("score_profile_p6_default".to_owned()),
                map_node: None,
                related_to: None,
                since: None,
                limit: 20,
            })
            .unwrap();
        assert!(general_surface.records.iter().all(|record| {
            record.record_id != format!("semantic:{}", fixture.case_semantic)
                && record
                    .score
                    .as_ref()
                    .is_some_and(|score| score.eligible_for_surface)
        }));

        projection.delete().unwrap();
        assert_eq!(projection.status().unwrap().state, "missing");
        let rebuilt = projection.rebuild().unwrap();
        assert_eq!(rebuilt.status.source_fingerprint, fingerprint);
        let after = {
            let raw = Connection::open(fixture.paths.raw_database()).unwrap();
            raw.query_row(
                "SELECT (SELECT COUNT(*) FROM items),
                        (SELECT COUNT(*) FROM revisions),
                        (SELECT COUNT(*) FROM semantic_entries),
                        (SELECT COUNT(*) FROM relevance_scores)",
                [],
                |row| {
                    Ok((
                        row.get::<_, u64>(0)?,
                        row.get::<_, u64>(1)?,
                        row.get::<_, u64>(2)?,
                        row.get::<_, u64>(3)?,
                    ))
                },
            )
            .unwrap()
        };
        assert_eq!(before, after);
    }

    #[test]
    fn projection_fingerprint_tracks_searchable_authority_changes() {
        let fixture = fixture();
        let projection = SqliteReadProjection::new(fixture.paths.clone(), 100);
        let before = projection
            .rebuild()
            .unwrap()
            .status
            .source_fingerprint
            .unwrap();

        let raw = Connection::open(fixture.paths.raw_database()).unwrap();
        raw.execute(
            "UPDATE semantic_tags SET display_name = display_name || '-changed'",
            [],
        )
        .unwrap();
        drop(raw);

        let after = projection
            .rebuild()
            .unwrap()
            .status
            .source_fingerprint
            .unwrap();
        assert_ne!(before, after);
    }

    #[test]
    fn projection_migration_checksum_change_fails_closed() {
        let fixture = fixture();
        let projection = SqliteReadProjection::new(fixture.paths.clone(), 100);
        projection.rebuild().unwrap();
        let database = Connection::open(fixture.paths.search_projection_database()).unwrap();
        database
            .execute(
                "UPDATE projection_schema_migrations
                 SET checksum_sha256 = 'tampered' WHERE version = 1",
                [],
            )
            .unwrap();
        drop(database);

        let error = projection.rebuild().unwrap_err();
        assert_eq!(error.code(), "integrity_failed");
        assert!(error.to_string().contains("migration checksum changed"));
    }
}

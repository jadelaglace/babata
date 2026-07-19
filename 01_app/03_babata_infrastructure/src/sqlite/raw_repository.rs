use std::sync::{Arc, Mutex};

use babata_application::{
    ApplicationError, AssetDetail, CaptureProvenanceDetail, CollectionDetail, RecordDetail,
    RelationDetail, RevisionDetail,
    ports::{
        NewAsset, NewCaptureOperation, NewCollection, NewItem, NewRelation, NewRevision,
        NewRouteEvidence, NewSource, PersistGraph, RawRepositoryPort,
    },
};
use babata_domain::{
    AssetId, AssetRole, CollectionId, ContentType, ItemId, Metadata, RawState, RelationId,
    RelationKind, RevisionId, RevisionKind, RouteCoverage, RouteEvidence, Sha256, SourceId,
    SourceKind, SourceRouteId, UtcTimestamp,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

#[derive(Clone)]
pub struct SqliteRawRepository {
    pub(crate) connection: Arc<Mutex<Connection>>,
}

impl SqliteRawRepository {
    pub(crate) fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }
    pub(crate) fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ApplicationError> {
        self.connection
            .lock()
            .map_err(|_| ApplicationError::Storage("SQLite connection lock poisoned".to_owned()))
    }
}

impl RawRepositoryPort for SqliteRawRepository {
    fn find_source(
        &self,
        kind: SourceKind,
        provider: &str,
        account: Option<&str>,
    ) -> Result<Option<NewSource>, ApplicationError> {
        let connection = self.lock()?;
        connection.query_row("SELECT source_id, source_kind, provider, account_or_workspace, created_at FROM sources WHERE source_kind = ?1 AND provider = ?2 AND account_or_workspace IS ?3", params![source_kind(kind), provider, account], source_from_row).optional().map_err(storage)
    }

    fn find_item(&self, item_id: &ItemId) -> Result<Option<NewItem>, ApplicationError> {
        let connection = self.lock()?;
        connection.query_row("SELECT item_id, source_id, source_native_id, source_locator, source_identity_key, content_type, source_published_at, first_captured_at, metadata_json FROM items WHERE item_id = ?1", params![item_id.to_string()], item_from_row).optional().map_err(storage)
    }

    fn find_revision(
        &self,
        revision_id: &RevisionId,
    ) -> Result<Option<NewRevision>, ApplicationError> {
        let connection = self.lock()?;
        connection.query_row("SELECT revision_id, item_id, parent_revision_id, revision_kind, ordinal, captured_at, authored_at, revision_note, raw_text, text_sha256, metadata_json FROM revisions WHERE revision_id = ?1", params![revision_id.to_string()], revision_from_row).optional().map_err(storage)
    }

    fn find_revision_state(
        &self,
        revision_id: &RevisionId,
    ) -> Result<Option<RawState>, ApplicationError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT state FROM revisions WHERE revision_id = ?1",
                params![revision_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage)?
            .map(|state| parse_raw_state(&state))
            .transpose()
    }

    fn find_asset(&self, asset_id: &AssetId) -> Result<Option<NewAsset>, ApplicationError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT asset_id, revision_id, asset_role, logical_path, sha256, byte_size, media_type, original_filename FROM assets WHERE asset_id = ?1",
                params![asset_id.to_string()],
                new_asset_from_row,
            )
            .optional()
            .map_err(storage)
    }

    fn list_assets_for_revision(
        &self,
        revision_id: &RevisionId,
    ) -> Result<Vec<NewAsset>, ApplicationError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT asset_id, revision_id, asset_role, logical_path, sha256, byte_size, media_type, original_filename FROM assets WHERE revision_id = ?1 ORDER BY created_at",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(params![revision_id.to_string()], new_asset_from_row)
            .map_err(storage)?;
        rows.map(|row| row.map_err(storage)).collect()
    }

    fn find_by_source_identity(
        &self,
        source_id: &SourceId,
        identity: &str,
    ) -> Result<Option<(NewItem, NewRevision)>, ApplicationError> {
        let connection = self.lock()?;
        let item = connection.query_row("SELECT item_id, source_id, source_native_id, source_locator, source_identity_key, content_type, source_published_at, first_captured_at, metadata_json FROM items WHERE source_id = ?1 AND source_identity_key = ?2", params![source_id.to_string(), identity], item_from_row).optional().map_err(storage)?;
        item.map(|item| {
            let revision = connection.query_row("SELECT revision_id, item_id, parent_revision_id, revision_kind, ordinal, captured_at, authored_at, revision_note, raw_text, text_sha256, metadata_json FROM revisions WHERE item_id = ?1 ORDER BY ordinal DESC LIMIT 1", params![item.id.to_string()], revision_from_row).map_err(storage)?;
            Ok((item, revision))
        }).transpose()
    }

    fn next_ordinal(&self, item_id: &ItemId) -> Result<u32, ApplicationError> {
        let connection = self.lock()?;
        let ordinal: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(ordinal), 0) + 1 FROM revisions WHERE item_id = ?1",
                params![item_id.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        u32::try_from(ordinal)
            .map_err(|_| ApplicationError::Integrity("revision ordinal overflow".to_owned()))
    }

    fn find_duplicate_text(
        &self,
        item_id: &ItemId,
        hash: &Sha256,
    ) -> Result<Option<RevisionId>, ApplicationError> {
        let connection = self.lock()?;
        connection.query_row("SELECT revision_id FROM revisions WHERE item_id = ?1 AND text_sha256 = ?2 AND state = 'ready' ORDER BY ordinal DESC LIMIT 1", params![item_id.to_string(), hash.as_str()], |row| row.get::<_, String>(0)).optional().map_err(storage)?.map(parse_revision).transpose()
    }

    fn insert_capture_graph(&self, graph: &PersistGraph) -> Result<(), ApplicationError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        insert_source(&transaction, &graph.source)?;
        insert_item(&transaction, &graph.item)?;
        if let Some(collection) = &graph.collection {
            insert_collection(&transaction, collection)?;
            insert_item_collection(&transaction, &graph.item.id, collection)?;
        }
        insert_revision(&transaction, &graph.revision)?;
        for asset in &graph.assets {
            insert_asset(&transaction, asset)?;
        }
        for relation in &graph.relations {
            insert_relation(&transaction, relation)?;
        }
        insert_capture_operation(&transaction, &graph.operation)?;
        transaction.commit().map_err(storage)
    }

    fn mark_ready(&self, revision_id: &RevisionId) -> Result<(), ApplicationError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        let invalid_assets: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM assets WHERE revision_id = ?1 AND state <> 'pending'",
                params![revision_id.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if invalid_assets != 0 {
            return Err(ApplicationError::Integrity(
                "capture assets are not all pending before ready transition".to_owned(),
            ));
        }
        let changed = transaction
            .execute(
                "UPDATE revisions SET state = 'ready' WHERE revision_id = ?1 AND state = 'pending'",
                params![revision_id.to_string()],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(ApplicationError::Integrity(
                "capture revision is missing or is not pending".to_owned(),
            ));
        }
        transaction
            .execute(
                "UPDATE assets SET state = 'ready' WHERE revision_id = ?1 AND state = 'pending'",
                params![revision_id.to_string()],
            )
            .map_err(storage)?;
        let operation_changed = transaction
            .execute(
                "UPDATE capture_operations SET state = 'ready', completed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE revision_id = ?1 AND state = 'pending'",
                params![revision_id.to_string()],
            )
            .map_err(storage)?;
        if operation_changed != 1 {
            return Err(ApplicationError::Integrity(
                "capture operation is missing or is not pending".to_owned(),
            ));
        }
        transaction.commit().map_err(storage)
    }

    fn quarantine(
        &self,
        revision_id: &RevisionId,
        failure_code: &str,
    ) -> Result<(), ApplicationError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        let changed = transaction
            .execute(
                "UPDATE revisions SET state = 'quarantined' WHERE revision_id = ?1 AND state <> 'ready'",
                params![revision_id.to_string()],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(ApplicationError::Integrity(
                "capture revision cannot be quarantined".to_owned(),
            ));
        }
        transaction
            .execute(
                "UPDATE assets SET state = 'quarantined' WHERE revision_id = ?1 AND state <> 'ready'",
                params![revision_id.to_string()],
            )
            .map_err(storage)?;
        let operation_changed = transaction
            .execute(
                "UPDATE capture_operations SET state = 'quarantined', failure_code = ?2, completed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE revision_id = ?1 AND state = 'pending'",
                params![revision_id.to_string(), failure_code],
            )
            .map_err(storage)?;
        if operation_changed != 1 {
            return Err(ApplicationError::Integrity(
                "capture operation cannot be quarantined".to_owned(),
            ));
        }
        transaction.commit().map_err(storage)
    }

    fn load_detail(&self, item_id: &ItemId) -> Result<RecordDetail, ApplicationError> {
        let connection = self.lock()?;
        let header = load_item_header(&connection, item_id)?;
        Ok(RecordDetail {
            item_id: item_id.clone(),
            source_id: header.source_id,
            source_kind: header.source_kind,
            provider: header.provider,
            content_type: header.content_type,
            source_native_id: header.source_native_id,
            source_locator: header.source_locator,
            source_identity_key: header.source_identity_key,
            metadata: header.metadata,
            collections: load_collections(&connection, item_id)?,
            revisions: load_revisions(&connection, item_id)?,
            assets: load_assets(&connection, item_id)?,
            relations: load_relations(&connection, item_id)?,
        })
    }

    fn record_route_evidence(&self, evidence: &NewRouteEvidence) -> Result<(), ApplicationError> {
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO route_evidence (evidence_id, route_id, authorization_id, source_reference, item_id, revision_id, metadata_covered, attachments_covered, revisions_covered, limitations_json, reimported, recorded_at) VALUES ('route_evidence_' || lower(hex(randomblob(16))), ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    evidence.route_id,
                    evidence.authorization_id,
                    evidence.source_reference,
                    evidence.item_id.to_string(),
                    evidence.revision_id.to_string(),
                    evidence.coverage.metadata,
                    evidence.coverage.attachments,
                    evidence.coverage.revisions,
                    serde_json::to_string(&evidence.coverage.limitations)
                        .map_err(|error| ApplicationError::Integrity(error.to_string()))?,
                    evidence.reimported,
                    evidence.recorded_at.as_str(),
                ],
            )
            .map_err(storage)?;
        Ok(())
    }

    fn route_evidence(&self, route_id: &str) -> Result<Vec<RouteEvidence>, ApplicationError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare("SELECT route_id, authorization_id, source_reference, item_id, revision_id, metadata_covered, attachments_covered, revisions_covered, limitations_json, reimported, recorded_at FROM route_evidence WHERE route_id = ?1 ORDER BY recorded_at")
            .map_err(storage)?;
        let rows = statement
            .query_map(params![route_id], |row| {
                let limitations: Vec<String> = serde_json::from_str(&row.get::<_, String>(8)?)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            8,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(RouteEvidence {
                    route_id: SourceRouteId(row.get(0)?),
                    authorization_id: row.get(1)?,
                    source_reference: row.get(2)?,
                    item_id: ItemId::parse(row.get::<_, String>(3)?).map_err(to_sql)?,
                    revision_id: RevisionId::parse(row.get::<_, String>(4)?).map_err(to_sql)?,
                    coverage: RouteCoverage {
                        metadata: row.get(5)?,
                        attachments: row.get(6)?,
                        revisions: row.get(7)?,
                        limitations,
                    },
                    reimported: row.get(9)?,
                    recorded_at: UtcTimestamp::parse(row.get::<_, String>(10)?).map_err(to_sql)?,
                })
            })
            .map_err(storage)?;
        rows.map(|row| row.map_err(storage)).collect()
    }
}

struct ItemHeader {
    source_id: SourceId,
    source_kind: SourceKind,
    provider: String,
    content_type: ContentType,
    source_native_id: Option<String>,
    source_locator: Option<String>,
    source_identity_key: Option<String>,
    metadata: Metadata,
}

fn load_item_header(
    connection: &Connection,
    item_id: &ItemId,
) -> Result<ItemHeader, ApplicationError> {
    connection
        .query_row(
            "SELECT s.source_id, s.source_kind, s.provider, i.content_type,
                    i.source_native_id, i.source_locator, i.source_identity_key, i.metadata_json
             FROM items i JOIN sources s ON s.source_id = i.source_id
             WHERE i.item_id = ?1",
            params![item_id.to_string()],
            |row| {
                Ok(ItemHeader {
                    source_id: SourceId::parse(row.get::<_, String>(0)?).map_err(to_sql)?,
                    source_kind: parse_source_kind(&row.get::<_, String>(1)?).map_err(to_sql)?,
                    provider: row.get(2)?,
                    content_type: parse_content_type(&row.get::<_, String>(3)?).map_err(to_sql)?,
                    source_native_id: row.get(4)?,
                    source_locator: row.get(5)?,
                    source_identity_key: row.get(6)?,
                    metadata: Metadata::parse(&row.get::<_, String>(7)?).map_err(to_sql)?,
                })
            },
        )
        .map_err(storage)
}

fn load_collections(
    connection: &Connection,
    item_id: &ItemId,
) -> Result<Vec<CollectionDetail>, ApplicationError> {
    let mut statement = connection.prepare("SELECT c.collection_id, c.native_id, c.collection_kind, c.title, c.observed_at FROM collections c JOIN item_collections ic ON ic.collection_id = c.collection_id WHERE ic.item_id = ?1 ORDER BY c.created_at").map_err(storage)?;
    let rows = statement
        .query_map(params![item_id.to_string()], |row| {
            Ok(CollectionDetail {
                collection_id: CollectionId::parse(row.get::<_, String>(0)?).map_err(to_sql)?,
                native_id: row.get(1)?,
                kind: row.get(2)?,
                title: row.get(3)?,
                observed_at: UtcTimestamp::parse(row.get::<_, String>(4)?).map_err(to_sql)?,
            })
        })
        .map_err(storage)?;
    rows.map(|row| row.map_err(storage)).collect()
}

fn load_revisions(
    connection: &Connection,
    item_id: &ItemId,
) -> Result<Vec<RevisionDetail>, ApplicationError> {
    let mut statement = connection.prepare("SELECT r.revision_id, r.parent_revision_id, r.revision_kind, r.ordinal, r.captured_at, r.authored_at, r.revision_note, r.raw_text, r.text_sha256, r.metadata_json, r.state, o.operation_id, o.source_native_id, o.source_locator, o.source_published_at, o.metadata_json, o.state, o.failure_code FROM revisions r LEFT JOIN capture_operations o ON o.revision_id = r.revision_id WHERE r.item_id = ?1 ORDER BY r.ordinal").map_err(storage)?;
    let rows = statement
        .query_map(params![item_id.to_string()], |row| {
            Ok(RevisionDetail {
                revision_id: parse_revision(row.get::<_, String>(0)?).map_err(to_sql)?,
                parent_revision_id: row
                    .get::<_, Option<String>>(1)?
                    .map(parse_revision)
                    .transpose()
                    .map_err(to_sql)?,
                kind: row.get(2)?,
                ordinal: row.get::<_, i64>(3)? as u32,
                captured_at: UtcTimestamp::parse(row.get::<_, String>(4)?).map_err(to_sql)?,
                authored_at: row
                    .get::<_, Option<String>>(5)?
                    .map(UtcTimestamp::parse)
                    .transpose()
                    .map_err(to_sql)?,
                revision_note: row.get(6)?,
                raw_text: row.get(7)?,
                text_sha256: row.get(8)?,
                metadata: Metadata::parse(&row.get::<_, String>(9)?).map_err(to_sql)?,
                state: parse_raw_state(&row.get::<_, String>(10)?).map_err(to_sql)?,
                provenance: row
                    .get::<_, Option<String>>(11)?
                    .map(|operation_id| {
                        Ok::<CaptureProvenanceDetail, rusqlite::Error>(CaptureProvenanceDetail {
                            operation_id,
                            source_native_id: row.get(12)?,
                            source_locator: row.get(13)?,
                            source_published_at: row
                                .get::<_, Option<String>>(14)?
                                .map(UtcTimestamp::parse)
                                .transpose()
                                .map_err(to_sql)?,
                            metadata: Metadata::parse(&row.get::<_, String>(15)?)
                                .map_err(to_sql)?,
                            state: parse_raw_state(&row.get::<_, String>(16)?).map_err(to_sql)?,
                            failure_code: row.get(17)?,
                        })
                    })
                    .transpose()?,
            })
        })
        .map_err(storage)?;
    rows.map(|row| row.map_err(storage)).collect()
}

fn new_asset_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NewAsset> {
    Ok(NewAsset {
        id: AssetId::parse(row.get::<_, String>(0)?).map_err(to_sql)?,
        revision_id: RevisionId::parse(row.get::<_, String>(1)?).map_err(to_sql)?,
        role: parse_asset_role(&row.get::<_, String>(2)?).map_err(to_sql)?,
        logical_path: row.get(3)?,
        sha256: Sha256::parse(row.get::<_, String>(4)?).map_err(to_sql)?,
        byte_size: row.get::<_, i64>(5)? as u64,
        media_type: row.get(6)?,
        original_filename: row.get(7)?,
    })
}

fn load_assets(
    connection: &Connection,
    item_id: &ItemId,
) -> Result<Vec<AssetDetail>, ApplicationError> {
    let mut statement = connection.prepare("SELECT a.asset_id, a.asset_role, a.logical_path, a.sha256, a.byte_size, a.media_type, a.original_filename, a.state FROM assets a JOIN revisions r ON r.revision_id = a.revision_id WHERE r.item_id = ?1 ORDER BY a.created_at").map_err(storage)?;
    let rows = statement
        .query_map(params![item_id.to_string()], |row| {
            Ok(AssetDetail {
                asset_id: AssetId::parse(row.get::<_, String>(0)?).map_err(to_sql)?,
                role: parse_asset_role(&row.get::<_, String>(1)?).map_err(to_sql)?,
                logical_path: row.get(2)?,
                sha256: row.get(3)?,
                byte_size: row.get::<_, i64>(4)? as u64,
                media_type: row.get(5)?,
                original_filename: row.get(6)?,
                state: parse_raw_state(&row.get::<_, String>(7)?).map_err(to_sql)?,
            })
        })
        .map_err(storage)?;
    rows.map(|row| row.map_err(storage)).collect()
}

fn load_relations(
    connection: &Connection,
    item_id: &ItemId,
) -> Result<Vec<RelationDetail>, ApplicationError> {
    let mut statement = connection.prepare("SELECT relation_kind, from_item_id, from_revision_id, to_item_id, to_revision_id FROM relations WHERE from_item_id = ?1 OR to_item_id = ?1 ORDER BY created_at").map_err(storage)?;
    let rows = statement
        .query_map(params![item_id.to_string()], |row| {
            Ok(RelationDetail {
                kind: parse_relation_kind(&row.get::<_, String>(0)?).map_err(to_sql)?,
                from_item_id: ItemId::parse(row.get::<_, String>(1)?).map_err(to_sql)?,
                from_revision_id: row
                    .get::<_, Option<String>>(2)?
                    .map(parse_revision)
                    .transpose()
                    .map_err(to_sql)?,
                to_item_id: ItemId::parse(row.get::<_, String>(3)?).map_err(to_sql)?,
                to_revision_id: row
                    .get::<_, Option<String>>(4)?
                    .map(parse_revision)
                    .transpose()
                    .map_err(to_sql)?,
            })
        })
        .map_err(storage)?;
    rows.map(|row| row.map_err(storage)).collect()
}

fn insert_source(
    transaction: &Transaction<'_>,
    source: &NewSource,
) -> Result<(), ApplicationError> {
    transaction.execute("INSERT OR IGNORE INTO sources (source_id, source_kind, provider, account_or_workspace, created_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![source.id.to_string(), source_kind(source.kind), source.provider, source.account_or_workspace, source.created_at.as_str()]).map_err(storage)?;
    Ok(())
}

fn insert_item(transaction: &Transaction<'_>, item: &NewItem) -> Result<(), ApplicationError> {
    transaction.execute("INSERT OR IGNORE INTO items (item_id, source_id, source_native_id, source_locator, source_identity_key, content_type, source_published_at, first_captured_at, metadata_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?8)", params![item.id.to_string(), item.source_id.to_string(), item.source_native_id, item.source_locator, item.source_identity_key, content_type(item.content_type), item.source_published_at.as_ref().map(UtcTimestamp::as_str), item.first_captured_at.as_str(), item.metadata.to_json()]).map_err(storage)?;
    Ok(())
}

fn insert_collection(
    transaction: &Transaction<'_>,
    collection: &NewCollection,
) -> Result<(), ApplicationError> {
    transaction
        .execute(
            "INSERT OR IGNORE INTO collections (collection_id, source_id, native_id, collection_kind, title, metadata_json, observed_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                collection.id.to_string(),
                collection.source_id.to_string(),
                collection.native_id,
                collection.collection_kind,
                collection.title,
                collection.metadata.to_json(),
                collection.observed_at.as_str(),
            ],
        )
        .map_err(storage)?;
    Ok(())
}

fn insert_item_collection(
    transaction: &Transaction<'_>,
    item_id: &ItemId,
    collection: &NewCollection,
) -> Result<(), ApplicationError> {
    let collection_id: String = transaction
        .query_row(
            "SELECT collection_id FROM collections WHERE source_id = ?1 AND native_id = ?2",
            params![collection.source_id.to_string(), collection.native_id],
            |row| row.get(0),
        )
        .map_err(storage)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO item_collections (item_id, collection_id, observed_at, metadata_json) VALUES (?1, ?2, ?3, ?4)",
            params![
                item_id.to_string(),
                collection_id,
                collection.observed_at.as_str(),
                collection.metadata.to_json(),
            ],
        )
        .map_err(storage)?;
    Ok(())
}

fn insert_revision(
    transaction: &Transaction<'_>,
    revision: &NewRevision,
) -> Result<(), ApplicationError> {
    transaction.execute("INSERT INTO revisions (revision_id, item_id, parent_revision_id, revision_kind, ordinal, captured_at, authored_at, revision_note, raw_text, text_sha256, metadata_json, state, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'pending', ?6)", params![revision.id.to_string(), revision.item_id.to_string(), revision.parent_revision_id.as_ref().map(ToString::to_string), revision_kind(revision.kind), revision.ordinal, revision.captured_at.as_str(), revision.authored_at.as_ref().map(UtcTimestamp::as_str), revision.revision_note, revision.raw_text, revision.text_sha256.as_ref().map(Sha256::as_str), revision.metadata.to_json()]).map_err(storage)?;
    Ok(())
}

fn insert_asset(transaction: &Transaction<'_>, asset: &NewAsset) -> Result<(), ApplicationError> {
    transaction.execute("INSERT INTO assets (asset_id, revision_id, asset_role, logical_path, sha256, byte_size, media_type, original_filename, state, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))", params![asset.id.to_string(), asset.revision_id.to_string(), asset_role(asset.role), asset.logical_path, asset.sha256.as_str(), asset.byte_size as i64, asset.media_type, asset.original_filename]).map_err(storage)?;
    Ok(())
}

fn insert_relation(
    transaction: &Transaction<'_>,
    relation: &NewRelation,
) -> Result<(), ApplicationError> {
    transaction.execute("INSERT INTO relations (relation_id, from_item_id, from_revision_id, relation_kind, to_item_id, to_revision_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))", params![RelationId::new().to_string(), relation.from_item_id.to_string(), relation.from_revision_id.as_ref().map(ToString::to_string), relation_kind(relation.kind), relation.to_item_id.to_string(), relation.to_revision_id.as_ref().map(ToString::to_string)]).map_err(storage)?;
    Ok(())
}

fn insert_capture_operation(
    transaction: &Transaction<'_>,
    operation: &NewCaptureOperation,
) -> Result<(), ApplicationError> {
    transaction.execute(
        "INSERT INTO capture_operations (operation_id, item_id, revision_id, source_native_id, source_locator, source_published_at, metadata_json, state, started_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)",
        params![
            operation.operation_id,
            operation.item_id.to_string(),
            operation.revision_id.to_string(),
            operation.source_native_id,
            operation.source_locator,
            operation
                .source_published_at
                .as_ref()
                .map(UtcTimestamp::as_str),
            operation.metadata.to_json(),
            operation.started_at.as_str(),
        ],
    ).map_err(storage)?;
    Ok(())
}

fn source_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NewSource> {
    Ok(NewSource {
        id: SourceId::parse(row.get::<_, String>(0)?).map_err(to_sql)?,
        kind: parse_source_kind(&row.get::<_, String>(1)?).map_err(to_sql)?,
        provider: row.get(2)?,
        account_or_workspace: row.get(3)?,
        created_at: UtcTimestamp::parse(row.get::<_, String>(4)?).map_err(to_sql)?,
    })
}
fn item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NewItem> {
    Ok(NewItem {
        id: ItemId::parse(row.get::<_, String>(0)?).map_err(to_sql)?,
        source_id: SourceId::parse(row.get::<_, String>(1)?).map_err(to_sql)?,
        source_native_id: row.get(2)?,
        source_locator: row.get(3)?,
        source_identity_key: row.get(4)?,
        content_type: parse_content_type(&row.get::<_, String>(5)?).map_err(to_sql)?,
        source_published_at: row
            .get::<_, Option<String>>(6)?
            .map(UtcTimestamp::parse)
            .transpose()
            .map_err(to_sql)?,
        first_captured_at: UtcTimestamp::parse(row.get::<_, String>(7)?).map_err(to_sql)?,
        metadata: Metadata::parse(&row.get::<_, String>(8)?).map_err(to_sql)?,
    })
}
fn revision_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NewRevision> {
    Ok(NewRevision {
        id: RevisionId::parse(row.get::<_, String>(0)?).map_err(to_sql)?,
        item_id: ItemId::parse(row.get::<_, String>(1)?).map_err(to_sql)?,
        parent_revision_id: row
            .get::<_, Option<String>>(2)?
            .map(parse_revision)
            .transpose()
            .map_err(to_sql)?,
        kind: parse_revision_kind(&row.get::<_, String>(3)?).map_err(to_sql)?,
        ordinal: row.get::<_, i64>(4)? as u32,
        captured_at: UtcTimestamp::parse(row.get::<_, String>(5)?).map_err(to_sql)?,
        authored_at: row
            .get::<_, Option<String>>(6)?
            .map(UtcTimestamp::parse)
            .transpose()
            .map_err(to_sql)?,
        revision_note: row.get(7)?,
        raw_text: row.get(8)?,
        text_sha256: row
            .get::<_, Option<String>>(9)?
            .map(Sha256::parse)
            .transpose()
            .map_err(to_sql)?,
        metadata: Metadata::parse(&row.get::<_, String>(10)?).map_err(to_sql)?,
    })
}

fn parse_revision(value: String) -> Result<RevisionId, ApplicationError> {
    RevisionId::parse(value).map_err(ApplicationError::from)
}
fn storage(error: rusqlite::Error) -> ApplicationError {
    ApplicationError::Storage(error.to_string())
}
fn to_sql<E>(error: E) -> rusqlite::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}
fn source_kind(value: SourceKind) -> &'static str {
    match value {
        SourceKind::External => "external",
        SourceKind::FirstParty => "first_party",
    }
}
fn revision_kind(value: RevisionKind) -> &'static str {
    match value {
        RevisionKind::Capture => "capture",
        RevisionKind::Import => "import",
        RevisionKind::Authored => "authored",
        RevisionKind::Edit => "edit",
        RevisionKind::Annotation => "annotation",
    }
}
fn content_type(value: ContentType) -> &'static str {
    match value {
        ContentType::Text => "text",
        ContentType::Document => "document",
        ContentType::Image => "image",
        ContentType::Audio => "audio",
        ContentType::Video => "video",
        ContentType::WebPage => "web_page",
        ContentType::Archive => "archive",
        ContentType::Unknown => "unknown",
    }
}
fn asset_role(value: AssetRole) -> &'static str {
    match value {
        AssetRole::Original => "original",
        AssetRole::Attachment => "attachment",
        AssetRole::Export => "export",
        AssetRole::Cover => "cover",
        AssetRole::Derived => "derived",
        AssetRole::Preview => "preview",
    }
}
fn relation_kind(value: RelationKind) -> &'static str {
    match value {
        RelationKind::Revises => "revises",
        RelationKind::Annotates => "annotates",
        RelationKind::Quotes => "quotes",
        RelationKind::RespondsTo => "responds_to",
        RelationKind::RelatedTo => "related_to",
    }
}
fn parse_source_kind(value: &str) -> Result<SourceKind, ApplicationError> {
    match value {
        "external" => Ok(SourceKind::External),
        "first_party" => Ok(SourceKind::FirstParty),
        _ => Err(ApplicationError::Integrity(format!(
            "unknown source kind: {value}"
        ))),
    }
}
fn parse_revision_kind(value: &str) -> Result<RevisionKind, ApplicationError> {
    match value {
        "capture" => Ok(RevisionKind::Capture),
        "import" => Ok(RevisionKind::Import),
        "authored" => Ok(RevisionKind::Authored),
        "edit" => Ok(RevisionKind::Edit),
        "annotation" => Ok(RevisionKind::Annotation),
        _ => Err(ApplicationError::Integrity(format!(
            "unknown revision kind: {value}"
        ))),
    }
}
fn parse_content_type(value: &str) -> Result<ContentType, ApplicationError> {
    match value {
        "text" => Ok(ContentType::Text),
        "document" => Ok(ContentType::Document),
        "image" => Ok(ContentType::Image),
        "audio" => Ok(ContentType::Audio),
        "video" => Ok(ContentType::Video),
        "web_page" => Ok(ContentType::WebPage),
        "archive" => Ok(ContentType::Archive),
        "unknown" => Ok(ContentType::Unknown),
        _ => Err(ApplicationError::Integrity(format!(
            "unknown content type: {value}"
        ))),
    }
}
fn parse_raw_state(value: &str) -> Result<RawState, ApplicationError> {
    match value {
        "pending" => Ok(RawState::Pending),
        "ready" => Ok(RawState::Ready),
        "quarantined" => Ok(RawState::Quarantined),
        _ => Err(ApplicationError::Integrity(format!(
            "unknown raw state: {value}"
        ))),
    }
}
fn parse_asset_role(value: &str) -> Result<AssetRole, ApplicationError> {
    match value {
        "original" => Ok(AssetRole::Original),
        "attachment" => Ok(AssetRole::Attachment),
        "export" => Ok(AssetRole::Export),
        "cover" => Ok(AssetRole::Cover),
        "derived" => Ok(AssetRole::Derived),
        "preview" => Ok(AssetRole::Preview),
        _ => Err(ApplicationError::Integrity(format!(
            "unknown asset role: {value}"
        ))),
    }
}
fn parse_relation_kind(value: &str) -> Result<RelationKind, ApplicationError> {
    match value {
        "revises" => Ok(RelationKind::Revises),
        "annotates" => Ok(RelationKind::Annotates),
        "quotes" => Ok(RelationKind::Quotes),
        "responds_to" => Ok(RelationKind::RespondsTo),
        "related_to" => Ok(RelationKind::RelatedTo),
        _ => Err(ApplicationError::Integrity(format!(
            "unknown relation kind: {value}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use babata_application::{
        CaptureFileCommand, CaptureService,
        ports::{AssetStorePort, FinalizeAssetOutcome, RawRepositoryPort, StagedAsset},
    };
    use babata_domain::{ContentType, LogicalPath, Metadata, Sha256};
    use tempfile::tempdir;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum AssetFault {
        Finalize,
        Verify,
        Cleanup,
    }

    #[derive(Clone)]
    struct FaultingAssetStore {
        inner: crate::FileAssetStore,
        fault: AssetFault,
    }

    impl AssetStorePort for FaultingAssetStore {
        fn begin_operation(&self, operation_id: &str) -> Result<(), ApplicationError> {
            self.inner.begin_operation(operation_id)
        }

        fn preserve_operation(
            &self,
            operation_id: &str,
            revision_id: &str,
            failure_code: &str,
        ) -> Result<(), ApplicationError> {
            self.inner
                .preserve_operation(operation_id, revision_id, failure_code)
        }

        fn complete_operation(&self, operation_id: &str) -> Result<(), ApplicationError> {
            if self.fault == AssetFault::Cleanup {
                return Err(ApplicationError::Asset(
                    "injected cleanup failure".to_owned(),
                ));
            }
            self.inner.complete_operation(operation_id)
        }

        fn stage(
            &self,
            source: &str,
            role: AssetRole,
            operation_id: &str,
        ) -> Result<StagedAsset, ApplicationError> {
            self.inner.stage(source, role, operation_id)
        }

        fn hash(&self, source: &str) -> Result<Sha256, ApplicationError> {
            self.inner.hash(source)
        }

        fn finalize(&self, asset: &StagedAsset) -> Result<FinalizeAssetOutcome, ApplicationError> {
            if self.fault == AssetFault::Finalize {
                return Err(ApplicationError::Asset(
                    "injected finalization failure".to_owned(),
                ));
            }
            self.inner.finalize(asset)
        }

        fn discard_stage(&self, asset: &StagedAsset) -> Result<(), ApplicationError> {
            self.inner.discard_stage(asset)
        }

        fn open(&self, logical_path: &LogicalPath) -> Result<Vec<u8>, ApplicationError> {
            AssetStorePort::open(&self.inner, logical_path)
        }

        fn verify(&self, asset: &StagedAsset) -> Result<bool, ApplicationError> {
            if self.fault == AssetFault::Verify {
                return Ok(false);
            }
            self.inner.verify(asset)
        }

        fn quarantine_finalized(
            &self,
            asset: &StagedAsset,
            operation_id: &str,
            outcome: FinalizeAssetOutcome,
        ) -> Result<(), ApplicationError> {
            self.inner
                .quarantine_finalized(asset, operation_id, outcome)
        }
        fn hash_logical(&self, logical_path: &LogicalPath) -> Result<Sha256, ApplicationError> {
            self.inner.hash_logical(logical_path)
        }
        fn import_derived_file(
            &self,
            source: &str,
        ) -> Result<(LogicalPath, Sha256), ApplicationError> {
            self.inner.import_derived_file(source)
        }
    }

    fn file_command(path: &std::path::Path) -> CaptureFileCommand {
        CaptureFileCommand {
            provider: "fixture".to_owned(),
            path: path.to_string_lossy().into_owned(),
            context: None,
            locator: None,
            native_id: None,
            identity: None,
            metadata: Metadata::empty(),
            source_published_at: None,
        }
    }

    #[test]
    fn persists_and_reads_a_ready_text_record() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let repository = super::super::open_raw_database(&paths, 100).unwrap();
        let now = UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap();
        let source = NewSource {
            id: SourceId::new(),
            kind: SourceKind::External,
            provider: "fixture".to_owned(),
            account_or_workspace: None,
            created_at: now.clone(),
        };
        let item = NewItem {
            id: ItemId::new(),
            source_id: source.id.clone(),
            source_native_id: None,
            source_locator: None,
            source_identity_key: Some("text:test".to_owned()),
            content_type: ContentType::Text,
            source_published_at: None,
            first_captured_at: now.clone(),
            metadata: Metadata::empty(),
        };
        let revision = NewRevision {
            id: RevisionId::new(),
            item_id: item.id.clone(),
            parent_revision_id: None,
            kind: RevisionKind::Capture,
            ordinal: 1,
            captured_at: now,
            authored_at: None,
            revision_note: None,
            raw_text: Some("fixture".to_owned()),
            text_sha256: Some(Sha256::of_bytes(b"fixture")),
            metadata: Metadata::empty(),
        };
        repository
            .insert_capture_graph(&PersistGraph {
                operation: NewCaptureOperation {
                    operation_id: "op_ready_text".to_owned(),
                    item_id: item.id.clone(),
                    revision_id: revision.id.clone(),
                    source_native_id: None,
                    source_locator: None,
                    source_published_at: None,
                    metadata: Metadata::empty(),
                    started_at: revision.captured_at.clone(),
                },
                source,
                collection: None,
                item: item.clone(),
                revision: revision.clone(),
                assets: Vec::new(),
                relations: Vec::new(),
            })
            .unwrap();
        repository.mark_ready(&revision.id).unwrap();
        let detail = repository.load_detail(&item.id).unwrap();
        assert_eq!(detail.revisions.len(), 1);
        assert_eq!(detail.revisions[0].state, RawState::Ready);
    }

    #[test]
    fn invalid_relation_rolls_back_the_entire_capture_graph() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let repository = super::super::open_raw_database(&paths, 100).unwrap();
        let now = UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap();
        let source = NewSource {
            id: SourceId::new(),
            kind: SourceKind::External,
            provider: "fixture".to_owned(),
            account_or_workspace: None,
            created_at: now.clone(),
        };
        let item = NewItem {
            id: ItemId::new(),
            source_id: source.id.clone(),
            source_native_id: None,
            source_locator: None,
            source_identity_key: Some("rollback:test".to_owned()),
            content_type: ContentType::Text,
            source_published_at: None,
            first_captured_at: now.clone(),
            metadata: Metadata::empty(),
        };
        let revision = NewRevision {
            id: RevisionId::new(),
            item_id: item.id.clone(),
            parent_revision_id: None,
            kind: RevisionKind::Capture,
            ordinal: 1,
            captured_at: now,
            authored_at: None,
            revision_note: None,
            raw_text: Some("fixture".to_owned()),
            text_sha256: Some(Sha256::of_bytes(b"fixture")),
            metadata: Metadata::empty(),
        };
        let graph = PersistGraph {
            operation: NewCaptureOperation {
                operation_id: "op_rollback".to_owned(),
                item_id: item.id.clone(),
                revision_id: revision.id.clone(),
                source_native_id: None,
                source_locator: None,
                source_published_at: None,
                metadata: Metadata::empty(),
                started_at: revision.captured_at.clone(),
            },
            source: source.clone(),
            collection: None,
            item: item.clone(),
            revision,
            assets: Vec::new(),
            relations: vec![NewRelation {
                kind: RelationKind::RelatedTo,
                from_item_id: item.id.clone(),
                from_revision_id: None,
                to_item_id: item.id,
                to_revision_id: None,
            }],
        };
        assert!(repository.insert_capture_graph(&graph).is_err());
        assert!(
            repository
                .find_source(SourceKind::External, "fixture", None)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn collection_context_is_persisted_and_linked_to_its_item() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let repository = super::super::open_raw_database(&paths, 100).unwrap();
        let now = UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap();
        let source = NewSource {
            id: SourceId::new(),
            kind: SourceKind::External,
            provider: "fixture".to_owned(),
            account_or_workspace: None,
            created_at: now.clone(),
        };
        let collection = NewCollection {
            id: babata_domain::CollectionId::new(),
            source_id: source.id.clone(),
            native_id: "favorites".to_owned(),
            collection_kind: "context".to_owned(),
            title: "favorites".to_owned(),
            observed_at: now.clone(),
            metadata: Metadata::empty(),
        };
        let item = NewItem {
            id: ItemId::new(),
            source_id: source.id.clone(),
            source_native_id: None,
            source_locator: None,
            source_identity_key: Some("context:test".to_owned()),
            content_type: ContentType::Text,
            source_published_at: None,
            first_captured_at: now.clone(),
            metadata: Metadata::empty(),
        };
        let revision = NewRevision {
            id: RevisionId::new(),
            item_id: item.id.clone(),
            parent_revision_id: None,
            kind: RevisionKind::Capture,
            ordinal: 1,
            captured_at: now,
            authored_at: None,
            revision_note: None,
            raw_text: Some("fixture".to_owned()),
            text_sha256: Some(Sha256::of_bytes(b"fixture")),
            metadata: Metadata::empty(),
        };
        repository
            .insert_capture_graph(&PersistGraph {
                operation: NewCaptureOperation {
                    operation_id: "op_collection".to_owned(),
                    item_id: item.id.clone(),
                    revision_id: revision.id.clone(),
                    source_native_id: None,
                    source_locator: None,
                    source_published_at: None,
                    metadata: Metadata::empty(),
                    started_at: revision.captured_at.clone(),
                },
                source,
                collection: Some(collection),
                item,
                revision,
                assets: Vec::new(),
                relations: Vec::new(),
            })
            .unwrap();
        let connection = Connection::open(paths.raw_database()).unwrap();
        let memberships: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM collections c JOIN item_collections ic ON ic.collection_id = c.collection_id WHERE c.native_id = 'favorites'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(memberships, 1);
    }

    #[test]
    fn graph_failure_rolls_back_c0_and_cleans_staging_journal() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().join("data"));
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("fixture.txt");
        std::fs::write(&input, "graph rollback bytes").unwrap();
        let repository = super::super::open_raw_database(&paths, 100).unwrap();
        repository
            .connection
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_graph BEFORE INSERT ON revisions
                 BEGIN SELECT RAISE(ABORT, 'injected graph failure'); END;",
            )
            .unwrap();
        let error = CaptureService::new(
            repository.clone(),
            crate::FileAssetStore::new(paths.clone()),
            crate::SystemClock,
        )
        .capture_file(file_command(&input))
        .unwrap_err();
        let operation_id = error.operation_id().unwrap();
        let connection = repository.connection.lock().unwrap();
        for table in ["revisions", "assets", "capture_operations"] {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "{table}");
        }
        drop(connection);
        assert!(!paths.staging(operation_id).exists());
        assert_eq!(std::fs::read_dir(paths.journal()).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(paths.orphan()).unwrap().count(), 0);
    }

    #[test]
    fn finalization_failure_quarantines_operation_without_false_orphan() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().join("data"));
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("fixture.txt");
        std::fs::write(&input, "finalization failure bytes").unwrap();
        let repository = super::super::open_raw_database(&paths, 100).unwrap();
        let error = CaptureService::new(
            repository.clone(),
            FaultingAssetStore {
                inner: crate::FileAssetStore::new(paths.clone()),
                fault: AssetFault::Finalize,
            },
            crate::SystemClock,
        )
        .capture_file(file_command(&input))
        .unwrap_err();
        let operation_id = error.operation_id().unwrap();
        let connection = repository.connection.lock().unwrap();
        let (operation_state, revision_state, asset_state, logical_path): (
            String,
            String,
            String,
            String,
        ) = connection
            .query_row(
                "SELECT o.state, r.state, a.state, a.logical_path
                 FROM capture_operations o
                 JOIN revisions r ON r.revision_id = o.revision_id
                 JOIN assets a ON a.revision_id = r.revision_id
                 WHERE o.operation_id = ?1",
                params![operation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(operation_state, "quarantined");
        assert_eq!(revision_state, "quarantined");
        assert_eq!(asset_state, "quarantined");
        drop(connection);
        assert!(!paths.root().join(logical_path).exists());
        assert!(!paths.staging(operation_id).exists());
        assert_eq!(std::fs::read_dir(paths.journal()).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(paths.orphan()).unwrap().count(), 0);
    }

    #[test]
    fn verification_failure_preserves_final_bytes_and_recovery_evidence() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().join("data"));
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("fixture.txt");
        std::fs::write(&input, "verification failure bytes").unwrap();
        let repository = super::super::open_raw_database(&paths, 100).unwrap();
        let error = CaptureService::new(
            repository.clone(),
            FaultingAssetStore {
                inner: crate::FileAssetStore::new(paths.clone()),
                fault: AssetFault::Verify,
            },
            crate::SystemClock,
        )
        .capture_file(file_command(&input))
        .unwrap_err();
        let operation_id = error.operation_id().unwrap();
        let connection = repository.connection.lock().unwrap();
        let (operation_state, revision_state, asset_state, logical_path): (
            String,
            String,
            String,
            String,
        ) = connection
            .query_row(
                "SELECT o.state, r.state, a.state, a.logical_path
                 FROM capture_operations o
                 JOIN revisions r ON r.revision_id = o.revision_id
                 JOIN assets a ON a.revision_id = r.revision_id
                 WHERE o.operation_id = ?1",
                params![operation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(operation_state, "quarantined");
        assert_eq!(revision_state, "quarantined");
        assert_eq!(asset_state, "quarantined");
        drop(connection);
        assert_eq!(
            std::fs::read(paths.root().join(logical_path)).unwrap(),
            b"verification failure bytes"
        );
        assert_eq!(std::fs::read_dir(paths.journal()).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(paths.orphan()).unwrap().count(), 1);
    }

    #[test]
    fn cleanup_failure_returns_ready_with_pending_journal_warning() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().join("data"));
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("fixture.txt");
        std::fs::write(&input, "cleanup failure bytes").unwrap();
        let repository = super::super::open_raw_database(&paths, 100).unwrap();
        let outcome = CaptureService::new(
            repository.clone(),
            FaultingAssetStore {
                inner: crate::FileAssetStore::new(paths.clone()),
                fault: AssetFault::Cleanup,
            },
            crate::SystemClock,
        )
        .capture_file(file_command(&input))
        .unwrap();
        assert_eq!(outcome.status, "ready");
        assert!(outcome.record.is_some());
        assert!(
            outcome
                .warnings
                .iter()
                .any(|warning| warning.contains("journal cleanup"))
        );
        let connection = repository.connection.lock().unwrap();
        let (operation_state, revision_state, asset_state, logical_path): (
            String,
            String,
            String,
            String,
        ) = connection
            .query_row(
                "SELECT o.state, r.state, a.state, a.logical_path
                 FROM capture_operations o
                 JOIN revisions r ON r.revision_id = o.revision_id
                 JOIN assets a ON a.revision_id = r.revision_id
                 WHERE o.operation_id = ?1",
                params![outcome.operation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(operation_state, "ready");
        assert_eq!(revision_state, "ready");
        assert_eq!(asset_state, "ready");
        drop(connection);
        assert_eq!(
            std::fs::read(paths.root().join(logical_path)).unwrap(),
            b"cleanup failure bytes"
        );
        assert_eq!(std::fs::read_dir(paths.journal()).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(paths.orphan()).unwrap().count(), 0);
    }

    #[test]
    fn ready_transition_failure_keeps_recovery_evidence_and_no_false_ready() {
        let temporary = tempdir().unwrap();
        let paths = crate::paths::DataPaths::new(temporary.path().join("data"));
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("fixture.txt");
        std::fs::write(&input, "recover after ready failure").unwrap();
        let repository = super::super::open_raw_database(&paths, 100).unwrap();
        repository
            .connection
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_ready BEFORE UPDATE OF state ON revisions
                 WHEN NEW.state = 'ready'
                 BEGIN SELECT RAISE(ABORT, 'injected ready failure'); END;",
            )
            .unwrap();
        let service = CaptureService::new(
            repository.clone(),
            crate::FileAssetStore::new(paths.clone()),
            crate::SystemClock,
        );
        let error = service.capture_file(file_command(&input)).unwrap_err();
        let operation_id = error.operation_id().unwrap();
        let connection = repository.connection.lock().unwrap();
        let (operation_state, revision_state, asset_state, logical_path): (
            String,
            String,
            String,
            String,
        ) = connection
            .query_row(
                "SELECT o.state, r.state, a.state, a.logical_path
                 FROM capture_operations o
                 JOIN revisions r ON r.revision_id = o.revision_id
                 JOIN assets a ON a.revision_id = r.revision_id
                 WHERE o.operation_id = ?1",
                params![operation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(operation_state, "quarantined");
        assert_eq!(revision_state, "quarantined");
        assert_eq!(asset_state, "quarantined");
        drop(connection);
        assert!(paths.root().join(logical_path).exists());
        let status = super::super::raw_status(&paths, 100).unwrap();
        assert_eq!(status.quarantined_revisions, 1);
        assert_eq!(status.quarantined_operations, 1);
        assert_eq!(status.pending_journals, 1);
        assert_eq!(status.orphans, 1);
    }
}

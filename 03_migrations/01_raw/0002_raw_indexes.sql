CREATE UNIQUE INDEX ux_sources_identity
    ON sources(source_kind, provider, COALESCE(account_or_workspace, ''));
CREATE UNIQUE INDEX ux_items_external_identity
    ON items(source_id, source_identity_key)
    WHERE source_identity_key IS NOT NULL;
CREATE INDEX ix_items_source_capture ON items(source_id, first_captured_at DESC);
CREATE INDEX ix_revisions_item_ordinal ON revisions(item_id, ordinal DESC);
CREATE INDEX ix_revisions_parent ON revisions(parent_revision_id) WHERE parent_revision_id IS NOT NULL;
CREATE INDEX ix_revisions_text_hash ON revisions(text_sha256) WHERE text_sha256 IS NOT NULL;
CREATE INDEX ix_assets_sha256 ON assets(sha256);
CREATE INDEX ix_assets_revision ON assets(revision_id);
CREATE INDEX ix_relations_from ON relations(from_item_id, from_revision_id);
CREATE INDEX ix_relations_to ON relations(to_item_id, to_revision_id);
CREATE INDEX ix_item_collections_collection ON item_collections(collection_id);

-- P6.1 first vertical slice. Knowledge prose remains in first-party C0
-- revisions; these tables only preserve semantic identity and provenance.

CREATE TABLE knowledge_records (
    knowledge_id TEXT PRIMARY KEY,
    semantic_kind TEXT NOT NULL CHECK (semantic_kind IN (
        'map_direction', 'knowledge', 'case', 'log', 'insight'
    )),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    first_party_item_id TEXT NOT NULL UNIQUE REFERENCES items(item_id),
    source_item_id TEXT NOT NULL REFERENCES items(item_id),
    source_revision_id TEXT NOT NULL REFERENCES revisions(revision_id),
    created_at TEXT NOT NULL
);

CREATE TABLE knowledge_versions (
    knowledge_id TEXT NOT NULL REFERENCES knowledge_records(knowledge_id),
    ordinal INTEGER NOT NULL CHECK (ordinal >= 1),
    first_party_revision_id TEXT NOT NULL UNIQUE REFERENCES revisions(revision_id),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    created_at TEXT NOT NULL,
    PRIMARY KEY (knowledge_id, ordinal)
);

CREATE INDEX knowledge_records_source_revision_idx
ON knowledge_records(source_revision_id, created_at);

CREATE TRIGGER knowledge_records_bindings_insert
BEFORE INSERT ON knowledge_records
WHEN NOT EXISTS (
        SELECT 1 FROM revisions source_revision
        WHERE source_revision.revision_id = NEW.source_revision_id
          AND source_revision.item_id = NEW.source_item_id
          AND source_revision.state = 'ready'
     )
  OR NOT EXISTS (
        SELECT 1 FROM items first_party_item
        JOIN sources source ON source.source_id = first_party_item.source_id
        WHERE first_party_item.item_id = NEW.first_party_item_id
          AND source.source_kind = 'first_party'
     )
BEGIN
    SELECT RAISE(ABORT, 'knowledge record bindings are not ready C0 identities');
END;

CREATE TRIGGER knowledge_records_immutable_update
BEFORE UPDATE ON knowledge_records
BEGIN
    SELECT RAISE(ABORT, 'knowledge records are immutable');
END;

CREATE TRIGGER knowledge_records_append_only_delete
BEFORE DELETE ON knowledge_records
BEGIN
    SELECT RAISE(ABORT, 'knowledge records are append-only');
END;

CREATE TRIGGER knowledge_versions_bindings_insert
BEFORE INSERT ON knowledge_versions
WHEN NOT EXISTS (
        SELECT 1 FROM knowledge_records record
        JOIN revisions revision
          ON revision.revision_id = NEW.first_party_revision_id
        WHERE record.knowledge_id = NEW.knowledge_id
          AND revision.item_id = record.first_party_item_id
          AND revision.state = 'ready'
          AND revision.revision_kind IN ('authored', 'edit')
     )
  OR NEW.ordinal <> (
        SELECT COALESCE(MAX(existing.ordinal), 0) + 1
        FROM knowledge_versions existing
        WHERE existing.knowledge_id = NEW.knowledge_id
     )
BEGIN
    SELECT RAISE(ABORT, 'knowledge version is not the next ready first-party revision');
END;

CREATE TRIGGER knowledge_versions_immutable_update
BEFORE UPDATE ON knowledge_versions
BEGIN
    SELECT RAISE(ABORT, 'knowledge versions are immutable');
END;

CREATE TRIGGER knowledge_versions_append_only_delete
BEFORE DELETE ON knowledge_versions
BEGIN
    SELECT RAISE(ABORT, 'knowledge versions are append-only');
END;

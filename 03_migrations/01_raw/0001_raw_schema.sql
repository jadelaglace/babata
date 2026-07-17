CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    applied_at TEXT NOT NULL,
    checksum_sha256 TEXT NOT NULL
);

CREATE TABLE sources (
    source_id TEXT PRIMARY KEY,
    source_kind TEXT NOT NULL CHECK (source_kind IN ('external', 'first_party')),
    provider TEXT NOT NULL,
    display_name TEXT,
    account_or_workspace TEXT,
    base_locator TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    UNIQUE (source_kind, provider, account_or_workspace)
);

CREATE TABLE collections (
    collection_id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES sources(source_id),
    parent_collection_id TEXT REFERENCES collections(collection_id),
    native_id TEXT,
    locator TEXT,
    collection_kind TEXT NOT NULL,
    title TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    observed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (source_id, native_id)
);

CREATE TABLE items (
    item_id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES sources(source_id),
    source_native_id TEXT,
    source_locator TEXT,
    source_identity_key TEXT,
    content_type TEXT NOT NULL CHECK (content_type IN ('text', 'document', 'image', 'audio', 'video', 'web_page', 'archive', 'unknown')),
    source_published_at TEXT,
    source_updated_at TEXT,
    first_captured_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);

CREATE TABLE item_collections (
    item_id TEXT NOT NULL REFERENCES items(item_id),
    collection_id TEXT NOT NULL REFERENCES collections(collection_id),
    membership_role TEXT NOT NULL DEFAULT 'observed',
    observed_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (item_id, collection_id, membership_role)
);

CREATE TABLE revisions (
    revision_id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES items(item_id),
    parent_revision_id TEXT REFERENCES revisions(revision_id),
    revision_kind TEXT NOT NULL CHECK (revision_kind IN ('capture', 'import', 'authored', 'edit', 'annotation')),
    ordinal INTEGER NOT NULL CHECK (ordinal >= 1),
    captured_at TEXT NOT NULL,
    authored_at TEXT,
    revision_note TEXT,
    raw_text TEXT,
    text_sha256 TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    state TEXT NOT NULL CHECK (state IN ('pending', 'ready', 'quarantined')),
    created_at TEXT NOT NULL,
    UNIQUE (item_id, ordinal),
    CHECK ((raw_text IS NULL AND text_sha256 IS NULL) OR (raw_text IS NOT NULL AND length(text_sha256) = 64))
);

CREATE TABLE assets (
    asset_id TEXT PRIMARY KEY,
    revision_id TEXT NOT NULL REFERENCES revisions(revision_id),
    asset_role TEXT NOT NULL CHECK (asset_role IN ('original', 'attachment', 'export', 'cover', 'derived', 'preview')),
    logical_path TEXT NOT NULL,
    sha256 TEXT NOT NULL CHECK (length(sha256) = 64),
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    media_type TEXT,
    original_filename TEXT,
    state TEXT NOT NULL CHECK (state IN ('pending', 'ready', 'quarantined')),
    created_at TEXT NOT NULL,
    UNIQUE (revision_id, logical_path)
);

CREATE TABLE relations (
    relation_id TEXT PRIMARY KEY,
    from_item_id TEXT NOT NULL REFERENCES items(item_id),
    from_revision_id TEXT REFERENCES revisions(revision_id),
    relation_kind TEXT NOT NULL CHECK (relation_kind IN ('revises', 'annotates', 'quotes', 'responds_to', 'related_to')),
    to_item_id TEXT NOT NULL REFERENCES items(item_id),
    to_revision_id TEXT REFERENCES revisions(revision_id),
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    CHECK (from_item_id <> to_item_id OR relation_kind = 'revises')
);

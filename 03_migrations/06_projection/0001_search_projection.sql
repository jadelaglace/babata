CREATE TABLE projection_schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    applied_at TEXT NOT NULL,
    checksum_sha256 TEXT NOT NULL
);

CREATE TABLE projection_metadata (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    built_at TEXT NOT NULL,
    raw_items INTEGER NOT NULL,
    semantic_entries INTEGER NOT NULL,
    relations INTEGER NOT NULL,
    source_fingerprint TEXT NOT NULL
);

CREATE TABLE search_records (
    record_id TEXT PRIMARY KEY,
    record_kind TEXT NOT NULL CHECK (record_kind IN ('raw_item', 'semantic_entry')),
    item_id TEXT,
    revision_id TEXT,
    semantic_id TEXT,
    source_id TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    provider TEXT NOT NULL,
    content_type TEXT NOT NULL,
    semantic_kind TEXT,
    realm TEXT,
    title TEXT NOT NULL,
    body_text TEXT NOT NULL DEFAULT '',
    state TEXT NOT NULL,
    processing_state TEXT NOT NULL,
    origin_kind TEXT NOT NULL,
    review_state TEXT,
    event_at TEXT NOT NULL,
    restricted INTEGER NOT NULL CHECK (restricted IN (0, 1)),
    missing INTEGER NOT NULL CHECK (missing IN (0, 1)),
    media_only INTEGER NOT NULL CHECK (media_only IN (0, 1)),
    attachment_only INTEGER NOT NULL CHECK (attachment_only IN (0, 1)),
    human_judgment INTEGER NOT NULL CHECK (human_judgment IN (0, 1)),
    confirmed_fact INTEGER NOT NULL CHECK (confirmed_fact IN (0, 1)),
    metadata_json TEXT NOT NULL CHECK (json_valid(metadata_json))
);

CREATE VIRTUAL TABLE search_records_fts USING fts5(
    record_id UNINDEXED,
    title,
    body_text,
    facets,
    tokenize = 'unicode61'
);

CREATE TABLE search_people (
    record_id TEXT NOT NULL REFERENCES search_records(record_id) ON DELETE CASCADE,
    person TEXT NOT NULL COLLATE NOCASE,
    PRIMARY KEY (record_id, person)
);

CREATE TABLE search_maps (
    record_id TEXT NOT NULL REFERENCES search_records(record_id) ON DELETE CASCADE,
    map_node_id TEXT NOT NULL,
    name TEXT NOT NULL COLLATE NOCASE,
    level TEXT NOT NULL,
    lifecycle TEXT NOT NULL,
    PRIMARY KEY (record_id, map_node_id)
);

CREATE TABLE search_tags (
    record_id TEXT NOT NULL REFERENCES search_records(record_id) ON DELETE CASCADE,
    tag TEXT NOT NULL COLLATE NOCASE,
    PRIMARY KEY (record_id, tag)
);

CREATE TABLE search_scores (
    record_id TEXT NOT NULL REFERENCES search_records(record_id) ON DELETE CASCADE,
    score_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    profile_id TEXT NOT NULL,
    profile_ordinal INTEGER NOT NULL,
    interest_weight INTEGER NOT NULL,
    strategy_weight INTEGER NOT NULL,
    consensus_weight INTEGER NOT NULL,
    interest INTEGER NOT NULL,
    strategy INTEGER NOT NULL,
    consensus INTEGER NOT NULL,
    weighted_score INTEGER NOT NULL,
    rationale TEXT NOT NULL,
    provenance_kind TEXT NOT NULL,
    author TEXT NOT NULL,
    created_at TEXT NOT NULL,
    eligible_for_surface INTEGER NOT NULL CHECK (eligible_for_surface IN (0, 1)),
    PRIMARY KEY (record_id, score_id)
);

CREATE TABLE search_revisions (
    record_id TEXT NOT NULL REFERENCES search_records(record_id) ON DELETE CASCADE,
    revision_id TEXT NOT NULL,
    parent_revision_id TEXT,
    ordinal INTEGER NOT NULL,
    kind TEXT NOT NULL,
    state TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    authored_at TEXT,
    text_sha256 TEXT,
    PRIMARY KEY (record_id, revision_id)
);

CREATE TABLE search_assets (
    record_id TEXT NOT NULL REFERENCES search_records(record_id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL,
    revision_id TEXT NOT NULL,
    role TEXT NOT NULL,
    logical_path TEXT NOT NULL,
    media_type TEXT,
    state TEXT NOT NULL,
    missing INTEGER NOT NULL CHECK (missing IN (0, 1)),
    PRIMARY KEY (record_id, asset_id)
);

CREATE TABLE search_derivatives (
    record_id TEXT NOT NULL REFERENCES search_records(record_id) ON DELETE CASCADE,
    derivative_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    revision_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    processing_state TEXT NOT NULL,
    output_sha256 TEXT,
    logical_path TEXT,
    media_type TEXT,
    invalidated INTEGER NOT NULL CHECK (invalidated IN (0, 1)),
    missing INTEGER NOT NULL CHECK (missing IN (0, 1)),
    created_at TEXT NOT NULL,
    PRIMARY KEY (record_id, derivative_id)
);

CREATE TABLE search_relations (
    relation_key TEXT PRIMARY KEY,
    record_id TEXT NOT NULL REFERENCES search_records(record_id) ON DELETE CASCADE,
    direction TEXT NOT NULL CHECK (direction IN ('outgoing', 'incoming')),
    relation_kind TEXT NOT NULL,
    related_record_id TEXT,
    related_entity_id TEXT NOT NULL,
    related_title TEXT,
    evidence TEXT,
    broken INTEGER NOT NULL CHECK (broken IN (0, 1))
);

CREATE INDEX search_records_source_idx
ON search_records(source_kind, provider, content_type, event_at);
CREATE INDEX search_records_semantic_idx
ON search_records(semantic_kind, realm, origin_kind, review_state);
CREATE INDEX search_records_state_idx
ON search_records(state, processing_state, restricted, missing);
CREATE INDEX search_people_person_idx ON search_people(person, record_id);
CREATE INDEX search_maps_node_idx ON search_maps(map_node_id, name, record_id);
CREATE INDEX search_tags_tag_idx ON search_tags(tag, record_id);
CREATE INDEX search_scores_profile_idx
ON search_scores(profile_id, weighted_score, created_at, record_id);
CREATE INDEX search_relations_record_idx
ON search_relations(record_id, relation_kind, direction);
CREATE INDEX search_relations_related_idx
ON search_relations(related_record_id, relation_kind);

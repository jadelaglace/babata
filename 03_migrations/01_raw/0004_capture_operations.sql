CREATE TABLE capture_operations (
    operation_id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES items(item_id),
    revision_id TEXT NOT NULL UNIQUE REFERENCES revisions(revision_id),
    source_native_id TEXT,
    source_locator TEXT,
    source_published_at TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    state TEXT NOT NULL CHECK (state IN ('pending', 'ready', 'quarantined')),
    failure_code TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE INDEX ix_capture_operations_item
    ON capture_operations(item_id, started_at);
CREATE INDEX ix_capture_operations_state
    ON capture_operations(state, started_at);

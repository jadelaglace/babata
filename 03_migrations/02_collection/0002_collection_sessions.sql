CREATE TABLE collection_sessions (
    session_id TEXT PRIMARY KEY,
    route_id TEXT NOT NULL,
    source_reference TEXT NOT NULL,
    scope_description TEXT NOT NULL,
    authorisation_id TEXT NOT NULL,
    state TEXT NOT NULL CHECK (
        state IN ('discovering', 'awaiting_selection', 'running', 'completed', 'cancelled', 'failed')
    ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE collection_candidates (
    session_id TEXT NOT NULL REFERENCES collection_sessions(session_id) ON DELETE CASCADE,
    candidate_id TEXT NOT NULL,
    route_id TEXT NOT NULL,
    source_native_id TEXT,
    title TEXT,
    source_location TEXT,
    hierarchy_json TEXT NOT NULL,
    content_type TEXT NOT NULL,
    source_updated_at TEXT,
    attachment_available INTEGER CHECK (attachment_available IN (0, 1)),
    limitations_json TEXT NOT NULL,
    selection_capabilities_json TEXT NOT NULL,
    prefetched_envelope_json TEXT,
    PRIMARY KEY (session_id, candidate_id)
);

CREATE TABLE collection_items (
    session_id TEXT NOT NULL,
    candidate_id TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('queued', 'running', 'saved', 'skipped', 'failed')),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    reason TEXT,
    retryable INTEGER NOT NULL DEFAULT 0 CHECK (retryable IN (0, 1)),
    item_id TEXT REFERENCES items(item_id),
    revision_id TEXT REFERENCES revisions(revision_id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (session_id, candidate_id),
    FOREIGN KEY (session_id, candidate_id)
        REFERENCES collection_candidates(session_id, candidate_id)
);

CREATE TABLE collection_recollection_checks (
    check_id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES items(item_id),
    state TEXT NOT NULL CHECK (state IN ('changed', 'unchanged', 'inaccessible', 'removed')),
    previous_revision_id TEXT NOT NULL REFERENCES revisions(revision_id),
    new_revision_id TEXT REFERENCES revisions(revision_id),
    reason TEXT,
    checked_at TEXT NOT NULL
);

CREATE INDEX ix_collection_sessions_route_updated
    ON collection_sessions(route_id, updated_at DESC);
CREATE INDEX ix_collection_items_state_updated
    ON collection_items(state, updated_at DESC);
CREATE INDEX ix_collection_items_item
    ON collection_items(item_id, updated_at DESC);
CREATE INDEX ix_collection_recollection_item_checked
    ON collection_recollection_checks(item_id, checked_at DESC);

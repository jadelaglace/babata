-- C1 derived schema: process runs and derivatives.
-- Retries create new process_runs rows; derivatives reference runs only.

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    applied_at TEXT NOT NULL,
    checksum_sha256 TEXT NOT NULL
);

CREATE TABLE process_runs (
    run_id TEXT PRIMARY KEY,
    pipeline_id TEXT NOT NULL,
    input_revision_id TEXT NOT NULL,
    input_item_id TEXT,
    input_sha256 TEXT NOT NULL CHECK (length(input_sha256) = 64),
    state TEXT NOT NULL CHECK (state IN ('pending', 'running', 'succeeded', 'failed', 'cancelled')),
    provider TEXT NOT NULL,
    tool_or_model TEXT,
    tool_version TEXT,
    attempt INTEGER NOT NULL DEFAULT 1 CHECK (attempt >= 1),
    retry_of_run_id TEXT REFERENCES process_runs(run_id),
    error_code TEXT,
    error_message TEXT,
    params_json TEXT NOT NULL DEFAULT '{}',
    usage_json TEXT NOT NULL DEFAULT '{}',
    loss_notes TEXT,
    created_at TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT
);

CREATE INDEX process_runs_revision_idx ON process_runs(input_revision_id, created_at);
CREATE INDEX process_runs_pipeline_idx ON process_runs(pipeline_id, state);
CREATE INDEX process_runs_retry_idx ON process_runs(retry_of_run_id);

CREATE TABLE derivatives (
    derivative_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES process_runs(run_id),
    kind TEXT NOT NULL CHECK (kind IN (
        'extracted_text',
        'ocr_text',
        'transcript',
        'subtitle',
        'summary',
        'visual_description',
        'key_frame',
        'tags',
        'structured_result',
        'media_metadata'
    )),
    output_sha256 TEXT CHECK (output_sha256 IS NULL OR length(output_sha256) = 64),
    content_text TEXT,
    content_json TEXT,
    logical_path TEXT,
    media_type TEXT,
    language TEXT,
    input_asset_id TEXT,
    loss_notes TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    CHECK (
        content_text IS NOT NULL
        OR content_json IS NOT NULL
        OR logical_path IS NOT NULL
        OR output_sha256 IS NOT NULL
    )
);

CREATE INDEX derivatives_run_idx ON derivatives(run_id);
CREATE INDEX derivatives_kind_idx ON derivatives(kind);

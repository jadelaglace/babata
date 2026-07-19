CREATE TABLE runtime_schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    applied_at TEXT NOT NULL,
    checksum_sha256 TEXT NOT NULL
);

CREATE TABLE process_jobs (
    job_id TEXT PRIMARY KEY,
    pipeline_id TEXT NOT NULL,
    input_revision_id TEXT NOT NULL,
    input_item_id TEXT,
    input_sha256 TEXT NOT NULL CHECK (length(input_sha256) = 64),
    target_kind TEXT NOT NULL CHECK (target_kind IN (
        'extracted_text', 'ocr_text', 'transcript', 'subtitle', 'summary',
        'visual_description', 'key_frame', 'tags', 'structured_result', 'media_metadata'
    )),
    input_asset_id TEXT,
    state TEXT NOT NULL CHECK (state IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
    provider TEXT NOT NULL,
    tool_or_model TEXT NOT NULL,
    tool_version TEXT NOT NULL,
    attempt INTEGER NOT NULL CHECK (attempt >= 1),
    retry_of_job_id TEXT REFERENCES process_jobs(job_id),
    worker_id TEXT,
    lease_expires_at TEXT,
    provider_task_provider TEXT,
    provider_task_id TEXT,
    error_code TEXT,
    error_message TEXT,
    result_run_id TEXT,
    cancel_requested INTEGER NOT NULL DEFAULT 0 CHECK (cancel_requested IN (0, 1)),
    params_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    started_at TEXT,
    heartbeat_at TEXT,
    finished_at TEXT
);

CREATE INDEX process_jobs_claim_idx ON process_jobs(state, created_at);
CREATE INDEX process_jobs_revision_idx ON process_jobs(input_revision_id, created_at);
CREATE INDEX process_jobs_retry_idx ON process_jobs(retry_of_job_id);
CREATE INDEX process_jobs_lease_idx ON process_jobs(state, lease_expires_at);

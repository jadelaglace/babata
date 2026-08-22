-- C1B may retain irreducible audio, continuous-video, or attachment excerpts.
-- Rebuild both tables so their closed derivative-kind constraints stay aligned.

PRAGMA foreign_keys = OFF;
PRAGMA defer_foreign_keys = ON;

CREATE TABLE process_runs_new (
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
    retry_of_run_id TEXT REFERENCES process_runs_new(run_id),
    error_code TEXT,
    error_message TEXT,
    params_json TEXT NOT NULL DEFAULT '{}',
    usage_json TEXT NOT NULL DEFAULT '{}',
    loss_notes TEXT,
    created_at TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    target_kind TEXT CHECK (target_kind IS NULL OR target_kind IN (
        'extracted_text',
        'ocr_text',
        'transcript',
        'subtitle',
        'summary',
        'visual_description',
        'key_frame',
        'audio_excerpt',
        'video_excerpt',
        'attachment_excerpt',
        'tags',
        'structured_result',
        'media_metadata'
    )),
    input_asset_id TEXT,
    invalidated_at TEXT,
    invalidation_reason TEXT
);

INSERT INTO process_runs_new (
    run_id, pipeline_id, input_revision_id, input_item_id, input_sha256,
    state, provider, tool_or_model, tool_version, attempt, retry_of_run_id,
    error_code, error_message, params_json, usage_json, loss_notes, created_at,
    started_at, finished_at, target_kind, input_asset_id, invalidated_at,
    invalidation_reason
)
SELECT
    run_id, pipeline_id, input_revision_id, input_item_id, input_sha256,
    state, provider, tool_or_model, tool_version, attempt, retry_of_run_id,
    error_code, error_message, params_json, usage_json, loss_notes, created_at,
    started_at, finished_at, target_kind, input_asset_id, invalidated_at,
    invalidation_reason
FROM process_runs;

CREATE TABLE derivatives_new (
    derivative_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES process_runs_new(run_id),
    kind TEXT NOT NULL CHECK (kind IN (
        'extracted_text',
        'ocr_text',
        'transcript',
        'subtitle',
        'summary',
        'visual_description',
        'key_frame',
        'audio_excerpt',
        'video_excerpt',
        'attachment_excerpt',
        'tags',
        'structured_result',
        'media_metadata'
    )),
    output_sha256 TEXT NOT NULL CHECK (length(output_sha256) = 64),
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
    )
);

INSERT INTO derivatives_new (
    derivative_id, run_id, kind, output_sha256, content_text, content_json,
    logical_path, media_type, language, input_asset_id, loss_notes,
    metadata_json, created_at
)
SELECT
    derivative_id, run_id, kind, output_sha256, content_text, content_json,
    logical_path, media_type, language, input_asset_id, loss_notes,
    metadata_json, created_at
FROM derivatives;

DROP TABLE derivatives;
DROP TABLE process_runs;
ALTER TABLE process_runs_new RENAME TO process_runs;
ALTER TABLE derivatives_new RENAME TO derivatives;

CREATE INDEX process_runs_revision_idx ON process_runs(input_revision_id, created_at);
CREATE INDEX process_runs_pipeline_idx ON process_runs(pipeline_id, state);
CREATE INDEX process_runs_retry_idx ON process_runs(retry_of_run_id);
CREATE INDEX process_runs_active_revision_idx
ON process_runs(input_revision_id, created_at)
WHERE invalidated_at IS NULL;
CREATE INDEX derivatives_run_idx ON derivatives(run_id);
CREATE INDEX derivatives_kind_idx ON derivatives(kind);

PRAGMA foreign_keys = ON;

-- C1 hardening (#48): every derivative must carry a verifiable output hash.
-- A run+derivative commit is atomic, so succeeded runs always have outputs.

PRAGMA foreign_keys = OFF;

CREATE TABLE derivatives_new (
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
ALTER TABLE derivatives_new RENAME TO derivatives;

CREATE INDEX derivatives_run_idx ON derivatives(run_id);
CREATE INDEX derivatives_kind_idx ON derivatives(kind);

PRAGMA foreign_keys = ON;
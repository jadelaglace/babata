-- Failed attempts need enough identity to prove that a retry is the same task.
-- Existing successful rows are backfilled from their single derivative. Legacy
-- failed rows remain NULL and cannot be used as retry parents.

ALTER TABLE process_runs ADD COLUMN target_kind TEXT CHECK (target_kind IS NULL OR target_kind IN (
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
));

ALTER TABLE process_runs ADD COLUMN input_asset_id TEXT;

UPDATE process_runs
SET target_kind = (
        SELECT kind FROM derivatives
        WHERE derivatives.run_id = process_runs.run_id
        ORDER BY created_at ASC LIMIT 1
    ),
    input_asset_id = (
        SELECT input_asset_id FROM derivatives
        WHERE derivatives.run_id = process_runs.run_id
        ORDER BY created_at ASC LIMIT 1
    )
WHERE state = 'succeeded';

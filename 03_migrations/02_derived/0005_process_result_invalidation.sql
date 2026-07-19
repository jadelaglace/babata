-- C1 results can be logically deleted and rebuilt without changing C0 or
-- erasing processing history. The run remains an audit record, while these
-- fields make it explicit that its derivative is no longer authoritative.

ALTER TABLE process_runs ADD COLUMN invalidated_at TEXT;
ALTER TABLE process_runs ADD COLUMN invalidation_reason TEXT;

CREATE INDEX process_runs_active_revision_idx
ON process_runs(input_revision_id, created_at)
WHERE invalidated_at IS NULL;

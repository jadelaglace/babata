-- Attachment-only recovery appends assets and provenance to an existing ready
-- revision. It must not manufacture a duplicate content revision.

CREATE TABLE asset_attachment_operations (
    operation_id TEXT PRIMARY KEY,
    revision_id TEXT NOT NULL REFERENCES revisions(revision_id),
    reason TEXT NOT NULL CHECK (length(trim(reason)) > 0),
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(metadata_json)),
    state TEXT NOT NULL CHECK (state IN ('pending', 'ready', 'quarantined')),
    failure_code TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    CHECK (
        (state = 'pending' AND failure_code IS NULL AND completed_at IS NULL)
        OR (state = 'ready' AND failure_code IS NULL AND completed_at IS NOT NULL)
        OR (state = 'quarantined' AND failure_code IS NOT NULL AND completed_at IS NOT NULL)
    )
);

CREATE TABLE asset_attachment_members (
    operation_id TEXT NOT NULL REFERENCES asset_attachment_operations(operation_id),
    asset_id TEXT NOT NULL UNIQUE REFERENCES assets(asset_id),
    PRIMARY KEY (operation_id, asset_id)
);

CREATE TRIGGER asset_attachment_revision_guard
BEFORE INSERT ON asset_attachment_members
WHEN (
    SELECT asset.revision_id
    FROM assets asset
    WHERE asset.asset_id = NEW.asset_id
) <> (
    SELECT operation.revision_id
    FROM asset_attachment_operations operation
    WHERE operation.operation_id = NEW.operation_id
)
BEGIN
    SELECT RAISE(ABORT, 'attachment asset and operation revision mismatch');
END;

CREATE INDEX asset_attachment_operations_revision_idx
ON asset_attachment_operations(revision_id, started_at, operation_id);

CREATE INDEX asset_attachment_operations_state_idx
ON asset_attachment_operations(state, started_at, operation_id);

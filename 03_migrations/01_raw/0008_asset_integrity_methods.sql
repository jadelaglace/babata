-- Assets may be admitted either with a SHA-256 proof or with an honest
-- size/source snapshot. The latter deliberately leaves sha256 NULL.
PRAGMA defer_foreign_keys = ON;

CREATE TABLE assets_v2 (
    asset_id TEXT PRIMARY KEY,
    revision_id TEXT NOT NULL REFERENCES revisions(revision_id),
    asset_role TEXT NOT NULL CHECK (asset_role IN ('original', 'attachment', 'export', 'cover', 'derived', 'preview')),
    logical_path TEXT NOT NULL,
    sha256 TEXT CHECK (sha256 IS NULL OR length(sha256) = 64),
    integrity_method TEXT NOT NULL DEFAULT 'sha256_v1' CHECK (integrity_method IN ('sha256_v1', 'size_snapshot_v1')),
    integrity_metadata_json TEXT NOT NULL DEFAULT '{}',
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    media_type TEXT,
    original_filename TEXT,
    state TEXT NOT NULL CHECK (state IN ('pending', 'ready', 'quarantined')),
    created_at TEXT NOT NULL,
    UNIQUE (revision_id, logical_path),
    CHECK (
        (integrity_method = 'sha256_v1' AND sha256 IS NOT NULL)
        OR (integrity_method = 'size_snapshot_v1' AND sha256 IS NULL)
    )
);

INSERT INTO assets_v2 (
    asset_id, revision_id, asset_role, logical_path, sha256,
    integrity_method, integrity_metadata_json, byte_size, media_type,
    original_filename, state, created_at
)
SELECT asset_id, revision_id, asset_role, logical_path, sha256,
       'sha256_v1', json_object('method', 'sha256_v1'), byte_size, media_type,
       original_filename, state, created_at
FROM assets;

DROP TRIGGER asset_attachment_revision_guard;
CREATE TEMP TABLE asset_attachment_members_backup AS
SELECT operation_id, asset_id
FROM asset_attachment_members;
DROP TABLE asset_attachment_members;
DROP TABLE assets;
ALTER TABLE assets_v2 RENAME TO assets;
CREATE INDEX ix_assets_sha256 ON assets(sha256) WHERE sha256 IS NOT NULL;
CREATE INDEX ix_assets_revision ON assets(revision_id);

CREATE TABLE asset_attachment_members (
    operation_id TEXT NOT NULL REFERENCES asset_attachment_operations(operation_id),
    asset_id TEXT NOT NULL UNIQUE REFERENCES assets(asset_id),
    PRIMARY KEY (operation_id, asset_id)
);
INSERT INTO asset_attachment_members (operation_id, asset_id)
SELECT operation_id, asset_id
FROM asset_attachment_members_backup;
DROP TABLE asset_attachment_members_backup;

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

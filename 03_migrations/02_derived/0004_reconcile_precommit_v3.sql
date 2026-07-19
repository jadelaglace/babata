-- A pre-commit P5 binary applied the final v3 schema with a different SQL
-- checksum to the real data root. The Rust migration runner accepts that one
-- checksum only after verifying the required v3 columns. This migration then
-- records the repair and normalizes migration history to the committed v3.

CREATE TABLE schema_migration_repairs (
    repair_id INTEGER PRIMARY KEY AUTOINCREMENT,
    version INTEGER NOT NULL,
    name TEXT NOT NULL,
    old_checksum_sha256 TEXT NOT NULL,
    new_checksum_sha256 TEXT NOT NULL,
    reason TEXT NOT NULL,
    repaired_at TEXT NOT NULL
);

INSERT INTO schema_migration_repairs (
    version, name, old_checksum_sha256, new_checksum_sha256, reason, repaired_at
)
SELECT
    version,
    name,
    checksum_sha256,
    '3659f6ae590666210b3ef5ed0fe5d125675199a9ec04cd1055b9d0fadb703423',
    'reconcile verified pre-commit P5 v3 schema',
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM schema_migrations
WHERE version = 3
  AND checksum_sha256 = 'abe984dfb0f288497d7f18a17d5ef4293d652d7417b7287160d2932ddd17fbef';

UPDATE schema_migrations
SET checksum_sha256 = '3659f6ae590666210b3ef5ed0fe5d125675199a9ec04cd1055b9d0fadb703423'
WHERE version = 3
  AND checksum_sha256 = 'abe984dfb0f288497d7f18a17d5ef4293d652d7417b7287160d2932ddd17fbef';

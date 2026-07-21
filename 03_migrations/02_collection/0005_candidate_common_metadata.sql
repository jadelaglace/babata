-- Keep the original discovery columns readable while new writers persist the
-- shared versioned C0 metadata contract beside them.

ALTER TABLE collection_candidates ADD COLUMN common_metadata_json TEXT NOT NULL
DEFAULT '{"schema":"babata.c0.common/v1"}'
CHECK (json_valid(common_metadata_json));

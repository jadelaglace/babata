-- Items retain first-observed source facts. Every later source observation is
-- append-only and does not imply that source content changed.

ALTER TABLE items ADD COLUMN common_metadata_json TEXT NOT NULL
DEFAULT '{"schema":"babata.c0.common/v1"}'
CHECK (json_valid(common_metadata_json));

CREATE TABLE source_observations (
    observation_id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES items(item_id),
    revision_id TEXT NOT NULL REFERENCES revisions(revision_id),
    capture_operation_id TEXT UNIQUE REFERENCES capture_operations(operation_id),
    collection_session_id TEXT,
    candidate_id TEXT,
    observation_kind TEXT NOT NULL CHECK (observation_kind IN ('capture', 'recollection')),
    recollection_state TEXT CHECK (
        recollection_state IN ('unchanged', 'inaccessible', 'removed')
    ),
    source_native_id TEXT,
    source_locator TEXT,
    context TEXT,
    common_metadata_json TEXT NOT NULL CHECK (json_valid(common_metadata_json)),
    provider_metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(provider_metadata_json)),
    reason TEXT,
    observed_at TEXT NOT NULL,
    CHECK (
        (observation_kind = 'capture'
         AND capture_operation_id IS NOT NULL
         AND recollection_state IS NULL)
        OR
        (observation_kind = 'recollection'
         AND capture_operation_id IS NULL
         AND recollection_state IS NOT NULL)
    ),
    CHECK (
        (collection_session_id IS NULL AND candidate_id IS NULL)
        OR (collection_session_id IS NOT NULL AND candidate_id IS NOT NULL)
    )
);

CREATE TRIGGER source_observation_revision_item_guard
BEFORE INSERT ON source_observations
WHEN EXISTS (
    SELECT 1 FROM revisions revision
    WHERE revision.revision_id = NEW.revision_id
      AND revision.item_id <> NEW.item_id
)
BEGIN
    SELECT RAISE(ABORT, 'source observation revision does not belong to item');
END;

CREATE TRIGGER source_observation_capture_guard
BEFORE INSERT ON source_observations
WHEN NEW.capture_operation_id IS NOT NULL
  AND EXISTS (
      SELECT 1 FROM capture_operations operation
      WHERE operation.operation_id = NEW.capture_operation_id
        AND (operation.item_id <> NEW.item_id OR operation.revision_id <> NEW.revision_id)
  )
BEGIN
    SELECT RAISE(ABORT, 'source observation capture operation mismatch');
END;

CREATE TRIGGER source_observations_append_only_update
BEFORE UPDATE ON source_observations
BEGIN
    SELECT RAISE(ABORT, 'source observations are append-only');
END;

CREATE TRIGGER source_observations_append_only_delete
BEFORE DELETE ON source_observations
BEGIN
    SELECT RAISE(ABORT, 'source observations are append-only');
END;

CREATE INDEX source_observations_item_observed_idx
ON source_observations(item_id, observed_at, observation_id);

CREATE INDEX source_observations_revision_idx
ON source_observations(revision_id, observed_at, observation_id);

-- Normalize the culture-formatted UTC timestamps emitted by the pre-RFC3339
-- knowledge writer. The old writer stored UTC, but formatted it as
-- MM/DD/YYYY HH:MM:SS on Windows. Keep the repair narrow and deterministic;
-- all new writes must continue to use RFC3339 UTC. The one protected append-only
-- table is opened only for this exact repair inside the migration transaction;
-- its immutable trigger is restored before the transaction commits.

DROP TRIGGER IF EXISTS model_suggestions_immutable_update;

UPDATE model_suggestions
SET created_at = substr(created_at, 7, 4) || '-' ||
                 substr(created_at, 1, 2) || '-' ||
                 substr(created_at, 4, 2) || 'T' ||
                 substr(created_at, 12, 2) || ':' ||
                 substr(created_at, 15, 2) || ':' ||
                 substr(created_at, 18, 2) || 'Z'
WHERE created_at GLOB
      '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]';

CREATE TRIGGER model_suggestions_immutable_update
BEFORE UPDATE ON model_suggestions
BEGIN
    SELECT RAISE(ABORT, 'model suggestions are immutable');
END;

UPDATE semantic_entries
SET created_at = substr(created_at, 7, 4) || '-' ||
                 substr(created_at, 1, 2) || '-' ||
                 substr(created_at, 4, 2) || 'T' ||
                 substr(created_at, 12, 2) || ':' ||
                 substr(created_at, 15, 2) || ':' ||
                 substr(created_at, 18, 2) || 'Z'
WHERE created_at GLOB
      '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]';

UPDATE semantic_map_assignment_events
SET created_at = substr(created_at, 7, 4) || '-' ||
                 substr(created_at, 1, 2) || '-' ||
                 substr(created_at, 4, 2) || 'T' ||
                 substr(created_at, 12, 2) || ':' ||
                 substr(created_at, 15, 2) || ':' ||
                 substr(created_at, 18, 2) || 'Z'
WHERE created_at GLOB
      '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]';

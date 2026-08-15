-- Complete the repair after the source was traced to PowerShell JSON date
-- coercion. Version 0006 remains immutable because it is already recorded in
-- real migration history. Normalize every semantic row that can inherit the
-- malformed package generated_at while preserving IDs and references.

DROP TRIGGER IF EXISTS model_suggestions_immutable_update;

UPDATE model_suggestions
SET generated_at = CASE
        WHEN generated_at GLOB
             '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]'
        THEN substr(generated_at, 7, 4) || '-' ||
             substr(generated_at, 1, 2) || '-' ||
             substr(generated_at, 4, 2) || 'T' ||
             substr(generated_at, 12, 2) || ':' ||
             substr(generated_at, 15, 2) || ':' ||
             substr(generated_at, 18, 2) || 'Z'
        ELSE generated_at
    END,
    created_at = CASE
        WHEN created_at GLOB
             '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]'
        THEN substr(created_at, 7, 4) || '-' ||
             substr(created_at, 1, 2) || '-' ||
             substr(created_at, 4, 2) || 'T' ||
             substr(created_at, 12, 2) || ':' ||
             substr(created_at, 15, 2) || ':' ||
             substr(created_at, 18, 2) || 'Z'
        ELSE created_at
    END
WHERE generated_at GLOB
      '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]'
   OR created_at GLOB
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

UPDATE semantic_map_assignments
SET created_at = substr(created_at, 7, 4) || '-' ||
                 substr(created_at, 1, 2) || '-' ||
                 substr(created_at, 4, 2) || 'T' ||
                 substr(created_at, 12, 2) || ':' ||
                 substr(created_at, 15, 2) || ':' ||
                 substr(created_at, 18, 2) || 'Z'
WHERE created_at GLOB
      '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]';

UPDATE semantic_tags
SET created_at = substr(created_at, 7, 4) || '-' ||
                 substr(created_at, 1, 2) || '-' ||
                 substr(created_at, 4, 2) || 'T' ||
                 substr(created_at, 12, 2) || ':' ||
                 substr(created_at, 15, 2) || ':' ||
                 substr(created_at, 18, 2) || 'Z'
WHERE created_at GLOB
      '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]';

UPDATE semantic_tag_assignments
SET created_at = substr(created_at, 7, 4) || '-' ||
                 substr(created_at, 1, 2) || '-' ||
                 substr(created_at, 4, 2) || 'T' ||
                 substr(created_at, 12, 2) || ':' ||
                 substr(created_at, 15, 2) || ':' ||
                 substr(created_at, 18, 2) || 'Z'
WHERE created_at GLOB
      '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]';

UPDATE dense_expressions
SET created_at = substr(created_at, 7, 4) || '-' ||
                 substr(created_at, 1, 2) || '-' ||
                 substr(created_at, 4, 2) || 'T' ||
                 substr(created_at, 12, 2) || ':' ||
                 substr(created_at, 15, 2) || ':' ||
                 substr(created_at, 18, 2) || 'Z'
WHERE created_at GLOB
      '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]';

DROP TRIGGER IF EXISTS relevance_scores_immutable_update;

UPDATE relevance_scores
SET created_at = substr(created_at, 7, 4) || '-' ||
                 substr(created_at, 1, 2) || '-' ||
                 substr(created_at, 4, 2) || 'T' ||
                 substr(created_at, 12, 2) || ':' ||
                 substr(created_at, 15, 2) || ':' ||
                 substr(created_at, 18, 2) || 'Z'
WHERE created_at GLOB
      '[0-9][0-9]/[0-9][0-9]/[0-9][0-9][0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]';

CREATE TRIGGER relevance_scores_immutable_update
BEFORE UPDATE ON relevance_scores
BEGIN
    SELECT RAISE(ABORT, 'relevance scores are immutable');
END;

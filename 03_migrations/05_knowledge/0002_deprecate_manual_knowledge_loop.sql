-- Issue #63 corrects the manual Knowledge v1/v2 product model from migration 0001.
-- Preserve every potential row, but remove the old table names so older application
-- code fails closed instead of continuing to write the superseded model.

ALTER TABLE knowledge_records
RENAME TO deprecated_manual_knowledge_records;

ALTER TABLE knowledge_versions
RENAME TO deprecated_manual_knowledge_versions;

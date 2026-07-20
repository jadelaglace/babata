-- P6 preflight: keep every optional revision endpoint bound to its declared item.
-- Existing rows are audited by the Rust runner before these triggers are installed.

CREATE TRIGGER revisions_parent_item_insert
BEFORE INSERT ON revisions
WHEN NEW.parent_revision_id IS NOT NULL
  AND EXISTS (
      SELECT 1 FROM revisions parent
      WHERE parent.revision_id = NEW.parent_revision_id
        AND (parent.item_id <> NEW.item_id OR parent.ordinal >= NEW.ordinal)
  )
BEGIN
    SELECT RAISE(ABORT, 'parent revision must be an earlier revision of the same item');
END;

CREATE TRIGGER revisions_parent_item_update
BEFORE UPDATE OF item_id, parent_revision_id, ordinal ON revisions
WHEN NEW.parent_revision_id IS NOT NULL
  AND EXISTS (
      SELECT 1 FROM revisions parent
      WHERE parent.revision_id = NEW.parent_revision_id
        AND (parent.item_id <> NEW.item_id OR parent.ordinal >= NEW.ordinal)
  )
BEGIN
    SELECT RAISE(ABORT, 'parent revision must be an earlier revision of the same item');
END;

CREATE TRIGGER capture_operations_revision_item_insert
BEFORE INSERT ON capture_operations
WHEN EXISTS (
    SELECT 1 FROM revisions revision
    WHERE revision.revision_id = NEW.revision_id
      AND revision.item_id <> NEW.item_id
)
BEGIN
    SELECT RAISE(ABORT, 'capture operation revision does not belong to item');
END;

CREATE TRIGGER capture_operations_revision_item_update
BEFORE UPDATE OF item_id, revision_id ON capture_operations
WHEN EXISTS (
    SELECT 1 FROM revisions revision
    WHERE revision.revision_id = NEW.revision_id
      AND revision.item_id <> NEW.item_id
)
BEGIN
    SELECT RAISE(ABORT, 'capture operation revision does not belong to item');
END;

CREATE TRIGGER relations_revision_items_insert
BEFORE INSERT ON relations
WHEN (NEW.from_revision_id IS NOT NULL AND EXISTS (
          SELECT 1 FROM revisions revision
          WHERE revision.revision_id = NEW.from_revision_id
            AND revision.item_id <> NEW.from_item_id
      ))
   OR (NEW.to_revision_id IS NOT NULL AND EXISTS (
          SELECT 1 FROM revisions revision
          WHERE revision.revision_id = NEW.to_revision_id
            AND revision.item_id <> NEW.to_item_id
      ))
BEGIN
    SELECT RAISE(ABORT, 'relation revision endpoint does not belong to item');
END;

CREATE TRIGGER relations_revision_items_update
BEFORE UPDATE OF from_item_id, from_revision_id, to_item_id, to_revision_id ON relations
WHEN (NEW.from_revision_id IS NOT NULL AND EXISTS (
          SELECT 1 FROM revisions revision
          WHERE revision.revision_id = NEW.from_revision_id
            AND revision.item_id <> NEW.from_item_id
      ))
   OR (NEW.to_revision_id IS NOT NULL AND EXISTS (
          SELECT 1 FROM revisions revision
          WHERE revision.revision_id = NEW.to_revision_id
            AND revision.item_id <> NEW.to_item_id
      ))
BEGIN
    SELECT RAISE(ABORT, 'relation revision endpoint does not belong to item');
END;

-- Collection rows share the raw database and must not pair an item with a
-- revision belonging to another item.

CREATE TRIGGER route_evidence_revision_item_insert
BEFORE INSERT ON route_evidence
WHEN EXISTS (
    SELECT 1 FROM revisions revision
    WHERE revision.revision_id = NEW.revision_id
      AND revision.item_id <> NEW.item_id
)
BEGIN
    SELECT RAISE(ABORT, 'route evidence revision does not belong to item');
END;

CREATE TRIGGER route_evidence_revision_item_update
BEFORE UPDATE OF item_id, revision_id ON route_evidence
WHEN EXISTS (
    SELECT 1 FROM revisions revision
    WHERE revision.revision_id = NEW.revision_id
      AND revision.item_id <> NEW.item_id
)
BEGIN
    SELECT RAISE(ABORT, 'route evidence revision does not belong to item');
END;

CREATE TRIGGER collection_items_revision_item_insert
BEFORE INSERT ON collection_items
WHEN NEW.item_id IS NOT NULL
  AND NEW.revision_id IS NOT NULL
  AND EXISTS (
      SELECT 1 FROM revisions revision
      WHERE revision.revision_id = NEW.revision_id
        AND revision.item_id <> NEW.item_id
  )
BEGIN
    SELECT RAISE(ABORT, 'collection item revision does not belong to item');
END;

CREATE TRIGGER collection_items_revision_item_update
BEFORE UPDATE OF item_id, revision_id ON collection_items
WHEN NEW.item_id IS NOT NULL
  AND NEW.revision_id IS NOT NULL
  AND EXISTS (
      SELECT 1 FROM revisions revision
      WHERE revision.revision_id = NEW.revision_id
        AND revision.item_id <> NEW.item_id
  )
BEGIN
    SELECT RAISE(ABORT, 'collection item revision does not belong to item');
END;

CREATE TRIGGER recollection_revision_items_insert
BEFORE INSERT ON collection_recollection_checks
WHEN EXISTS (
        SELECT 1 FROM revisions revision
        WHERE revision.revision_id = NEW.previous_revision_id
          AND revision.item_id <> NEW.item_id
     )
  OR (NEW.new_revision_id IS NOT NULL AND EXISTS (
        SELECT 1 FROM revisions revision
        WHERE revision.revision_id = NEW.new_revision_id
          AND revision.item_id <> NEW.item_id
     ))
BEGIN
    SELECT RAISE(ABORT, 'recollection revision does not belong to item');
END;

CREATE TRIGGER recollection_revision_items_update
BEFORE UPDATE OF item_id, previous_revision_id, new_revision_id ON collection_recollection_checks
WHEN EXISTS (
        SELECT 1 FROM revisions revision
        WHERE revision.revision_id = NEW.previous_revision_id
          AND revision.item_id <> NEW.item_id
     )
  OR (NEW.new_revision_id IS NOT NULL AND EXISTS (
        SELECT 1 FROM revisions revision
        WHERE revision.revision_id = NEW.new_revision_id
          AND revision.item_id <> NEW.item_id
     ))
BEGIN
    SELECT RAISE(ABORT, 'recollection revision does not belong to item');
END;

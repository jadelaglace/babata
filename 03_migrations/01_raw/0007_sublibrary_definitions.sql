CREATE UNIQUE INDEX sublibrary_definition_version_identity
ON revisions (
    json_extract(raw_text, '$.id'),
    CAST(json_extract(raw_text, '$.version') AS INTEGER)
)
WHERE CASE WHEN json_valid(raw_text)
    THEN json_extract(raw_text, '$.schema_version') = 'babata.sublibrary/v1'
    ELSE 0
END;

CREATE TRIGGER sublibrary_definition_insert_guard
BEFORE INSERT ON revisions
WHEN CASE WHEN json_valid(NEW.raw_text)
    THEN json_extract(NEW.raw_text, '$.schema_version') = 'babata.sublibrary/v1'
    ELSE 0
END
BEGIN
    SELECT CASE WHEN
        json_type(NEW.raw_text, '$.id') <> 'text'
        OR json_extract(NEW.raw_text, '$.id') NOT GLOB 'sublibrary_??????????????????????????'
        OR json_type(NEW.raw_text, '$.version') <> 'integer'
        OR CAST(json_extract(NEW.raw_text, '$.version') AS INTEGER) < 1
        OR NEW.ordinal <> CAST(json_extract(NEW.raw_text, '$.version') AS INTEGER)
        OR trim(COALESCE(json_extract(NEW.raw_text, '$.title'), '')) = ''
        OR trim(COALESCE(json_extract(NEW.raw_text, '$.purpose'), '')) = ''
        OR trim(COALESCE(json_extract(NEW.raw_text, '$.author'), '')) = ''
        OR json_type(NEW.raw_text, '$.selection') <> 'object'
        OR json_type(NEW.raw_text, '$.manual_include') <> 'array'
        OR json_type(NEW.raw_text, '$.manual_exclude') <> 'array'
        OR json_type(NEW.raw_text, '$.organisation_rules') <> 'array'
        OR json_type(NEW.raw_text, '$.include_unreviewed') NOT IN ('true', 'false')
    THEN RAISE(ABORT, 'invalid sublibrary definition document') END;

    SELECT CASE WHEN NOT EXISTS (
        SELECT 1
        FROM items item
        JOIN sources source ON source.source_id = item.source_id
        WHERE item.item_id = NEW.item_id
          AND source.source_kind = 'first_party'
          AND source.provider = 'babata'
    ) THEN RAISE(ABORT, 'sublibrary definition must be first-party C0') END;

    SELECT CASE WHEN
        (NEW.ordinal = 1 AND (NEW.parent_revision_id IS NOT NULL OR NEW.revision_kind <> 'authored'))
        OR (NEW.ordinal > 1 AND (NEW.parent_revision_id IS NULL OR NEW.revision_kind <> 'edit'))
    THEN RAISE(ABORT, 'sublibrary definition revision lineage is invalid') END;

    SELECT CASE WHEN NEW.ordinal > 1 AND NOT EXISTS (
        SELECT 1
        FROM revisions parent
        WHERE parent.revision_id = NEW.parent_revision_id
          AND parent.item_id = NEW.item_id
          AND parent.state = 'ready'
          AND json_extract(parent.raw_text, '$.schema_version') = 'babata.sublibrary/v1'
          AND json_extract(parent.raw_text, '$.id') = json_extract(NEW.raw_text, '$.id')
          AND CAST(json_extract(parent.raw_text, '$.version') AS INTEGER) = NEW.ordinal - 1
    ) THEN RAISE(ABORT, 'sublibrary definition parent must be the prior ready version') END;
END;

CREATE TRIGGER sublibrary_definition_content_immutable
BEFORE UPDATE ON revisions
WHEN CASE WHEN json_valid(OLD.raw_text)
    THEN json_extract(OLD.raw_text, '$.schema_version') = 'babata.sublibrary/v1'
    ELSE 0
END
 AND (
    NEW.revision_id IS NOT OLD.revision_id
    OR NEW.item_id IS NOT OLD.item_id
    OR NEW.parent_revision_id IS NOT OLD.parent_revision_id
    OR NEW.revision_kind IS NOT OLD.revision_kind
    OR NEW.ordinal IS NOT OLD.ordinal
    OR NEW.captured_at IS NOT OLD.captured_at
    OR NEW.authored_at IS NOT OLD.authored_at
    OR NEW.revision_note IS NOT OLD.revision_note
    OR NEW.raw_text IS NOT OLD.raw_text
    OR NEW.text_sha256 IS NOT OLD.text_sha256
    OR NEW.metadata_json IS NOT OLD.metadata_json
 )
BEGIN
    SELECT RAISE(ABORT, 'sublibrary definition history is immutable');
END;

CREATE TRIGGER sublibrary_definition_delete_guard
BEFORE DELETE ON revisions
WHEN CASE WHEN json_valid(OLD.raw_text)
    THEN json_extract(OLD.raw_text, '$.schema_version') = 'babata.sublibrary/v1'
    ELSE 0
END
BEGIN
    SELECT RAISE(ABORT, 'sublibrary definition history is append-only');
END;

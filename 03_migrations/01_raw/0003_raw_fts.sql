CREATE VIRTUAL TABLE revision_text_fts USING fts5(
    revision_id UNINDEXED,
    item_id UNINDEXED,
    raw_text,
    tokenize = 'unicode61'
);

CREATE TRIGGER revisions_ai_fts AFTER INSERT ON revisions
WHEN NEW.state = 'ready' AND NEW.raw_text IS NOT NULL
BEGIN
    INSERT INTO revision_text_fts (revision_id, item_id, raw_text)
    VALUES (NEW.revision_id, NEW.item_id, NEW.raw_text);
END;

CREATE TRIGGER revisions_au_fts AFTER UPDATE OF state, raw_text, item_id ON revisions
BEGIN
    DELETE FROM revision_text_fts WHERE revision_id = OLD.revision_id;
    INSERT INTO revision_text_fts (revision_id, item_id, raw_text)
    SELECT NEW.revision_id, NEW.item_id, NEW.raw_text
    WHERE NEW.state = 'ready' AND NEW.raw_text IS NOT NULL;
END;

CREATE TRIGGER revisions_ad_fts AFTER DELETE ON revisions
BEGIN
    DELETE FROM revision_text_fts WHERE revision_id = OLD.revision_id;
END;

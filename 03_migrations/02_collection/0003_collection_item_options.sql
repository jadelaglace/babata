ALTER TABLE collection_items
ADD COLUMN requested_attachments INTEGER NOT NULL DEFAULT 0
CHECK (requested_attachments IN (0, 1));

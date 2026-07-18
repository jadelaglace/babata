CREATE TABLE route_evidence (
    evidence_id TEXT PRIMARY KEY,
    route_id TEXT NOT NULL,
    authorization_id TEXT NOT NULL,
    source_reference TEXT NOT NULL,
    item_id TEXT NOT NULL REFERENCES items(item_id),
    revision_id TEXT NOT NULL REFERENCES revisions(revision_id),
    metadata_covered INTEGER NOT NULL CHECK (metadata_covered IN (0, 1)),
    attachments_covered INTEGER NOT NULL CHECK (attachments_covered IN (0, 1)),
    revisions_covered INTEGER NOT NULL CHECK (revisions_covered IN (0, 1)),
    limitations_json TEXT NOT NULL,
    reimported INTEGER NOT NULL CHECK (reimported IN (0, 1)),
    recorded_at TEXT NOT NULL
);

CREATE INDEX ix_route_evidence_route_recorded
    ON route_evidence(route_id, recorded_at DESC);

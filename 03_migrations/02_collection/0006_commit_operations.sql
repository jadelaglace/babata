-- Durable replay identity, not a disposable runtime cache.
CREATE TABLE collection_commit_operations (
    request_key TEXT PRIMARY KEY CHECK (length(request_key) BETWEEN 1 AND 128),
    input_sha256 TEXT NOT NULL CHECK (length(input_sha256) = 64),
    session_id TEXT NOT NULL,
    candidate_id TEXT NOT NULL,
    operation_json TEXT NOT NULL CHECK (json_valid(operation_json)),
    UNIQUE (session_id, candidate_id),
    FOREIGN KEY (session_id, candidate_id)
        REFERENCES collection_candidates(session_id, candidate_id)
);

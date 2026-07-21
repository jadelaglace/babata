-- P6.1 append-only history for map evolution and semantic assignments.
-- The four foundations remain immutable; disciplines and branches may evolve.

ALTER TABLE knowledge_map_nodes
ADD COLUMN lifecycle_state TEXT NOT NULL DEFAULT 'active'
CHECK (lifecycle_state IN ('active', 'inactive', 'merged'));

CREATE TRIGGER knowledge_map_foundation_insert_guard
BEFORE INSERT ON knowledge_map_nodes
WHEN NEW.node_level = 'foundation'
 AND NEW.map_version_id = 'map_version_p6_baseline'
BEGIN
    SELECT RAISE(ABORT, 'P6 foundation nodes are fixed');
END;

CREATE TRIGGER knowledge_map_foundation_update_guard
BEFORE UPDATE ON knowledge_map_nodes
WHEN (OLD.node_level = 'foundation' OR NEW.node_level = 'foundation')
 AND OLD.map_version_id = 'map_version_p6_baseline'
BEGIN
    SELECT RAISE(ABORT, 'P6 foundation nodes are immutable');
END;

CREATE TRIGGER knowledge_map_foundation_delete_guard
BEFORE DELETE ON knowledge_map_nodes
WHEN OLD.node_level = 'foundation'
 AND OLD.map_version_id = 'map_version_p6_baseline'
BEGIN
    SELECT RAISE(ABORT, 'P6 foundation nodes are immutable');
END;

CREATE TABLE knowledge_map_node_events (
    map_event_id TEXT PRIMARY KEY,
    map_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    event_kind TEXT NOT NULL CHECK (event_kind IN (
        'created', 'renamed', 'deactivated', 'merged'
    )),
    previous_name TEXT,
    current_name TEXT,
    merged_into_map_node_id TEXT REFERENCES knowledge_map_nodes(map_node_id),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN (
        'system', 'machine', 'first_party'
    )),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL,
    CHECK (merged_into_map_node_id IS NULL OR merged_into_map_node_id <> map_node_id),
    CHECK (
        (event_kind = 'created' AND previous_name IS NULL AND current_name IS NOT NULL
         AND merged_into_map_node_id IS NULL)
        OR
        (event_kind = 'renamed' AND previous_name IS NOT NULL AND current_name IS NOT NULL
         AND merged_into_map_node_id IS NULL)
        OR
        (event_kind = 'deactivated' AND previous_name IS NOT NULL AND current_name IS NOT NULL
         AND merged_into_map_node_id IS NULL)
        OR
        (event_kind = 'merged' AND previous_name IS NOT NULL AND current_name IS NOT NULL
         AND merged_into_map_node_id IS NOT NULL)
    )
);

CREATE TABLE knowledge_map_edge_events (
    map_edge_event_id TEXT PRIMARY KEY,
    parent_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    child_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    event_kind TEXT NOT NULL CHECK (event_kind IN ('assigned', 'unassigned')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN (
        'system', 'machine', 'first_party'
    )),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL,
    CHECK (parent_node_id <> child_node_id)
);

CREATE TABLE semantic_map_assignment_events (
    semantic_map_event_id TEXT PRIMARY KEY,
    semantic_id TEXT NOT NULL REFERENCES semantic_entries(semantic_id),
    map_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    event_kind TEXT NOT NULL CHECK (event_kind IN ('assigned', 'unassigned')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN (
        'machine', 'first_party'
    )),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL
);

CREATE TABLE map_node_tag_assignments (
    map_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    tag_id TEXT NOT NULL REFERENCES semantic_tags(tag_id),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN (
        'machine', 'first_party'
    )),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL,
    PRIMARY KEY (map_node_id, tag_id, provenance_kind)
);

CREATE TABLE map_node_tag_events (
    map_tag_event_id TEXT PRIMARY KEY,
    map_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    tag_id TEXT NOT NULL REFERENCES semantic_tags(tag_id),
    event_kind TEXT NOT NULL CHECK (event_kind IN ('assigned', 'unassigned')),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN (
        'machine', 'first_party'
    )),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL
);

CREATE INDEX knowledge_map_node_events_node_idx
ON knowledge_map_node_events(map_node_id, created_at, map_event_id);
CREATE INDEX knowledge_map_edge_events_child_idx
ON knowledge_map_edge_events(child_node_id, created_at, map_edge_event_id);
CREATE INDEX semantic_map_assignment_events_semantic_idx
ON semantic_map_assignment_events(semantic_id, created_at, semantic_map_event_id);
CREATE INDEX map_node_tag_assignments_node_idx
ON map_node_tag_assignments(map_node_id, tag_id);
CREATE INDEX map_node_tag_events_node_idx
ON map_node_tag_events(map_node_id, created_at, map_tag_event_id);

INSERT INTO knowledge_map_node_events
    (map_event_id, map_node_id, event_kind, previous_name, current_name,
     merged_into_map_node_id, rationale, provenance_kind, author, suggestion_id, created_at)
SELECT 'map_event_migrated_' || map_node_id, map_node_id, 'created', NULL, name, NULL,
       'Backfilled current map node when enabling P6.1 evolution history',
       provenance_kind, 'babata-migration', suggestion_id, created_at
FROM knowledge_map_nodes;

INSERT INTO knowledge_map_edge_events
    (map_edge_event_id, parent_node_id, child_node_id, event_kind, rationale,
     provenance_kind, author, suggestion_id, created_at)
SELECT 'map_edge_event_migrated_' || parent_node_id || '_' || child_node_id,
       parent_node_id, child_node_id, 'assigned',
       'Backfilled current map parent when enabling P6.1 evolution history',
       provenance_kind, 'babata-migration', suggestion_id, created_at
FROM knowledge_map_edges;

INSERT INTO semantic_map_assignment_events
    (semantic_map_event_id, semantic_id, map_node_id, event_kind, rationale,
     provenance_kind, author, suggestion_id, created_at)
SELECT 'semantic_map_event_migrated_' || semantic_id || '_' || map_node_id || '_' || provenance_kind,
       semantic_id, map_node_id, 'assigned',
       'Backfilled current semantic assignment when enabling P6.1 evolution history',
       provenance_kind, 'babata-migration', suggestion_id, created_at
FROM semantic_map_assignments;

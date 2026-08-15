-- Successor course registration contract. Existing semantic entries, map nodes,
-- assignments, sublibrary definitions, packages and acceptance receipts remain unchanged.

CREATE TABLE courses (
    course_id TEXT PRIMARY KEY,
    course_key TEXT NOT NULL,
    course_version INTEGER NOT NULL CHECK (course_version >= 1),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    source TEXT NOT NULL CHECK (length(trim(source)) > 0),
    term TEXT NOT NULL CHECK (length(trim(term)) > 0),
    acceptance_state TEXT NOT NULL CHECK (acceptance_state IN (
        'pending_user_acceptance', 'accepted'
    )),
    closure_state TEXT NOT NULL CHECK (closure_state IN ('open', 'closed')),
    definition_sha256 TEXT NOT NULL CHECK (length(definition_sha256) = 64),
    definition_json TEXT NOT NULL CHECK (json_valid(definition_json)),
    author_kind TEXT NOT NULL CHECK (author_kind IN ('system', 'machine', 'first_party')),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    created_at TEXT NOT NULL,
    UNIQUE (course_key, course_version)
);

CREATE TABLE course_branch_covers (
    course_id TEXT NOT NULL REFERENCES courses(course_id),
    branch_map_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    relation_kind TEXT NOT NULL CHECK (relation_kind = 'covers'),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    created_at TEXT NOT NULL,
    PRIMARY KEY (course_id, branch_map_node_id)
);

CREATE TABLE course_semantic_modules (
    course_id TEXT NOT NULL REFERENCES courses(course_id),
    module_id TEXT NOT NULL CHECK (length(trim(module_id)) > 0),
    semantic_id TEXT NOT NULL REFERENCES semantic_entries(semantic_id),
    chapter_id TEXT NOT NULL CHECK (length(trim(chapter_id)) > 0),
    created_at TEXT NOT NULL,
    PRIMARY KEY (course_id, module_id),
    UNIQUE (course_id, semantic_id)
);

CREATE TABLE course_semantic_map_assignments (
    course_id TEXT NOT NULL REFERENCES courses(course_id),
    semantic_id TEXT NOT NULL REFERENCES semantic_entries(semantic_id),
    map_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    assignment_role TEXT NOT NULL CHECK (assignment_role IN (
        'primary', 'secondary', 'contextual'
    )),
    strength INTEGER NOT NULL CHECK (strength BETWEEN 0 AND 100),
    confidence INTEGER NOT NULL CHECK (confidence BETWEEN 0 AND 100),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    method_version TEXT NOT NULL CHECK (length(trim(method_version)) > 0),
    created_at TEXT NOT NULL,
    PRIMARY KEY (course_id, semantic_id, map_node_id),
    FOREIGN KEY (course_id, semantic_id)
        REFERENCES course_semantic_modules(course_id, semantic_id)
);

CREATE TABLE course_map_node_relations (
    map_relation_id TEXT PRIMARY KEY,
    course_id TEXT NOT NULL REFERENCES courses(course_id),
    from_map_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    relation_kind TEXT NOT NULL CHECK (relation_kind IN (
        'intersects_with', 'draws_from', 'applies_to', 'prerequisite_of'
    )),
    to_map_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    author_kind TEXT NOT NULL CHECK (author_kind IN ('system', 'machine', 'first_party')),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    created_at TEXT NOT NULL,
    CHECK (from_map_node_id <> to_map_node_id),
    UNIQUE (course_id, from_map_node_id, relation_kind, to_map_node_id)
);

CREATE TABLE course_lens_memberships (
    course_id TEXT NOT NULL REFERENCES courses(course_id),
    sublibrary_id TEXT NOT NULL,
    definition_version INTEGER NOT NULL CHECK (definition_version >= 1),
    created_at TEXT NOT NULL,
    PRIMARY KEY (course_id, sublibrary_id, definition_version)
);

CREATE INDEX course_branch_covers_branch_idx
ON course_branch_covers(branch_map_node_id, course_id);
CREATE INDEX course_semantic_modules_semantic_idx
ON course_semantic_modules(semantic_id, course_id);
CREATE INDEX course_semantic_map_assignments_map_idx
ON course_semantic_map_assignments(map_node_id, course_id, semantic_id);
CREATE INDEX course_map_node_relations_from_idx
ON course_map_node_relations(from_map_node_id, relation_kind, to_map_node_id);
CREATE INDEX course_map_node_relations_to_idx
ON course_map_node_relations(to_map_node_id, relation_kind, from_map_node_id);
CREATE INDEX course_lens_memberships_lens_idx
ON course_lens_memberships(sublibrary_id, definition_version, course_id);

CREATE TRIGGER courses_immutable_update BEFORE UPDATE ON courses
BEGIN SELECT RAISE(ABORT, 'course registrations are immutable'); END;
CREATE TRIGGER courses_append_only_delete BEFORE DELETE ON courses
BEGIN SELECT RAISE(ABORT, 'course registrations are append-only'); END;
CREATE TRIGGER course_branch_covers_immutable_update BEFORE UPDATE ON course_branch_covers
BEGIN SELECT RAISE(ABORT, 'course branch covers are immutable'); END;
CREATE TRIGGER course_branch_covers_append_only_delete BEFORE DELETE ON course_branch_covers
BEGIN SELECT RAISE(ABORT, 'course branch covers are append-only'); END;
CREATE TRIGGER course_semantic_modules_immutable_update BEFORE UPDATE ON course_semantic_modules
BEGIN SELECT RAISE(ABORT, 'course semantic memberships are immutable'); END;
CREATE TRIGGER course_semantic_modules_append_only_delete BEFORE DELETE ON course_semantic_modules
BEGIN SELECT RAISE(ABORT, 'course semantic memberships are append-only'); END;
CREATE TRIGGER course_semantic_map_assignments_immutable_update BEFORE UPDATE ON course_semantic_map_assignments
BEGIN SELECT RAISE(ABORT, 'course map assessments are immutable'); END;
CREATE TRIGGER course_semantic_map_assignments_append_only_delete BEFORE DELETE ON course_semantic_map_assignments
BEGIN SELECT RAISE(ABORT, 'course map assessments are append-only'); END;
CREATE TRIGGER course_map_node_relations_immutable_update BEFORE UPDATE ON course_map_node_relations
BEGIN SELECT RAISE(ABORT, 'typed map relations are immutable'); END;
CREATE TRIGGER course_map_node_relations_append_only_delete BEFORE DELETE ON course_map_node_relations
BEGIN SELECT RAISE(ABORT, 'typed map relations are append-only'); END;
CREATE TRIGGER course_lens_memberships_immutable_update BEFORE UPDATE ON course_lens_memberships
BEGIN SELECT RAISE(ABORT, 'course lens memberships are immutable'); END;
CREATE TRIGGER course_lens_memberships_append_only_delete BEFORE DELETE ON course_lens_memberships
BEGIN SELECT RAISE(ABORT, 'course lens memberships are append-only'); END;

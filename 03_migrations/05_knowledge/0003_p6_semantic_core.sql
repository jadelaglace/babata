-- P6.1 semantic core. Machine-produced normalized records remain C1 and
-- retain their source derivative; first-party records retain their C0 identity.

CREATE TABLE worldview_map_versions (
    map_version_id TEXT PRIMARY KEY,
    ordinal INTEGER NOT NULL UNIQUE CHECK (ordinal >= 1),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    author_kind TEXT NOT NULL CHECK (author_kind IN ('system', 'machine', 'first_party')),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    created_at TEXT NOT NULL
);

CREATE TABLE knowledge_map_nodes (
    map_node_id TEXT PRIMARY KEY,
    map_version_id TEXT NOT NULL REFERENCES worldview_map_versions(map_version_id),
    node_level TEXT NOT NULL CHECK (node_level IN ('foundation', 'discipline', 'branch')),
    canonical_key TEXT NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN ('system', 'machine', 'first_party')),
    suggestion_id TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (map_version_id, canonical_key),
    UNIQUE (map_version_id, node_level, name)
);

CREATE TABLE knowledge_map_edges (
    map_version_id TEXT NOT NULL REFERENCES worldview_map_versions(map_version_id),
    parent_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    child_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN ('system', 'machine', 'first_party')),
    suggestion_id TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (map_version_id, parent_node_id, child_node_id),
    CHECK (parent_node_id <> child_node_id)
);

CREATE TABLE score_profiles (
    profile_id TEXT PRIMARY KEY,
    ordinal INTEGER NOT NULL UNIQUE CHECK (ordinal >= 1),
    interest_weight INTEGER NOT NULL CHECK (interest_weight BETWEEN 0 AND 100),
    strategy_weight INTEGER NOT NULL CHECK (strategy_weight BETWEEN 0 AND 100),
    consensus_weight INTEGER NOT NULL CHECK (consensus_weight BETWEEN 0 AND 100),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    author_kind TEXT NOT NULL CHECK (author_kind IN ('system', 'machine', 'first_party')),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    created_at TEXT NOT NULL,
    CHECK (interest_weight + strategy_weight + consensus_weight = 100)
);

CREATE TABLE model_suggestions (
    suggestion_id TEXT PRIMARY KEY,
    suggestion_kind TEXT NOT NULL CHECK (suggestion_kind = 'semantic_package'),
    source_item_id TEXT NOT NULL REFERENCES items(item_id),
    source_revision_id TEXT NOT NULL REFERENCES revisions(revision_id),
    source_derivative_id TEXT NOT NULL,
    source_output_sha256 TEXT NOT NULL CHECK (length(source_output_sha256) = 64),
    provider TEXT NOT NULL CHECK (length(trim(provider)) > 0),
    model TEXT NOT NULL CHECK (length(trim(model)) > 0),
    model_version TEXT NOT NULL CHECK (length(trim(model_version)) > 0),
    prompt_version TEXT NOT NULL CHECK (length(trim(prompt_version)) > 0),
    generated_at TEXT NOT NULL,
    evidence_derivatives_json TEXT NOT NULL CHECK (json_valid(evidence_derivatives_json)),
    limitations_json TEXT NOT NULL CHECK (json_valid(limitations_json)),
    created_at TEXT NOT NULL,
    UNIQUE (source_derivative_id, source_output_sha256)
);

CREATE TABLE semantic_entries (
    semantic_id TEXT PRIMARY KEY,
    semantic_kind TEXT NOT NULL CHECK (semantic_kind IN (
        'map_direction', 'knowledge', 'case', 'log', 'insight'
    )),
    realm TEXT NOT NULL CHECK (realm IN (
        'knowledge_map', 'knowledge_and_cases', 'cognitive_trail'
    )),
    origin_kind TEXT NOT NULL CHECK (origin_kind IN ('machine', 'first_party')),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    source_item_id TEXT REFERENCES items(item_id),
    source_revision_id TEXT REFERENCES revisions(revision_id),
    first_party_item_id TEXT REFERENCES items(item_id),
    first_party_revision_id TEXT REFERENCES revisions(revision_id),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL,
    CHECK (
        (origin_kind = 'machine' AND suggestion_id IS NOT NULL
         AND source_item_id IS NOT NULL AND source_revision_id IS NOT NULL
         AND first_party_item_id IS NULL AND first_party_revision_id IS NULL)
        OR
        (origin_kind = 'first_party' AND suggestion_id IS NULL
         AND first_party_item_id IS NOT NULL AND first_party_revision_id IS NOT NULL)
    )
);

CREATE TABLE semantic_map_assignments (
    semantic_id TEXT NOT NULL REFERENCES semantic_entries(semantic_id),
    map_node_id TEXT NOT NULL REFERENCES knowledge_map_nodes(map_node_id),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN ('machine', 'first_party')),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL,
    PRIMARY KEY (semantic_id, map_node_id, provenance_kind)
);

CREATE TABLE semantic_tags (
    tag_id TEXT PRIMARY KEY,
    canonical_name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    created_at TEXT NOT NULL
);

CREATE TABLE semantic_tag_assignments (
    semantic_id TEXT NOT NULL REFERENCES semantic_entries(semantic_id),
    tag_id TEXT NOT NULL REFERENCES semantic_tags(tag_id),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN ('machine', 'first_party')),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL,
    PRIMARY KEY (semantic_id, tag_id, provenance_kind)
);

CREATE TABLE semantic_relations (
    semantic_relation_id TEXT PRIMARY KEY,
    from_semantic_id TEXT NOT NULL REFERENCES semantic_entries(semantic_id),
    relation_kind TEXT NOT NULL CHECK (length(trim(relation_kind)) > 0),
    to_semantic_id TEXT NOT NULL REFERENCES semantic_entries(semantic_id),
    evidence TEXT NOT NULL CHECK (length(trim(evidence)) > 0),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN ('machine', 'first_party')),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL,
    CHECK (from_semantic_id <> to_semantic_id)
);

CREATE TABLE dense_expressions (
    expression_id TEXT PRIMARY KEY,
    semantic_id TEXT NOT NULL REFERENCES semantic_entries(semantic_id),
    expression_kind TEXT NOT NULL CHECK (expression_kind IN (
        'mind_map', 'mermaid', 'model', 'formula', 'checklist', 'process', 'outline'
    )),
    content_text TEXT NOT NULL CHECK (length(trim(content_text)) > 0),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN ('machine', 'first_party')),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL
);

CREATE TABLE relevance_scores (
    score_id TEXT PRIMARY KEY,
    target_kind TEXT NOT NULL CHECK (target_kind IN ('map_node', 'semantic')),
    target_id TEXT NOT NULL,
    profile_id TEXT NOT NULL REFERENCES score_profiles(profile_id),
    interest INTEGER NOT NULL CHECK (interest BETWEEN 0 AND 100),
    strategy INTEGER NOT NULL CHECK (strategy BETWEEN 0 AND 100),
    consensus INTEGER NOT NULL CHECK (consensus BETWEEN 0 AND 100),
    weighted_score INTEGER NOT NULL CHECK (weighted_score BETWEEN 0 AND 10000),
    rationale TEXT NOT NULL CHECK (length(trim(rationale)) > 0),
    provenance_kind TEXT NOT NULL CHECK (provenance_kind IN ('machine', 'first_party')),
    author TEXT NOT NULL CHECK (length(trim(author)) > 0),
    suggestion_id TEXT REFERENCES model_suggestions(suggestion_id),
    created_at TEXT NOT NULL
);

CREATE TABLE suggestion_reviews (
    review_id TEXT PRIMARY KEY,
    suggestion_id TEXT NOT NULL REFERENCES model_suggestions(suggestion_id),
    decision TEXT NOT NULL CHECK (decision IN ('accepted', 'modified', 'rejected')),
    reason TEXT,
    first_party_item_id TEXT REFERENCES items(item_id),
    first_party_revision_id TEXT REFERENCES revisions(revision_id),
    reviewer TEXT NOT NULL CHECK (length(trim(reviewer)) > 0),
    created_at TEXT NOT NULL,
    CHECK (
        decision <> 'modified'
        OR (first_party_item_id IS NOT NULL AND first_party_revision_id IS NOT NULL)
    )
);

CREATE INDEX semantic_entries_source_idx
ON semantic_entries(source_item_id, source_revision_id, created_at);
CREATE UNIQUE INDEX semantic_entries_first_party_revision_idx
ON semantic_entries(first_party_revision_id)
WHERE first_party_revision_id IS NOT NULL;
CREATE INDEX semantic_assignments_map_idx
ON semantic_map_assignments(map_node_id, semantic_id);
CREATE INDEX semantic_relations_from_idx
ON semantic_relations(from_semantic_id, relation_kind);
CREATE INDEX semantic_relations_to_idx
ON semantic_relations(to_semantic_id, relation_kind);
CREATE INDEX relevance_scores_target_idx
ON relevance_scores(target_kind, target_id, created_at);
CREATE INDEX suggestion_reviews_suggestion_idx
ON suggestion_reviews(suggestion_id, created_at);

INSERT INTO worldview_map_versions
    (map_version_id, ordinal, rationale, author_kind, author, created_at)
VALUES
    ('map_version_p6_baseline', 1, 'P6 confirmed four-foundation worldview map',
     'system', 'babata', '2026-07-20T00:00:00Z');

INSERT INTO knowledge_map_nodes
    (map_node_id, map_version_id, node_level, canonical_key, name,
     provenance_kind, suggestion_id, created_at)
VALUES
    ('mapnode_p6_time', 'map_version_p6_baseline', 'foundation', 'foundation:time',
     '时间', 'system', NULL, '2026-07-20T00:00:00Z'),
    ('mapnode_p6_space', 'map_version_p6_baseline', 'foundation', 'foundation:space',
     '空间', 'system', NULL, '2026-07-20T00:00:00Z'),
    ('mapnode_p6_matter', 'map_version_p6_baseline', 'foundation', 'foundation:matter',
     '物质', 'system', NULL, '2026-07-20T00:00:00Z'),
    ('mapnode_p6_consciousness', 'map_version_p6_baseline', 'foundation',
     'foundation:consciousness', '意识', 'system', NULL, '2026-07-20T00:00:00Z');

INSERT INTO score_profiles
    (profile_id, ordinal, interest_weight, strategy_weight, consensus_weight,
     rationale, author_kind, author, created_at)
VALUES
    ('score_profile_p6_default', 1, 40, 35, 25,
     'P6 default: present interest / future strategy / past consensus',
     'system', 'babata', '2026-07-20T00:00:00Z');

CREATE TRIGGER model_suggestions_immutable_update BEFORE UPDATE ON model_suggestions
BEGIN SELECT RAISE(ABORT, 'model suggestions are immutable'); END;
CREATE TRIGGER model_suggestions_append_only_delete BEFORE DELETE ON model_suggestions
BEGIN SELECT RAISE(ABORT, 'model suggestions are append-only'); END;
CREATE TRIGGER score_profiles_immutable_update BEFORE UPDATE ON score_profiles
BEGIN SELECT RAISE(ABORT, 'score profiles are immutable'); END;
CREATE TRIGGER score_profiles_append_only_delete BEFORE DELETE ON score_profiles
BEGIN SELECT RAISE(ABORT, 'score profiles are append-only'); END;
CREATE TRIGGER relevance_scores_immutable_update BEFORE UPDATE ON relevance_scores
BEGIN SELECT RAISE(ABORT, 'relevance scores are immutable'); END;
CREATE TRIGGER relevance_scores_append_only_delete BEFORE DELETE ON relevance_scores
BEGIN SELECT RAISE(ABORT, 'relevance scores are append-only'); END;
CREATE TRIGGER suggestion_reviews_immutable_update BEFORE UPDATE ON suggestion_reviews
BEGIN SELECT RAISE(ABORT, 'suggestion reviews are immutable'); END;
CREATE TRIGGER suggestion_reviews_append_only_delete BEFORE DELETE ON suggestion_reviews
BEGIN SELECT RAISE(ABORT, 'suggestion reviews are append-only'); END;

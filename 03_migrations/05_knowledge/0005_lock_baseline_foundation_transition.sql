-- Close the remaining transition path into the fixed P6 baseline foundations.
-- Migration 0004 is already applied to real data and remains immutable.

DROP TRIGGER knowledge_map_foundation_update_guard;

CREATE TRIGGER knowledge_map_foundation_update_guard
BEFORE UPDATE ON knowledge_map_nodes
WHEN (OLD.node_level = 'foundation' OR NEW.node_level = 'foundation')
 AND (OLD.map_version_id = 'map_version_p6_baseline'
      OR NEW.map_version_id = 'map_version_p6_baseline')
BEGIN
    SELECT RAISE(ABORT, 'P6 foundation nodes are immutable');
END;

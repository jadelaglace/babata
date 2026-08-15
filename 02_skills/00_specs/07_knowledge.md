# Knowledge Skill Specification

- Target CLI: `babata knowledge ...`
- Input: item/revision references, human record, relation, classification, model, score, analysis, or suggestion decision
- Output: first-party knowledge record and version references
- Permissions: explicit human action; invoke Rust application services only
- Errors: validation, not found, conflict, `capability_unavailable`
- Activation: P6 after TC-05

## Course and cross-map registration

`babata knowledge register-course --definition <json>` is the only writer for the successor course
registration contract. The input schema is `babata.course-registration/v1`; the command is atomic and
idempotent by `(course_key, version, definition_sha256)`, and `show-course` must read the complete result
back from the raw/knowledge authority.

The definition contains:

- a distinct course key, version, title, source/term identity and `pending_user_acceptance` state;
- one or more typed `covers` relations to existing active `Branch` nodes;
- the exact course module -> semantic-entry membership;
- the complete intended semantic -> map-node assignment set, with independent
  `primary/secondary/contextual` role, `0..100` strength, separate `0..100` confidence, rationale and
  method version; values are not required to sum to `100` and do not reuse relevance-score fields;
- optional non-parent map-node relations limited to declared types such as `intersects_with`,
  `draws_from`, `applies_to` and `prerequisite_of`;
- one versioned `SublibraryDefinition` lens reference whose definition explicitly contains the course
  reference and all covered branch references.

Registration may add missing intended semantic assignments but never deletes or replaces an existing
assignment. Course identity is never a map node and cannot equal a branch identity. Parent/subfield edges
remain in `knowledge_map_edges`; non-parent relations remain separately typed. A failed definition writes
nothing. Replaying the same definition returns the same course identity; a different definition for an
existing course key/version fails closed.

Historical MBA single-path registrations remain readable. Compatibility migration creates new course/lens
relations through this command and does not alter prior C1B, semantic content, package/live bytes, closure
receipts or accepted/closed facts.

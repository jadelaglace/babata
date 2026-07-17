# Knowledge Skill Specification

- Target CLI: `babata knowledge ...`
- Input: item/revision references, human record, relation, classification, model, score, analysis, or suggestion decision
- Output: first-party knowledge record and version references
- Permissions: explicit human action; invoke Rust application services only
- Errors: validation, not found, conflict, `capability_unavailable`
- Activation: P6 after TC-05

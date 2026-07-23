# Sublibraries Skill Specification

- Target CLI: `babata sublibraries ...`
- Input: versioned selection, include/exclude, and organisation rules
- Output: sublibrary definition or rebuildable materialisation reference
- Permissions: definitions write through Rust core; materialisation is read-only C2
- Errors: invalid scope, not found, build failure, `capability_unavailable`
- Underlying CLI: enabled in P6.3 after TC-07
- Formal Skill activation: P7; this specification does not itself expose an active Skill

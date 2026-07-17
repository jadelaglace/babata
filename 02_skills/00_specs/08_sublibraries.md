# Sublibraries Skill Specification

- Target CLI: `babata sublibraries ...`
- Input: versioned selection, include/exclude, and organisation rules
- Output: sublibrary definition or rebuildable materialisation reference
- Permissions: definitions write through Rust core; materialisation is read-only C2
- Errors: invalid scope, not found, build failure, `capability_unavailable`
- Activation: P6 after TC-07

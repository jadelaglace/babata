# Operations Skill Specification

- Target CLI: `babata ops ...`
- Input: status/doctor command or snapshot reference
- Output: health and restore-verification evidence
- Permissions: invoke Rust CLI only; secrets remain external
- Errors: backup, restore, integrity, `capability_unavailable`
- Activation: P8 after TC-10; full-system recovery evidence also contributes to TC-11

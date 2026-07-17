# Capture Skill Specification

- Target CLI: `babata capture ...`
- Input: text, local path/export, or a validated candidate reference
- Output: machine-readable capture outcome or error envelope
- Permissions: local input read; no direct SQLite or asset-finalisation access
- Errors: validation, I/O, integrity, `capability_unavailable`
- Activation: P3 for engineering/recovery capture after the relevant TC-03/TC-06 subset; P4 contextual collection after TC-01/TC-02

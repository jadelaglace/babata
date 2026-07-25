# Capture Internal Capability Specification

- Target CLI: `babata capture ...`
- Input: text, local path/export, or a validated candidate reference
- Output: machine-readable capture outcome or error envelope
- Permissions: local input read; no direct SQLite or asset-finalisation access
- Errors: validation, I/O, integrity, `capability_unavailable`
- Ownership: internal formal-C0 capability used by `babata-collect`; it is not a separate user-visible Skill
- Boundary: ends after Rust application/core commit and read-back; never invokes Process/C1
- Activation: P3 for engineering/recovery capture after the relevant TC-03/TC-06 subset; P4 contextual collection after TC-01/TC-02

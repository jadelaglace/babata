# Outputs Skill Specification

- Target CLI: `babata outputs ...`
- Input: explicit item/sublibrary scope and output kind
- Output: build status, output reference, and provenance manifest
- Permissions: read C0/C1 through Rust services; no reverse write from generated output
- Errors: unsupported kind, invalid scope, build failure, `capability_unavailable`
- Underlying CLI: enabled in P6.3 after TC-08 for Markdown and structured JSON only
- Formal Skill activation: P7; this specification does not itself expose an active Skill

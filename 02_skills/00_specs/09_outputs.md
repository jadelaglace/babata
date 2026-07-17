# Outputs Skill Specification

- Target CLI: `babata outputs ...`
- Input: explicit item/sublibrary scope and output kind
- Output: build status, output reference, and provenance manifest
- Permissions: read C0/C1 through Rust services; no reverse write from generated output
- Errors: unsupported kind, invalid scope, build failure, `capability_unavailable`
- Activation: P6 after TC-08

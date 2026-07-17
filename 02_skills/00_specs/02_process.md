# Process Skill Specification

- Target CLI: `babata process ...`
- Input: revision and pipeline identifiers
- Output: job/run status and derivative references
- Permissions: invoke Rust CLI only; no provider credentials in the Skill
- Errors: provider, retry, privacy, `capability_unavailable`
- Activation: P5 after TC-03 and TC-04

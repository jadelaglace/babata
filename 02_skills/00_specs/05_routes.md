# Routes Skill Specification

- Target CLI: `babata routes ...`
- Input: route identifier and authorised source reference
- Output: descriptor, coverage, or candidate envelope
- Permissions: explicit user confirmation for collection; no direct persistence
- Errors: unauthorised, incomplete coverage, `capability_unavailable`
- Activation: P4/P7 after TC-01 and TC-02 for each real source route

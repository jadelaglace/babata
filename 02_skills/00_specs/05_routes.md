# Routes Internal Capability Specification

- Target CLI: `babata routes ...`
- Input: route identifier and authorised source reference
- Output: descriptor, coverage, or candidate envelope
- Permissions: explicit user confirmation for collection; no direct persistence
- Errors: unauthorised, incomplete coverage, `capability_unavailable`
- Ownership: source identification and recipe selection inside `babata-collect`; it is not a second user-visible routing Skill
- Extension: add or revise a source recipe, capability declaration and real tests; a new case or content shape does not create a Skill
- Activation: P4/P7 after TC-01 and TC-02 for each real source route

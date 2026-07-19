# Process Skill Specification

- Target CLI (formal, later): `babata process ...`
- Agent guidance skill (P5 now): `02_skills/babata-bailian-clean/`
- Input: revision / pipeline identifiers (CLI); or local file/dir scope (Agent skill)
- Output: job/run status and derivative references (CLI); or traceable derivatives under `BABATA_DATA_HOME` (Agent skill)
- Permissions:
  - Formal Skill: invoke Rust CLI only; no provider credentials in the Skill
  - Agent skill: may call local tools and `bl` using user-configured Bailian auth; never write secrets into Git
- Errors: provider, retry, privacy, `capability_unavailable`
- Activation:
  - Agent skill `babata-bailian-clean`: activated in P5 for real multimodal cleaning trials after Bailian CLI path is proven
  - Formal `babata process` Skill / capability: after TC-03 and TC-04 and C1 process/provider gates
- Non-goals for the Agent skill: overwriting C0 originals; claiming AC-03/AC-04 passed without core registration; silent replacement of prior derivatives

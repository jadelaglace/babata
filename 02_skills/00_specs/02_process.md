# Process Skill Specification

- Target CLI: `babata process ...`
- Agent guidance skill (parallel): `02_skills/babata-bailian-clean/`
- Input: revision id, pipeline id, derivative payload (text/json/path)
- Output: process_run id, derivative id, status; list pipelines / list runs / show run
- Permissions: invoke Rust CLI only for formal registration; no provider credentials in Skill
- Errors: provider, retry, privacy, `capability_unavailable` (job queue still P5+)
- Activation:
  - P5 now: `list-pipelines`, `register`, `show-run`, `list-runs` against `derived.sqlite`
  - P5+: enqueue/run-once provider execution, full AC-03/AC-04/TC-03/TC-04
- Guarantees: C0 untouched; retries create new runs; derivatives are C1 only

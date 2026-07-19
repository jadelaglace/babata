# Process Skill Specification

- Target CLI: `babata process ...`
- Agent guidance skill (parallel): `02_skills/babata-bailian-clean/`
- Input: revision id, pipeline id, derivative payload (text/json/path), or a queue command
- Output: process_run id, derivative id, process job status; list pipelines / list runs / show run
- Permissions: invoke Rust CLI only for formal registration; no provider credentials in Skill
- Errors: provider, retry, lease, cancellation, privacy, `capability_unavailable`
- Activation:
  - P5: `list-pipelines`, `register`, `register-failure`, `show-run`, `list-runs`, `delete-result`
    against `derived.sqlite`
  - P5: `enqueue`, `run-once`, `status`, `retry`, `cancel` against C3 `runtime.sqlite`; callable
    pipelines are `local_extract_text` and authenticated `bailian_summary`
  - P5+: OCR/ASR/visual queue providers remain unavailable until implemented and evidenced
- Guarantees: C0 untouched; retries create new jobs and runs; C3 never writes C1 directly;
  derivatives are C1 only

# Derived migrations (C1)

| File | Purpose | Activation |
| --- | --- | --- |
| `0001_derived_schema.sql` | `process_runs` + `derivatives` | P5 |
| `0002_derivative_output_hash_required.sql` | Require a verifiable hash for every C1 derivative | P5 / #48 |
| `0003_process_target_identity.sql` | Persist target kind and input asset identity on each run | P5 / #48 |
| `0004_reconcile_precommit_v3.sql` | Audit and reconcile the one verified pre-commit v3 checksum applied to the real data root | P5 / #48 |
| `0005_process_result_invalidation.sql` | Logically delete/rebuild C1 results while retaining process history | P5 / #48 / TC-03A |

Owner: `SqliteDerivedRepository` → `derived.sqlite` under `02_derived/index/`.

## Field notes

- **process_runs**: one attempt per row; retries set `retry_of_run_id` and increment `attempt`.
- **C1 deletion**: `invalidated_at` + `invalidation_reason` remove a result from active authority without erasing the process evidence; rebuilds create a new run.
- **derivatives**: C1 outputs only; never mutate C0. Text/json body may live in-row; binaries use `logical_path` + `output_sha256` under data home.
- **schema_migration_repairs**: immutable evidence for an explicitly recognized migration-history repair; unknown checksums still fail closed.
- Provider credentials never stored; only `provider`, `tool_or_model`, `tool_version`, `params_json`, `usage_json`.

# Derived migrations (C1)

| File | Purpose | Activation |
| --- | --- | --- |
| `0001_derived_schema.sql` | `process_runs` + `derivatives` | P5 |

Owner: `SqliteDerivedRepository` → `derived.sqlite` under `02_derived/index/`.

## Field notes

- **process_runs**: one attempt per row; retries set `retry_of_run_id` and increment `attempt`.
- **derivatives**: C1 outputs only; never mutate C0. Text/json body may live in-row; binaries use `logical_path` + `output_sha256` under data home.
- Provider credentials never stored; only `provider`, `tool_or_model`, `tool_version`, `params_json`, `usage_json`.

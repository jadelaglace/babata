# Runtime migrations

| File | Purpose | Activation |
| --- | --- | --- |
| `0001_process_jobs.sql` | C3 processing queue, worker lease, retry/cancel state, provider task and C1 run reference | P5 / #46 |

Owner: `SqliteJobRepository` -> `runtime.sqlite` under `04_runtime/index/`.

`process_jobs` is disposable C3 runtime state, not a second C1 authority. A provider attempt reaches
formal history only through `ProcessService::register_derivative` or `register_failure` in
`derived.sqlite`. Retries create new jobs and preserve the failed parent; expired worker leases become
failed jobs and are reconciled to a C1 failed run before retry. Provider credentials and auth output are
never stored.

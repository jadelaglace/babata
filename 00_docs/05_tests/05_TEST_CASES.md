# Compass Reboot Test Cases

| ID | Acceptance | Scenario | Expected result |
| --- | --- | --- | --- |
| TC-01 | AC-01 | Configure a temporary data root; run Git scan | SQLite/assets/logs remain outside Git and configuration uses data-root-relative paths |
| TC-02 | AC-02 | Capture text, a local file/export, and a first-party note; re-import/revise | Each has a stable revision/hash/context; re-import/revision creates links without overwrite |
| TC-03 | AC-03 | Run text extraction and one media derivative | Raw source text/assets retain hashes; derivatives reference input revision/run |
| TC-04 | AC-04 | Force one provider failure then retry; delete derived output and rerun | Failure/retry is recorded; raw remains unchanged; rebuilt derivative is traceable |
| TC-05 | AC-05 | Create note, revision, and annotation of an external fixture | First-party provenance, parent/relation links, and original wording are queryable |
| TC-06 | AC-06 | Search raw/derived fixture; build and remove generated view | Search reveals lineage; removing/rebuilding view changes no authority data |
| TC-07 | AC-07 | Import permitted Feishu/export and browser/bookmark fixture; try incomplete route | Successful routes record coverage/limits; incomplete route is not marked enabled |
| TC-08 | AC-08 | Back up fixture SQLite/assets, restore into isolated path, sample hashes | Restored indexes open and sampled asset hashes match; no live database sync is required |
| TC-09 | AC-09 | Inspect Rust CLI/API use-case dependency graph; run JS/Python adapter fixtures | Rust owns all writes/finalisation/queue state; JS is browser-only; Python is an exception-only candidate adapter; loopback API rejects missing/invalid token and never binds remotely |
| TC-10 | AC-10 | Run workspace dependency checks and architecture tests with a mock port implementation | Domain/application compile without IO infrastructure; infrastructure implements ports; CLI/API/worker only compose use cases; reverse dependencies and direct secondary writers are rejected |
| TC-11 | AC-11 | Compare the implemented R1 workspace against the file-level blueprint and compile its public surfaces | Required files and public functions have one owner; no deferred API/worker/provider files appear without an activation decision; each required file has its mapped test home |

## Skill test rule

A Skill is added only after its underlying CLI command has passed the mapped
test case. Skill tests validate routing and output references; they do not
replace capture, processing, workspace, or backup tests.

# Compass Reboot Architecture

## 1. Logical architecture

```text
01_capture -> 02_raw -> 03_process -> 02_derived -> 05_views
                  ^                         |
                  |---- 04_workspace -------|
```

The arrows are in-process command/data dependencies within one repository, not
network services. Raw data is append-only; processors read raw and write only
derived records. Workspace writes first-party raw revisions and relations.
Views read indexes and generated artifacts only.

## 2. Repository and data boundary

`C:\Users\Aiano\Compass` is Git and contains no real data. The application
uses `COMPASS_DATA_HOME`, initially `C:\Users\Aiano\CompassData`, with this
ordered partitioning:

```text
00_inbox     temporary files, exports, personal input
01_raw       raw.sqlite, originals, imports, quarantine, manifests
02_derived   derived.sqlite, text/media/structured/task artifacts
03_views     Datasette, Obsidian, exports, sublibrary outputs
04_runtime   queue, cache, indexes, sessions
05_logs      capture, process, view, ops logs
```

`01_raw` and first-party revisions are P0. `02_derived` is P1. `03_views` is
P2. Runtime/logs are P3. Database rows refer to logical IDs and paths relative
to their data partition, so changing the data root does not rewrite records.

## 3. Database authority

`raw.sqlite` owns sources, collections, root items, immutable revisions, raw
assets, and relations. `derived.sqlite` owns processing runs, tasks, and
derivatives. A derived row must name its raw revision and input hash. A view is
never a database writer. SQLite migrations are code-reviewed files under
`03_migrations/`.

## 4. Processing boundary

Pipelines are ordered as mechanical extraction, faithful text/media derivatives,
then optional model interpretation. Bailian CLI is the interactive provider;
Bailian/Qwen APIs and batches are queue providers. Provider credentials live in
local protected configuration outside Git/data snapshots unless explicitly
encrypted by the backup policy. Per-source privacy policy can deny model upload.

## 5. Skill boundary

Each Skill shells into a tested `compass` CLI command and returns command
results/references. It has no direct database implementation, credential store,
or authority to alter a source record outside that command's validation.

## 6. Backup architecture

Backup creates SQLite-consistent copies/checkpoints in an isolated staging area,
captures raw/workspace first, then derived, and verifies restore hashes in a
separate destination. NAS/cloud receives backup snapshots, not live database
directories.

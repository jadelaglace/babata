# Compass Reboot Technical Architecture

## 1. Architectural decision

Compass is one Rust application workspace with local SQLite databases and an
external numbered data root. Rust is the default implementation for every
capability and the sole business/persistence core. The system is CLI-first, with
an optional loopback API for genuine local callers. JavaScript/TypeScript and
Python are constrained boundary exceptions, never alternative application cores.

```text
Skill / shell / scheduler              browser extension or local UI
          |                                           |
          +------------- compass CLI -----------------+
                                |                       |
                                +--- loopback API -------+
                                           |
                                 Rust application core
       +---------------------------+--------------------------+
       |                           |                          |
 raw capture/workspace        process/worker              explore/views/ops
       |                           |                          |
 raw.sqlite + raw assets       derived.sqlite            generated views/snapshots
```

This is an in-process architecture, not a networked microservice graph. The API
is a convenience entry point that invokes the same Rust use cases as the CLI.

## 2. Repository layout and Rust workspace

The numbered repository layout remains the governing physical layout. Rust code
under `01_app/` is a Cargo workspace rather than a collection of standalone
applications.

```text
01_app/
├── Cargo.toml                     # workspace manifest
├── 01_compass_domain/             # IDs, domain types, invariants, errors
├── 02_compass_application/        # use cases, input/output types, port traits
├── 03_compass_infrastructure/     # SQLite, assets, config, Bailian, backup, adapters
├── 04_compass_cli/                # clap `compass` executable and composition root
├── 05_compass_local_api/          # axum loopback API and composition root, when needed
└── 06_compass_worker/             # queue worker and composition root
```

Names may be shortened in Cargo package metadata, but the numeric directories
are retained for repository readability. The core crate dependency direction is:

```text
domain <- application <- infrastructure
       ^                ^
       +---- cli / local_api / worker (composition roots) ----+
```

- `01_compass_domain` contains no filesystem, SQLite, HTTP, provider, or CLI
  dependency.
- `02_compass_application` defines business invariants and the repository,
  asset, provider, clock, and backup port traits it requires. It never imports
  SQLite, filesystem, HTTP, provider SDK, or process-execution crates.
- `03_compass_infrastructure` is the only component that opens raw/derived
  SQLite or finalises assets under the data root. It implements application
  ports and supplies Rust source importers, Bailian providers, and backup
  drivers.
- CLI, API, and worker are composition roots: they construct infrastructure,
  call application use cases, map I/O, and contain no business decisions.
- Skills, JS, and Python are callers or candidate producers; provider adapters
  may read inputs and return outputs but never write data directly.

## 3. Runtime configuration and data root

The executable resolves `COMPASS_DATA_HOME` first, then an explicit config path,
then a documented local default. The initial default is
`C:\Users\Aiano\CompassData`.

```text
00_inbox/     temporary external exports and first-party input
01_raw/       raw index, originals, imports, quarantine, manifests
02_derived/   derived index, text/media/structured artifacts
03_views/     Datasette, Obsidian, exports, sublibraries
04_runtime/   queue, cache, indexes, sessions, protected local config
05_logs/      capture, process, views, operations logs
```

Configuration types:

```text
DataRootConfig       data root and partition resolution
SqliteConfig         journal mode, busy timeout, migration policy
ProviderConfig       provider executable/endpoint/model selection; no secrets in Git
PrivacyPolicy        source/type processing permission and upload rules
ApiConfig            loopback bind, enabled flag, token location, allowed origins
BackupConfig         snapshot staging and target policy
```

Database paths and asset references are logical keys relative to numbered
partitions. Moving `COMPASS_DATA_HOME` does not modify row content.

## 4. Persistence and concurrency model

`raw.sqlite` is the authority for sources, contexts, items, immutable revisions,
raw assets, and relations. `derived.sqlite` is the authority for process runs,
jobs, and derivative artifacts. Their schemas live in
`03_migrations/01_raw` and `03_migrations/02_derived`; a migration ledger is
stored in each database.

SQLite runs in WAL mode with foreign keys enabled, a bounded busy timeout, and
short write transactions. A write use case validates input, stages/copies assets
into a temporary partition, hashes them, starts `BEGIN IMMEDIATE`, inserts rows,
atomically finalises staged files, and commits. Failure removes staging or leaves
an explicit recoverable journal entry; it never presents a partial revision as
complete.

The initial topology is one active writer machine. A worker claims process jobs
transactionally with a lease/heartbeat; it may process many jobs concurrently,
but each claim/result transition is short and transactional. NAS/cloud is backup
or restored-copy storage, not a live multi-writer database mount.

## 5. Domain types and core services

These are Rust types/use cases, not external contracts. They are intentionally
small enough to evolve with the working system.

```text
SourceKind             External | FirstParty
RevisionKind           Capture | Import | Authored | Edit | Annotation
ContentType            Text | Document | Image | Audio | Video | WebPage | Archive | Unknown
DerivativeKind         FaithfulText | OcrText | Subtitle | Transcript |
                       VisualDescription | Keyframes | Summary | Structure | Interpretation
ProcessingState        Queued | Running | Succeeded | Failed | Skipped | Cancelled
AssetRole              Original | Attachment | Export | Cover | Derived | Preview
RelationKind           Revises | Annotates | Quotes | RespondsTo | RelatedTo
```

Core request/result types:

```text
CaptureRequest         source kind/provider/locator/context/raw payload/assets/metadata
CaptureResult          item ID/revision ID/asset IDs/duplicate signal/status
CreateRequest          first-party content/authoring context/assets/metadata
ReviseRequest          parent revision/content/assets/revision note
AnnotateRequest        target item/revision/content/authoring context
ProcessRequest         raw revision/pipeline/options/priority/privacy approval
ProcessResult          process run/job IDs/state/derivative IDs/cost/error
QueryRequest           text/metadata/source/time/type/status filters/page cursor
RecordDetail           item/revisions/assets/relations/derivatives/lineage
BuildViewRequest       view kind/filter/template/build target
BackupRequest          partition scope/staging target/verification mode
```

Use-case service interfaces:

```text
CaptureService.capture(CaptureRequest) -> CaptureResult
WorkspaceService.create(CreateRequest) -> CaptureResult
WorkspaceService.revise(ReviseRequest) -> CaptureResult
WorkspaceService.annotate(AnnotateRequest) -> CaptureResult
ProcessService.enqueue(ProcessRequest) -> ProcessResult
ProcessService.run_once(job_id) -> ProcessResult
ProcessService.retry(job_id) -> ProcessResult
ExploreService.search(QueryRequest) -> Page<RecordSummary>
ExploreService.show(item_or_revision_id) -> RecordDetail
ViewService.build(BuildViewRequest) -> BuildResult
OpsService.status() -> SystemStatus
OpsService.backup(BackupRequest) -> BackupResult
OpsService.restore_verify(snapshot_ref) -> RestoreReport
```

Application port traits below those services:

```text
RawRepositoryPort      source/context/item/revision/relation transactions
DerivedRepositoryPort  jobs/runs/derivatives transactions
AssetStorePort         stage/hash/finalise/open asset by logical key
JobRepositoryPort      enqueue/claim/heartbeat/complete/fail/retry
ProcessProviderPort    prepare/run/poll/cancel/fetch output
CandidateRunnerPort    execute peripheral adapter and parse candidate envelope
ViewBuilderPort        query/read only, write generated view files
BackupDriverPort       SQLite-consistent snapshot/restore/hash verification
```

No provider, adapter, or view builder receives a mutable database connection.
Only the Rust `Sqlite*Repository` and `FileAssetStore` infrastructure
implementations mutate persistent state, and only when called by an application
use case.

## 6. CLI surface

The first executable is `compass`. Human operators, Skills, scheduled tasks,
JS bridges, and Python wrappers prefer this interface. Output defaults to
human-readable text; `--json` emits stable command result objects for automation.

```text
compass data status
compass capture text --provider <name> --text <text> [--context <id>]
compass capture file --provider <name> --path <file> [--context <id>]
compass capture export --provider <name> --path <export> [--context <id>]
compass capture candidate --path <candidate-envelope.json>

compass create --path <file>|--text <text>
compass revise --parent <revision-id> --path <file>|--text <text>
compass annotate --target <item-or-revision-id> --path <file>|--text <text>

compass process enqueue --revision <id> --pipeline <name> [--priority <n>]
compass process run --job <id>
compass process worker
compass process status [--job <id>]
compass process retry --job <id>
compass process cancel --job <id>

compass explore search <query> [filters]
compass explore show <item-or-revision-id>
compass views build datasette|obsidian [filters]

compass routes list|show|evaluate
compass ops backup [--scope raw|derived|all]
compass ops restore-verify --snapshot <ref>
compass ops doctor
```

The commands are definitions, not an instruction to implement all commands at
once. Capture functions precede processing; process precedes views; a Skill is
created only after its corresponding command is working.

## 7. Loopback local API

The local API is disabled by default and begins only when a browser extension or
local UI has a demonstrated need. It binds to `127.0.0.1` or `::1`, never a LAN
interface. It has an installation-local bearer token stored outside Git and
strict request-size/origin configuration.

It maps directly to use cases; it is not a second implementation:

```text
POST /v1/captures/text              -> CaptureService.capture
POST /v1/captures/file              -> CaptureService.capture
POST /v1/captures/web               -> CaptureService.capture
POST /v1/workspace/notes            -> WorkspaceService.create
POST /v1/workspace/revisions        -> WorkspaceService.revise
POST /v1/workspace/annotations      -> WorkspaceService.annotate
POST /v1/process/jobs               -> ProcessService.enqueue
POST /v1/process/jobs/{id}/retry    -> ProcessService.retry
GET  /v1/process/jobs/{id}          -> job/run status
GET  /v1/records/{id}               -> ExploreService.show
GET  /v1/search                     -> ExploreService.search
GET  /v1/health                     -> OpsService.status
```

The API returns command-result-style JSON IDs and status, never raw SQLite
handles or direct filesystem authority. Asset upload/download, CORS policy,
extension pairing, and web capture payload limits are implementation decisions
for the later concrete development plan.

## 8. Peripheral adapters

### JavaScript / TypeScript

Use only when browser execution is the most direct solution:

```text
browser extension/userscript
  -> gather URL/title/selected or extracted DOM/declared page metadata
  -> submit to loopback API after explicit local pairing
  -> or save a candidate envelope for `compass capture candidate`
```

It does not include a SQLite driver, data-root write permission, provider
credential store, or independent processing rules.

### Python (exception only)

Use only when a maintained Python-only parser/library/tool has a demonstrated
benefit that a Rust crate, Rust implementation, or stable CLI cannot reasonably
provide. Rust source importers are the default.

```text
versioned python child process
  -> reads authorised input
  -> writes temporary files only under 04_runtime staging
  -> emits CandidateEnvelope JSON to stdout/file
  -> Rust `CandidateRunner` validates and calls CaptureService
```

`CandidateEnvelope` contains provider/source/context metadata, text/file
references relative to the adapter staging directory, declared asset roles,
and adapter name/version. It contains no credentials or direct database paths.
The Rust core hashes, copies/finalises assets, assigns IDs, and persists results.
It records the child process name/version and rejects envelopes outside the
declared staging root.

## 9. Processing provider architecture

`ProcessProvider` has two initial implementations:

```text
BailianCliProvider
  - invokes configured `bl` executable for an approved pipeline
  - stages only authorised inputs
  - records command version, normalized arguments, task IDs, stderr/stdout refs,
    output artifacts, cost when available, and exit/error state

BailianApiProvider
  - uses the configured Bailian/Qwen API for queued or batch execution
  - manages submit/poll/fetch/cancel and provider task IDs
  - follows the same ProcessProvider result path as the CLI provider
```

Pipeline definitions are versioned configuration, not hard-coded model truth:

```text
mechanical_document_extract
faithful_text
image_ocr_and_description
audio_transcript
video_subtitle_keyframes_and_description
structure_and_summary
```

The pipeline decides which derivatives it may produce. A privacy policy resolves
before any provider receives bytes; denied items remain raw and are reported as
skipped rather than silently processed.

## 10. Skills and views

Skills are thin descriptions around proven CLI commands:

```text
01_compass_capture   -> compass capture / create / revise / annotate
02_compass_process   -> compass process
03_compass_workspace -> compass create / revise / annotate / explore show
04_compass_explore   -> compass explore / views build
05_compass_ops       -> compass data / ops
```

Datasette opens local read-only SQLite/query views. Obsidian generation is a
`ViewBuilder` target that writes only `03_views/02_obsidian`; deletion/rebuild
does not affect raw or derived authority.

## 11. Backup and recovery

`BackupDriver` checkpoints/copies each SQLite database through SQLite's backup
mechanism into isolated staging, records an inventory with logical asset keys
and hashes, then invokes the selected encrypted incremental backup target.
Restore writes to an isolated data root, opens indexes, and samples hashes before
any operator switches a live data root. P0 raw/first-party data precedes P1
derived data; P2/P3 may be rebuilt.

## 12. Architecture coverage

| Acceptance | Architectural enforcement |
| --- | --- |
| AC-01 | DataRootConfig, relative asset keys, Git ignore boundary |
| AC-02, AC-05 | RawRepository, Capture/Workspace services, immutable revisions |
| AC-03, AC-04 | DerivedRepository, ProcessProvider, JobRepository, no raw writer |
| AC-06 | Read-only QueryService and ViewBuilder |
| AC-07 | Route evaluation records through CaptureService/configuration |
| AC-08 | BackupDriver and isolated restore verification |
| AC-09 | Rust domain/store/usecase ownership; CLI/API shared services; peripheral runner boundary |

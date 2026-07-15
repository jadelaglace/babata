# Babata P3 原始资料入库底座实现蓝图

> P2 creates the complete Babata skeleton defined by
> `04_SYSTEM_SKELETON_BLUEPRINT.md`. This document activates only the 29
> raw-foundation files that receive real business and persistence behaviour in
> P3; it is not the inventory of the whole workspace. SQL ownership, transaction
> order, implementation sequence and command verification are preserved in
> `06_RAW_FOUNDATION_EXECUTION_PLAN.md`.

## 1. P3 原始资料入库底座范围、文件数量与依赖规则

P3 covers only data-root resolution, raw SQLite opening/migration, text/file/
export capture, first-party create/revise/annotate, raw asset staging/finalising,
read-back detail, and CLI output.

```text
Package                         Rust files   Responsibility
01_babata_domain                         6   Pure model and validation
02_babata_application                    9   Use cases, DTOs, ports
03_babata_infrastructure                 9   Config, SQLite, assets, logging
04_babata_cli                            5   CLI parsing, wiring, rendering
Total                                    29
```

The count excludes `Cargo.toml`, SQL migrations, generated files, fixture files,
and test modules. `05_babata_local_api`, `06_babata_worker` and all later
capability modules already have P2 skeletons, but P3 does not activate their
functional behaviour.

```text
domain <- application <- infrastructure
       ^                ^
       +--- cli (P3) / local_api / worker composition roots ---+
```

`domain` depends on no Babata crate. `application` depends only on `domain`.
`infrastructure` implements application port traits. CLI/API/worker may depend
on all three and only compose dependencies/map I/O. No reverse dependency is
permitted.

## 2. Cargo workspace and permitted dependencies

```text
01_app/
├── Cargo.toml
├── 01_babata_domain/Cargo.toml
├── 02_babata_application/Cargo.toml
├── 03_babata_infrastructure/Cargo.toml
├── 04_babata_cli/Cargo.toml
├── 05_babata_local_api/Cargo.toml
└── 06_babata_worker/Cargo.toml
```

```text
domain:          serde, uuid/ulid, time, thiserror, validation helpers
application:     domain, async-trait, serde, thiserror
infrastructure:  domain, application, SQLite, filesystem/hash/config/process crates
cli:             domain, application, infrastructure, clap, serde_json
```

Outside Infrastructure, forbid SQLite drivers, filesystem mutation, provider
SDKs, process execution, HTTP client/server, secret loading, and direct data-root
paths. Domain also excludes system clock reads; time comes from input or a port.

## 3. Exact P3 file inventory

### 3.1 `01_babata_domain` (6 files)

| File | Public types/functions | Owns / forbids |
| --- | --- | --- |
| `src/lib.rs` | module exports | Re-exports only; no orchestration |
| `src/ids.rs` | `ItemId`, `RevisionId`, `AssetId`, `SourceId`, `CollectionId`; `new`, `parse`, `Display` | Stable opaque IDs; no DB integers |
| `src/kinds.rs` | `SourceKind`, `RevisionKind`, `ContentType`, `AssetRole`, `RelationKind`, `DerivativeKind`, `ProcessingState` | Closed enums/string representation; no provider logic |
| `src/entities.rs` | `SourceRef`, `CollectionContext`, `RawItem`, `RawRevision`, `AssetRef`, `Relation` | Immutable state/constructors; no I/O |
| `src/value.rs` | `LogicalPath`, `Sha256`, `UtcTimestamp`, `Metadata`, `TextPayload`, `AssetInput` | Relative paths, hashes, bounded metadata; no file reads |
| `src/error.rs` | `DomainError` | Validation/conflict/not-found vocabulary; no SQL/provider errors |

Target public surface: roughly 25 constructors/parsers/validators. Reading a
file, hashing bytes, starting a transaction, or invoking a provider is forbidden.
Tests live inline beside each owner; minimum six domain tests.

### 3.2 `02_babata_application` (9 files)

| File | Public types/functions/traits | Owns / forbids |
| --- | --- | --- |
| `src/lib.rs` | module exports | Use-case/port export only |
| `src/dto.rs` | `CaptureTextCommand`, `CaptureFileCommand`, `CaptureExportCommand`, `CreateNoteCommand`, `ReviseCommand`, `AnnotateCommand`, `CaptureOutcome`, `RecordDetail` | Command/result shapes; no transport types |
| `src/error.rs` | `ApplicationError` | Maps domain/port failures; no HTTP status |
| `src/ports/mod.rs` | port exports | Single import point |
| `src/ports/raw_repository.rs` | `RawRepositoryPort` | `find_source`, `find_item`, `find_revision`, `find_by_source_identity`, `insert_capture_graph`, `insert_relation`, `load_detail` |
| `src/ports/asset_store.rs` | `AssetStorePort` | `stage`, `hash_staged`, `finalize`, `discard_stage`, `open` |
| `src/usecases/mod.rs` | service exports | Module exposure only |
| `src/usecases/capture.rs` | `CaptureService::{capture_text,capture_file,capture_export}` | Shared private capture flow, duplicate signal, compensation; no SQLite/filesystem imports |
| `src/usecases/workspace.rs` | `WorkspaceService::{create,revise,annotate}` | First-party revisions/relations only; no SQLite/filesystem imports |

Target public surface: 13 service methods, seven raw-repository methods, five
asset-store methods. Capture/workspace use mocks in this package; minimum six
use-case tests. A `ClockPort` may be declared in `ports/mod.rs` only if needed
for deterministic tests; do not add a separate one-method file.

### 3.3 `03_babata_infrastructure` (9 files)

| File | Public types/functions | Owns / forbids |
| --- | --- | --- |
| `src/lib.rs` | infrastructure exports | Builders only; never expose mutable DB handles |
| `src/config.rs` | `AppConfig`, `DataRoot`, `SqliteOptions`, `load_config` | Env/config/default resolution; no business rules |
| `src/paths.rs` | `DataPaths`, `ensure_layout`, `staging_path` | All numbered partition mapping; prevent path escape |
| `src/sqlite/mod.rs` | `SqliteRawRepository`, `open_raw_database` | WAL/foreign keys/busy timeout; no use-case decisions |
| `src/sqlite/migrate.rs` | `migrate_raw` | Migration ledger/version validation |
| `src/sqlite/raw_repository.rs` | `RawRepositoryPort for SqliteRawRepository` | SQL mapping and transactions only |
| `src/assets/mod.rs` | `FileAssetStore` export | Asset-store builder only |
| `src/assets/file_store.rs` | `AssetStorePort for FileAssetStore` | Stage/hash/finalise/discard; no SQL |
| `src/observability.rs` | `init_tracing`, `OperationLog` | Redacted structured logs; no raw private payloads |

Only `sqlite/mod.rs`, `sqlite/migrate.rs`, and `sqlite/raw_repository.rs` open
SQLite. Only `FileAssetStore` finalises assets. Tests include at least six
SQLite/file integration cases.

Required raw SQL migration inventory:

```text
03_migrations/01_raw/
├── 0001_raw_schema.sql       # sources, collections, items, revisions, assets, relations
├── 0002_raw_indexes.sql      # source identity, root/revision, time/hash indexes
└── 0003_raw_fts.sql          # raw faithful-text FTS and triggers, if enabled in P3
```

### 3.4 `04_babata_cli` (5 files)

| File | Public types/functions | Owns / forbids |
| --- | --- | --- |
| `src/main.rs` | `main` | Exit code/tracing/bootstrap only |
| `src/app.rs` | `run`, `Dependencies::build` | Composition root: config, adapters, services |
| `src/commands/mod.rs` | `Command` export | Clap tree registration only |
| `src/commands/capture.rs` | `CaptureCommand`, `WorkspaceCommand`, `execute_capture`, `execute_workspace` | Parse/map command flags to DTOs; no business decisions |
| `src/render.rs` | `render_human`, `render_json`, `CliError` | Result/error rendering only |

Target surface: one entry, one dependency builder, four command executors, two
renderers. Minimum two parser/render command tests. Do not add process/explore/
view/API command variants before the matching application use case exists.

## 4. P3 commands and result envelopes

```text
babata data status
babata capture text --provider <name> --text <text> [--context <id>]
babata capture file --provider <name> --path <file> [--context <id>]
babata capture export --provider <name> --path <file> [--context <id>]
babata create --text <text>|--path <file>
babata revise --parent <revision-id> --text <text>|--path <file>
babata annotate --target <id> --text <text>|--path <file>
```

Success `--json` envelope:

```text
operation_id, item_id, revision_id, asset_ids[], status, duplicate_of?, warnings[]
```

Error `--json` envelope:

```text
code, message, operation_id?, retryable, details?
```

`details` never includes raw content, credentials, or secret absolute paths.
This is a local CLI result format, not an external distributed contract.

## 5. Required write sequence

Capture and first-party writes follow one shared sequence:

```text
1. Validate command/metadata and resolve source or first-party context.
2. Stage assets in 04_runtime; reject paths outside allowed input/staging roots.
3. Hash staged bytes and derive logical final paths.
4. Begin one raw SQLite write transaction.
5. Insert source/context/item/revision/asset/relation rows in pending state.
6. Atomically finalise staged files into 01_raw.
7. Mark ready and commit.
8. On failure: roll back; discard staging or write recoverable orphan journal.
```

If host filesystem/SQLite cannot be atomically combined, use a tested
compensating transaction. Never report ready content without assets or silently
delete an already-finalised original.

## 6. Required test inventory

P3 starts with at least 20 Rust tests:

```text
domain unit tests            >= 6
application mock-port tests  >= 6
infrastructure integration   >= 6
CLI parser/render smoke      >= 2
```

Required cases: invalid ID/value, logical-path traversal rejection, enum
serialization, duplicate signal without deletion, create/revise/annotate
lineage, staged-file failure, SQLite rollback, asset hash match, migration
idempotence/foreign keys, CLI DTO mapping/JSON envelope, and no-second-writer
dependency checks. Runtime test data is created outside Git.

## 7. Later-capability activation gates

| Capability | P2 skeleton location | Replace unavailable shell only when |
| --- | --- | --- |
| Derived/task queue | application `ports/{derived_repository,job_repository,process_provider}.rs`, `usecases/process.rs`; infrastructure SQLite repositories | Raw loop works and a real Bailian run is approved |
| Bailian CLI/API | infrastructure `processing/{bailian_cli,bailian_api}.rs` | Pipeline, privacy, cost/retry tests are approved |
| Source importers | infrastructure `sources/providers/<provider>.rs` | One permitted source has a real fixture/export and declared coverage |
| Python/browser candidates | infrastructure `sources/candidate.rs` and `08_adapters/` | A proven Python-only tool or browser handoff exists |
| Search/views | application `usecases/{explore,views}.rs`; infrastructure `views/{datasette,obsidian}.rs` | Real query/view requirement exists |
| Loopback API | `05_babata_local_api` | Browser/local UI needs more than CLI |
| Worker | `06_babata_worker` | Work must outlive one CLI invocation |
| Backup | application `usecases/ops.rs`; infrastructure `backup/{sqlite_snapshot,restic,manifest}.rs` | Real data needs protected backup beyond fixtures |

Each activation must add acceptance/test mapping, package/file count, public
functions, and dependency assertions before its P2 unavailable shell receives a
real implementation.

## 8. Forbidden file patterns

```text
src/db.rs outside infrastructure
src/models.rs containing SQL/HTTP/filesystem code
src/utils.rs as an unbounded cross-layer dumping ground
src/service.rs combining CLI parsing, business rules, and SQL
JS/Python with SQLite write credentials or data-root final paths
provider directly returning data to a view without ProcessService
```

Every new file must name one owner, inbound dependencies, outbound dependencies,
and its mapped test home. Shared code moves only to the lowest layer that can
own it without introducing a reverse dependency.

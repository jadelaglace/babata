# Babata P3 原始资料入库执行细则

本文保存原路线中已经定义好的原始资料入库实施顺序、数据库范围、事务逻辑、测试顺序和命令验收。它从旧 P2 整体顺延为 P3，不再控制当前 P2 的全系统骨架工作。

P3 开始前必须先完成 `04_SYSTEM_SKELETON_BLUEPRINT.md`；文件级职责以 `05_RAW_FOUNDATION_BLUEPRINT.md` 为准。

## 1. P3 功能范围

P3 只激活：

- 外部 text/file/export capture；
- first-party create/revise/annotate；
- raw SQLite migration 和 repository；
- 原件 stage/hash/finalise/open；
- immutable revision、relation、duplicate signal；
- CLI human/JSON 输出和 read-back；
- 失败补偿、orphan journal 与 quarantine 可见性。

P3 不激活真实平台 adapter、Bailian、derived job、search/view、loopback API、worker、Skill 或 backup。

## 2. 数据库与迁移

```text
03_migrations/01_raw/
├── 0001_raw_schema.sql
├── 0002_raw_indexes.sql
└── 0003_raw_fts.sql
```

`0001_raw_schema.sql` 拥有：

```text
schema_migrations
sources
collections
items
item_collections
revisions
assets
relations
```

关键约束：

- opaque text ID，不使用跨层暴露的数据库整数 ID；
- RFC 3339 UTC 时间；
- SHA-256 使用 64 位小写十六进制；
- metadata 必须是 JSON object；
- revision/asset 状态为 pending、ready 或 quarantined；
- item revision ordinal 在同一 item 内唯一；
- first-party 独立创作没有 external identity key；
- 原始 wording 和原件 asset 不可覆盖。

`0002_raw_indexes.sql` 建立 source identity、item/revision 时间、parent、text hash、asset hash 和 relation 双向索引。

`0003_raw_fts.sql` 只给 ready raw text 建立可重建 FTS5 索引，不增加 P3 search 产品能力，也不成为第二权威。

所有迁移在事务中执行并记录 version、filename、applied time、checksum；已记录版本的 checksum 变化必须失败。

## 3. 共享写入序列

```text
validate request and provenance
-> allocate operation ID
-> create 04_runtime/asset-journal entry
-> stage files under 04_runtime/staging/<operation-id>
-> hash bytes and derive logical final paths
-> BEGIN IMMEDIATE
-> insert source/context/item/revision/assets/relations as pending
-> atomically finalise or hash-deduplicate assets into 01_raw
-> mark rows ready
-> COMMIT
-> remove journal
-> read RecordDetail back through RawRepositoryPort
```

失败规则：

- finalise 前失败：rollback 并清理 staging；
- finalise 后 commit 失败：保留 journal，将原件保留或移动到 `01_raw/quarantine/orphans`；
- 永远不报告缺少原件的 ready revision；
- 永远不因为 duplicate 静默删除 capture event；
- status/doctor 暴露未解决 journal 和 orphan，不自动销毁原件。

## 4. 实施顺序

| 步骤 | 激活文件 | 本阶段实现 |
| --- | --- | --- |
| 1 | Cargo workspace 和四个 raw-active crate manifest | 对齐 P2 六 crate workspace，不移除 API/worker 骨架 |
| 2 | domain IDs、kinds、error | raw 所需 opaque ID、枚举和错误 |
| 3 | domain entities、value | raw entity、LogicalPath、SHA-256、metadata、text/asset input |
| 4 | application DTO/error/ports exports | capture/workspace command 和 outcome |
| 5 | RawRepositoryPort、AssetStorePort | P3 所需接口真实可用 |
| 6 | CaptureService | text/file/export、identity、duplicate、compensation |
| 7 | WorkspaceService | create/revise/annotate、parent 和 relation |
| 8 | config、paths、observability | data-root、分区、路径约束、脱敏日志 |
| 9 | sqlite open/migrate | WAL、foreign keys、timeout、ledger |
| 10 | raw repository | transaction 和 RecordDetail round-trip |
| 11 | file asset store | stage/hash/finalise/discard/open |
| 12 | CLI bootstrap/command tree | 激活 data/capture/workspace，其他组保持 unavailable |
| 13 | capture/workspace render | 稳定 human/JSON 输出和错误码 |

## 5. 命令范围

```text
babata data status
babata capture text --provider <name> --text <text>
babata capture file --provider <name> --path <file>
babata capture export --provider <name> --path <file>
babata create --text <text>|--path <file>
babata revise --parent <revision-id> --text <text>|--path <file>
babata annotate --target <id> --text <text>|--path <file>
```

成功 JSON：

```text
operation_id, item_id, revision_id, asset_ids[], status,
duplicate_of?, warnings[]
```

错误 JSON：

```text
code, message, operation_id?, retryable, details?
```

允许错误码：`validation_failed`、`not_found`、`conflict`、`io_failed`、`migration_failed`、`integrity_failed`、`internal`。

## 6. 测试顺序

1. Domain 至少 6 项：ID、enum、path traversal、hash、metadata、immutable entity。
2. Application mock-port 至少 6 项：new/re-import/duplicate/stage failure/create-revise/annotation。
3. Infrastructure 至少 7 项：layout、empty migration、idempotence/checksum、foreign key、asset hash、rollback/orphan、detail round-trip。
4. CLI 至少 3 项：DTO mapping、JSON envelope/redaction、临时数据根端到端。
5. `check-p3-raw-inventory.ps1` 与 Rust boundary check。

功能测试总数不少于 22；架构脚本不替代功能测试。

## 7. P3 命令验收

所有命令使用全新的临时数据根：

```powershell
$env:BABATA_DATA_HOME = Join-Path $env:TEMP ("babata-p3-" + [guid]::NewGuid())
cargo fmt --all --check --manifest-path .\01_app\Cargo.toml
cargo test --workspace --manifest-path .\01_app\Cargo.toml
powershell -ExecutionPolicy Bypass -File .\05_scripts\check-rust-boundaries.ps1
powershell -ExecutionPolicy Bypass -File .\05_scripts\check-p3-raw-inventory.ps1
cargo run --manifest-path .\01_app\Cargo.toml -p babata-cli -- data status --json
cargo run --manifest-path .\01_app\Cargo.toml -p babata-cli -- capture text --provider fixture --text "raw wording" --json
cargo run --manifest-path .\01_app\Cargo.toml -p babata-cli -- capture file --provider fixture --path .\04_tests\03_fixtures\02_files\sample.txt --json
cargo run --manifest-path .\01_app\Cargo.toml -p babata-cli -- capture export --provider fixture --path .\04_tests\03_fixtures\03_exports\sample-export.md --json
```

验收后检查：

- 临时根存在 `01_raw/index/raw.sqlite` 和 `01_raw/assets`；
- ID、hash、source、revision、relation 和 read-back 一致；
- Git 工作树没有生成数据；
- 失败时先保留临时根排查，再人工删除；
- P3 通过后才进入 P4 真实来源。

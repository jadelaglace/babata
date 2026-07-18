# Babata P3 C0 原始资料底座执行细则

## 1. 前置条件与范围

本文只规定 `05_RAW_FOUNDATION_BLUEPRINT.md` 的实施顺序、migration 所有权、事务、
测试和工程命令。P3 必须在修正后的 P2 骨架门通过后开始；已经存在的 29 文件实现
属于提前工作，需按本文重新审阅，不能作为 P3 已完成的证据。

P3 激活显式 text/file/export 与 first-party create/revise/annotate，不激活真实平台
候选、清洗、Knowledge、检索、输出、local API listener、worker、Skill 或备份。

## 2. Migration 所有权

```text
03_migrations/01_raw/
├── 0001_raw_schema.sql
├── 0002_raw_indexes.sql
└── 0003_raw_fts.sql
```

`0001_raw_schema.sql` 最低拥有：

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

约束：

- opaque text ID，不跨层暴露数据库整数；
- RFC 3339 UTC 时间；
- SHA-256 为统一小写十六进制；
- metadata 是有大小边界的 JSON object；
- revision/asset 使用 pending、ready、quarantined 等明确状态；
- 同一 item 的 revision ordinal 唯一；
- first-party 新建没有伪造 external identity；
- 原始 wording、原件 asset 和旧 revision 不覆盖。

`0002` 建立 source identity、item/revision 时间、parent、text/asset hash 和 relation
双向索引。`0003` 只允许对 ready raw text 建立可重建 FTS；它不开放 P6 search，
不进入 C0 权威判断。`0004` 建立 `capture_operations`，把 operation、revision、每次
来源 locator/native reference/timestamp/metadata 及 pending/ready/quarantined 状态关联起来；
这是 P3 recovery/provenance，不是 P4 route evidence 或 collector session。

每个 migration 在事务中登记 version、filename、applied time、checksum。已应用文件
checksum 改变必须失败。P4 route evidence 和 collector session 不属于本 migration 范围。

## 3. 统一写入事务

```text
validate request and provenance
  -> allocate operation ID
  -> create 04_runtime journal
  -> stage allowed input under 04_runtime/staging/<operation-id>
  -> hash bytes and derive logical final keys
  -> BEGIN IMMEDIATE
  -> insert pending C0 graph
  -> finalise or hash-share immutable assets in 01_raw
  -> mark ready
  -> COMMIT
  -> remove journal
  -> read RecordDetail through RawRepositoryPort
```

失败规则：

- finalise 前失败：回滚并清理 staging；
- finalise 后 commit 失败：保留 journal，原件保留或进入 quarantine/orphans；
- status/doctor 必须暴露未解决 journal 和 orphan；
- 缺少原件的 revision 永不报告 ready；
- duplicate 不删除新操作的溯源；
- 自动清理不得销毁无法确认是否唯一的原件。

## 4. 实施顺序

| 顺序 | 激活位置 | 本步结果 |
| ---: | --- | --- |
| 1 | P2 六 crate workspace | 137 文件目标骨架存在；P3 只选择 29 文件激活 |
| 2 | domain IDs/kinds/error | C0 opaque ID、枚举和错误稳定 |
| 3 | domain entities/value | C0 entity、相对路径、哈希、metadata 和输入值对象 |
| 4 | application DTO/error | 显式 capture/workspace 请求与结果，无 transport 类型 |
| 5 | RawRepositoryPort/AssetStorePort | P3 方法可由 mock 与 infrastructure 实现 |
| 6 | CaptureService | text/file/export 共用 C0 写入编排 |
| 7 | WorkspaceService | create/revise/annotate 和版本/关系 |
| 8 | config/paths/observability | 数据根、编号分区、防逃逸、脱敏日志 |
| 9 | sqlite open/migrate | WAL、foreign keys、timeout、ledger |
| 10 | raw repository | graph transaction 与详情 round-trip |
| 11 | file asset store | stage/hash/finalise/discard/open/verify |
| 12 | CLI composition | 激活 data/capture/workspace；其余能力保持 unavailable |
| 13 | result rendering | 稳定 human/JSON 状态、引用、warning 和脱敏错误 |

任何步骤发现 P2 port 或 ownership 错误，先修正 03/04 蓝图与 P2 骨架，再继续实现，
不在 P3 内增加旁路接口。

## 5. 工程命令范围

```text
babata data status
babata capture text
babata capture file
babata capture export
babata create
babata revise
babata annotate
```

成功结果最低包含：

```text
operation_id, item_id, revision_id, asset_ids[], status,
duplicate_of?, warnings[]
```

错误结果最低包含：

```text
code, message, operation_id?, retryable, details?
```

`details` 不含原始私密内容、凭据或不必要的绝对路径。该 JSON 是本地自动化结果，
不是多服务协议或日常收集 UI。

## 6. 测试顺序

1. Domain：ID、enum、逻辑路径逃逸、hash、metadata 边界、不可覆盖状态。
2. Application mock-port：new capture、re-import、duplicate、stage failure、create/revise、
   annotation。
3. Infrastructure：数据根 layout、migration 初始化/幂等/checksum、foreign key、资产哈希、
   transaction rollback、orphan journal、detail round-trip。
4. CLI：DTO mapping、human/JSON envelope、redaction、临时数据根 smoke。
5. Governance：P2 inventory、Rust boundary、interface ownership、no-secondary-writer。

不以固定测试数量代替场景完整性。功能测试与 P2 工程 gate 分开报告；架构脚本不能
替代真实写入与故障测试。

## 7. P3 验证命令

所有运行使用全新的临时数据根：

```powershell
$env:BABATA_DATA_HOME = Join-Path $env:TEMP ("babata-p3-" + [guid]::NewGuid())
cargo fmt --all --check --manifest-path .\01_app\Cargo.toml
cargo test --workspace --manifest-path .\01_app\Cargo.toml
powershell -ExecutionPolicy Bypass -File .\05_scripts\check-rust-boundaries.ps1
powershell -ExecutionPolicy Bypass -File .\05_scripts\check-no-secondary-writer.ps1
powershell -ExecutionPolicy Bypass -File .\05_scripts\check-p3-raw-inventory.ps1
cargo run --manifest-path .\01_app\Cargo.toml -p babata-cli -- data status --json
cargo run --manifest-path .\01_app\Cargo.toml -p babata-cli -- capture text --provider fixture --text "raw wording" --json
cargo run --manifest-path .\01_app\Cargo.toml -p babata-cli -- capture file --provider fixture --path .\04_tests\03_fixtures\02_files\sample.txt --json
cargo run --manifest-path .\01_app\Cargo.toml -p babata-cli -- capture export --provider fixture --path .\04_tests\03_fixtures\03_exports\sample-export.md --json
```

在 P2 仍未满足 137 文件目标时，`check-p2-skeleton-inventory.ps1` 应明确失败；不得为了
让 P3 命令通过而暂时回退 P2 清单。

## 8. P3 完成判定

P3 只有同时满足以下条件才可在开发流程中改为已完成：

- P2-G1 至 P2-G7 已通过；
- P3-G1：外部数据根和编号分区正确；
- P3-G2：text/file/export 形成可回读 C0；
- P3-G3：first-party create/revise/annotate 版本关系正确；
- P3-G4：故障不产生伪 ready，journal/orphan 可诊断；
- P3-G5：数据库和资产写入 owner 唯一；
- P3-G6：P2 gate 继续成立且没有提前激活其他阶段；
- 全新临时根包含可打开的 C0 index 与哈希正确的原件；
- text/file/export 与 create/revise/annotate 均可回读；
- 故障测试不留下伪 ready，journal/orphan 可诊断；
- Git 工作树没有生成真实数据；
- 其他阶段能力仍诚实 unavailable。

P3 完成只允许宣告 C0 底座可用。P4 的真实上下文收集、P5 的 C1 清洗和 P8 的恢复
仍必须各自通过对应门槛。

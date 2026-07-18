# Babata P3 C0 原始资料底座蓝图

## 1. 文档定位

P2 先建立 `04_SYSTEM_SKELETON_BLUEPRINT.md` 定义的 137 文件全系统骨架。P3 只激活
其中与 C0 原件、第一方版本和统一写入底座直接相关的 29 个既有 Rust 文件。

这 29 个文件是实现子集，不是全系统清单。它们可以作为提前工作存在，但只有 P2
门槛通过、接口与新架构一致、P3 自身功能门通过后，才能宣告 P3 完成。

P3 的目的，是证明“明确给到 Babata 的资料可以可靠、不可覆盖、可回读地进入 C0”。
它不证明飞书/浏览器的正常上下文收集体验、C1 清洗、核心知识沉淀、搜索、输出、
正式 Skill 或备份恢复已经可用。

## 2. P3 范围

P3 激活：

- 外部 text、local file 和 authorised export 的显式提交；
- first-party `create`、`revise`、`annotate`；
- `BABATA_DATA_HOME` 解析与编号数据根；
- C0 SQLite migration、repository 和 read-back；
- 原件 stage、hash、finalise、open 与完整性校验；
- immutable revision、attachment、relation、duplicate signal；
- 失败回滚、orphan journal、quarantine 和可见诊断；
- CLI human/JSON 结果，作为工程、恢复和自动化入口。

P3 不激活：

- 飞书、浏览器或其他平台的候选发现与用户选择；
- `queued/running/...` 集合会话和重收集状态机；
- 百炼、C1 derived/job、模型建议；
- Knowledge、Sublibrary、Output 的业务行为；
- 搜索产品、生成视图、loopback listener、worker、正式 Skill 和备份。

显式文件/导出 CLI 是底座验收与恢复路径，不得被描述为日常收集的正常产品体验。

## 3. 激活文件：29 个 Rust 文件

| Crate | 激活文件数 | P3 责任 |
| --- | ---: | --- |
| `01_babata_domain` | 6 | C0 标识、类型、值对象和不变量 |
| `02_babata_application` | 9 | capture/workspace DTO、port 和 use case |
| `03_babata_infrastructure` | 9 | 配置、路径、raw SQLite、资产和日志 |
| `04_babata_cli` | 5 | 命令映射、composition 和结果渲染 |
| **合计** | **29** | 不含 Cargo、SQL、测试和夹具 |

`05_babata_local_api`、`06_babata_worker` 和其他 P2 模块继续保持可编译的 unavailable
骨架，P3 不激活其真实行为。

依赖方向继续为：

```text
domain <- application <- infrastructure
       ^                ^
       +--- cli / local_api / worker composition roots ---+
```

## 4. Domain 激活子集：6 个文件

| 文件 | P3 公开责任 | 禁止 |
| --- | --- | --- |
| `src/lib.rs` | 模块导出 | 编排与 I/O |
| `src/ids.rs` | Item/Revision/Asset/Source/Collection opaque ID | 暴露数据库整数 ID |
| `src/kinds.rs` | Source/Revision/Content/Asset/Relation 状态 | provider 逻辑 |
| `src/entities.rs` | SourceRef、CollectionContext、RawItem、RawRevision、AssetRef、Relation | 文件/数据库操作 |
| `src/value.rs` | LogicalPath、Sha256、UtcTimestamp、Metadata、TextPayload、AssetInput | 读取文件或系统时钟 |
| `src/error.rs` | validation/conflict/not-found/integrity 领域错误 | SQL/provider 错误细节 |

Domain 测试与类型 owner 同文件维护。路径必须是逻辑相对路径，metadata 必须有边界，
原始文本和资产引用一旦 ready 就不原地修改。

## 5. Application 激活子集：9 个文件

| 文件 | P3 公开责任 | 禁止 |
| --- | --- | --- |
| `src/lib.rs` | use case 与 port 导出 | composition |
| `src/dto.rs` | text/file/export/create/revise/annotate 请求与结果 | transport/SQL 类型 |
| `src/error.rs` | domain/port 错误映射 | HTTP status |
| `src/ports/mod.rs` | P3 port 导出 | 具体实现 |
| `src/ports/raw_repository.rs` | source/item/revision/asset/relation 写入与详情读取 | SQLite 类型泄漏 |
| `src/ports/asset_store.rs` | stage/hash/finalise/discard/open/verify | SQL |
| `src/usecases/mod.rs` | service 导出 | 业务逻辑 |
| `src/usecases/capture.rs` | 显式 text/file/export 的共享 C0 提交流程 | 来源候选发现、SQLite、文件系统 |
| `src/usecases/workspace.rs` | first-party create/revise/annotate | 人工知识建模、SQLite、文件系统 |

P2 已扩展的 `RawRepositoryPort` 最终还会承载人工知识记录和子库定义；P3 只实现其中
原件、第一方和基础关系所需的子集，不用临时旁路为未来能力建立第二 repository。

## 6. Infrastructure 激活子集：9 个文件

| 文件 | P3 公开责任 | 禁止 |
| --- | --- | --- |
| `src/lib.rs` | infrastructure builder 导出 | 暴露可写 DB handle |
| `src/config.rs` | AppConfig、DataRoot、SqliteOptions | 业务规则 |
| `src/paths.rs` | 编号分区、暂存路径和防逃逸 | 来源判断 |
| `src/sqlite/mod.rs` | 打开 raw database、WAL/foreign keys/timeout | use case 决策 |
| `src/sqlite/migrate.rs` | migration ledger、版本与 checksum | 修改已应用迁移 |
| `src/sqlite/raw_repository.rs` | RawRepositoryPort SQL 与事务 | 资产文件最终落盘 |
| `src/assets/mod.rs` | FileAssetStore builder | SQL |
| `src/assets/file_store.rs` | stage/hash/finalise/discard/open/verify | 业务 ID/版本判断 |
| `src/observability.rs` | 脱敏 tracing、operation/journal 诊断 | 原始私密 payload 日志 |

只有 SQLite infrastructure 文件可以打开数据库；只有 FileAssetStore 可以最终落盘
C0 资产；两者都必须由 application 用例调用。

## 7. C0 migration 范围

```text
03_migrations/01_raw/
├── 0001_raw_schema.sql
├── 0002_raw_indexes.sql
├── 0003_raw_fts.sql
└── 0004_capture_operations.sql
```

`0001` 拥有 source、collection、item、revision、asset、relation 与 migration ledger；
`0002` 拥有 identity、version、time、hash 和 relation 索引；`0003` 若保留 FTS，只能是
可重建的早期读投影，不代表 P3 已激活搜索产品，也不能成为 C0 权威；`0004` 保存每次
进入 pending C0 的 operation 状态及 revision 级来源观测，使无资产失败和重导入仍可溯源。

P4 的 route evidence、collector session 或来源授权记录不得塞进 P3 migration 以伪装
P4 已经开始；它们由对应阶段和 C0/C3 责任单独设计。

关键不变量：

- opaque text ID；
- RFC 3339 UTC 时间；
- SHA-256 统一表示；
- metadata 是 JSON object 且有大小边界；
- revision/asset 有 pending、ready、quarantined 等明确状态；
- operation 与 revision 一一关联，ready/quarantined 状态同步提交；
- 同一 item 的版本顺序唯一；
- first-party 新建没有伪造 external identity；
- 原始 wording、原件 asset 和历史版本不覆盖。

## 8. CLI 激活子集：5 个文件

| 文件 | P3 责任 |
| --- | --- |
| `src/main.rs` | bootstrap、tracing 和 exit code |
| `src/app.rs` | config、repository、asset store、service 的 composition |
| `src/commands/mod.rs` | P3 命令注册与其他命令 unavailable |
| `src/commands/capture.rs` | capture/workspace 参数到 DTO 的映射 |
| `src/render.rs` | human/JSON 结果、错误与脱敏 |

P3 可以提供：

```text
babata data status
babata capture text
babata capture file
babata capture export
babata create
babata revise
babata annotate
```

这些命令不构成对外分布式协议，也不替代 P4 的上下文候选与用户选择。

## 9. 统一 C0 写入序列

```text
校验请求、来源/创作上下文和允许输入
  -> 分配 operation ID
  -> 在 04_runtime 建立 journal 与 staging
  -> 读取并计算哈希，生成逻辑最终路径
  -> 开启短事务
  -> 写入 pending source/item/revision/asset/relation graph
  -> 原子 finalise 或按哈希复用不可变资产
  -> 标记 ready 并提交
  -> 删除 journal
  -> 通过 repository read-back 返回 RecordDetail
```

跨文件系统与数据库无法真正原子时，使用经过测试的补偿事务。任何失败都不能报告
缺少原件的 ready revision；已 finalise 但未 commit 的原件进入可诊断 orphan/
quarantine，不自动销毁。

duplicate 只产生信号或关系，不删除新的收集/导入事件。

## 10. P3 交付门槛

| Gate | 完成证据 |
| --- | --- |
| P3-G1 数据根 | 新临时数据根产生正确编号分区，Git 无真实运行数据 |
| P3-G2 C0 写入 | text、file、export 各形成可回读版本、原件、哈希和上下文 |
| P3-G3 First-party | create/revise/annotate 保留版本和独立批注关系 |
| P3-G4 故障完整性 | stage、transaction、finalise 的失败不会留下伪 ready；journal/orphan 可诊断 |
| P3-G5 单一写入 | CLI 只 composition；DB 和资产写入只有 infrastructure owner |
| P3-G6 回归 | P2 架构门继续通过，新增 P3 行为没有激活其他阶段能力 |

P3 为 AC-03、AC-06 和 AC-10 提供底座证据；AC-03 还需要 P5 的真实派生物，AC-10
还需要 P8 的一致备份恢复，不能在 P3 提前宣布这些产品验收全部完成。

## 11. 后续能力激活条件

| 能力 | P2 位置 | 激活条件 |
| --- | --- | --- |
| CollectorSession / 真实来源 | `collector.rs`、SourceAdapter、browser/local API | P3 C0 提交稳定，真实来源获得授权 |
| Derived / Process | C1 ports、process use case、processing providers | 一个真实清洗样本和隐私范围获批准 |
| Knowledge | knowledge domain/use case | 原件与 C1 聚合读取稳定，核心工作例明确 |
| Explore / Sublibrary / Output | read projection、sublibrary/output use case 和 builders | 核心人工资料存在并有真实检索/输出用途 |
| Skill / Agent | specs、Capability registry | 对应底层能力通过自己的 AC/TC |
| Backup | Ops/BackupDriver | 真实 C0 需要受保护恢复 |

每次激活先更新对应架构补充、开发流程和测试映射，再把 unavailable 壳替换为真实实现。

## 12. 禁止模式

```text
infrastructure 之外打开 SQLite 或 finalise 资产
CLI 参数解析、业务规则与 SQL 混在一个文件
first-party 修改原地覆盖旧版本
导出/文件命令被描述成所有来源的正常日常体验
FTS 或视图成为第二权威
JS/Python 获得数据根最终路径或数据库写权限
为了已存在代码提前宣告 P3、P4 或产品 AC 完成
```

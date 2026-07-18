# Babata P2 全系统骨架蓝图

## 1. P2 的目的

P2 在任何单一业务模块深入实现前，为 00–03 已经确认的所有能力建立明确位置、
所有权、依赖方向、内部接口、调用入口、测试位置和激活阶段。

P2 不是产品功能验收。它不证明真实来源能收集、百炼能处理、核心区能形成沉淀，
也不证明视图或备份可用。P2 只证明系统没有因当前优先级而漏掉收集、清洗、核心、
输出中的任何一段，也没有产生第二写入者或反向依赖。

P2 允许：

- 类型、DTO、trait、use case 壳和 composition root；
- CLI 命令模块、local API 路由模块、worker 生命周期；
- adapter/provider/builder descriptor 和配置类型；
- 统一的 `capability_unavailable`；
- 架构、契约、清单与边界测试。

P2 不允许：

- 真实平台登录、抓取或静默收集；
- OCR、转写、模型推理、搜索排序、知识判断和输出模板算法；
- 空 Skill、空接口或 provider 壳对外声称已支持；
- 为保持旧文件数量而漏掉新 PRD/架构已经确认的能力。

## 2. 全局模块图

```text
来源上下文 / 窄本地 UI / 浏览器扩展      CLI / Skill / scheduler
                    |                               |
                    +--------- composition roots --+
                                      |
                              application use cases
                                      |
       +------------------------------+------------------------------+
       |               |               |              |               |
 CollectorSession   Capture/Process  Workspace/Knowledge  Explore/Output  Ops
       |               |               |              |               |
       +--------------------- application ports ---------------------+
                                      |
                              infrastructure adapters
                                      |
                   SQLite / assets / sources / providers / views / backup
```

所有 C0/C1 最终写入只允许经 application use case 调用 infrastructure。浏览器、
Python、Skill、provider 和 C2 builder 都不获得最终资产或数据库写权限。

## 3. 仓库级骨架

```text
Babata/
├── 00_docs/                       文档链与架构补充
├── 01_app/                        Rust Cargo workspace
├── 02_skills/                     不可用能力的规格；真实可用后才创建 Skill
├── 03_migrations/                 C0 / C1 / C3 迁移所有权
├── 04_tests/                      架构、契约、集成、端到端与夹具位置
├── 05_scripts/                    清单、依赖、所有权、追溯与写入边界检查
├── 06_config/                     可提交配置模板
├── 07_docs_assets/                文档图片和示意资源
└── 08_adapters/                   浏览器与例外 Python 边界
```

真实原件、第一方资料、数据库、派生物、视图、缓存、日志、凭据和运行状态全部位于
外部 `BABATA_DATA_HOME` 或受保护本地配置位置。

## 4. Rust workspace 目标：6 个 crate、137 个源文件

旧的 117 文件骨架没有覆盖核心沉淀、上下文收集、子库和广义输出。P2 目标在保留
现有文件的基础上增加 20 个责任文件，不通过删除或合并用户当前工作来凑数字。

| Crate | 目标 Rust 源文件 | P2 责任 |
| --- | ---: | --- |
| `01_babata_domain` | 16 | 全系统标识、状态、收集、知识、子库和输出领域类型 |
| `02_babata_application` | 30 | DTO、13 个 port、12 个 service |
| `03_babata_infrastructure` | 52 | C0/C1/C3、来源、处理、读投影、输出与备份位置 |
| `04_babata_cli` | 17 | 13 组自动化/恢复/运维命令模块 |
| `05_babata_local_api` | 16 | 受保护 local API 及真实本地调用者的路由位置 |
| `06_babata_worker` | 6 | 队列执行、租约、关闭和指标壳 |
| **合计** | **137** | 不含 Cargo manifest、SQL、测试、生成文件和外围适配器 |

依赖方向：

```text
domain <- application <- infrastructure
       ^                ^
       +--- cli / local_api / worker composition roots ---+
```

### 4.1 相对旧骨架新增的 20 个责任文件

| Crate | 新增文件 | 责任 |
| --- | --- | --- |
| domain | `collection.rs` | 候选、会话、逐条收集状态和重收集结果 |
| domain | `knowledge.rs` | 人工记录/判断/关系/分类/模型/评分/分析与模型建议决定 |
| domain | `sublibrary.rs` | 子库定义、成员选择和组织规则 |
| domain | `output.rs` | 输出类型、范围、manifest 和 build 状态 |
| application port | `read_projection.rs` | 从 C0/C1 构建与查询可重建读投影 |
| application port | `output_builder.rs` | 只读生成广义 C2 输出及 manifest |
| application use case | `collector.rs` | 发现候选、选择、逐条状态和重收集 |
| application use case | `knowledge.rs` | 人工沉淀与模型建议决定 |
| application use case | `sublibraries.rs` | 创建、修订、查看和物化子库定义 |
| application use case | `outputs.rs` | 列出、构建、查看和验证输出 |
| infrastructure | `sqlite/read_projection.rs` | 可重建读投影的 SQLite 实现位置 |
| infrastructure | `views/sublibrary.rs` | 子库 C2 物化实现位置 |
| infrastructure | `views/output.rs` | 报告/网页/结构化输出 builder 位置 |
| infrastructure | `views/manifest.rs` | 输出输入范围、版本和生成信息清单 |
| CLI | `commands/collector.rs` | 来源发现、选择、状态和重收集的运维/自动化入口 |
| CLI | `commands/knowledge.rs` | 人工沉淀的调试/自动化入口，不替代日常 UI |
| CLI | `commands/sublibraries.rs` | 子库定义和物化入口 |
| CLI | `commands/outputs.rs` | 广义输出与 manifest 入口 |
| local API | `routes/collector.rs` | 浏览器/窄 UI 的候选与选择路由位置 |
| local API | `routes/outputs.rs` | 真实本地输出调用者的路由位置 |

现有 `routes.rs` 继续拥有来源描述、覆盖证据与能力状态；新增 `collector.rs` 拥有
用户上下文会话。现有 `views.rs` 继续拥有视图类型和 C2 view build；新增
`outputs.rs` 拥有带范围与 manifest 的广义输出。责任不得再次合并成万能 service。

## 5. Domain 骨架：16 个文件

```text
01_babata_domain/src/
├── lib.rs
├── ids.rs
├── kinds.rs
├── entities.rs
├── value.rs
├── error.rs
├── capability.rs
├── route.rs
├── collection.rs
├── processing.rs
├── knowledge.rs
├── query.rs
├── sublibrary.rs
├── output.rs
├── view.rs
└── ops.rs
```

| 文件组 | 公开骨架 |
| --- | --- |
| `ids.rs` | item/revision/asset/source/collection/session/run/job/derivative/knowledge/sublibrary/output/snapshot 的 opaque ID |
| `kinds.rs` | source、revision、content、asset、relation、derivative、processing、knowledge、output 枚举 |
| `entities.rs` | SourceRef、RawItem、RawRevision、AssetRef、Relation 与稳定构造约束 |
| `collection.rs` | CandidateSummary、CollectionSelection、CollectionItemState、RecollectionState |
| `processing.rs` | ProcessRun、JobRef、DerivativeRef、ProviderTaskRef |
| `knowledge.rs` | KnowledgeRecord、ModelSuggestion、SuggestionDecision 与人工/机器来源 |
| `query.rs` | QueryFilter、PageCursor、RecordSummary、RecordDetail 聚合引用 |
| `sublibrary.rs` | SublibraryDefinition、include/exclude/organise 规则与版本 |
| `output.rs` | OutputKind、OutputScope、OutputBuild、OutputManifestRef |
| `view.rs` | 具体 C2 view descriptor，不拥有子库定义或输出权威 |
| `ops.rs` | 数据级别、健康、快照和恢复状态 |

Domain 不读取时钟、文件、环境变量或网络，不依赖 SQLite、HTTP、CLI、UI 和 provider。

## 6. Application 骨架：30 个文件

```text
02_babata_application/src/
├── lib.rs
├── dto.rs
├── error.rs
├── ports/
│   ├── mod.rs
│   ├── raw_repository.rs
│   ├── asset_store.rs
│   ├── derived_repository.rs
│   ├── job_repository.rs
│   ├── process_provider.rs
│   ├── source_adapter.rs
│   ├── candidate_runner.rs
│   ├── read_projection.rs
│   ├── view_builder.rs
│   ├── output_builder.rs
│   ├── backup_driver.rs
│   ├── capability_registry.rs
│   └── clock.rs
└── usecases/
    ├── mod.rs
    ├── collector.rs
    ├── capture.rs
    ├── workspace.rs
    ├── knowledge.rs
    ├── process.rs
    ├── explore.rs
    ├── sublibraries.rs
    ├── views.rs
    ├── outputs.rs
    ├── routes.rs
    ├── ops.rs
    └── capabilities.rs
```

### 6.1 12 个 service、46 个最低公开方法

| Service | 最低方法骨架 |
| --- | --- |
| CollectorSessionService | `start`、`candidates`、`select`、`status`、`recollect` |
| CaptureService | `capture_text`、`capture_file`、`capture_export`、`capture_candidate` |
| WorkspaceService | `create`、`revise`、`annotate` |
| KnowledgeService | `record`、`relate`、`classify`、`model`、`score`、`analyze`、`decide_suggestion` |
| ProcessService | `enqueue`、`run_once`、`status`、`retry`、`cancel`、`list_pipelines` |
| ExploreService | `search`、`show` |
| SublibraryService | `create`、`revise`、`show`、`materialize` |
| ViewService | `list`、`build` |
| OutputService | `list`、`build`、`status`、`verify` |
| RouteService | `list`、`show`、`evaluate`、`collect` |
| OpsService | `status`、`doctor`、`backup`、`restore_verify` |
| CapabilityService | `list` |

`CaptureService` 只负责已取得内容的 C0 提交；`CollectorSessionService` 负责写入前的
候选、选择和逐条状态。`WorkspaceService` 负责第一方正文版本；`KnowledgeService`
负责核心区人工沉淀。`ViewService` 负责具体展示 view；`OutputService` 负责带 manifest
的广义输出。

### 6.2 13 个 port、57 个最低接口方法

| Port | 最低方法数 | 责任 |
| --- | ---: | --- |
| RawRepositoryPort | 8 | C0 来源、资料、版本、第一方与人工知识记录的事务边界 |
| AssetStorePort | 6 | stage/hash/finalise/discard/open/verify |
| DerivedRepositoryPort | 6 | C1 process run、derivative 和 model suggestion |
| JobRepositoryPort | 7 | enqueue/claim/heartbeat/complete/fail/retry/cancel |
| ProcessProviderPort | 6 | describe/prepare/submit/poll/cancel/fetch |
| SourceAdapterPort | 4 | describe/discover/collect/coverage |
| CandidateRunnerPort | 2 | 受控外围子进程与候选验证 |
| ReadProjectionPort | 5 | rebuild/search/show/traverse/status |
| ViewBuilderPort | 3 | describe/build/verify 具体 view |
| OutputBuilderPort | 4 | describe/build/status/verify 广义输出 |
| BackupDriverPort | 3 | snapshot/restore/verify |
| CapabilityRegistryPort | 2 | list/get |
| ClockPort | 1 | now |

Application 不导入 SQLite、文件系统、HTTP、provider SDK、进程执行和 transport 类型。

## 7. Infrastructure 骨架：52 个文件

原 48 个 infrastructure 文件全部保留，新增以下 4 个位置：

```text
03_babata_infrastructure/src/
├── sqlite/read_projection.rs
└── views/
    ├── sublibrary.rs
    ├── output.rs
    └── manifest.rs
```

现有目录责任继续保持：

- `sqlite/`：C0/C1/C3 repository 和可重建读投影实现；
- `assets/`：唯一资产暂存、哈希与最终落盘实现；
- `sources/`：来源 descriptor、只读候选发现与被选内容读取；
- `processing/`：本地提取、百炼 CLI/API provider 壳；
- `views/`：Datasette、Obsidian、子库物化、广义输出与 manifest；
- `backup/`：一致快照、Restic 与清单；
- `tools/`：受控外部命令；
- `security/`：凭据和隐私策略。

P2 中 provider、builder 和工具文件只返回 descriptor/config/unavailable，不登录来源、
不调用模型、不产生真实输出。SQLite repository 与 FileAssetStore 是唯一持久化实现
位置，但必须由 application 用例调用。

## 8. CLI、local API 与 worker 骨架

### 8.1 CLI：17 个文件、13 组命令模块

现有 9 组 `data`、`capabilities`、`capture`、`workspace`、`process`、`explore`、
`views`、`routes`、`ops`，新增：

```text
commands/collector.rs
commands/knowledge.rs
commands/sublibraries.rs
commands/outputs.rs
```

CLI 是自动化、恢复、诊断、运维和底层调试入口，不是正常日常收集的默认产品界面。
P2 命令模块可以编译并报告 capability，但不得要求所有命令在 P2 完成真实业务。

### 8.2 Local API：16 个文件，不固定 endpoint 数量

新增：

```text
routes/collector.rs
routes/outputs.rs
```

`routes/workspace.rs` 可以映射 first-party 与 knowledge 调用，`routes/explore.rs` 可以
映射 search 与 sublibrary 读取，`routes/outputs.rs` 映射真实输出调用者。

P2 固定鉴权、loopback-only、请求/响应映射边界和路由模块所有权，不把“12 个 endpoint”
作为门槛。只有浏览器扩展或窄本地 UI 出现真实调用时，才固定对应 endpoint；未被真实
调用的路由不因占位存在而成为支持承诺。

### 8.3 Worker：6 个文件

继续保留 `build`、`run`、`claim_once`、`heartbeat`、`shutdown` 生命周期壳。P2 不领取
真实任务；worker 只能调用 application use case，不直接执行业务写入。

## 9. 外围适配器骨架

### 9.1 浏览器扩展

```text
08_adapters/01_browser_extension/
├── package.json
├── tsconfig.json
├── manifest.json
└── src/
    ├── index.ts
    ├── capture.ts
    ├── transport.ts
    └── types.ts
```

P2 定义当前页面/选区/链接/书签候选、明确选择、配对和 transport 类型；不抓取真实
网站、不写数据根、不存最终原件。

### 9.2 Python 例外桥

```text
08_adapters/02_python_bridge/
├── pyproject.toml
└── src/babata_adapter/
    ├── __init__.py
    ├── runner.py
    └── envelope.py
```

只定义受控子进程和候选/处理结果边界，不绑定具体 Python 工具，不拥有 SQLite 或最终
资产路径。

## 10. Skill、迁移、测试、脚本和配置位置

### 10.1 Skill 规格

现有 6 份规格继续保留，新增：

```text
02_skills/00_specs/
├── 07_knowledge.md
├── 08_sublibraries.md
└── 09_outputs.md
```

规格只定义目标能力、输入、输出、权限、失败和激活门；真实 `SKILL.md` 在底层能力
通过对应验收后创建。

### 10.2 迁移位置

```text
03_migrations/
├── 00_REGISTRY.md
├── 01_raw/       C0：外部原件、第一方、人工知识、关系与子库定义
├── 02_derived/   C1：处理运行、派生物和模型建议
└── 03_runtime/   C3：队列、租约、会话和运行状态
```

P2 固定所有者、命名和依赖；不要求写完后续阶段 SQL。

### 10.3 测试与工程检查

```text
04_tests/
├── 01_architecture/
├── 02_contract/
├── 03_integration/
├── 04_end_to_end/
└── 05_fixtures/

05_scripts/
├── check-p2-skeleton-inventory.ps1
├── check-rust-boundaries.ps1
├── check-interface-ownership.ps1
├── check-doc-traceability.ps1
└── check-no-secondary-writer.ps1
```

P2 的架构与契约测试只证明边界和壳存在；产品 TC 在对应功能阶段运行。

### 10.4 配置模板

数据根、应用、来源、provider、pipeline、view、隐私和备份模板全部保留。模板不含
凭据，未激活能力默认关闭。

## 11. 工具所有权

| 工具 | 用途 | 所有者/边界 | 激活阶段 |
| --- | --- | --- | --- |
| Cargo、rustfmt、clippy、cargo metadata | Rust 构建和边界检查 | 工程工具 | P2 |
| PowerShell 检查 | 清单、依赖、所有权、追溯、第二写入者 | `05_scripts` | P2 |
| `babata` CLI | 自动化、恢复、诊断和运维 | Rust CLI | P2 壳；逐项激活 |
| Browser Extension APIs | 浏览器候选、选择和配对 | JS 边界 | P4 |
| 飞书 OpenAPI | 飞书候选发现和被选内容读取 | SourceAdapter | P4 |
| 百炼 CLI / API | 多模态处理 | ProcessProvider | P5 |
| Datasette / Obsidian | 可重建 C2 阅读视图 | ViewBuilder | P6 |
| `yt-dlp` | 获授权媒体候选/输入 | SourceAdapter/CommandRunner | P7，或真实需求提前证明后 |
| Restic | 加密增量备份 | BackupDriver | P8 |

## 12. P2 交付门槛

P2 使用工程交付门，不使用产品 AC/TC 冒充骨架验收：

| Gate | 完成证据 |
| --- | --- |
| P2-G1 清单完整 | 6 crate、137 个目标 Rust 源文件和外围规格位置全部存在 |
| P2-G2 责任完整 | 12 service、13 port、13 CLI 模块、local API/worker/adapter 位置均有唯一 owner |
| P2-G3 依赖正确 | workspace 编译，domain/application 无 IO 反向依赖，composition root 无业务规则 |
| P2-G4 能力诚实 | 未激活能力统一 unavailable，不启动真实来源/provider/worker 或生成真实视图 |
| P2-G5 单一写入 | JS、Python、Skill、provider、view/output builder 均无 C0/C1 直接写入路径 |
| P2-G6 文档同步 | 00–05 与蓝图、脚本、配置和测试位置没有旧编号或旧责任冲突 |
| P2-G7 工具路线 | `08_SOURCE_TOOL_RESEARCH.md` 覆盖 00 列出的全部来源；每条来源都有证据等级、最小授权、直接使用/薄包装/窄适配/回退决策和诚实缺口；可在当前机器调用的代表性官方/通用工具有实际调用或连接证据 |

现有 raw capture 代码可以作为 P3 提前工作保留，但它不能替代任何 P2 gate，也不能
证明 P3 完成。137 文件、接口和工程检查全部通过后，如果逐来源调查、路线决策或代表性
工具实证仍缺失，P2 仍然只能标记进行中。P2-G7 不要求每个具体来源先达到 E3；真实候选、
正文/附件、逐条状态和重收集属于 P4/P7，不能反过来成为进入 P3 的前置条件。

## 13. P2 完成后的激活顺序

- P3：C0 原件与 first-party 版本的统一写入底座；
- P4：飞书与浏览器上下文候选、选择、状态和重收集；
- P5：C1 多模态清洗与百炼；
- P6：核心人工沉淀、检索、子库和输出；
- P7：扩展来源、正式 Skill 和受控 Agent；
- P8：备份、恢复、运维与长期加固。

具体状态和阶段门只由 `04_DEVELOPMENT_PROCESS.md` 维护。

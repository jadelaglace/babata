# Babata P2 全系统骨架蓝图

本文定义 P2 必须一次性建立的完整工程骨架。P2 的目标不是优先做完某一个功能，而是让所有已知子模块都有明确位置、所有权、依赖方向、公开接口、工具入口和后续激活阶段。

P2 可以保留已经存在的原始入库实现，但不以该模块的功能验收作为完成标准。新增骨架不得实现平台抓取、模型处理、搜索排序、备份策略等具体业务算法；允许提供类型、trait、DTO、命令树、路由树、构造器和统一的 `CapabilityUnavailable` 返回。

## 1. 全局模块图

```text
人 / Agent / Skill / shell / scheduler
                    |
                 babata CLI
                    |
 browser adapter -> loopback API ----+
                    |                 |
                    +------ application services
                                  |
             +--------------------+--------------------+
             |                    |                    |
          domain               port traits       capability registry
                                  |
             +--------------------+--------------------+
             |                    |                    |
       infrastructure         local worker        generated views
             |
   SQLite / assets / source adapters / Bailian / external tools / backup
```

所有持久化写入最终仍只允许经过 Rust application use case 调用 infrastructure。P2 只建立这条结构，不要求每条业务链真实运行。

## 2. 仓库级骨架

```text
Babata/
├── 00_docs/                       文档链和架构蓝图
├── 01_app/                        Rust Cargo workspace
├── 02_skills/                     Skill 规格；可用后才激活 SKILL.md
├── 03_migrations/                 raw / derived / runtime 迁移位置
├── 04_tests/                      架构、契约、集成、端到端测试
├── 05_scripts/                    边界、清单、追踪和写入权检查
├── 06_config/                     可提交的配置模板
├── 07_docs_assets/                文档图片和示意资源
└── 08_adapters/                   浏览器与例外 Python 边界适配器
```

真实内容、SQLite、模型产物、缓存、凭据和日志仍位于外部 `BABATA_DATA_HOME`，不进入这个仓库。

## 3. Rust workspace：6 个 crate、117 个源文件

| Crate | Rust 源文件 | P2 责任 |
| --- | ---: | --- |
| `01_babata_domain` | 12 | 全系统稳定类型、状态、标识和约束 |
| `02_babata_application` | 24 | DTO、11 个 port、8 个 service |
| `03_babata_infrastructure` | 48 | 所有持久化、来源、处理、视图、工具和备份适配位置 |
| `04_babata_cli` | 13 | 9 组命令与统一输出 |
| `05_babata_local_api` | 14 | loopback API、鉴权和路由映射 |
| `06_babata_worker` | 6 | 队列执行壳、租约、关闭和指标 |
| **合计** | **117** | 不含 Cargo manifest、SQL、测试和外围适配器 |

依赖方向固定为：

```text
domain <- application <- infrastructure
       ^                ^
       +---- cli / local_api / worker ----+
```

### 3.1 `01_babata_domain`：12 个文件

```text
src/
├── lib.rs
├── ids.rs
├── kinds.rs
├── entities.rs
├── value.rs
├── error.rs
├── capability.rs
├── route.rs
├── processing.rs
├── query.rs
├── view.rs
└── ops.rs
```

| 文件 | 公开骨架 |
| --- | --- |
| `ids.rs` | Item/Revision/Asset/Source/Collection/Run/Job/Derivative/View/Snapshot ID |
| `kinds.rs` | source、revision、content、asset、relation、derivative、processing 枚举 |
| `entities.rs` | SourceRef、RawItem、RawRevision、AssetRef、Relation |
| `value.rs` | LogicalPath、Sha256、UtcTimestamp、Metadata、TextPayload、AssetInput |
| `capability.rs` | CapabilityId、CapabilityStatus、CapabilityDescriptor |
| `route.rs` | SourceRouteId、SourceRouteDescriptor、RouteCoverage、CandidateEnvelope |
| `processing.rs` | PipelineId、ProcessRun、JobRef、DerivativeRef、ProviderTaskRef |
| `query.rs` | QueryFilter、PageCursor、RecordSummary |
| `view.rs` | ViewKind、ViewDescriptor、BuildTarget |
| `ops.rs` | BackupClass、HealthState、SnapshotRef、RestoreState |

Domain 不读时钟、文件、环境变量或网络，不依赖 SQLite、HTTP、CLI 和 provider SDK。

### 3.2 `02_babata_application`：24 个文件

```text
src/
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
│   ├── view_builder.rs
│   ├── backup_driver.rs
│   ├── capability_registry.rs
│   └── clock.rs
└── usecases/
    ├── mod.rs
    ├── capture.rs
    ├── workspace.rs
    ├── process.rs
    ├── explore.rs
    ├── views.rs
    ├── routes.rs
    ├── ops.rs
    └── capabilities.rs
```

8 个 service 共定义 26 个公开方法：

| Service | 方法骨架 |
| --- | --- |
| CaptureService | `capture_text`、`capture_file`、`capture_export`、`capture_candidate` |
| WorkspaceService | `create`、`revise`、`annotate` |
| ProcessService | `enqueue`、`run_once`、`status`、`retry`、`cancel`、`list_pipelines` |
| ExploreService | `search`、`show` |
| ViewService | `list`、`build` |
| RouteService | `list`、`show`、`evaluate`、`collect` |
| OpsService | `status`、`doctor`、`backup`、`restore_verify` |
| CapabilityService | `list` |

11 个 port 共定义 48 个接口方法。P2 只固定签名和所有权：

| Port | 方法数量 | 责任 |
| --- | ---: | --- |
| RawRepositoryPort | 8 | source/context/item/revision/relation 读写边界 |
| AssetStorePort | 6 | stage/hash/finalise/discard/open/verify |
| DerivedRepositoryPort | 6 | process run 与 derivative 记录 |
| JobRepositoryPort | 7 | enqueue/claim/heartbeat/complete/fail/retry/cancel |
| ProcessProviderPort | 6 | describe/prepare/submit/poll/cancel/fetch |
| SourceAdapterPort | 4 | describe/probe/collect/coverage |
| CandidateRunnerPort | 2 | run/validate envelope |
| ViewBuilderPort | 3 | describe/build/verify |
| BackupDriverPort | 3 | snapshot/restore/verify |
| CapabilityRegistryPort | 2 | list/get |
| ClockPort | 1 | now |

Application 不导入 SQLite、文件系统、HTTP、provider SDK 或进程执行库。

### 3.3 `03_babata_infrastructure`：48 个文件

```text
src/
├── lib.rs
├── config.rs
├── paths.rs
├── observability.rs
├── capabilities.rs
├── sqlite/
│   ├── mod.rs
│   ├── migrate.rs
│   ├── raw_repository.rs
│   ├── derived_repository.rs
│   └── job_repository.rs
├── assets/
│   ├── mod.rs
│   └── file_store.rs
├── sources/
│   ├── mod.rs
│   ├── registry.rs
│   ├── candidate.rs
│   └── providers/
│       ├── mod.rs
│       ├── feishu.rs
│       ├── yuque.rs
│       ├── onenote.rs
│       ├── evernote.rs
│       ├── wechat.rs
│       ├── zhihu.rs
│       ├── bilibili.rs
│       ├── xiaohongshu.rs
│       ├── douyin.rs
│       ├── browser.rs
│       ├── conversations.rs
│       ├── local_files.rs
│       └── first_party.rs
├── processing/
│   ├── mod.rs
│   ├── registry.rs
│   ├── local_extract.rs
│   ├── bailian_cli.rs
│   └── bailian_api.rs
├── views/
│   ├── mod.rs
│   ├── datasette.rs
│   ├── obsidian.rs
│   └── exports.rs
├── backup/
│   ├── mod.rs
│   ├── sqlite_snapshot.rs
│   ├── restic.rs
│   └── manifest.rs
├── tools/
│   ├── mod.rs
│   ├── command_runner.rs
│   └── yt_dlp.rs
└── security/
    ├── mod.rs
    ├── secrets.rs
    └── privacy.rs
```

P2 中各 provider 文件只提供 descriptor、配置类型和 `CapabilityUnavailable` 壳；不登录平台、不抓取、不调用模型。SQLite、FileAssetStore 仍是唯一持久化实现位置。

### 3.4 `04_babata_cli`：13 个文件

```text
src/
├── main.rs
├── app.rs
├── render.rs
└── commands/
    ├── mod.rs
    ├── data.rs
    ├── capabilities.rs
    ├── capture.rs
    ├── workspace.rs
    ├── process.rs
    ├── explore.rs
    ├── views.rs
    ├── routes.rs
    └── ops.rs
```

9 组命令为 `data`、`capabilities`、`capture`、`workspace`、`process`、`explore`、`views`、`routes`、`ops`。P2 注册完整命令树；未激活能力返回机器可读的 `capability_unavailable`，不伪装成功。

### 3.5 `05_babata_local_api`：14 个文件

```text
src/
├── lib.rs
├── main.rs
├── app.rs
├── state.rs
├── auth.rs
├── error.rs
├── requests.rs
├── responses.rs
└── routes/
    ├── mod.rs
    ├── capture.rs
    ├── workspace.rs
    ├── process.rs
    ├── explore.rs
    └── health.rs
```

P2 定义 12 个 endpoint 的请求/响应映射；服务默认 disabled，只绑定 loopback，且不在 P2 启动真实 listener。

### 3.6 `06_babata_worker`：6 个文件

```text
src/
├── main.rs
├── app.rs
├── runner.rs
├── lease.rs
├── shutdown.rs
└── metrics.rs
```

公开壳为 `build`、`run`、`claim_once`、`heartbeat`、`shutdown`。P2 不执行真实队列任务。

## 4. 外围适配器骨架

### 4.1 浏览器适配器

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

仅定义 DOM capture payload、候选包类型和 loopback transport；P2 不抓取真实网站。

### 4.2 Python 例外适配器

```text
08_adapters/02_python_bridge/
├── pyproject.toml
└── src/babata_adapter/
    ├── __init__.py
    ├── runner.py
    └── envelope.py
```

只定义版本化子进程协议和 CandidateEnvelope，不绑定任何 Python-only 工具。Python 永远没有 SQLite 写权限。

## 5. Skill、迁移、测试、脚本和配置骨架

### 5.1 Skill 规格

P2 创建不可激活的规格文件，不提前发布空 Skill：

```text
02_skills/00_specs/
├── 01_capture.md
├── 02_process.md
├── 03_workspace.md
├── 04_explore.md
├── 05_routes.md
└── 06_ops.md
```

每份规格定义目标 CLI、输入、输出、权限、错误和激活阶段。真实 `SKILL.md` 仍在命令通过功能验收后创建。

### 5.2 数据迁移位置

```text
03_migrations/
├── 00_REGISTRY.md
├── 01_raw/
├── 02_derived/
└── 03_runtime/
```

P2 固定目录、数据库所有者和迁移命名规则；不要求完成 P3 之后的 SQL。

### 5.3 测试骨架

```text
04_tests/
├── 01_architecture/
├── 02_contract/
├── 03_integration/
├── 04_end_to_end/
└── 05_fixtures/
```

P2 只要求架构和契约测试可运行；功能 fixture 和端到端数据在对应阶段补充。

### 5.4 工程脚本

```text
05_scripts/
├── check-p2-skeleton-inventory.ps1
├── check-rust-boundaries.ps1
├── check-interface-ownership.ps1
├── check-doc-traceability.ps1
└── check-no-secondary-writer.ps1
```

### 5.5 配置模板

```text
06_config/
├── data-root.example.toml
├── app.example.toml
├── routes.example.toml
├── providers.example.toml
├── pipelines.example.toml
├── views.example.toml
├── privacy.example.toml
└── backup.example.toml
```

模板没有凭据，所有未激活能力默认关闭。

## 6. 工具清单与所有权

| 工具 | 用途 | 所有者/边界 | 激活阶段 |
| --- | --- | --- | --- |
| Cargo、rustfmt、clippy、cargo metadata | Rust 构建和边界检查 | 工程工具 | P2 |
| PowerShell 验证脚本 | 文件清单、追踪、第二写入者检查 | `05_scripts` | P2 |
| `babata` CLI | 人、Skill、脚本的默认入口 | Rust CLI | P2 骨架；后续逐项激活 |
| Browser Extension APIs | 浏览器 DOM 和交互 | JS 边界适配器 | P4 |
| Bailian `bl` CLI | 交互式多模态处理 | ProcessProvider | P5 |
| Bailian/Qwen API | 队列和批处理 | ProcessProvider | P5+ |
| Datasette | 本地只读检查与查询视图 | ViewBuilder | P6 |
| Obsidian | 可重建阅读视图 | ViewBuilder | P6 |
| `yt-dlp` | 获授权媒体获取候选 | SourceAdapter/CommandRunner | P4/P7 |
| Restic | 加密增量备份 | BackupDriver | P8 |
| Git/gh | 代码和文档协作 | 仓库运维，不承载真实数据 | 全程 |

## 7. P2 完成标准

P2 只在以下条件同时满足时完成：

1. 本文列出的仓库目录、6 个 crate、117 个 Rust 源文件和外围骨架全部存在。
2. 所有 service、port、命令组、API 路由、worker 生命周期和工具适配点有唯一所有者。
3. Cargo workspace 全量 `cargo check` 通过；API 和 worker 默认禁用，不开启真实外部行为。
4. 未激活功能统一返回 `capability_unavailable`，不存在伪成功、隐式落盘或直接外部调用。
5. 五个 P2 工程检查脚本通过：清单、依赖方向、接口所有权、文档追踪、第二写入者。
6. 每个模块写明后续激活阶段和对应 AC/TC；P2 不要求任何单一业务模块完成真实功能验收。
7. 已经存在的原始入库代码被纳入正确层次，但其功能正确性仍归 P3 验收，不提前宣告完成。

## 8. P2 明确不做

- 不以飞书、浏览器、音视频、百炼、搜索、视图或备份中的任一条真实链路作为 P2 门槛。
- 不实现平台登录、抓取、解析、OCR、转写、模型推理、搜索排序、导出模板或备份保留算法。
- 不为追求“全”而复制业务规则；骨架只定义边界、类型、签名、依赖和激活状态。
- 不让空 Skill、空 API 或空 provider 对外声称已支持。

完成 P2 后，P3 才集中完成并验收原始资料入库与不可变存储；其详细设计见 `05_RAW_FOUNDATION_BLUEPRINT.md`。

# Babata Development Process

本文是 Babata 唯一的实时进度和交付顺序来源。需求、PRD、验收标准与架构定义“为什么、做什么、怎样算完成、边界在哪里”；本文只维护“现在到哪、下一步做什么、哪道门通过后才能进入下一阶段”。

## 1. 当前状态

**更新时间：2026-07-16**

```text
P0  冻结旧版本                                    已完成
P1  需求、PRD、验收、全局技术架构                   已完成
P2  全系统模块与代码骨架                            当前阶段：文档已定义，骨架尚未实施
P3  原始资料入库与不可变存储                        延后；已有早期实现，不视为完成
P4  首批真实收集路径                                未开始
P5  多模态清洗与百炼处理                            未开始
P6  检索、视图、输出与子库                          未开始
P7  扩展来源、Skill 与 Agent 自动化                 未开始
P8  备份、恢复、运维与长期加固                      未开始
```

现有 29 个原始入库 Rust 文件是提前出现的 P3 工作。它们保留在工作区，但不再驱动当前路线，也不能代替 P2 的全系统骨架。P2 必须先把其余模块、文件、接口和工具壳补齐，再回到 P3 做原始入库功能验收。

项目阶段只使用 P0–P8。数据备份等级单独使用 C0–C3。

## 2. 进度维护规则

1. 本节是唯一实时状态；阶段变化时必须与对应文档/代码证据在同一提交中更新。
2. 状态只使用“未开始、进行中、已完成、阻塞”，并写明完成证据和下一道门。
3. 已经提前写出的代码只记为“提前工作”，不得反向篡改阶段顺序。
4. 单个模块跑通不再自动推动全局阶段；必须满足当前阶段的全局完成标准。
5. 对产品范围的修改按 `Requirements -> PRD -> Acceptance -> Architecture -> Development Process -> Test Cases` 顺序更新。
6. `AGENTS.md` 只记录本地协作，不是产品或进度权威。

## 3. P2：全系统模块与代码骨架

### 3.1 P2 目的

在任何单一模块深入开发前，先建立完整、可导航、可编译、边界清楚的系统架子：

- 全部 Rust crate 和子模块目录；
- 全部计划文件及唯一职责；
- domain 类型、application DTO/service/port 签名；
- infrastructure 中来源、处理、视图、备份、工具适配位置；
- CLI 命令树、loopback API 路由树、worker 生命周期；
- 浏览器/Python 边界适配器；
- Skill 规格、迁移目录、测试目录、工程脚本和配置模板；
- 每个能力的当前状态与未来激活阶段。

P2 不追求任何一个模块的业务闭环，不实现平台抓取、模型处理、搜索、导出或备份算法。详细文件清单和接口定义以 `04_SYSTEM_SKELETON_BLUEPRINT.md` 为准。

### 3.2 P2.1：文档与清单基线

1. 固定 6 个 Rust crate、117 个 Rust 源文件及外围目录清单。
2. 固定 8 个 application service、11 个 port、26 个 service 方法和 48 个 port 方法。
3. 固定 9 组 CLI、12 个 API endpoint、worker 生命周期和工具注册表。
4. 给每个文件写明 owner、允许依赖、禁止依赖、激活阶段和测试位置。
5. 将现有原始入库代码标记为 P3 提前实现，不删除、不提前验收。

完成证据：架构、骨架蓝图、AC-11 和 TC-11 对同一清单无冲突。

### 3.3 P2.2：仓库和 Cargo 骨架

1. 创建 6 个 Cargo crate 及全部 module 文件。
2. 创建 `08_adapters`、Skill 规格、迁移、测试、脚本和配置模板目录。
3. 所有未激活能力使用统一 `CapabilityStatus` 和 `capability_unavailable` 错误。
4. Cargo features 只控制是否编译/启用外围能力，不改变 domain/application 规则。

完成证据：`cargo metadata` 显示完整 workspace，`cargo check --workspace` 通过。

### 3.4 P2.3：接口和组合根骨架

1. 定义 domain 全局类型，但不加入 IO 或 provider 逻辑。
2. 定义全部 DTO、service 与 port 签名。
3. CLI 注册完整命令树；API 注册完整路由映射；worker 建立生命周期壳。
4. Infrastructure 每个 adapter 只暴露 descriptor、配置和 unavailable 壳。
5. Composition root 能构造 capability registry，但默认不启动 API、worker 或外部工具。

完成证据：接口所有权检查通过，所有命令/路由能报告真实 capability 状态。

### 3.5 P2.4：工程工具和全局边界

建立并通过：

```text
check-p2-skeleton-inventory.ps1
check-rust-boundaries.ps1
check-interface-ownership.ps1
check-doc-traceability.ps1
check-no-secondary-writer.ps1
```

这些检查证明文件齐全、依赖单向、接口只有一个 owner、文档可追踪、Rust core 仍是唯一写入者。它们不测试业务算法。

### 3.6 P2 完成门

P2 完成必须同时满足：

- `04_SYSTEM_SKELETON_BLUEPRINT.md` 的全部目录、文件和接口存在；
- 6 个 crate 与外围边界都可被工具识别；
- workspace 编译、格式化和架构检查通过；
- 未激活能力显式 unavailable，不伪装支持；
- 没有新增业务算法、真实 provider 调用或第二持久化写入者；
- 不以原始入库、飞书、百炼或任何单一模块的功能验收作为 P2 完成条件。

## 4. P3：原始资料入库与不可变存储

P3 才接回原来的“原始资料入库底座”工作。29 个活跃实现文件见 `05_RAW_FOUNDATION_BLUEPRINT.md`；SQL 所有权、写入顺序、fixtures、测试和命令验收见 `06_RAW_FOUNDATION_EXECUTION_PLAN.md`。

P3 工作包括：

1. 将现有提前实现与 P2 全系统接口统一；
2. 完成 text/file/export 与 first-party create/revise/annotate；
3. 完成 raw SQLite、原件资产、哈希、溯源、失败补偿和 read-back；
4. 使用临时数据根完成命令级验收；
5. 不接真实平台、不做多模态处理、不做视图。

P3 完成后，才能把“资料可靠入库”标记为可用。

## 5. P4：首批真实收集路径

P4 证明来源模块不是空架子，首批只做两条真实路径：

1. 飞书文档/Wiki/知识库：优先官方导出或 OpenAPI，记录标题、层级、原链接、附件、native identity、重导行为和缺失字段。
2. 浏览器书签/页面/剪藏：优先 HTML 导出或浏览器扩展候选包；确有需要才启用 loopback API。

来源 adapter 只能生成 CandidateEnvelope 或调用 CaptureService，不能写 SQLite。

## 6. P5：多模态清洗与百炼处理

依次激活：

1. derived schema、job、process run 和 derivative 存储；
2. 本地机械文本/文档提取；
3. Bailian CLI 的一个真实 pipeline；
4. 图片 OCR、音频转写、视频字幕/关键帧/视觉描述；
5. Bailian/Qwen API 或批处理，以及重试、成本、隐私和限流。

原件、原文和原哈希永不被覆盖。

## 7. P6：检索、视图、输出与子库

激活 ExploreService、ViewService、Datasette、Obsidian 和导出 builder：

- 查询 raw 与 derived；
- 展示来源、版本、处理链和附件；
- 生成可删除重建的阅读视图；
- 输出分类子库、报告或供其他应用消费的快照；
- 下游视图不得反写权威数据。

## 8. P7：扩展来源、Skill 与 Agent 自动化

按实际价值逐条扩展飞书以外的来源：语雀、OneNote、印象笔记、微信、知乎、Bilibili、小红书、抖音、浏览器、豆包/Kimi/GPT、local files 和 first-party。

每条路径遵循：官方导出/API -> 成熟 CLI/SDK/开源工具 -> 浏览器适配 -> 窄适配器 -> 手动截图/PDF/录屏。

P2 中的 Skill 规格在对应 CLI 真实通过验收后才转成可用 `SKILL.md`。Agent 自动触发默认保持人工确认，可在积累到一定程度后手动批处理。

## 9. P8：备份、恢复、运维与长期加固

实现 SQLite-consistent snapshot、Restic 加密增量备份、隔离恢复验证、完整性抽样、日志轮转、doctor、成本监控和故障恢复。

C0 原始/first-party 数据优先于 C1 derived；C2 views 和 C3 runtime/logs 可重建。

## 10. 提交与验收纪律

- P2 的提交按“文档/清单 -> crate/目录 -> 接口 -> composition roots -> 工程检查”分组，不混入业务算法。
- P3 以后每次功能提交引用对应 AC/TC。
- 功能阶段若发现骨架接口错误，先更新架构与 P2 蓝图，再改代码。
- 当前文档更改完成并 push 后，下一步是实施 P2.2，而不是继续验收现有原始入库模块。

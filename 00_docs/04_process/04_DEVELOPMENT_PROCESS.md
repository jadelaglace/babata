# Babata 开发流程与实时进度

本文是 Babata 唯一的实时阶段状态和交付顺序来源。00–03 定义为什么做、做什么、
怎样算产品完成以及技术边界；架构补充定义文件和阶段设计；本文只维护现在到哪、
下一步是什么、通过哪道交付门才能进入下一阶段。

## 1. 当前状态

**更新时间：2026-07-17**

```text
P0  冻结旧版本                                    已完成
P1  真实需求、PRD、产品验收、全局技术架构           已完成
P2  全系统模块、目录、代码与工具骨架                 进行中
P3  C0 原始资料与第一方版本底座                     未开始（存在提前工作）
P4  飞书与浏览器首批真实收集路径                     未开始（存在提前工作）
P5  C1 多模态清洗与百炼处理                         未开始
P6  核心沉淀、检索、子库与输出                      未开始
P7  扩展来源、正式 Skill 与受控 Agent               未开始
P8  备份、恢复、运维与长期加固                      未开始
```

当前真实情况：

- P2 已在旧 117 文件基础上补齐 20 个 Rust 责任文件和 3 份 Skill 规格，达到 6 个
  crate、137 个 Rust 源文件；CollectorSession、Knowledge、Sublibrary、Output、
  ReadProjection 和 OutputBuilder 均有明确位置与 unavailable 壳。
- P2 的工程骨架 gate 已通过；逐来源现有工具调查和路线决策已经写入
  `03_architecture/08_SOURCE_TOOL_RESEARCH.md`。飞书官方 `lark-cli` 已达到真实授权
  调用证据；OpenCLI、语雀、OneNote、Evernote、微信、内容平台、浏览器和对话平台
  已明确首选工具与最低授权，但多数还没有真实账号下的候选、选择、附件和重收集证据，
  因此 P2 不得标记完成。
- 已有 29 个 P3 活跃文件、raw migrations、命令与测试属于提前工作。它们应保留并
  在 P2 修正后重新审阅，不代表 P3 已开始或完成。
- 已有飞书导出、书签 HTML、CandidateEnvelope、route evidence 和 fixture 属于 P4
  提前工作/回退路径证据。正常的飞书上下文候选、浏览器扩展候选与选择尚未通过，
  所以 P4 不得标记进行中或 enabled。

项目阶段只使用 P0–P8；C0–C3 是数据权威级别，不是项目阶段。

## 2. 状态维护规则

1. 状态只使用“未开始、进行中、已完成、阻塞”；提前代码写在说明中，不改变阶段。
2. 阶段状态变化必须与对应文档、代码和验证证据在同一提交中更新。
3. 局部实现、旧测试通过、文件已经存在或接口能够返回，不自动推动阶段。
4. P2 使用工程交付 gate；P3 以后按 phase gate 和对应 AC/TC 判断，二者不得混用。
5. 产品意图变化按 `00 -> 01 -> 02 -> 03 -> process -> tests -> code` 顺序传播。
6. 架构补充与主架构冲突时先改补充和骨架，再改代码；不为保留旧数字扭曲产品。
7. `AGENTS.md` 只提供本地操作上下文，不是产品、架构或进度权威。

## 3. P0：冻结旧版本

旧版本保留在 `C:\Users\Aiano\Babata-2.0-frozen`，不在 reboot 工作区继续演化。
P0 已完成。

## 4. P1：真实需求到全局架构

P1 交付链：

```text
00_REQUIREMENTS.md（含 append-only 原话证据）
  -> 01_PRD.md
  -> 02_ACCEPTANCE_CRITERIA.md
  -> 03_ARCHITECTURE.md
```

P1 当前已完成：00 恢复真实意图，01 恢复四段产品行为，02 将 PRD-01..10 映射到
AC-01..11，03 明确四段信息流、C0–C3、唯一 Rust writer 和代码边界。

后续若真实意图变化，P1 文档按链路重新打开；不能在 process 或 code 中偷偷新增
产品决定。

## 5. P2：全系统骨架

### 5.1 P2 目的

在单一模块深入实现前，建立修正后全系统的完整位置：

- 6 个 Rust crate、137 个目标 Rust 源文件；
- 12 个 application service、13 个 port；
- 13 个 CLI 命令模块、受保护 local API 路由模块和 worker 生命周期；
- 浏览器/Python 边界；
- 9 份 Skill 规格；
- C0/C1/C3 migration、测试、脚本和配置位置；
- 每个能力的 owner、允许/禁止依赖和激活阶段。

完整清单见 `03_architecture/04_SYSTEM_SKELETON_BLUEPRINT.md`。

### 5.2 P2.1：文档和目标清单

1. 以 00–03 为上游，修正 04–07 架构补充、开发流程和测试映射。
2. 固定旧 117 文件之外新增的 20 个责任文件。
3. 固定 service/port/CLI/local API/worker/Skill 的所有权。
4. 区分产品 AC/TC 与 P2 工程 gate。

完成证据：文档追溯检查覆盖 PRD-01..10、AC-01..11、TC-01..11；下游不存在旧
`AC-11 = 117 文件` 或 `P4 = 导出导入` 的表述。

### 5.3 P2.2：代码与外围骨架对齐

1. 保留现有 117 文件，不 reset、checkout 或盲目删除用户工作。
2. 添加蓝图列出的 20 个责任文件，目标达到 137。
3. 添加 Knowledge、Sublibrary、Output Skill 规格位置。
4. 更新 module export、DTO、capability descriptor 和 unavailable 壳。
5. 不在此步骤实现真实来源、模型、知识算法、搜索排序和输出模板。

完成证据：P2 inventory 检查报告 6 crate、137 文件和外围规格位置齐全。

### 5.4 P2.3：接口和 composition roots

1. 新增 CollectorSession、Knowledge、Sublibrary、Output service 壳。
2. 新增 ReadProjectionPort 和 OutputBuilderPort。
3. 扩展 RawRepositoryPort 的未来责任但不提前实现 P6 SQL。
4. CLI 添加对应模块；local API 只添加路由 owner，不固定没有真实调用者的 endpoint。
5. worker、browser、Python、provider、view/output builder 全部只调用 application 用例。

完成证据：interface ownership 和 Rust boundary 检查使用新清单；无万能 service、
反向依赖或第二 C0/C1 写入者。

### 5.5 P2.4：工程 gate

必须同时通过 `04_SYSTEM_SKELETON_BLUEPRINT.md` 的工程门：

| Gate | 本阶段判定 |
| --- | --- |
| P2-G1 | 6 crate、137 文件和外围规格位置齐全 |
| P2-G2 | service、port、CLI、API/worker owner 完整 |
| P2-G3 | 依赖单向、workspace 可编译 |
| P2-G4 | 未激活能力诚实 unavailable |
| P2-G5 | 只有 Rust application/infrastructure 可最终写 C0/C1 |
| P2-G6 | 文档、蓝图、脚本和测试追溯一致 |
| P2-G7 | 00 列出的来源完成现有工具调研、实际验证、最小授权和路线决策 |

```text
check-p2-skeleton-inventory.ps1
check-rust-boundaries.ps1
check-interface-ownership.ps1
check-doc-traceability.ps1
check-no-secondary-writer.ps1
cargo metadata / check / fmt / clippy / architecture tests
```

这些 gate 证明骨架完整、依赖正确、能力诚实和写入边界唯一。它们不证明任何产品
AC 已完成。

### 5.6 P2 当前证据与未完成项（2026-07-17）

- 6 个 crate、137 个 Rust 源文件、12 个 application service、13 个 port、13 个 CLI
  命令模块、local API route owner、worker 生命周期和 9 份 Skill 规格位置全部存在；
- `cargo check --workspace`、`cargo fmt --all --check`、`cargo clippy --workspace
  --all-targets -- -D warnings` 通过；
- `cargo test --workspace` 通过 41 个测试；
- P2 inventory、interface ownership、document traceability、Rust boundary 和
  no-secondary-writer 检查全部通过；
- 新增 CollectorSession、Knowledge、Sublibrary、Output、ReadProjection 和
  OutputBuilder 只提供边界与 unavailable 壳，没有业务算法；
- 离线 route evidence 可以记录覆盖，但不能单独把飞书/浏览器标记 enabled；来源仍
  等待 P4 真实上下文候选与选择证据；
- 来源工具调查已覆盖 00 点名的全部来源。已实际核验本机 `lark-cli 1.0.68` 的用户
  OAuth 和 Wiki/Docs 只读调用；已实际运行 OpenCLI 1.8.6 的命令发现、站点 help 和
  doctor，确认其覆盖 Bilibili、知乎、小红书、豆包、Kimi、ChatGPT 和公众号文章，
  同时确认本机 Browser Bridge 尚未连接；
- 已淘汰飞书手动 Markdown 主路线、已归档的 BBDown/bilibili-api-python 和已被 DMCA
  屏蔽的 `wx-cli`；语雀、OneNote、Evernote、微信和浏览器均已有明确直接使用、组合
  工具或窄适配决策；
- 29 文件 P3 提前实现及 34 个 raw 功能测试继续可运行，但不作为 P2 产品验收，也不
  代表 P3 已开始。

上述证明 P2-G1 至 P2-G6，并完成了 P2-G7 的调查与路线决策部分。P2-G7 仍部分通过：
除飞书外，多数来源还只有工具/命令证据，没有真实授权的候选 -> 用户选择 -> 正文/
附件 -> 重收集探针。下一步按 `03_architecture/08_SOURCE_TOOL_RESEARCH.md` 第 13 节做
小范围只读实证；在此之前不继续增加来源 adapter、协议或手工导出主路径。

## 6. P3：C0 原始资料与第一方版本底座

前置：P2-G1 至 P2-G7 全部通过。

P3 按 `05_RAW_FOUNDATION_BLUEPRINT.md` 和 `06_RAW_FOUNDATION_EXECUTION_PLAN.md`
重新审阅已有 29 文件提前实现，完成：

1. 显式 text/file/export 的统一 C0 提交；
2. first-party create/revise/annotate；
3. raw SQLite、不可变资产、哈希、版本、关系与 read-back；
4. transaction、journal、orphan/quarantine 和故障补偿；
5. 临时数据根下的工程/恢复 CLI 验证。

P3 gate：

| Gate | 本阶段判定 |
| --- | --- |
| P3-G1 | 外部数据根与编号分区正确 |
| P3-G2 | text/file/export 形成可回读 C0 |
| P3-G3 | first-party create/revise/annotate 版本关系正确 |
| P3-G4 | 失败不产生伪 ready，journal/orphan 可诊断 |
| P3-G5 | DB/资产写入 owner 唯一 |
| P3-G6 | P2 gate 继续成立且未提前激活其他能力 |

P3 为 AC-03、AC-06、AC-10 提供部分底座，不满足 AC-01、AC-02 或完整 AC-11。

## 7. P4：飞书与浏览器首批真实收集路径

前置：P3 C0 写入和故障边界稳定。

P4 按 `07_P4_FIRST_COLLECTION_PATHS.md` 实现：

1. 飞书官方授权连接、文档/Wiki/知识库候选、层级和附件限制；
2. 浏览器扩展配对、页面/选区/链接/书签候选；
3. 用户选择单条、可见集合或明确范围后才写 C0；
4. queued/running/saved/skipped/failed、局部成功和重试；
5. changed/unchanged/inaccessible/removed 重收集；
6. 真实授权证据与 fixture 机制证据分开。

已有导出、书签 HTML 和 CandidateEnvelope 只作为回退/提前证据。P4 gate：

| Gate | 本阶段判定 |
| --- | --- |
| P4-G1 | 飞书真实上下文候选成立 |
| P4-G2 | 浏览器扩展候选与配对成立 |
| P4-G3 | 未确认不写入，只提交选择范围 |
| P4-G4 | 逐条状态、局部成功和重试成立 |
| P4-G5 | 四种重收集结果不覆盖旧 C0 |
| P4-G6 | 真实证据与 fixture 分开，未验证来源保持 disabled |

只有 P4-G1 至 P4-G6 和 TC-01、TC-02 通过后，才能把首批来源标记 available，并
满足 AC-01、AC-02。

## 8. P5：C1 多模态清洗与百炼

前置：至少一条真实 C0 来源可稳定回看。

依次激活：

1. C1 schema、process run、job 和 derivative；
2. 文档/网页机械提取；
3. 百炼 CLI 的真实 pipeline；
4. 图片 OCR、音频转写、视频字幕/关键帧/视觉描述；
5. 百炼/通义 API 或批处理、隐私、成本、限流和重试；
6. 原件/派生物对照与转换损失说明。

P5 主要交付 AC-03、AC-04 和 TC-03、TC-04。C1 不覆盖 C0，模型输出不自动成为
人工判断。

## 9. P6：核心沉淀、检索、子库与输出

P6 必须按核心价值顺序进行，不能直接跳到 Datasette/Obsidian：

### P6.1 核心人工沉淀

- 聚合查看原件、派生物、来源、版本和关系；
- 人工记录、判断、关系、分类、主题/结构模型、评分和分析；
- ModelSuggestion 与人工接受/修改/拒绝分离；
- first-party 与人工知识记录的版本历史。

交付 AC-05、AC-06 和 TC-05、TC-06。

### P6.2 检索与关系导航

- C0/C1 可重建读投影；
- 正文、来源、时间、类型、状态、人物、分类、关系和处理状态检索；
- 媒体-only、附件-only 和受限资料仍可发现；
- 版本、来源、关系导航。

交付 AC-07 的检索和关系部分。

### P6.3 子库与输出

- 版本化 SublibraryDefinition；
- 可删除重建的子库物化；
- 人类可读和结构化输出；
- manifest、来源/版本回溯和只读 builder；
- Obsidian、网页、报告等在真实用途出现后逐项启用。

交付 AC-07、AC-08 和 TC-07、TC-08。

## 10. P7：扩展来源、正式 Skill 与受控 Agent

按真实价值扩展语雀、OneNote、印象笔记、微信、知乎、Bilibili、小红书、抖音、
豆包/Kimi/GPT、本地文件等来源。每条来源继续优先官方能力和现有工具，不先造重型
爬虫。

对应底层能力通过自己的 AC/TC 后，P2 Skill 规格才转成真实 `SKILL.md`。Agent 默认
人工触发或确认，批处理携带明确范围，不自动扩张授权或把模型判断升级为事实。

P7 主要交付 AC-09 和 TC-09。

## 11. P8：备份、恢复、运维与长期加固

实现一致快照、加密增量备份、NAS/云端副本、隔离恢复、hash 验证、日志轮转、doctor、
成本与故障监控。恢复报告区分 C0 损坏、C1 可重建、C2/C3 未重建和凭据重授权。

P8 完成 AC-10、TC-10，并与 P4–P7 的真实路径共同完成 AC-11、TC-11 的完整本地
raw-to-view 闭环。

## 12. 阶段与验收映射

| 阶段 | 主要产品验收 | 说明 |
| --- | --- | --- |
| P2 | 无产品 AC；P2-G1..G7 | 工程骨架与现有工具路线门，不冒充产品完成 |
| P3 | AC-03/06/10 的底座部分 | C0、first-party、单一写入；无真实来源/清洗/恢复 |
| P4 | AC-01、AC-02 | 首批真实上下文收集 |
| P5 | AC-03、AC-04 | 原件/派生物与忠实清洗 |
| P6 | AC-05、AC-06、AC-07、AC-08 | 核心先于检索/视图/输出 |
| P7 | AC-09 | 扩展来源、Skill、Agent |
| P8 | AC-10、AC-11 | 一致恢复与完整系统闭环 |

## 13. 提交与验收纪律

- P2 提交按文档/清单、责任文件、接口、composition roots、工程检查分组，不混入算法。
- P3 以后每项功能提交引用 phase gate 和对应 AC/TC。
- 功能阶段发现接口不对，先更新 03 架构/补充与 P2 蓝图，再改代码。
- 真实数据、授权信息、数据库、模型输出和日志不进入 Git。
- 未通过当前阶段门，不提前激活或宣告下一阶段。
- 工作树中已有用户改动默认保留；不 reset、checkout、删除或盲目提交。

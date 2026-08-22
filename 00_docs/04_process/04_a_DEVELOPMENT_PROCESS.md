# Babata 开发流程与 Phase 模型

<!-- DOC-ID: DOC-PROCESS -->

<!-- DOC-AUTHORITY-BOUNDARY: delivery-process -->

## 1. 文档职责

本文定义 P0–P9 的稳定含义、交付顺序、phase gate，以及收官后的使用、开发、缺陷和发布纪律。
它不定义新产品行为，
不维护来源/课程/批次的当前数量，也不保存逐次运行日志。当前 phase 和真实使用进度唯一见
`04_b_USAGE_STATUS.md`；产品行为见 PRD，完成口径见 AC，测试步骤见 TC。

P0–P9 是项目交付阶段；C0–C3 是数据权威级别。某个 phase 完成不等于所有来源、资料或课程
全量完成。某个明确范围全量跑通是 usage 事实，不能反向写入 PRD 或改变 phase 定义。

## 2. PRD、Phase 与使用的边界

| 问题 | 权威 |
| --- | --- |
| 用户需要什么、哪些边界不可退让 | requirements |
| 产品反复提供什么能力和行为 | PRD |
| 什么可观察结果证明能力成立 | AC |
| 哪些稳定 writer/data/recovery 边界实现它 | architecture |
| 先交付什么、每阶段怎样退出 | 本文 |
| 当前实际跑到哪里、某范围是否完成 | usage status |
| 怎样重复验证 | TC/spec/checker |

试跑、试点和模板/profile 可以属于 PRD，因为产品需要定义它们的行为、标记和退出方式；某一次
试跑/试点结果、某模板实例 accepted、某课程或来源 `N/N` 全量跑通属于 usage。Phase 可以安排
何时验证这些能力，但不能因为排在某阶段就把交付动作改写成产品功能。

## 3. 状态与完成声明

Phase 状态只使用：`未开始`、`进行中`、`已完成`、`阻塞`。状态只在 usage status 中维护。

一个 phase 只有同时满足以下条件才可标为已完成：

1. 本 phase 的目标和非目标清楚；
2. 适用 engineering gate 通过；
3. 适用 AC/TC 的真实证据成立；
4. 仍未覆盖的来源、模态、范围和风险明确，不被成功样本吞掉；
5. 当前状态与 Git 外 receipt/evidence 同步。

编译、文件数量、接口存在、fixture、dry-run、pilot 或单个全量批次不能单独推动 phase。

## 4. P0–P9 Phase 模型

### P0：冻结旧版本

目标：冻结 predecessor，建立 reboot workspace 和历史只读边界。

退出：旧版本可读但不继续演化；新仓库和数据根独立。

### P1：需求、PRD、AC 与稳定架构

目标：从直接用户意图建立 requirements -> PRD -> AC -> architecture authority chain。

退出：产品边界、数据权威、writer、恢复边界和开放决定可追溯；阶段计划不冒充产品定义。

### P2：系统骨架与工程责任

目标：为已定义能力建立最小模块、ports/services、CLI/API composition roots 和工程 gate；
unavailable 能力保持不可用，不用空壳冒充支持。

Gates：

- P2-G1：inventory 与 blueprint 一致；
- P2-G2：workspace/format/lint/compile；
- P2-G3：ports/services/entry ownership；
- P2-G4：crate/dependency boundary；
- P2-G5：single writer/no secondary persistence；
- P2-G6：placeholder/unavailable 诚实；
- P2-G7：逐来源 route evidence、授权、限制和 fallback。

当前是否关闭以及关闭证据只由 `DOC-USAGE`/receipt 维护；本节只保留 gate 定义。

### P3：C0 与 First-party 底座

目标：建立 text/file/export/first-party 的统一 Rust application/core writer、版本、asset、
provenance、relation、transaction、read-back 和 migration。

Gates：

- P3-G1：外部数据根、schema 和 C0 authority 正确；
- P3-G2：text/file/export 形成可回读 C0；
- P3-G3：first-party create/revise/annotate 版本关系正确；
- P3-G4：stage/transaction/finalise 故障不产生伪 ready；
- P3-G5：数据库和资产 writer 唯一；
- P3-G6：P2 gate 继续成立且不提前激活后续能力。

P3 不要求来源全量、C1 或 C2。

### P4：首批真实收集路径

目标：用少量真实授权来源证明 candidate -> explicit selection -> C0 -> per-item status ->
failure/retry -> unchanged recollection。

Gates：

- P4-G1：真实授权连接产生可解释候选；
- P4-G2：首个点名浏览器来源可选择性收集；
- P4-G3：未确认不写 C0，确认后只写所选范围；
- P4-G4：逐项状态、局部失败和重试可观察；
- P4-G5：重收集区分 changed/unchanged/inaccessible/removed 且不覆盖旧 C0；
- P4-G6：真实证据和 fixture 分开，未验证来源保持 disabled。

P4 完成只证明首批路径，不证明全部来源 enabled。

### P5：C1 多模态清洗

目标：用真实 C0 证明文档/网页提取、OCR、音视频处理、模型/工具 provenance、局部失败、重试、
多结果并存和删除重建；C1 不改变 C0。

退出：TC-03A、TC-04 和适用 AC 通过；未启用模态保持 unavailable。C1B 内容判断可在后续真实
课程试点中成熟，但不能由 C2 直接绕过。

### P6：核心沉淀、检索、子库与通用输出

目标：建立三大界、语义类型、关系、三维相关度、机器建议/人工审阅、检索/浮现、子库定义和
可追溯只读输出。

交付顺序：

1. P6.1 核心语义沉淀；
2. P6.2 C0 metadata/observation preflight、搜索、浮现与关系导航；
3. P6.3 子库定义、物化和通用输出。

历史 baseline 退出：在 P6 关闭时适用的 TC-03B、TC-05..08 与对应 AC 通过；Obsidian/Web 不因
通用输出通过自动标为可用。P6 关闭后新采用的 ontology successor（map-node 非父边类型化关系、
Course/Branch、`covers`、MBA lens、基石强度/置信度）不倒改历史 gate，由 P8.9 单独交付和追踪，
必须通过现行 AC/TC 后才能声明新 ontology conformance。

### P7：扩展来源、统一 Skill 与受控 Agent

目标：扩展真实来源路线；用唯一 `babata-collect` 路由 capability/recipe；Agent 在一次授权范围内
连续执行、逐项隔离失败，不自动扩张范围、启动 C1 或接受知识建议。

退出：TC-09 与对应来源真实路径通过。代表样本足以证明 capability，但不等于账号或来源全量。

### P8：真实资料回收与使用扩展

目标：在既有架构和产品能力上处理用户授权的真实存量、补齐主权深度/管理就绪、验证大范围
可用性，并把成功模式固化为可重复 Skill/spec/profile。

P8 的子阶段是 usage scope，不是新的产品模块：

| 子阶段 | 目的 | 不能声称 |
| --- | --- | --- |
| P8.1 | 计划内来源最低真实广度 | A2、registered 或长期自动化 |
| P8.2 | 指定高优先来源后续回收 | 暂缓范围完成 |
| P8.3 | 计划内来源 A1 范围盘点/收口 | 已耗尽路线外的完整取得 |
| P8.4 | 每来源有界 A2 深度样本 | 全来源 A2 |
| P8.5 | 对冻结存量判断 A3 并准备 B | 全来源 B/C |
| P8.6 | 将同一权威范围正式登记 C | 新抓取路线 enabled |
| P8.7 | 本地 MBA 小批 C1 试点 | 当前权威 MBA 输入 |
| P8.8 | 网站权威 MBA C1 覆盖 | MBA C2B/用户知识库完成 |
| P8.9 | C1B/C2B、模板/profile、Obsidian 和兼容层真实使用 | 所有 MBA 课程自动完成 |

具体分母、暂缓项、财务标杆和其余课程进度只在 usage status 更新。

### P9：外部备份同步

目标：在已有一致快照、加密备份和隔离恢复能力之上，选择一个 NAS/云盘/云 Git 等外部目标，
证明纯复制/同步和恢复。P9 不扩张成通用运维平台。

退出：TC-10 对选定外部目标通过；未选目标前保持未开始。

## 5. C1B/C2B 与 Obsidian 交付流程

本节只定义稳定交付顺序，不保存某门课程的当前数字。

1. 从外部主权库/C0 的正式完整 C1 开始，不读取旧 C2B 作为新内容输入。
2. 执行 C1B 本质/模态判断并通过正式 handoff。
3. 正式登记 C2B semantic entries、reviews 和知识宇宙归属。
4. 根据 accepted output profile 生成内容、课程 index、Mermaid/PNG 和 manifest。
5. 在全新 staging root 验证正文、媒体、链接、profile、重建和 package hash。
6. publisher 只复制 hash-verified package 到唯一 live。
7. 用户在真实 Obsidian 中查看；Agent 不替代用户视觉验收。
8. closure verifier 写 Git 外 receipt；usage status 更新当前成果。

门禁：

- `C2B-DOCS-FIRST-GATE`：按 `DOC-INDEX` 的唯一顺序 review wording/requirements -> PRD -> AC ->
  architecture/spec/profile -> process/TC -> implementation/publish -> usage；只更新真实受影响的角色。
  这是每个语义变更波次的一次有序 review，不是每个对象、阶段或 checker 的人工审批点。
- `C1B-FORMAL-HANDOFF-GATE`：staging sidecar 不冒充正式 C1B。
- `C2B-KNOWLEDGE-UNIVERSE-GATE`：正式知识宇宙归属先于发布。
- `C2B-PACKAGE-OWNED-COURSE-MAP`：脑图源/位图归 package，publisher 不渲染。
- `C2B-MECE-COURSE-MAP-GATE`、`C2B-CRASH-COURSE-MAP-GATE`：结构清楚、内容可复习。
- `C2B-MODERN-VISUAL-MAP-GATE`、`C2B-RIGHT-GROWING-MINDMAP-GATE`：遵循 accepted profile。
- `C2B-RESPONSIVE-MAP-GATE`：Mermaid 为唯一默认主图，PNG 默认折叠回退。
- `OBSIDIAN-HUMAN-VIEW-BOUNDARY`：只启动课程 manifest/usage status 登记的唯一 live URI，不代替用户看。

课程 index 只负责课程内部导航；宇宙级大 Index 是独立产品表面，课程发布不得冒充或静默修改。

## 6. 试跑、试点、模板和全量运行流程

### 6.1 试跑 / Dry-run

用于预览授权范围、预计动作、写入、限制、成本和失败条件。dry-run 不写正式结果，不可标记
capability/phase/usage 完成。结果进入 runtime receipt，不写 PRD/AC/TC。

### 6.2 试点 / Pilot

用于一个有界真实范围验证尚未普遍启用的能力。开始前声明分母、成功/退出条件、不可外推范围；
结束后分别记录工程 gate、产品 TC、用户结果和未覆盖项。通过后可固化 Skill/spec/profile，
但不能自动把其他来源或课程标为完成。

### 6.3 模板 / Profile

模板是可复用输出合同：定义 note shape、导航、媒体、视觉、链接、manifest 和 publish gate。
模板不拥有知识，不保存某课程当前状态。模板实例的 accepted/candidate/rejected 与应用成果记录
在 usage status；profile 合同记录在 spec。

### 6.4 明确范围全量运行

全量只对声明分母成立。运行必须逐项记录 success/failed/skipped/gap，允许断点续跑并保持成功项；
完成后由 receipt 证明分母覆盖和完整性。全量完成是使用事实，不是 PRD 功能，也不自动改变 phase。

### 6.5 完整执行轮次与缺陷收敛

多阶段或重复跑批以执行轮次为最小收敛单位，遵守 `BATCH-ROUND-TERMINAL-GATE`：

当前最小实现入口为 `05_scripts/invoke-babata-execution-round.ps1`。它消费 Git 外
`babata.execution-round-plan/v1`，将 plan、阶段脚本和显式输入冻结后，只运行逐阶段声明的 `.ps1`
与参数，并在全新 Git 外 round root 写 `babata.execution-round-ledger/v1` 和日志。runner 只编排、
观察和留证，不编辑实现、不自动修复、不把 shell 字符串当命令执行，也不拥有 usage 状态。

1. **开轮冻结**：记录 round identity、授权范围/分母、输入 identity/hash、代码与配置身份、全新
   staging root、目标终端、阶段列表、验收矩阵和 fail-fast 类别；这些内容在轮内不可变。
2. **整轮执行**：按阶段推进并记录结果。不会使后续证据失效的内容、链接、视觉、计数或兼容性
   缺陷进入缺陷账，分母内其余独立对象和仍有意义的后续阶段继续到明确终态；轮内不修改实现、
   配置、输入或验收标准。fail-closed 默认隔离受影响对象，不自动升级为整轮停止。
3. **失效性终止**：数据破坏/不可恢复风险、writer/authority/授权越界、安全问题、冻结输入漂移，
   或会让所有后续结论失真的合同错误立即终止。实际终端记为 aborted，禁止发布或接受该候选。
4. **终端验收**：到达声明终端后只形成一次本轮结论，逐项填写验收矩阵，并汇总完整缺陷账、
   未覆盖项和证据位置。观察用 checker 不单独推动状态。
5. **轮后修复**：按共同根因和 blast radius 聚类，形成一个有界修复集；针对性测试用于验证修复，
   不冒充轮次验收，也不在旧候选上拼接新的通过证据。
6. **新轮复验**：实现、配置、输入或合同一旦改变，旧轮关闭为历史证据；从新的 staging root 和
   round identity 完整重跑到终端。只有新轮终端矩阵通过，才能进入发布、用户验收或关闭。

断点续跑只允许恢复同一冻结轮次中尚未执行的工作，且代码、配置、输入和验收矩阵完全未变；
它不能跨修复集复用 round identity。一次轮次可以因失效性风险提前终止，但不能因为遇到第一个
普通缺陷就把“边跑边修”伪装成严谨。

## 7. 文档与实现顺序

0. **恢复与最新输入**：每次恢复边界先按根浅层钩子执行 Goal/task-state ->
   `DOC-ACTIVE-PLAN` 的 `CURRENT-ACTIVE`，再处理新输入。输入捕获、held queue、阶段结论和终端清理
   的唯一稳定合同见 `DOC-INTENT-PLAN-GOVERNANCE`；本文不复制该状态机。
1. current intent：任务终端后维护 00_b 当前有效意图，不把整理文本伪装成逐字引文；
2. requirements：更新当前结果/约束；
3. PRD：仅当可重复产品行为改变时更新；
4. AC：仅当完成口径改变时更新；
5. architecture/spec：更新稳定 writer/data/profile 合同；
6. 本文/active plan/usage：分别更新稳定顺序、当前执行恢复入口和实际交付状态；
7. TC/implementation：更新重复验证与实现；
8. 发布后把结果写 usage/receipt，不把数字回填 PRD/AC/TC。

语义未改变的下游文档不机械改写。candidate supplement 不能静默成为现行 requirements、PRD、
architecture 或 current status。

## 8. 使用、开发、缺陷与发布纪律

P0–P9 收官后默认进入 **usage stage**。是否需要 Issue/分支/PR 取决于有没有改变 Babata 本体，
不取决于一次材料处理有多大。

### 8.1 日常使用

1. 用现有 Babata 收集、清洗、登记、生成或发布一个明确授权的资料范围，是 usage，不要求为了
   开跑而创建 GitHub Issue、开发分支或 PR。
2. 真实数据、媒体、SQLite、模型输出、日志、secret、browser profile、生成视图和逐次 receipt
   继续留在 Git 外数据根。只在当前使用结论发生变化时更新 `DOC-USAGE`。
3. 每个新 execution round/关键 usage receipt 必须记录 `babata_build.version`、`release_tag`、
   `git_commit` 和 `worktree_dirty`；还应保留实际 profile、配置、provider/model 版本。无 tag、dirty
   或临时 commit 可以运行，但必须如实标记，不能冒充正式发布版本。
4. 只有 Git 外 receipt 或运行结果时不制造仓库提交。仅回写 usage 状态或证据指针的低风险
   housekeeping 不要求 Issue；经对应文档 checker 后可直接提交 main，也可在需要 review 时用短 PR。
5. 使用失败、受限和未覆盖项如实保留，不为结束一次使用而压成成功。

### 8.2 缺陷收集与修复

1. 使用中发现异常，先在 Git 外 defect ledger 留下观测版本/tag/commit/dirty、授权范围、最小复现、
   预期/实际结果、严重度、日志/receipt 指针和临时绕行；状态使用 `observed / triaged / deferred /
   promoted-for-fix / fixed / verified`。
2. 记录 Bug 不等于立即修复。冻结 execution round 内不边跑边改代码；普通缺陷进入轮次 defect
   ledger，轮次到终端后再决定是否晋升为修复工作。
3. 只有决定修复时才创建 GitHub Issue，冻结修复范围和回归证据，再按开发流程实施。不能用新版本
   的修复证据改写旧 usage round 的实际结果。

### 8.3 Babata 本体开发

新增产品能力、改变行为/合同/schema/migration、安全或权限边界、升级会改变运行结果的依赖，
以及已晋升的 Bug 修复，均是 development：从 GitHub Issue 和短生命周期 `codex/` 分支开始，
通过 PR 与适用门禁合并，不直接推 main。Issue 写清范围、非目标、AC/TC、数据/权限影响和退出条件。
工作区已有用户改动默认保留；不 reset、checkout 或删除无关内容。

### 8.4 版本与发布

1. `01_app/Cargo.toml` 的 `[workspace.package].version` 是 Babata 产品版本唯一 Git 权威；所有 Rust
   crate 继承该版本，`babata --version` 必须返回相同值。
2. 版本遵循 SemVer；兼容 Bug 修复升 patch，兼容新能力升 minor，破坏性合同/数据迁移升 major。
   `0.x` 明确表示 public-beta/尚未承诺 1.0 稳定性；需要候选版时使用 SemVer prerelease。
3. 发布 tag 固定为 annotated `vMAJOR.MINOR.PATCH[-prerelease]`。tag 的版本必须与 workspace version
   一致，目标 commit 必须可从 `main` 到达，工作区干净且适用发布门禁已通过；已推送 tag 不移动。
4. 每个正式使用范围优先选择一个发布 tag；若必须使用未发布 commit，receipt 必须记录完整 commit
   和 dirty 状态。旧 receipt 不倒填不存在的 tag。
5. 当前发布与实际使用状态由 `DOC-USAGE` 记录；版本文件和 tag 证明代码身份，不替代 usage 验收。

## 9. 验证节奏

- 开轮前：验证冻结输入、staging、代码/配置身份、目标终端和验收矩阵可执行；
- 轮内：checker 作为观察点记录结果；除失效性风险外，不因单个失败进入修复循环；
- 轮次终端：运行该阶段预先声明的 terminal gate，一次性形成矩阵和缺陷账；
- 轮后修复：运行目标 checker、mutation test、parse/diff check，证明修复集本身；
- shared contract、writer、data root、authority-chain 改动：新轮前运行 `05_scripts/check-boundary.ps1`；
- 合并/发布/phase 关闭：新轮终端通过后运行一次 `05_scripts/check-full.ps1` 或仓库当前等价全量门禁；
- 真实使用关闭：对应 product TC、package/read-back/hash、用户验收和 Git 外 closure receipt。

检查频率按风险调整，不为凑次数重复全量检查，也不把临时目标测试当成阶段性验收。工程 gate
通过后仍需判断它是否覆盖用户结果。

## 10. Phase 与验收映射

| Phase | 主要 AC/TC | 说明 |
| --- | --- | --- |
| P2 | engineering GT-P2-01..07 | 不证明产品可用 |
| P3 | AC-03/06，TC-03A/06 | C0/first-party 底座 |
| P4 | AC-01/02，TC-01/02 | 首批真实收集路径 |
| P5 | AC-03/04，TC-03A/04 | C1 清洗，不提前证明 C2 |
| P6 | 关闭时版本的 AC-05..08，TC-03B/05..08 | 历史 baseline 已关闭；后采用 ontology successor 由 P8.9 交付 |
| P7 | AC-09，TC-09 | 统一 Skill 与受控 Agent |
| P8 | AC-01..11 的真实使用扩展 | 每个 scope 独立报告 |
| P9 | AC-10，TC-10 | 外部同步/恢复目标 |

当前是否通过、完成到哪一批只查 `04_b_USAGE_STATUS.md`，不从本表推导。

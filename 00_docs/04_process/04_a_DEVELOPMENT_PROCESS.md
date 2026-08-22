# Babata 使用、开发与恢复流程

<!-- DOC-ID: DOC-PROCESS -->
<!-- DOC-AUTHORITY-BOUNDARY: operations-delivery-and-recovery -->

## 1. 文档职责

本文是 Babata 收官后的稳定操作权威，定义日常使用、execution round、缺陷、开发、发布、文档更新、
Active Plan 和恢复生命周期。它不定义新产品行为，不保存具体批次或完成历史。

P0-P9 的完整开发过程和 phase gate 已关闭并进入 `DOC-P0-P9-ARCHIVE`；当前状态只查 `DOC-USAGE`。

## 2. 当前生命周期

Babata 当前是 `v0.1.0 / public-beta / usage-stage`。默认动作是使用现有能力处理明确授权资料，而不是
继续扩张产品。

| 工作类型 | 是否需要 Issue/branch/PR | 状态与证据 |
| --- | --- | --- |
| 使用现有 release 处理材料 | 否 | Git 外 execution receipt + `DOC-USAGE` 必要摘要 |
| 观察或登记 Bug | 否 | Git 外 defect ledger |
| 低风险状态/证据指针修正 | 视影响；可 housekeeping | 文档定向检查 |
| 已决定修复 Bug | 是 | Issue + `codex/` 分支 + PR + 新版本判断 |
| 新能力、行为/合同/schema/权限变化 | 是 | docs-first authority chain + Issue/PR |
| 不可逆资料删除 | 不是普通开发审批；必须单独授权 | snapshot、清单、可恢复策略和 receipt |

已经 `accepted / closed` 的 MBA、Cherno、P9、TC-10 或历史 phase 不因新任务或文档整理重跑。

## 3. 日常使用流程

1. **冻结授权范围**：来源、对象、分母、允许动作、不可逆边界和期望输出。
2. **记录 build identity**：`babata_build.version`、`release_tag`、`git_commit`、`worktree_dirty`、
   profile/config、provider/model/tool version。
3. **预检**：输入 identity/hash、route capability、权限、空间、成本和 fail-fast 风险。
4. **执行**：按适用 C0/snapshot -> C1/C1B -> core -> C2 顺序，逐项保留 terminal state。
5. **验证**：read-back、hash、链接、package、恢复或用户可见结果；不把 checkpoint 冒充终端。
6. **记账**：调用量、价格来源/时间、实际或估算成本；provider 未返回 usage 时不得伪造实际扣费。
7. **关闭**：记录明确分母、结果、缺口、evidence pointer 和用户 acceptance；只更新受影响权威。

无 tag、临时 commit 或 dirty worktree 可以运行，但必须如实标记，不能冒充正式 release。

## 4. Execution Round 与缺陷收敛

重复或多阶段处理以完整轮次为最小收敛单位。

### 4.1 开轮冻结

冻结以下内容：

- round ID、授权范围和逐项分母；
- 输入 identity/hash 和依赖；
- build/config/profile/processor identity；
- 全新 staging root；
- 阶段顺序、声明终端和验收矩阵；
- 只包含数据损失、越权、安全、输入漂移或共享证据失效的 fail-fast 条件。

### 4.2 轮内执行

- runner 只编排、观察和留证，不编辑实现或自动修复。
- 非失效性缺陷进入 defect ledger，所有独立工作继续到显式终端。
- 单项失败只影响该对象；共享输入或合同失效才阻断依赖阶段。
- checkpoint 是观察，不改变冻结输入和实现 identity。

### 4.3 终端与修复

1. 一次性输出每阶段/对象的 terminal matrix 和一个 defect ledger。
2. 按 root cause、owner 和 blast radius 聚类缺陷，形成有界 repair set。
3. targeted check 证明修复，不证明原轮、phase 或用户结果。
4. 实现/config/input/acceptance 发生变化后从全新 staging 开新轮，跑到同一终端。
5. 合并、发布或 acceptance 前运行与影响面匹配的完整 gate。

当前 runner 入口为 `05_scripts/invoke-babata-execution-round.ps1`；schema 和 receipt 位于 Git 外。

## 5. Backs 记忆档案执行边界

Backs 是资料治理和清洗 usage，同时可能暴露通用能力缺口。正式开工前必须另立有界计划，至少包含：

1. **只读 inventory round**：完整文件/目录、可靠时间字段、强 hash、访问失败和结构截屏。
2. **规则 dry-run**：精确重复组、最旧 survivor 规则、垃圾类别、近似候选和预计释放空间。
3. **人工/Agent 风险审阅**：不能可靠决定最旧、内容近似、个人意义未知或敏感对象不进入自动删除。
4. **可恢复清理 round**：明确候选先进入隔离/回收站，写 occurrence 和 before/after receipt。
5. **内容清洗 round**：截图、照片、文本和其他有价值对象按模态处理，保留 provenance 和失败项。
6. **年度归档 round**：生成 `归档分析/<年份>/` package，验证时间线、日志、感悟和证据回链。
7. **用户验收**：先验收结构和清理结果，再验收年度内容；未验收不永久处置隔离对象。

冻结 round 内不边跑边修改 Babata。观察到产品缺口先进入 defect ledger；只有 promoted-for-fix 后
才创建实现 Issue。

## 6. 缺陷生命周期

使用中发现异常时，先在 Git 外 defect ledger 记录：

```text
defect_id
observed_at
babata version/tag/commit/dirty
input scope and stable identity
minimal reproduction
expected vs actual
severity and impact
logs/receipt pointers
temporary workaround
status: observed | triaged | deferred | promoted-for-fix | fixed | verified | closed
```

- 登记 Bug 不等于立即修复；冻结 usage round 中不改实现。
- `promoted-for-fix` 需要明确用户/治理决定，之后创建 Issue、短分支和 PR。
- 修复后在新 round 或最小真实路径验证；fixture/targeted check 不能单独关闭真实缺陷。

## 7. 产品开发流程

1. 从最新直接用户意图和当前 Requirements 开始，确认是不是行为/合同变化。
2. 建 GitHub Issue，创建 `codex/` 短分支；工作区已有相关用户改动必须保留。
3. 按 `Requirements -> PRD -> AC -> Architecture/spec -> Process/TC -> implementation -> Usage`
   顺序更新，只改真实受影响角色。
4. 优先既有能力和窄实现；没有真实 caller 不新增 service、repo、protocol 或 abstraction。
5. targeted checks 提供反馈；边界变化跑 boundary gate；PR 前跑 full gate。
6. PR 描述列出用户结果、影响角色、验证、未完成项和 release impact。
7. 合并后更新 status/evidence，删除成功 Active Plan 临时项；不把 PR 合并冒充用户 acceptance。

## 8. 版本与发布

- 唯一版本源为 `01_app/Cargo.toml` 的 `[workspace.package].version`。
- 兼容 Bug 修复升 patch；兼容新能力升 minor；破坏合同或数据迁移升 major。
- `0.x` public beta 不冒充 `1.0.0` 稳定承诺。
- 当前只维护 annotated Git tag，不创建 GitHub Release，除非用户另行决定。
- tag 必须解析到声明的 release commit，worktree/build identity 和 `babata --version` 一致。

## 9. 文档权威与更新顺序

```text
direct user input
  -> DOC-REQ current intent and requirements
  -> DOC-PRD reusable behavior
  -> DOC-AC observable acceptance
  -> DOC-ARCH stable ownership and contracts
  -> DOC-PROCESS / DOC-TC operating and verification rules
  -> implementation/spec/skill
  -> DOC-USAGE and Git-external evidence
```

- 每个事实只有一个 primary authority；其他文档只引用或解释。
- run 数字、批次、课程、accepted instance 和 receipt 属于 Usage/evidence，不进入稳定产品合同。
- 历史原话、旧 phase、被替代设计和关闭过程进入收官归档、Git 或 receipt，不进入日常热路径。
- 新产品意图先逐字捕获到有恢复能力的输入证据；当前结构已将低频原话历史冻结在 Git/归档，
  新的产品纠偏应先在 Active Plan 标出 source，再进入 Requirements，不能在聊天中悬空。
- 只在语义、合同、状态或证据真的改变时更新下游；review 后无影响的角色不制造编辑。

## 10. Active Plan 合同

`DOC-ACTIVE-PLAN` 是唯一当前执行和恢复入口，目标保持一个屏幕内；硬上限 120 行且 6,000 字符。

一个 active item 至少说明：

```text
source anchor and transition evidence
user goal
current stage
declared terminal
protected boundary
Agent stage conclusion when route-changing
next action
evidence/recovery entry
```

- 同时最多一个 `CURRENT-ACTIVE`；队列不是 backlog，也不参与恢复目标选择。
- `requires-explicit-resume`、held 或 paused 项不能自动晋升。
- 初始启动、显式覆盖、合法终端晋升或授权 blocker replan 才能改变 active identity。
- 成功且无后续义务的临时项在 durable consequences 提升后删除；失败/阻塞/中断保留 terminal reason。

## 11. 三层恢复结构

完整恢复边界包括新 session、上下文压缩、可能丢失控制/上下文的交接或中断、状态不确定的长暂停，
以及用户明确说“继续”“恢复”等。

恢复顺序固定为：

```text
Goal/task-state API
  -> root shallow recovery hook
  -> DOC-ACTIVE-PLAN unique CURRENT-ACTIVE
  -> DOC-INDEX
  -> only the current authorities needed
```

三层状态为：

1. **实时对话层**：最新明确指令和已发生结果，最及时但易受压缩影响。
2. **Goal 层**：跨 session 的任务范围和状态；`null` 只表示 unknown。
3. **文档层**：当前计划、terminal 和硬边界，最耐久但可能滞后一次终端回写。

恢复时先排除已 terminal 的 instruction instance，再按以下闭序选择：

```text
newest unfinished explicit live instruction
-> started Goal
-> durable CURRENT-ACTIVE
-> paused Goal
-> compacted locating evidence
```

压缩摘要、交接文字、旧消息和 tool checkpoint 只能定位，不能独立授权执行。已有 live completion、
Goal terminal 或 durable terminal 都禁止重跑；如果只是 writeback lag，只补终端维护。

普通同步命令/API 明确失败且当前 turn/任务身份完整时原地处理，不触发恢复。只有 stateful operation
结果不明且控制上下文或 governing task 也可能丢失时，才重新执行完整钩子。

## 12. 终端维护

任务成功、失败、阻塞、中断或 handoff 时按顺序执行：

1. 把 durable decision、requirement、architecture、usage、evidence、failure 和未决义务写入唯一权威。
2. 更新当前状态和 evidence pointer；完整 matrix/defect 留在 Git 外。
3. 保留有效 intent，历史证据进入归档/Git，不删除独有用户决定。
4. 成功且无义务时删除 Active Plan 临时项；合法队列项才可晋升，否则 `CURRENT-ACTIVE: none`。
5. 失败/阻塞/中断项写 terminal reason、next authority 和恢复入口。

## 13. 验证节奏

- 文案/链接修正：`git diff --check`、文档 checker 和 stale-reference 搜索。
- authority、路径、恢复或 checker 变化：相应 mutation/negative tests + boundary gate。
- implementation、schema、writer 或 release：targeted test + boundary + full gate。
- 一次 checker 成功只证明其实际检查的规则；实时 capability 和 usage 必须查询运行系统或 receipt。

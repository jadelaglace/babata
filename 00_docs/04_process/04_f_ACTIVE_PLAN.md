# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

## 1. 职责与维护边界

本文是 Babata **唯一的当前执行计划与进度控制面**，只保存尚在执行、需要恢复或仍有后续义务的
工作。它回答“下一步做什么、为什么、当前到了哪里、最近形成了什么阶段结论”。产品意图由
Requirements/PRD 拥有，实际交付状态由 `DOC-USAGE` 拥有，完成证据由 TC/运行回执拥有；本文不得
成为这些角色的竞争副本。

新治理输入按 docs-first 规则先进入 `DOC-WORDING-RECOVERY`、当前活动项或下次开工队列。会改变
当前路线的输入直接并入当前活动项；不应打断当前工作的通用待办插入“下次开工队列”顶部。Agent
形成足以改变后续路线的阶段结论、完成一个执行轮终端或发现共享阻断后，必须先更新对应活动项的
临时子计划，再继续大量检索或下一轮操作。

每次恢复边界，包括新 session、Agent/任务交接、Agent 或工具中断、长暂停、上下文压缩，以及用户
明确说“继续”“恢复”“接着做”时，任何状态写入前先核对可用的线程/外部 Goal 与当前活动项。
信息缺失只产生 `unknown`，不得改变 active 身份、优先级或 terminal 状态；“继续/恢复”只授权继续
既有 Goal，不授权选择最近可见子话题。没有明确用户覆盖、合法终端晋升或有依据的授权重排时，
当前 active 默认不可变，`resolved/superseded/closed` 默认不得重开。

活动项结束时，先把长期有效的决定、产品语义、状态、证据和未决义务提升到各自权威；成功且
无遗留义务的临时活动项随后删除，再把队列首项晋升为当前活动项。失败、阻塞、中断或仍需交接时
保留并标明终态、原因和恢复入口。本文不积累已完成任务编年史；Git 历史和正式权威承担长期追溯。

## 2. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20260815-05 -->

恢复、压缩、session/Agent 交接、Agent 或工具中断及“继续/恢复”后，只能继续下列 active。不得从
历史 Goal、最近可见子话题、旧 AP 编号或后续队列自行选择工作。

### AP-20260815-05：修复恢复漂移并重整浅层 Docs 治理入口

- 来源锚点：`DFC-20260815-02`；2026-08-15 用户明确要求停止做课，先修复所有恢复边界的 Goal
  漂移，并全面检查根目录浅层入口、Docs 杂冗与边界，先将当前批次建 PR，再从干净 Issue 分支继续。
- Goal 锚点：当前 Goal API 返回空值，按 `unknown` 处理；以最新明确用户指令作为持久化 Goal，
  不从交接摘要、最近可见子话题或工具断点推断其他 Goal。
- 状态转换依据：用户明确覆盖原课程 Goal，授权暂停 AP03 并把本治理修复设为唯一 active；这不是
  blocker、自主重排或从旧 resolved 项推导出的重开。
- 当前状态：`in_progress / governance-repair-and-validation`。
- 用户目标：修复 Agent 在信息不完整时擅自重开 terminal、切换 active 的漏洞；让根目录 AGENTS/
  README 或等价浅层入口对恢复必读权威形成强制钩子；全面去除 03/04 与跨文档杂冗、理清 authority
  边界，并把可复用方法反哺 product-docs Skill。
- 目标终端：先完成当前候选批次 gate、commit/push/PR；再从最新 `main` 建立关联 Issue 的干净分支；
  在新分支完成浅层强制恢复钩子、Docs 全链审计与去冗余、authority 边界修复、checker/负向测试和
  product-docs 反哺；所有恢复边界保持 Goal/current AP/terminal 约束，最终向用户报告并等待决定。
- 不改变：本轮不继续 successor Rust、决策会计 C1B/C2B 或任何课程构建；不重开已 resolved 的
  恢复条目；不改财务/供应链成果、外部主权数据、Git 提交状态或冻结前代。

#### 临时子计划与阶段结论

1. 已确认根因不是“压缩导致失忆”本身，而是恢复时未先查 Goal、把 `unknown` 当“未完成”、并把
   Agent 自身审计判断越权当作切换 active 的授权。
2. 先前被非法重开的 recovery capture 已恢复为 `resolved`，竞争 AP 已撤销；任何 terminal 重开
   必须保留原终态并追加 `reopened_by`、新证据和影响范围。
3. 正式 docs 与 product-docs Skill 已覆盖新 session、Agent/任务交接、Agent 或工具中断、长暂停、
   上下文压缩和明确“继续/恢复/接着做”指令；正在补齐 checker 的逐边界负向保护并跑完整治理 gate。
4. `DFC-20260815-02` 将浅层恢复钩子、Docs 去冗余/边界重整和“先 PR 后干净 Issue 分支”加入本
   active；旧课程 AP 降为次优先队列项并保留恢复入口，明确用户恢复前不得晋升或执行。

- 下一步：修正剩余 mutation fixture，跑当前批次完整 gate，审计 diff 后 commit/push/PR；PR 成功后
  创建/关联 Issue，从最新 `main` 建干净分支，再进行全面 Docs 重整，不启动课程工作。
- 证据入口：`DOC-WORDING-RECOVERY`、`DOC-INTENT-PLAN-GOVERNANCE`、本文件、
  `05_scripts/check-intent-plan-governance.ps1`、`05_scripts/test-intent-plan-governance.ps1`、外部
  product-docs `SKILL.md` 与 `references/intent-plan-lifecycle.md`。

## 3. 下次开工队列（禁止恢复时自动执行）

### AP-20260815-03：继续完成下一门 MBA 课程

- 来源锚点：原线程 Goal“继续完成下一课”；2026-08-15 用户明确将其降为次优先，但要求保留。
- 当前状态：`queued / paused-by-explicit-goal-override / requires-explicit-resume`。
- 用户目标：复用既有 MBA C1B/C2B 链路完成下一门课；下一课程为 `25春 OMBA 5401 决策会计`，
  权威 C1 coverage `33/33`，课件 19、视频 14。
- 下一步：只保存恢复入口；必须在 AP05 到达终端且用户明确恢复课程后，才可晋升并继续 successor
  ontology compatibility 前置轮和决策会计 fresh C1B/C2B。
- 恢复入口：`DOC-MBA-ROLLOUT`、`DOC-USAGE`、`02_skills/00_specs/07_knowledge.md`、migration `0008`、
  `01_app/01_babata_domain/src/course.rs`、`01_app/03_babata_infrastructure/src/sqlite/knowledge_core_repository.rs`。

禁止自动执行：恢复、压缩、session/Agent 交接、Agent 或工具中断及“继续/恢复”只继续
`CURRENT-ACTIVE`，不构成 AP03 的晋升授权。

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

恢复时同时核对实时对话、Goal 运行态和本文持久态。若实时对话已有对应结果并明确汇报完成，而
Goal 或本文尚未回写，只补终端状态、证据和清理，不重跑业务动作、收尾、测试或发布；否则才从
当前活动项的“当前状态”和“下一步”继续。已标为完成、已有结果、已终态登记或已按成功终端规则
从本文清理的步骤，不能因摘要、最近消息、旧指令或工具断点再次出现而重跑。

活动项结束时，先把长期有效的决定、产品语义、状态、证据和未决义务提升到各自权威；成功且
无遗留义务的临时活动项随后删除。只自动晋升明确允许 `auto-promote` 的队列项；
`requires-explicit-resume` 项继续留在队列。若没有可自动晋升项，写 `CURRENT-ACTIVE: none` 并等待
用户决定。失败、阻塞、中断或仍需交接时保留并标明终态、原因和恢复入口。本文不积累已完成任务
编年史；Git 历史和正式权威承担长期追溯。

## 2. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20260816-07 -->

### AP-20260816-07：按变更范围路由工程门禁

- 来源锚点：2026-08-16 用户纠正：纯 Docs/PowerShell 治理变更触发 Rust 全量检查属于必须修复的 CI 路由缺陷，不能因检查成功或既有 workflow 如此配置而合理化。
- Goal 锚点：MBA 全课程 Goal 保持 `paused`；本项是实时对话层明确覆盖的紧急治理修复，不授权恢复任何课程动作。
- 状态转换类型：`user-explicit-goal-override`
- 状态转换依据：用户明确覆盖当前执行路线，要求先处理错误触发 Rust 的 CI 根因；已暂停的 MBA Goal 保留原身份和现场，待本项终端后仍需明确恢复授权。
- 当前状态：`in_progress / engineering-gate-scope-routing`。
- 用户目标：修复工程门禁的错误扩大触发，让未改动的语言栈明确跳过，而不是用无关的全量检查结果掩盖路由缺陷。
- 目标终端：变更路径分类器具有定向测试；GitHub workflow 根据分类结果跳过无关语言栈；本 PR 的新 run 证明 `Rust` 与 `TypeScript and Python adapters` 为 `skipped`，`Architecture and docs` 实际执行并通过。
- 不改变/保护边界：不运行 Rust、Cargo、adapter 全量检查来验证本次纯治理修复；不恢复 MBA 课程；不把绿色的无关 job 当作正确路由证据；不提交仅有行尾变化的用户文件。
- 临时子计划与阶段结论：
  1. [完成] 采用成熟的 `dorny/paths-filter@v3` 做 changed-path scope detection，避免自造复杂分类机制。
  2. [完成] Rust 仅由 `01_app/**`、`03_migrations/**` 或其 gate orchestration 触发；adapters 仅由 `08_adapters/**` 或其 gate orchestration 触发；Architecture/docs 始终执行。
  3. [进行中] 只运行 workflow/治理静态检查和 `git diff --check`，推送后用新的 GitHub run 验证 skipped/executed 矩阵。
- 下一步：提交并推送；观察 PR #157 新 run，确认 Rust/adapters skipped、Architecture/docs passed。
- 证据入口：`.github/workflows/engineering-gates.yml`、PR #157 checks。

## 3. 下次开工队列（禁止恢复时自动执行）

### AP-20260816-06：MBA 全课程逐门闭环

- 来源锚点：2026-08-16 用户已明确启动的 MBA 全课程 Goal；本次由实时治理修复暂时避让。
- 当前状态：`queued / paused-by-live-governance-override / requires-explicit-resume`。
- 用户目标：MBA 全课程逐门闭环，覆盖 13 门课程，先导课算第 1 门。
- 已保留现场：第 2 门执行商务沟通 C1B 准备 `19/19`，`staged_only`；决策会计、财务管理、全球供应链和可持续运营已 `closed`。
- 下一步：本次 CI 修复终端后仍等待用户明确恢复 MBA Goal，再从执行商务沟通 C2B 正文继续。
- 恢复入口：Goal API、`CURRENT-ACTIVE` 和 `D:\BabataData\04_runtime\staging\model-workspaces\mba-course-plans\executive-business-communication-20260816-v1.json`。

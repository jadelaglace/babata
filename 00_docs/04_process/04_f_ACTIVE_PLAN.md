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

<!-- CURRENT-ACTIVE: AP-20260816-06 -->

### AP-20260816-06：MBA 全课程逐门闭环

- 来源锚点：2026-08-16 用户明确恢复“mba所有课程闭环，列好了一课一课跑，统一找我确认”；本次与 Goal API 中同名 active Goal 身份一致。
- Goal 锚点：Goal API 当前为 `active`，目标覆盖 MBA 全部课程逐门独立闭环；既有课程顺序与已关闭状态保持有效。
- 状态转换类型：`user-explicit-goal-start`
- 状态转换依据：用户明确启动已暂停的 MBA 全课程 Goal，满足本项 `requires-explicit-resume` 条件并授权按既定课程顺序继续。
- 当前状态：`in_progress / mba-course-sequence / executive-communication-c2b`。
- 用户目标：MBA 所有课程逐门闭环，一课一课执行，每门发布后统一请求用户内容与视觉确认。
- 目标终端：全部 13 门课程分别完成 C1 覆盖、C1B、C2B 内容与知识登记、package/live、用户内容与视觉确认及 closure verifier；DOC-USAGE 逐门记录 `accepted / closed`。
- 不改变/保护边界：不重跑已关闭课程；不重做执行商务沟通已完成的 `19/19` C1B 准备；每门未取得用户确认不得关闭或推进下一门；先导课仍计为独立一课。
- 临时子计划与阶段结论：
  1. [完成] 13 门课程顺序、分母与既有 closure 状态已经冻结；决策会计、财务管理、全球供应链和可持续运营已关闭。
  2. [完成] 执行商务沟通 C1B 已正式登记 `19/19` essence decisions 与 `16/16` retained media；v1 round 随后因通用 builder 将 `09-公式与决策工具` 写死而 fail-closed，下游知识登记/package/live 均未运行。
  3. [进行中] builder/materializer 已改为从课程 plan 解析 `09-/10-/11-` 学习文档并通过定向测试；使用全新 v2 round 复用正式 C1B fingerprint，继续生成本课 C2B 正文、知识登记、package/live。
  4. [待执行] 发布后停在 `pending_user_acceptance`，统一请求用户确认内容与视觉；确认后运行 closure verifier。
  5. [待执行] 每门关闭后更新 DOC-USAGE 和课程路线，再按既定顺序推进下一门，直至 13 门全部关闭。
- 下一步：启动全新 v2 execution round，从 fingerprint 复用 C1B 开始跑到唯一 live，不使用 v1 C2B 输出作为输入。
- 证据入口：`D:\BabataData\04_runtime\staging\execution-rounds\mba-executive-business-communication-20260816-v1\round-ledger.json`、`D:\BabataData\04_runtime\staging\model-workspaces\mba-executive-business-communication-c1b-registration-20260816-v1\c1b-registration-ledger.json`。

## 3. 下次开工队列（禁止恢复时自动执行）

队列当前无其他可自动晋升项。

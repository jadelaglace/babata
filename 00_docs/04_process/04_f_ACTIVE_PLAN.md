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

恢复、压缩、session/Agent 交接、Agent 或工具中断及“继续/恢复”后，只能继续下列 MBA 全课程活动项。
本次 Goal 已由用户明确授权按顺序逐门闭环；不从历史试跑、旧批次或未激活课程自行跳步。

### AP-20260816-06：MBA 全课程逐门闭环

- 来源锚点：2026-08-16 用户明确授权“mba 所有课程闭环，列好了一课一课跑，统一找我确认，先导课算一课”。
- Goal 锚点：Goal API 目标为 MBA 全部课程独立闭环；当前状态为 `paused`，活动身份仍由本条保存，本活动项覆盖 13 门课程，包括先导课。
- 状态转换类型：`user-explicit-goal-start`
- 状态转换依据：用户明确启动新的 MBA 全课程 Goal，授权按权威 `course_order` 顺序逐门推进，并要求每门统一请求用户确认；此前 AP03 已经 terminal，不属于被覆盖的 active Goal。
- 当前状态：`in_progress / mba-course-sequence / goal-paused`。
- 目标终端：13 门课程各自完成完整 C1 输入覆盖、C1B、C2B 内容/知识登记、package/live、用户内容与视觉确认及 closure verifier；DOC-USAGE 逐门记录 `accepted / closed`；无遗留义务后清理活动项。
- 不改变/保护边界：不改变已关闭三门课程及 AP03 的终端状态；先导课计为第 1 门；每门独立冻结分母、staging、live、验收和 closure，不因模板或其他课程通过批量标记；未收到用户确认不得关闭当前课程，不自动跳过顺序或晋升后续课程。
- 用户目标：完成权威 MBA C1 覆盖账中的全部 13 门课程（763/763 C1），逐门生成可学习 C2B 并统一请求用户确认。
- 当前路线（权威 `course_order`；`closed` 仅表示已有正式 closure receipt）：
  | 序号 | 课程 | C1 分母 | 状态 |
  | --- | --- | ---: | --- |
  | 1 | 美国加州多明尼克大学MBA先导课 | 119 | `pending` |
  | 2 | 25春 OMBA 5408 执行商务沟通 | 19 | `in_progress` |
  | 3 | 25春 OMBA 5401 决策会计 | 33 | `closed` |
  | 4 | 25春 MBAO5406 财务管理 | 37 | `closed` |
  | 5 | 25春 MBAO 5403 全球供应链和可持续运营 | 101 | `closed` |
  | 6 | 25春 OMBA 5402 创造价值的营销管理 | 70 | `pending` |
  | 7 | 25春 OMBA5404 组织绩效的战略领导力 | 42 | `pending` |
  | 8 | 25春 OMBA5409 组织行为学 | 75 | `pending` |
  | 9 | 25春 MBAO 5411 数据安全、道德和风险管理 | 43 | `pending` |
  | 10 | 25春 OMBA 5413 管理经济学 | 61 | `pending` |
  | 11 | 25春 MBAO5407 商业分析 | 51 | `pending` |
  | 12 | 25春 MBAO 5405 全球商业环境 | 38 | `pending` |
  | 13 | 25春 OMBA 5480 战略管理 | 74 | `pending` |
- 临时子计划与阶段结论：
  1. [完成] 冻结权威 13 门课程顺序、分母和既有 closure 状态；先导课作为第 1 门纳入分母。
  2. [进行中] 为第 2 门执行商务沟通建立全新课程 plan 和 execution round；C1B 准备已完成 `19/19`，当前保持 `staged_only`。
  3. [待执行] 每门 live 完成后暂停在 `pending_user_acceptance`，统一取得用户内容/视觉确认，再运行 closure verifier。
  4. [待执行] 每门关闭后更新 DOC-USAGE 和本路线，顺序推进下一门；全部 13 门关闭后形成 MBA 全量结论。
- 下一步：课程执行保持暂停；Goal 恢复为可执行状态后，从 `staged_only` round 继续生成执行商务沟通 C2B 学习正文，随后完成正式 C1B/知识登记、package/live；发布后停在 `pending_user_acceptance`。
- 证据入口：`D:\BabataData\04_runtime\staging\model-workspaces\mba-course-plans\executive-business-communication-20260816-v1.json`、`D:\BabataData\04_runtime\staging\model-workspaces\mba-executive-business-communication-c1b-20260816-v1\manifest.json`、`D:\BabataData\04_runtime\staging\model-workspaces\gaodun-mba-c1-20260803\coverage\c1-coverage-audit.json`、`05_scripts/build-mba-course-learning-docs.ps1`。

## 3. 下次开工队列（禁止恢复时自动执行）

队列当前无其他可自动晋升项。

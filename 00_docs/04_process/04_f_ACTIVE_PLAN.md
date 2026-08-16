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

<!-- CURRENT-ACTIVE: none -->

当前无活动项。CI 路由修复已经由 PR #157 run `31926103848` 证明：Rust 与 adapters 均为
`skipped`，Architecture/docs 实际执行并通过。MBA Goal 继续暂停并保留在显式恢复队列。

## 3. 下次开工队列（禁止恢复时自动执行）

### AP-20260816-06：MBA 全课程逐门闭环

- 来源锚点：2026-08-16 用户已明确启动的 MBA 全课程 Goal；本次由实时治理修复暂时避让。
- 当前状态：`queued / paused-by-live-governance-override / requires-explicit-resume`。
- 用户目标：MBA 全课程逐门闭环，覆盖 13 门课程，先导课算第 1 门。
- 已保留现场：第 2 门执行商务沟通 C1B 准备 `19/19`，`staged_only`；决策会计、财务管理、全球供应链和可持续运营已 `closed`。
- 下一步：本次 CI 修复终端后仍等待用户明确恢复 MBA Goal，再从执行商务沟通 C2B 正文继续。
- 恢复入口：Goal API、`CURRENT-ACTIVE` 和 `D:\BabataData\04_runtime\staging\model-workspaces\mba-course-plans\executive-business-communication-20260816-v1.json`。

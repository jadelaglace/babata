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
无遗留义务的临时活动项随后删除。只自动晋升明确允许 `auto-promote` 的队列项；
`requires-explicit-resume` 项继续留在队列。若没有可自动晋升项，写 `CURRENT-ACTIVE: none` 并等待
用户决定。失败、阻塞、中断或仍需交接时保留并标明终态、原因和恢复入口。本文不积累已完成任务
编年史；Git 历史和正式权威承担长期追溯。

## 2. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: none -->

当前无活动项，等待授权。

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

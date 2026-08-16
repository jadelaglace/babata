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
- 当前状态：`in_progress / mba-course-sequence / mba-primer-c2b`。
- 用户目标：MBA 所有课程按既定顺序一课一课执行；所有待做课程都到达 `pending_user_acceptance` 后，只统一请求一次内容与视觉确认。
- 目标终端：全部 13 门课程分别完成 C1 覆盖、C1B、C2B 内容与知识登记、package/live；所有待验收课程一次性取得用户内容与视觉确认后，分别运行 closure verifier 并由 DOC-USAGE 记录 `accepted / closed`。
- 不改变/保护边界：不重跑已关闭课程；不重做执行商务沟通已完成的 `19/19` C1B 准备；单课到达 `pending_user_acceptance` 后不打断用户、不等待逐门确认，直接按顺序推进下一门；先导课仍计为独立一课。
- 临时子计划与阶段结论：
  1. [完成] 13 门课程顺序、分母与既有 closure 状态已经冻结；决策会计、财务管理、全球供应链和可持续运营已关闭。
  2. [完成] 执行商务沟通 v3 execution round 无缺陷到达 `pending_user_acceptance`：C1B `19/19`、必要媒体 `16/16`、知识条目 `19/19`、学习文档 12 份、package/live 33/33；唯一 live 已发布，当前不运行 closure verifier。
  3. [完成] 通用 builder/materializer 已改为从课程 plan 解析 `09-/10-/11-` 学习文档并通过课程专属名称定向测试；证据整理合同明确至少 2200 字符，v3 已证明修复后的整轮交付。
  4. [完成] 第 1 门“美国加州多明尼克大学 MBA 先导课”已由权威 coverage audit 证明 C1 `119/119`，分为会计学原理 14、商务英语 16、市场营销 10、管理经济学 46、组织行为学 18、运营管理 15（6 份课件 + 113 段视频）；正式 C1B 已登记 `119/119` 本质判断和 72 个必要视觉，10/10 学习文档与 `119/119` 知识登记完成。修复六域脑图与 kebab-case Mermaid internal-link 后，v8 execution round 的 materialize、package gate、唯一 live 全部通过，87/87 package/live hash 相等，到达 `pending_user_acceptance`；按总 Goal 不运行本课 closure verifier、不单独请求验收。
  5. [进行中] 按冻结课程顺序进入下一门尚未关闭且尚未到待统一验收终端的课程；先盘点权威课程序列、C1 分母与已有阶段结果，只执行未完成步骤。
  6. [待执行] 所有待做课程均发布后，一次性汇总内容与视觉请求用户统一确认；确认后逐课运行 closure verifier、更新 DOC-USAGE 并关闭全量 Goal。
- 下一步：读取冻结的 13 门课程顺序和 MBA 763/763 coverage audit，确定第 2 门未完成课程及其已有状态；不得回头重跑先导课或其他已终端课程。
- 证据入口：执行商务沟通终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-executive-business-communication-20260816-v3\round-ledger.json`；先导课终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-primer-20260816-v8\round-ledger.json`，唯一 live 为 `C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\mba_primer_c2b_latest`。

## 3. 下次开工队列（禁止恢复时自动执行）

队列当前无其他可自动晋升项。

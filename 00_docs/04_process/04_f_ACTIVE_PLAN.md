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

<!-- CURRENT-ACTIVE: AP-20260815-05 -->

恢复、压缩、session/Agent 交接、Agent 或工具中断及“继续/恢复”后，只能继续下列 active。不得从
历史 Goal、最近可见子话题、旧 AP 编号或后续队列自行选择工作。

### AP-20260815-05：修复恢复漂移并重整浅层 Docs 治理入口

- 来源锚点：`DFC-20260815-02`、GitHub Issue `#152`；用户明确要求停止做课，先修复所有恢复边界的
  Goal 漂移，并全面检查根目录浅层入口、Docs 杂冗与 authority 边界。前序候选已由 PR `#151`
  合并到 `main@67c1a68`，当前在干净分支 `codex/152-docs-recovery-governance` 继续 AP05。
- Goal 锚点：当前 Goal API 返回空值，按 `unknown` 处理；以最新明确用户指令作为持久化 Goal，
  不从交接摘要、最近可见子话题或工具断点推断其他 Goal。
- 状态转换依据：用户明确覆盖原课程 Goal，授权暂停 AP03 并把本治理修复设为唯一 active；这不是
  blocker、自主重排或从旧 resolved 项推导出的重开。
- 当前状态：`in_progress / submission`。
- 用户目标：修复 Agent 在信息不完整时擅自重开 terminal、切换 active 的漏洞；让根目录 AGENTS/
  README 或等价浅层入口对恢复必读权威形成强制钩子；全面去除 03/04 与跨文档杂冗、理清 authority
  边界，并把可复用方法反哺 product-docs Skill。
- 目标终端：在 Issue `#152` 分支完成浅层强制恢复钩子、Docs 全链审计与去冗余、authority 边界
  修复、checker/负向测试和 product-docs 反哺；所有恢复边界保持 Goal/current AP/terminal 约束，
  通过完整 gate 并形成独立 PR，最终向用户报告并等待决定。
- 不改变：本轮不继续 successor Rust、决策会计 C1B/C2B 或任何课程构建；不重开已 resolved 的
  恢复条目；不改财务/供应链成果、外部主权数据、Git 提交状态或冻结前代。

#### 临时子计划与阶段结论

1. 已确认根因不是“压缩导致失忆”本身，而是恢复时未先查 Goal、把 `unknown` 当“未完成”、并把
   Agent 自身审计判断越权当作切换 active 的授权。
2. 先前被非法重开的 recovery capture 已恢复为 `resolved`，竞争 AP 已撤销；任何 terminal 重开
   必须保留原终态并追加 `reopened_by`、新证据和影响范围。
3. 前序候选已通过 GitHub Architecture/docs、Rust、Adapters 三个 gate，并由 PR `#151` 合并为
   `main@67c1a68`；CI 已补齐 `sqlite3` 依赖，intent checker 已统一 CRLF/LF 解析。
4. Issue `#152` 保持 open；旧本地分支无独立提交，已从最新 `main@67c1a68` 重建为干净分支。
5. `AP05-DOCS-AUDIT-20260816-01` 已按冻结范围完成根入口、00_b/00_c、03_a–03_g、04_a–04_g、
   provenance/traceability/intent checker 与 mutation、外部 product-docs Skill 的只读审计；终端为
   `completed_with_defects`，完整矩阵和 28 条统一缺陷账在 Git 外 evidence。
6. 审计确认四组共同根因：浅层入口没有强制恢复链；队列默认晋升与 `requires-explicit-resume`
   竞争；03/04 的 gate、当前状态和交付顺序存在多 owner；checker 主要保护固定文案而非状态组合。
7. 集中修复分为四批：B1 统一 Goal -> Active Plan、current-before-queue 和 held queue 状态机并建立
   根入口；B2 做结构化 checker/mutation；B3 去除 03/04 竞争 authority；B4 反哺并前向验证
   product-docs Skill，最后运行完整仓库 gate。
8. 旧课程 AP 保持次优先队列项并保留恢复入口；即使 AP05 到达 terminal，AP03 的
   `requires-explicit-resume` 也禁止自动晋升，必须等待用户明确恢复课程。
9. B1–B4 集中修复已完成：四组 focused governance checker/mutation 全部通过；旧路径、旧 DOC-ID
   和非法默认晋升语义已清理；外部 product-docs Skill 通过 `quick_validate.py`，最小上下文随行
   Agent 前向验证也正确恢复 AP05、保持 AP03 queued，并识别根钩子不具产品 authority。
10. `check-boundary.ps1` 与 `check-full.ps1` 已从头通过；完整 gate 覆盖 Rust fmt/check/clippy/tests、
    TypeScript typecheck/tests/build、Python smoke、Docs/authority/batch/data-root boundary 和 mutation。

- 下一步：审查最终 diff，提交、推送并为 Issue `#152` 创建独立 PR；PR 创建成功后维护 AP05/DFC02
  terminal。AP03 始终保持 queued，不因提交或 PR 自动晋升。
- 证据入口：`DOC-WORDING-RECOVERY`、`DOC-INTENT-PLAN-GOVERNANCE`、本文件、
  `05_scripts/check-intent-plan-governance.ps1`、`05_scripts/test-intent-plan-governance.ps1`、外部
  product-docs `SKILL.md` 与 `references/intent-plan-lifecycle.md`、
  `BABATA_EVIDENCE_HOME/ap05-docs-authority-audit-20260816/audit-ledger.md`、GitHub PR `#151`、Issue `#152`。

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

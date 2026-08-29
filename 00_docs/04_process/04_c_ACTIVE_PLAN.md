# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: backs-r5-annual-archive-analysis -->

### Backs R5：年度归档分析与 Obsidian 闭环

- transition: `user-explicit-goal-start`; evidence: 用户明确要求“快速review 修复。没问题就r5”，R4 独立复核通过后条件成立。
- goal: 仅消费 R4 正式 C1/C1B handoff、snapshot 元数据和终态 ledger，按年份形成日常日志、重要事件、关注主题、截图信息、年度感悟与证据索引，并发布到 Obsidian 唯一 `归档分析/` live。
- stage: `r4-review-passed-r5-authorized`
- terminal: 年度/主题/日志/感悟结论可回到正式对象、revision/derivative/hash；C2B package 与唯一 live 通过 manifest、链接、内容边界、hash 和 rebuild 验证；用户验收保持独立状态。
- protected boundary: 不重跑或改写 R4，不 reread/修改 `D:\Backs` 原件，不永久删除 R3 quarantine，不把模型建议冒充当年事实，不声称通用 `outputs.obsidian` 已实现。
- subplan: Agent 先冻结 1,822 个正式 promotion handoff 与年份分布，生成有引用的年度分析候选；再登记必要知识/日志/感悟身份，构建专用 C2B package，验证后原子发布唯一 live；缺证据年份或结论明确 skipped/uncertain。
- evidence: R4 终端 `D:\BabataData\04_runtime\staging\model-workspaces\backs-memory-archive-r4-20260829-v1\terminal-receipt.json`；R5 workspace 待建于同一 model-workspaces 根。

## 2. 下次开工队列（禁止恢复时自动执行）

当前没有排队项；R5 终态后不得自动启动其他来源、Web 输出或通用 Obsidian capability。

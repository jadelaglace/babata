# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-INTENT-PLAN-GOVERNANCE`](04_g_INTENT_AND_PLAN_GOVERNANCE.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20260821-01 -->

### AP-20260821-01：恢复热路径瘦身与 Skill 同步

- 来源锚点：`DFC-20260821-01`；用户采用 2026-08-21 的只读分析并明确要求全部实施及同步 skill。
- Goal 锚点：Goal API active，scope 为恢复钩子优化；具体运行态 thread ID 不进入 Docs authority。
- 状态转换类型：`user-explicit-goal-start`
- 状态转换依据：用户明确启动恢复钩子完整优化，要求同时修正共享 `product-docs` skill。
- 当前状态：`in_progress / implementation-verified / ready-for-pr`。
- 用户目标：缩短 Babata 新 session、恢复和继续后的启动路径，同时保持防误恢复能力。
- 目标终端：Active Plan 只含动态状态；普通无状态工具失败不触发全恢复；结构 checker 阻止任意层级
  历史 AP 绕过；仓库与 `product-docs` skill 验证通过；Issue/PR/Goal 闭环。
- 不改变/保护边界：保留 `Goal -> Active Plan` 强制顺序、三层核对和 terminal 防重放；不新增第二状态
  文件；不修改 Babata 业务能力、MBA 状态或运行数据；保留用户现有未提交修改并只合并必要钩子差异。
- 临时子计划与阶段结论：
  1. [完成 / Agent] 已量化旧 04_f 共 10,563 字符，其中 9,024 字符为已关闭 terminal record。
  2. [完成 / Agent] Issue #187 与短分支已建立；04_f、04_g、根钩子、checker 和 mutation 已更新。
  3. [完成 / Agent] 仓库外 `product-docs` skill 已同步更新，`quick_validate.py` 通过。
  4. [完成 / Agent] parser、provenance/traceability、治理 mutation、完整 boundary 与 skill validator 均通过；
     活动态热路径由 10,563 字符降至 1,390 字符（减少 86.8%）。
  5. [进行中 / Agent] 提交并创建 PR，随后写入终态引用、清理活动项并合并。
- 下一步：提交实现并创建 PR，完成终态提升与清理。
- 证据入口：本活动项、Issue/PR、治理 checker/mutation 输出、skill validator 输出。

## 2. 下次开工队列（禁止恢复时自动执行）

队列当前无其他可自动晋升项。

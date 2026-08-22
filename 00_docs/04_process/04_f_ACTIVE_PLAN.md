# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-INTENT-PLAN-GOVERNANCE`](04_g_INTENT_AND_PLAN_GOVERNANCE.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20260822-01 -->

### AP-20260822-01：Babata 收官后的使用与版本治理

- 来源锚点：2026-08-22 当前用户明确指令；不新建 Codex Goal。
- Goal 锚点：上一 Cherno Goal 已终态；当前用户只授权本次流程治理和版本基线维护。
- 状态转换类型：`user-explicit-goal-start`
- 状态转换依据：用户明确启动 Babata 收官后的使用阶段治理，要求日常处理不再强制 Issue/PR，
  开发变更另行收集和修复，并维护实际使用版本与 Git tag。
- 用户目标：区分 Babata 的日常使用和本体开发；记录每次使用所依赖的 Babata 版本；按公测/发布
  方式维护版本号和 Git tag；Bug 进入独立收集与修复流程。
- 当前状态：`in_progress`。
- 目标终端：权威流程明确 usage/development/release/bug 四类边界，当前收官版本有可验证版本号和
  Git tag，相关入口同步且门禁通过。
- 不改变：不重开 P0-P9、MBA 或 Cherno；不把一次材料处理自动升级为产品开发任务。
- 临时子计划与阶段结论：Agent 核对到仓库尚无 Git tag，六个 Rust crate 均为 `0.1.0`，README
  仍标记 active development。当前采用 `v0.1.0` public-beta 基线，不冒充 `1.0.0` GA；
  `01_app/Cargo.toml` 统一拥有版本，CLI 暴露版本，usage round ledger 自动记录 tag/commit/dirty。
- 下一步：实现并验证版本身份、usage/development/release/defect 边界，再经一次无 Issue 的治理 PR
  合并并创建 annotated `v0.1.0` tag。
- 证据入口：Git diff、版本文件/tag 和门禁输出。

## 2. 下次开工队列（禁止恢复时自动执行）

队列当前无其他可自动晋升项。

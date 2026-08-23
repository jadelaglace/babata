# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: none -->

当前没有活动任务。Backs R2-A 已达到 `passed_r2a_approval_package_ready`：1,867 个 resolved groups
形成 1,867 个 survivor、2,988 个拟隔离 occurrence 和逐项恢复映射；独立 verifier 通过，`D:\Backs`
未发生源文件动作。终端由
`D:\BabataData\04_runtime\staging\model-workspaces\backs-memory-archive-r2a-20260823-v1\terminal-receipt.json`
保存，不得重跑 R2-A 来代替 R2-B 授权。

## 2. 下次开工队列（禁止恢复时自动执行）

### AP-20260823-06：Backs R2-B 可恢复隔离（待单独授权）

- promotion policy：`requires-explicit-resume`；必须由用户明确批准执行可恢复隔离，R2-A 完成不构成批准。
- scope：仅限 R2-A 的 2,988 个候选、5,046,508,103 bytes；其中 11 个 medium-confidence groups
  必须在 R2-B 授权中单独确认是否纳入。
- protected boundary：624 个 unresolved groups、零字节、垃圾、近似内容、截图 OCR 和年度归档继续排除；
  R2-B 也不授权永久删除。
- next action：获得明确授权后，先冻结实际 quarantine root、空间/冲突预检、逐项 fail-closed move 和
  before/after/restore receipt；否则保持 held。
- recovery entry：上述 R2-A `terminal-receipt.json`、`quarantine-candidates.jsonl`、`restore-map.jsonl`
  和 `medium-confidence-review.jsonl`。

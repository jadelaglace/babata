# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: none -->

当前没有活动任务。Backs R0-R2 独立审计已达到 `passed_backs_r0_r2_audit`：源树、目录、隔离区、
survivors、分母和恢复映射均无业务漂移；唯一发现的 v2 post-terminal verifier 证据链缺陷已通过追加式
closure 修复，冻结 terminal 未改写。624 个 unresolved groups 和一个零字节组继续保留，R3 未启动。
审计终端位于 `D:\BabataData\04_runtime\staging\model-workspaces\backs-memory-archive-audit-20260824-v1\terminal-receipt.json`。

## 2. 下次开工队列（禁止恢复时自动执行）

### AP-20260823-08：Backs R3 明显垃圾治理（待明确启动）

- promotion policy：`requires-explicit-resume`；R2-C 完成不自动启动 R3。
- scope：明确缓存、临时文件、安装残留和无意义程序垃圾的规则 dry-run、风险审阅与后续可恢复隔离；
  具体子阶段在用户启动 R3 后冻结，已授权有界阶段内不做文件级重复确认。
- protected boundary：截图默认不是垃圾；624 个 unresolved exact groups 不因进入 R3 自动处置；不永久
  删除，不处理近似内容、内容清洗或年度归档。
- next action：用户明确要求完成 R3 或其首个子阶段后开新 round；否则保持 held。
- recovery entry：上述 R2-C terminal 与当前 `D:\Backs` 后置 manifest。

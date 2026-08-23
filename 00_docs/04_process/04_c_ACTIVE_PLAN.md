# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: none -->

当前没有活动任务。Backs R2-C v2 已达到 `passed_r2c_exact_dedupe_closed`：2,988/2,988 隔离对象与
1,867/1,867 survivors 全量回读通过，2,988/2,988 独立恢复演练通过且临时派生已清理，源树、2,952 个
目录路径和隔离区无漂移；replacement verifier 通过。1,867 个 resolved exact groups 已收口，624 个
unresolved 与一个零字节组继续明确保留；隔离区未永久删除。终端由
`D:\BabataData\04_runtime\staging\model-workspaces\backs-memory-archive-r2c-20260823-v2\terminal-receipt.json`
保存，不得重跑 R2-C；v1 只保留失败证据。

## 2. 下次开工队列（禁止恢复时自动执行）

### AP-20260823-08：Backs R3 明显垃圾治理（待明确启动）

- promotion policy：`requires-explicit-resume`；R2-C 完成不自动启动 R3。
- scope：明确缓存、临时文件、安装残留和无意义程序垃圾的规则 dry-run、风险审阅与后续可恢复隔离；
  具体子阶段在用户启动 R3 后冻结，已授权有界阶段内不做文件级重复确认。
- protected boundary：截图默认不是垃圾；624 个 unresolved exact groups 不因进入 R3 自动处置；不永久
  删除，不处理近似内容、内容清洗或年度归档。
- next action：用户明确要求完成 R3 或其首个子阶段后开新 round；否则保持 held。
- recovery entry：上述 R2-C terminal 与当前 `D:\Backs` 后置 manifest。

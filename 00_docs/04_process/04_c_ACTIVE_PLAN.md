# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: backs-r4-informative-content-cleaning -->

### Backs R4：有信息内容清洗

- transition: `user-explicit-goal-start`; evidence: 用户明确要求“快速 review，没问题就 r4”，且 R3 只读复核无阻断问题。
- goal: 对 `D:\Backs` 中截图、照片、文本、日常日志及其他可能有记忆价值的对象，生成可追溯的 C1/C1B 清洗结果。
- stage: `probe-passed-full-round-not-started`
- terminal: 形成 R4 acceptance matrix、逐项处理状态、C1/C1B provenance 与失败/不确定清单；未完成项明确 skipped/blocked，不冒充完成。
- protected boundary: C0/source、R3 snapshot、R3 quarantine 和原路径只读；不重跑 R3，不永久删除，不启动 R5 年度 `归档分析/`，不直接发布 Obsidian。
- subplan: 已完成 requirements/PRD/AC/architecture 读取、范围冻结、21,112 项候选哈希清单和 17 项本地探针；下一步先为选定对象建立真实 C0 身份，再分批执行 C1/C1B。完整逐项 OCR/清洗与正式登记尚未开始。
- evidence: `D:\BabataData\04_runtime\staging\model-workspaces\backs-memory-archive-r4-20260829-v1\scope.json`、`probe-report.json`、`REPORT.md`；R3 终端仍见 `D:\BabataData\04_runtime\staging\model-workspaces\backs-memory-archive-r3-20260824-v1\closure-receipt.json`。

## 2. 下次开工队列（禁止恢复时自动执行）

当前没有其他排队项；R3 收口后保持 `CURRENT-ACTIVE: none`，不得自动启动截图清洗或年度归档。

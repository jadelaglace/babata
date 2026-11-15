# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: none -->

当前没有待执行开发项。#235 render＋vision 已按用户 2026-11-15“全部接受”实施到声明终端：
冻结 48 题、轮后修复与新变体复验、既有回归和独立 PR 交付；结果见 `DOC-USAGE`，
证据根 `D:/BabataEvidence/render-vision-235/`（`REPORT.md`、`RUN-STATE.json`）。Round-1 已终态，
不从摘要或旧 Goal 重跑。PR 的审阅、合并、发布、技能安装和用户验收未被自动授权。
独立 PR #234 同样保持待审，不从旧 Goal 或摘要重跑实施。

## 2. 下次开工队列（禁止恢复时自动执行）

### #235 复杂发丝真正 alpha matting 后端：queued / requires-explicit-resume

- 用户原话（2026-11-15）：“先收尾，发丝的记一下，不急”。不由 Issue OPEN、空 Goal 或本条触发。
- 缺口：R04 冻结门槛为全图 MAE≤0.05 且发丝边缘带 MAE≤0.25；rembg 是分割模型，Round-1 带误差 0.436，
  加 `refine` 后新变体 0.343。需要可本地运行的 matting 模型（核许可、权重 hash、CPU 可行性），新 seed 复验。
- 证据：`D:/BabataEvidence/render-vision-235/REPORT.md` 第 3、5 节。

### #235 render／vision 技能安装：queued / requires-explicit-resume

- 用户原话（2026-11-15）：“pr好之后再安装”。条件：PR #236 审阅通过并合并后，再装到本机技能目录并逐文件回读 hash。

不得自动启动其他来源、Web 输出或通用 Obsidian capability。

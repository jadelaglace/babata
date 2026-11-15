# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: none -->

#241 的两个独立原子包已交付 PR #242；27 项自动测试、18 题作者场景走查、frontmatter 和
本地 All／boundary 通过，具体结论和限制在 `DOC-USAGE` §10 及 Git 外
`D:/BabataEvidence/git-server-atoms-20261115/REPORT.md`。当前 PR CI 已触发，不预判其结果。
本次终端为 PR 交付，不自动安装、合并、清理或操作真实服务器；#238 和发丝队列不由此恢复。
逐字输入、来源、验证及恢复证据已持久保留，本临时执行项退出恢复热路径。

## 2. 下次开工队列（禁止恢复时自动执行）

### #235 复杂发丝真正 alpha matting 后端：queued / requires-explicit-resume

- 用户原话（2026-11-15）：“先收尾，发丝的记一下，不急”。不由 Issue OPEN、空 Goal 或本条触发。
- 缺口：R04 冻结门槛为全图 MAE≤0.05 且发丝边缘带 MAE≤0.25；rembg 是分割模型，Round-1 带误差 0.436，
  加 `refine` 后新变体 0.343。需要可本地运行的 matting 模型（核许可、权重 hash、CPU 可行性），新 seed 复验。
- 证据：`D:/BabataEvidence/render-vision-235/REPORT.md` 第 3、5 节。

不得自动启动其他来源、Web 输出或通用 Obsidian capability。

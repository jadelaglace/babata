# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: none -->

#244 本机迁移与最小文件语义补充已完成：5,406 文件/775 目录全量 hash、mtime 与独立
Git 状态一致；桌面 junction 及4类文件读回、7项隔离演练、四根检查和 All gate 200.7s 通过。
当前交付边界只到 PR，不自动合并／发布；交付身份和实时 CI 查
`D:/BabataEvidence/research-workspace-20261115/DELIVERY.md`，真实迁移查同目录
`terminal-receipt.json` 和 `PATH-MAPPING.json`；Usage §10/11 拥有持久结果。
原目录 `research_reports.rollback-244` 保留，退出须另行明确授权，不自动清理或当独立备份。
未来自动化、自动收集、结构改造、重复治理不自动启动；迁移成功实例不得重跑。

## 2. 下次开工队列（禁止恢复时自动执行）

### #235 复杂发丝真正 alpha matting 后端：queued / requires-explicit-resume

- 用户原话（2026-11-15）：“先收尾，发丝的记一下，不急”。不由 Issue OPEN、空 Goal 或本条触发。
- 缺口：R04 冻结门槛为全图 MAE≤0.05 且发丝边缘带 MAE≤0.25；rembg 是分割模型，Round-1 带误差 0.436，
  加 `refine` 后新变体 0.343。需要可本地运行的 matting 模型（核许可、权重 hash、CPU 可行性），新 seed 复验。
- 证据：`D:/BabataEvidence/render-vision-235/REPORT.md` 第 3、5 节。

不得自动启动其他来源、Web 输出或通用 Obsidian capability。

# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: research-portable-246 -->

### research-portable-246

- 来源锚点：active Goal 原话见 `D:/BabataEvidence/research-portable-workspace-20261115/USER-INTENT.md`。
- transition：started-goal / explicit-development；Issue #246 属于 #210 / Epic #203。
- transition evidence：恢复时 task-state API 核实 active Goal，按 DOC-PROCESS §11 排除已终态 Git 删除后捕获。
- 用户目标：调研技能独立复制可用，默认 Assistant，显式路径可从任意项目调用。
- 当前状态：基于 #238 head `152b2d3` 创建独立 `codex/research-portable-246`，复用独立发布包；
  不合并 #238，也不复制其未决清理工作。PR 将以 #238 分支为 base，清楚标明依赖。
  路径8/8、真实默认目录 smoke、Skill validator 通过；All 首轮代码/原子部分通过，计划字段名不合 checker，修正后重跑。
- 合同：`--run` 精确路径，或 `--workspace`＋`--run-id`；省 workspace 才读既有 Assistant 配置。
  路径不齐、冲突或 Git 内输出明确失败；不新造环境变量、不改持久配置。
- 声明终端：最小 Docs 链、实现、独立复制与路径负例、完整 gate、PR/CI 结果；不自动合并/安装。
- 受保护边界：不动其他工作树 dirty 文件，不重跑研究目录删除，不 git init，不启动 matting 队列。
- 下一步：按合同实施，验证后提交和 PR；证据在上述外部目录，结果回写 DOC-USAGE。
- 恢复入口：本工作树、Issue #246、`D:/BabataEvidence/research-portable-workspace-20261115/REPORT.md`。

依赖 #238 仍 OPEN，其协议、独立发布实测及未决 CI/旧依赖清理状态由 DOC-USAGE 和
`D:/BabataEvidence/skill-standalone-238/` 拥有；本项不接管清理，不合并、不重装、不设置用户变量。
已结束的研究目录删除/取消版本管理不重跑，原迁移工作树 dirty 状态保留。

## 2. 下次开工队列（禁止恢复时自动执行）

### #235 复杂发丝真正 alpha matting 后端：queued / requires-explicit-resume

- 用户原话（2026-11-15）：“先收尾，发丝的记一下，不急”。不由 Issue OPEN、空 Goal 或本条触发。
- 缺口：R04 冻结门槛为全图 MAE≤0.05 且发丝边缘带 MAE≤0.25；rembg 是分割模型，Round-1 带误差 0.436，
  加 `refine` 后新变体 0.343。需要可本地运行的 matting 模型（核许可、权重 hash、CPU 可行性），新 seed 复验。
- 证据：`D:/BabataEvidence/render-vision-235/REPORT.md` 第 3、5 节。

不得自动启动其他来源、Web 输出或通用 Obsidian capability。

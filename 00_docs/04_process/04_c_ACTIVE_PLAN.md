# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: git-ops-atoms-241 -->

### git-ops-atoms-241

- transition evidence：最新直接用户消息授权蒸馏两个新技能并走流程，不恢复旧任务。

- 来源锚点：2026-11-15 用户要求“那就新蒸馏两个babata原子技能 解决上述问题。 走流程开始”。
- transition：explicit-override / new-implementation；逐字输入在 Git 外
  `D:/BabataEvidence/git-server-atoms-20261115/USER-INPUT.md`。
- 用户目标：独立可携的 `babata-git` 与 `babata-ops`，保留原技能和四件套；仓库政策优先、明确授权、
  无武断排障结论，提供窄检查工具而非执行平台。
- 基线：最新 origin/main `152b62e`；Issue #241 / Epic #203 / #210；
  分支 `codex/git-ops-atoms-241`，本工作树隔离实施。
- 当前状态：两包、文档合同、27 项自动测试和 18 题作者走查完成；All 195.1s／boundary 40.1s
  通过，frontmatter 通过；正在提交 PR，未安装／未合并／未发布。
- 声明终端：两个包及所需文档、测试、证据通过适用 gate 并提交 PR；未安装、未合并及真实远程
  运维未验证必须明确，不以脚本通过代替 Agent 行为或用户验收。
- 受保护边界：不修改源技能／四件套、不安装、不改用户环境、不操作真实服务器、不合并或清理分支；
  #238 保持原 PR，不复制其未合并实现；发丝 matting 继续 queued。
- 下一步：精确暂存、预检、提交推送并创建 PR，读回 head 与远端检查；不自动 merge 或安装。
- 恢复入口：本文件、Issue #241；Git 外证据 `D:/BabataEvidence/git-server-atoms-20261115/`。

## 2. 下次开工队列（禁止恢复时自动执行）

### #235 复杂发丝真正 alpha matting 后端：queued / requires-explicit-resume

- 用户原话（2026-11-15）：“先收尾，发丝的记一下，不急”。不由 Issue OPEN、空 Goal 或本条触发。
- 缺口：R04 冻结门槛为全图 MAE≤0.05 且发丝边缘带 MAE≤0.25；rembg 是分割模型，Round-1 带误差 0.436，
  加 `refine` 后新变体 0.343。需要可本地运行的 matting 模型（核许可、权重 hash、CPU 可行性），新 seed 复验。
- 证据：`D:/BabataEvidence/render-vision-235/REPORT.md` 第 3、5 节。

不得自动启动其他来源、Web 输出或通用 Obsidian capability。

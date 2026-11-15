# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: standalone-skill-238-rework -->

### standalone-skill-238-rework

- 来源锚点：2026-11-15 用户独立发布交接、“开始”及最新要求“环境变量之类的问题 也按流程 写进babata新协议 更新”；原话见 Git 外 `D:/BabataEvidence/skill-standalone-238/ENV-PROTOCOL-USER-INPUT-20261115.md`。
- transition：explicit-supplement / documentation-protocol。
- transition evidence：最新直接用户消息要求协议更新；沿用 #235 / PR #238，不新建 Goal，不改变安装和清理权限。
- 用户目标：三 Skill 独立包返工基础上，把环境变量、配置、凭据和依赖发现边界提升为 Babata 通用协议，按权威链更新并验证。
- 基线：`codex/skill-standalone-235` / `0eb1717`；原工作区干净。
- 阶段：依赖与目录边界 → Python/Node 实现 → 仓库外验证与 gate → 提交推送 #238。
- 当前状态：原返工本地 All gate 208.9s 通过且已推 #238；本次通用环境／依赖协议按权威链补齐，boundary 37.1s 通过，未改实现或用户环境。远端新 head 没有 checks/run，旧 CI 不适用；旧六路径清理仍受策略阻止。
- 声明终端：返工代码和文档经适用检查，推送原 PR；不能将 CI 绿等同用户验收。
- 受保护边界：不合并、不重装技能、不设置用户变量、不备份旧技能、不写反馈/记忆、不动主检出。
- 发丝 matting 队列不启动；datasets/dev* 不删除。第三方旧目录仅新路径成功后按授权清理。
- 下一步：本次协议补充提交推送原 PR 后交还；远端 CI 与旧目录清理未决项继续单列，待正常触发路径／策略变化再处理。不自动重开 PR、不绕过删除策略、不合并、重装或启动队列。
- 恢复入口：本工作树；`D:/BabataEvidence/skill-standalone-238/`；PR #238。

此前 #233／#235 已于 2026-11-15 经 PR #234、#236 合入 main（c13dc6a），
render／vision 技能已装到本机；结果见 `DOC-USAGE`，证据根 `D:/BabataEvidence/render-vision-235/`。
Round-1 已终态，不从摘要或旧 Goal 重跑。发布和用户验收未被自动授权。

## 2. 下次开工队列（禁止恢复时自动执行）

### #235 复杂发丝真正 alpha matting 后端：queued / requires-explicit-resume

- 用户原话（2026-11-15）：“先收尾，发丝的记一下，不急”。不由 Issue OPEN、空 Goal 或本条触发。
- 缺口：R04 冻结门槛为全图 MAE≤0.05 且发丝边缘带 MAE≤0.25；rembg 是分割模型，Round-1 带误差 0.436，
  加 `refine` 后新变体 0.343。需要可本地运行的 matting 模型（核许可、权重 hash、CPU 可行性），新 seed 复验。
- 证据：`D:/BabataEvidence/render-vision-235/REPORT.md` 第 3、5 节。

不得自动启动其他来源、Web 输出或通用 Obsidian capability。

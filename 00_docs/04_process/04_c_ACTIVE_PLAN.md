# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: issue-235-render-vision -->

### issue-235-render-vision

- 来源锚点：[#235](https://github.com/jadelaglace/babata/issues/235)；候选
  [`PRD`](../../proposals/2026-11-15-render-vision/PRD.md) 与
  [`架构建议`](../../proposals/2026-11-15-render-vision/ARCHITECTURE.md)。
- transition：2026-11-15 用户对三项拍板原话“全部接受”（静态首版边界、本地确定性优先＋真实后端、
  48 题与调用上限作开轮起点），由 held 晋升为活动项。
- transition evidence：`D:/BabataEvidence/render-vision-235/AUTHORIZATION-20261115.md`。
- 用户目标：增强 babata-render（素材加工）、新建 babata-vision（识别／检查），低耦合真实后端，
  带测试题与结果汇报。
- 当前状态：分支 `codex/render-vision-proposal-235`（依赖 #234 基线）；合同、实现、两 Skill 与契约测试在推进，
  48 题冻结轮见 Evidence 同目录。
- 声明终端：48 题冻结轮跑完并交付逐题结果、修复后复验、旧回归记录，独立 PR 打开；不自动 merge／release。
- 受保护边界：不改工厂库与 Hermes；不新增正式 writer；语义调用 ≤24＋4 次重试、费用上限见授权文件；
  只外传自造或公开许可题图。
- 下一步：按 Evidence 中 `RUN-STATE.json` 的最后完成阶段继续；不重跑已终态的冻结轮。
- 恢复入口：`D:/BabataEvidence/render-vision-235/RUN-STATE.json`。

独立 PR #234 的审阅、合并、发布和用户验收未被自动授权，不从旧 Goal 或摘要重跑实施。

## 2. 下次开工队列（禁止恢复时自动执行）

当前没有排队项；不得自动启动其他来源、Web 输出或通用 Obsidian capability。

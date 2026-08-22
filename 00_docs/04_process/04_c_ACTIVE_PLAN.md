# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20260823-01 -->

### AP-20260823-01：P0-P9 收官文档归档与现行权威收束

- 来源锚点：2026-08-23 当前用户对话；用户先要求制定全面归档/精简计划，随后明确回复“可以 执行”。
- transition：`user-explicit-goal-start`。
- transition evidence：user-explicit-goal-start：用户明确授权执行已汇报的 P0-P9 文档归档与现行
  权威收束计划；未授权创建新的产品能力或重跑已关闭 usage。
- 用户目标：先冻结并归档当前 Docs，提炼 P0-P9 的真实需求、流程、规范和经验，显式保留未完成、
  暂缓与跳过事项，减少日常文档数量并聚焦 usage stage 与后续真实工作。
- 当前状态：`in_progress / stage-3-full-verification`。
- 声明终端：当前 Docs 可由冻结 Git identity 与 manifest 精确恢复；现行权威链职责单一且显著收束；
  P0-P9 经验、精选原话、终态和开放项各有唯一去向；引用、恢复钩子、mutation 和 full gate 通过；
  通过 Pull Request 合并并完成 terminal cleanup。
- 受保护边界：不改变 Babata 产品行为、runtime capability 或已关闭 usage；不重跑 MBA、Cherno、P9
  或 TC-10；保留本轮开始前已捕获的 Backs 原话；不把 TC-11、通用 Obsidian/Web 或外部主权
  navigator 伪装为已完成。
- Agent 阶段结论：Issue `#193`、分支 `codex/docs-p0-p9-closeout` 和冻结提交 `9182a77` 已建立；
  当前 19 份 Docs 收束为 10 份现行权威加 1 份只读收官归档。`DOC-WORDING` 合入 Requirements，
  `DOC-WORDING-RECOVERY` 精选原话合入归档且全文由冻结提交恢复；P2/P3 蓝图、知识宇宙设计记录、
  C1B/C2B 补充、外部主权旧 candidate 和 MBA rollout 只提炼长期有效内容，不保留竞争文件；
  intent/plan 治理合入 Process。PRD、AC、route registry 和 TC 保持独立职责。
- Agent 验证结论：10 份活动权威、1 份只读归档、连续编号、DOC-ID、provenance、恢复钩子、C2B、
  ontology、execution round 和数据根 checker 已适配收束后的职责边界；`check-boundary.ps1` 全部通过。
- 用户追加决定：删除活动文档后编号向前补位；因此 route registry 从 `03_d` 重命名为 `03_b`，
  Active Plan 从 `04_f` 重命名为 `04_c`。`90_archive` 保持显式历史分区，不参与活动编号补位。
- 下一步：运行 full gate，完成 stale-reference/规模/恢复审计；通过后提交、推送、创建并合并 PR，
  最后把 Usage 与本文写为 terminal 并将 `CURRENT-ACTIVE` 置为 `none`。
- 恢复入口：本文、Issue `#193`、冻结提交 `9182a77`、当前分支 Git 状态和后续 PR。

## 2. 下次开工队列（禁止恢复时自动执行）

队列当前无其他可自动晋升项。

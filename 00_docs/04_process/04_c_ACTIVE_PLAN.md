# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: none -->

当前没有活动任务。一堂真实样本、`source.yitang`、PR #197、Issue #196 和 `v0.2.0` 已终态；结果由
`DOC-USAGE`、Git tag 和 Git 外 terminal receipt 保存，不得从本文重跑。Backs 仍只保留在 held 队列。

## 2. 下次开工队列（禁止恢复时自动执行）

### AP-20260823-05：Backs 下一阶段（范围待用户明确）

- promotion policy：`held-by-user`；用户于 2026-08-23 明确要求“先挂起”。
- 来源：R1 `passed_r1_dry_run` 终端；R1 只形成可审阅清单，没有执行源文件动作。
- 当前结论：可可靠提出 survivor 的 1,867 组与 `unresolved` 的 624 组必须保持区分；零字节不作为
  空间去重动作。后续可选范围尚未由用户确定，不从历史摘要自行选择。
- 下一动作：用户明确批准并指定下一阶段范围后，再冻结新的 round；在此之前不审阅为删除批准、
  不移动/删除/隔离/建链接，也不自动处理垃圾、近似内容或年度归档。
- 恢复入口：上述 R1 `terminal-receipt.json`、`summary.json`、`oldest-survivor-decisions.jsonl` 和
  `unresolved.jsonl`。

# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20260823-06 -->

### AP-20260823-06：一堂真实样本收集与 `source.yitang` 工具开发

- 来源锚点：用户提供的一堂 replay/document URL pair；GitHub Issue #196；
  `D:\BabataData\04_runtime\staging\model-workspaces\yitang-ai-knowledge-management-source-20260823-v1`。
- transition：`user-explicit-goal-start`。
- transition evidence：`user-explicit-goal-start`，用户于 2026-08-23 明确要求先挂起 Backs，先处理
  这一堂《AI×知识管理》开放麦，再以此为范本开发并验收一堂系列收集工具。
- 用户目标：完整取得本次回放与文稿，形成可复用、经真实验收的一堂收集工具；后续系列范围由用户另行提供。
- 当前状态：真实 acquisition 与 C0 已通过；115/115 资产完整回读。首轮暴露的大文件整读和跨 session
  伪 revision 缺陷已登记并修复，2.47 GiB 视频复验峰值工作集低于 20 MB，修复后新 session 未增加
  revision/asset row；Issue #196 和 `codex/source-yitang` 进入全量门禁与发布收尾。
- 声明终端：回放、文稿、来源 metadata/hash/read-back 完整；`source.yitang` 对该真实样本采集通过，重复
  收集保持幂等；相关 requirements/behavior/acceptance/route/implementation/tests 一致，PR 完成。
- 受保护边界：不扩张到未授权的一堂内容；不绕过付费、登录或访问控制；不把 locator/页面壳冒充
  正文或媒体；不覆盖来源；运行数据和媒体留在 `BABATA_DATA_HOME`；Backs 不自动恢复。
- Agent stage conclusion：正常路线固定为“登录态浏览器取得白名单 metadata/完整渲染正文 -> 公共 CDN
  下载 -> hash-bound prepared manifest -> Rust SourceAdapter -> 唯一 C0 writer”；加密 API body 和浏览器
  身份不保存，文稿内其他课程链接不递归。用户补充允许 MPMux 作为视频 acquisition 方式；当前已
  完成的 ffmpeg 无损下载不重跑，后续 MPMux/ffmpeg 均须归一为同一 MP4+duration+size/hash 合同。
- 下一步：冻结 terminal receipt，完成文档/Skill/boundary/full gates，更新 0.2.0 使用状态，通过 PR
  合并后仅发布 annotated `v0.2.0` tag（不创建 GitHub Release）。
- 恢复入口：上述 evidence root 的 `round-freeze.json`、`prepared-manifest.json`、`repair-verification.json`
  和 `defect-ledger.json`；代码在 `codex/source-yitang`。

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

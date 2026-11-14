# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20261114-RESEARCH-RENDER-233 -->

### AP-20261114-RESEARCH-RENDER-233

- 来源锚点：用户“实施”（2026-11-14）；完整原话、分析方案与授权见 Git 外 `D:/BabataEvidence/research-render-analysis-20261114/`。
- transition：explicit-resume；最新直接授权优先于此前 Goal blocked 状态（API 无 active setter）。
- transition evidence：同目录 `IMPLEMENTATION-AUTHORIZATION.md` 保留直接授权；旧 blocked 不覆盖本次恢复。
- 用户目标：搜索与调研原子（含搜索方法）＋Babata 输出物 PNG 渲染原子；独立可用且可组合，不缩成技巧文档或改名。
- 开发身份：Issue #233，属于 #210 / Epic #203 第一条；分支 `codex/research-render-atoms-233`，基于已获取最新 main `1b3b2b2`。
- 当前状态：两原子、本地技能和独立 PR #234 已交付；实现 commit `caca9eb`，Rust 1.100.0 最终 All gate 85.4s 通过。真实研究、独立／组合 PNG、正式输出／失败保留和超时树清理已验证，目视四图通过；正在等待本 PR 具体远端 CI。
- 受保护边界：保留原知识宇宙路径与旧入口；Rust 唯一正式 writer；助理工作与证据 Git 外；不公开私有搜索配置、不启用 disabled 来源、不合并／发布、不改正式数据。旧 #232 不动。
- 计划：①更新必要权威及最小合同；②搜索调研技能及可恢复证据结果；③浏览器 PNG 渲染＋正式 outputs 接入；④独立／组合／负例与恢复验收，回写证据与 PR。
- 声明终端：两个原子真实可用、恢复和质量边界有证据、必要 gates 通过并交付 PR；用户 acceptance 单独保留。
- 下一步：等待 PR #234 最新 SHA 的远端 CI，必要时有界修复；写 terminal receipt 和最终交付状态。PR 保持 open，不 merge/release。
- 恢复入口：使用本工作树；证据 `D:/BabataEvidence/research-render-233/`，最新完整检查以 terminal receipt 指定日志为准；旧工作树只保留迁移指针，不自动启动其旧任务。

## 2. 下次开工队列（禁止恢复时自动执行）

当前没有排队项；不得自动启动其他来源、Web 输出或通用 Obsidian capability。

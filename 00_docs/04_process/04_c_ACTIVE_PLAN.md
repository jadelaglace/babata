# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20261114-COLLECT-ATOM-IMPLEMENT -->

### AP-20261114-COLLECT-ATOM-IMPLEMENT

- 来源锚点：2026-11-14 用户原话：“跑 不用和之前合并”；独立实现 #210，不混旧 WBS／采集批次，不自动合并。
- transition：explicit-start。
- transition evidence：直接授权与本任务已创建的独立工作树。
- 用户目标：修 OpenCLI 2 JSON 边界，复用 Collector/Capture 做最小幂等收集样板。
- 当前状态：GitHub API 核实时 main = 1b3b2b2；10 个 CLI 集成测试、boundary、Rust 1.94.0 full gate 与真实两消息隔离样本通过；最终审查已将测试身份替换为虚构值，收尾复验及独立 PR 待完成。
- 受保护边界：不动旧 WBS／正式库，不全账号取回，不放宽图片原件校验，不启动 C1/C2。
- 恢复入口：当前工作树 codex/collect-atom-210，D:/BabataEvidence/collect-atom-research-20261114/DEFECTS.md；#210。
- 技术范围：既有 Rust/CLI/Skill；同键回放、异输入冲突、并发及提交后恢复；不新增通用原子框架。
- 保护：不动旧 WBS／正式库，不全账号取回，不放宽图片原件校验，不做 C1/C2，不处理凭据或发布消息。
- 验证输入：已有纯文本捕获或有界新读取，冻结输入，隔离 Git 外数据根。
- 声明终端：docs-first、窄实现、targeted/boundary/full 检查、隔离样本及独立 PR；非整个 Epic 验收。
- 证据：`D:/BabataEvidence/collect-atom-research-20261114/DEFECTS.md`；Epic #203 / #210。
- 下一步：核对最终 diff、回执与 scoped checks，提交独立 PR 并检查 CI；不自动合并，不关闭整个 #210/Epic。正式 gate 使用 CI 固定 Rust 1.94.0，不为 1.100 的旧代码 lint 放宽门禁。

## 2. 下次开工队列（禁止恢复时自动执行）

当前没有排队项；不得自动启动其他来源、Web 输出或通用 Obsidian capability。

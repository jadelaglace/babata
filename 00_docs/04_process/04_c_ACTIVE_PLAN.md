# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: qianwen-adapter-closure-20261109 -->

### qianwen-adapter-closure-20261109

- 来源锚点：用户原文“替千问收尾”。
- transition：显式新任务；接单前 CURRENT-ACTIVE 为 none，用户授权接手共享工作区剩余千问改动。
- transition evidence：千问 full.js 与三个文档仍未提交；元宝 PR #218/#219 已终态，不重跑。
- 用户目标：完成现有只读 qwen detail-full 薄适配器、测试、文档及 Issue/PR 合并，清理本轮遗留。
- 当前状态：Issue #221 / codex/qianwen-adapter-closure-221；6 个定向测试及两个真实会话读取已通过，各 1 个原生行；全套工程检查通过；PR 待提交。
- 声明终端：定向与全套适用检查通过，真实读取证据可追溯，PR 合并、交付回执及状态一致。
- 受保护边界：只收尾现有千问入口，不重采历史、不发送/删除消息、不写 C0 或启动 C1/C2；运行证据在 Git 外。
- route：复用 OpenCLI 登录态读取官方消息 API；完整性受 have_next_page=false 限制，分页不完整即拒绝。
- 下一步：建 Issue/短分支，审查并验证现有代码，提交和合并 PR，回写证据后移除本临时项。
- 恢复入口：D:/BabataData/04_runtime/receipts/qianwen-adapter-closure-20261109/。

## 2. 下次开工队列（禁止恢复时自动执行）

当前没有排队项；不得自动启动其他来源、Web 输出或通用 Obsidian capability。

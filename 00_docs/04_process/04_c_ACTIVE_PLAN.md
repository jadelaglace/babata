# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: qianwen-chat-recovery-20261107 -->

### qianwen-chat-recovery-20261107

- 来源锚点：2026-11-07 用户明确新启动，原话：
  “借鉴豆包，先试点，后全量 ，取回qianwen.com/chat 自己拆好 issue 到闭环，不懂就网络搜”。
- 用户补充原话（2026-11-07）：“补充下  纯点外卖的 排除”。
- transition：用户启动千问授权回收。
- transition evidence：Goal active；直接用户指令及本项补充范围为依据。
- 用户目标：取回已登录用户千问聊天；先冻结完整可发现会话分母，代表样本试点验证，再全量 C0 收集。
- Scope amendment：用户最新明确“c0 阶段混入了外卖的部分 切掉”；整段纯外卖会话排除，混合会话按原生轮次剔除下单、支付、配送和对应上下文，仅其余内容登记 C0；保留 Git 外未修改 acquisition 与精确排除位置，不删除远端。
- 当前状态：C0 全量完成，Rust/adapter 检查已通过，最终文档边界检查与既有 #204-207 Issue/PR 收口。
- Findings：23 个观察到的会话共 166 条原生行；排除 1 个纯外卖会话及混合会话中的外卖轮次，共排除 122 行、保留 22 会话/44 行。22 项 fresh API 重采为 13 unchanged、9 changed：后者原生响应中的 HTML/JS 与渲染耗时发生变化，已保留新 revision，不能称全部 unchanged。53 个媒体 URL、52 个逐 revision 唯一二进制映射通过原文/范围/hash 全量回读与 SQLite 检查。完整 acquisition 与精确排除清单留 Git 外。
- 声明终端：试点通过、全量逐项 terminal matrix、原件/附件 hash/read-back、必要 Issue/PR 闭合，
  Usage 与 Git 外 receipt；登录/访问阻塞必须如实留证，不能把可见子集冒充全量。
- 受保护边界：仅千问授权账号；不发送聊天、不删除远端资料，不把 runtime/登录/原文纳入 Git；
  Backs 等历史终态不重跑；用户 acceptance 独立；冻结 usage round 中不修改实现。
- 下一步：完成边界检查、提交 PR 并合并关闭既有 Issue，更新 terminal-receipt.json 的 delivery，清除此临时项并完成 Goal。history 最老边界返回成功空页但没有显式终止标志，不能声称覆盖被删除或未暴露数据。
- 恢复入口：本项 + `D:\BabataData\04_runtime\staging\model-workspaces\qianwen-chat-recovery-20261107-v1\`。

## 2. 下次开工队列（禁止恢复时自动执行）

当前没有排队项；不得自动启动其他来源、Web 输出或通用 Obsidian capability。

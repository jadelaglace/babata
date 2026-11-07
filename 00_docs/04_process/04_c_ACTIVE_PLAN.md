# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: qianwen-chat-recovery-20261107 -->

- Source/transition：2026-11-07 用户明确新启动；Goal active，原话：
  “借鉴豆包，先试点，后全量 ，取回qianwen.com/chat 自己拆好 issue 到闭环，不懂就网络搜”。
- Goal：取回已登录用户千问聊天；先冻结完整可发现会话分母，代表样本试点验证，再全量 C0 收集。
- Scope amendment：用户补充“纯点外卖的 排除”；仅纯外卖下单会话排除，混合/不确定会话保留待审；不删除远端。
- Stage：只读路线调查；复用豆包的完整分页、附件校验、正式 writer 和逐项终端经验；按缺口拆 Issue。
- Terminal：试点通过、全量逐项 terminal matrix、原件/附件 hash/read-back、必要 Issue/PR 闭合，
  Usage 与 Git 外 receipt；登录/访问阻塞必须如实留证，不能把可见子集冒充全量。
- Protected：仅千问授权账号；不发送聊天、不删除远端资料，不把 runtime/登录/原文纳入 Git；
  Backs 等历史终态不重跑；用户 acceptance 独立；冻结 usage round 中不修改实现。
- Next：检查豆包路线和现有浏览器登录工具，建立 Issue、codex 分支与外部 evidence；只读发现会话和数据形状。
- Recovery：本项 + `D:\BabataData\04_runtime\staging\model-workspaces\qianwen-chat-recovery-20261107-v1\`。

## 2. 下次开工队列（禁止恢复时自动执行）

当前没有排队项；不得自动启动其他来源、Web 输出或通用 Obsidian capability。

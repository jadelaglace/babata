# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: yuanbao-adapter-20261108 -->

### yuanbao-adapter-20261108

- 来源锚点：用户原文“简单写个元宝对话适配器 给你做了两个测试对话 仅仅写适配器用”。
- transition：显式新任务，恢复此前暂缓的适配器开发；历史 C0 回收任务不重跑。
- transition evidence：接单前 CURRENT-ACTIVE 为 none；本轮直接用户指令明确适配器及两个测试会话范围。
- 用户目标：复用现有来源与统一 Collector/writer，为 source.yuanbao 增加最小对话适配器；只用用户新建的两个测试对话验证。
- 当前状态：Issue #217 / codex/yuanbao-adapter-217；实现、定向测试与两个真实测试会话已通过，4条原生消息/1张图片回读一致，fresh重采2/2 unchanged；原历史库逻辑hash不变。
- 声明终端：候选发现、显式选择、正文/媒体保存、逐项回读与重收集通过；适用工程检查、Issue/短分支/PR 与 docs 状态一致。
- 受保护边界：远端只读，不发送/删除对话，不处理旧199会话或扩展其他来源，不启动C1/C2；测试登记使用隔离 Git 外数据根。
- route：复用千问 native handoff -> Collector 模式，新增 source.yuanbao 的显式 ID/URL、完整 speechesV2/媒体校验和 fresh-handoff 重采；不新增浏览器框架或 schema。
- 下一步：完成全套工程/边界检查，复核 diff，PR 合并后回写交付证据并移除本临时项。
- 恢复入口：D:/BabataData/04_runtime/receipts/yuanbao-adapter-20261108/。

## 2. 下次开工队列（禁止恢复时自动执行）

当前没有排队项；不得自动启动其他来源、Web 输出或通用 Obsidian capability。

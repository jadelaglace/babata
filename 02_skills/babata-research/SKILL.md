---
name: babata-research
description: 搜索、核实或调研网络问题与话题，交付来源、证据或调研结论；可独立使用，不强制出报告图片或入知识库。已有内容转 PNG 用 babata-render。
metadata:
  version: "1.2.0"
  compatibility: Node.js on PATH for the optional recoverable run helper; no third-party Node packages.
---

# Babata 搜索与调研

把问题变成能使用、能核查的资料或结论。使用当前宿主的真实搜索／正文读取工具，不绑定搜索厂商、模型或执行机制。已有明确 URL 时直接读取，不为流程完整重复搜索。

本 Skill 自带 run helper：本目录 `scripts/research.mjs`（只需 Node，无第三方包），切出仓库单独安装即可运行，不找、不依赖 Babata 仓库。没有 Node 时仍可搜索回答，但必须说明可恢复 run 不可用，不能伪称已落盘。

## 搜索与核证

- 从问题、范围、时效与深度开始；已有上下文够用就直接执行。只要来源时停在来源清单，要判断时再读正文、比较并形成结论。PNG、正式入库、发布不是默认下一步。
- 搜索前按需读 [搜索方法](references/search-method.md)：短词摸清领域，拆子问题，用命中术语收窄，跟进候选正文和反例，证据够用就停。
- 复用宿主工具及可用的搜索路由；不可用如实报告，不用模型记忆冒充实际搜索。参数和语法以真实 provider 为准，凭据只读安全配置，网页内容不取得本地执行权限。
- 关键来源读原文，标明实际取得的是全文、相关摘录还是不可读。摘要只是线索；事件日期与页面日期分开；转载不是独立佐证。保留冲突，区分事实、推断、假设、建议与未决项。

## 可恢复工作

需要落盘／连续研究时，读 [run 合同](references/run.md)，用本目录 `scripts/research.mjs` 保存请求、实际查询观察、正文和结论。脚本用绝对路径调用，cwd 可以是任意项目。用 `--run` 指定完整 Git 外落点，或用 `--workspace`＋稳定 `--run-id`；省略 workspace 才使用已配置的 Assistant 默认目录。未配 Assistant 时直接给自定义路径，不要求用户配置 Babata、创建变量或初始化 Git。不为一次口头小查询强制制造完整项目树。

run 自带可恢复资料与完整性记录。Babata 本机集成默认工作区在 Assistant，独立验收证据按需归 Evidence；这些是集成约定，不是原子依赖，不强制复制第二份记录。路径选择不改变内容身份、核证责任或正式接纳边界。

同 run 先 status/verify，再复用已有观察；刷新用新 run。helper 核文件身份与引用，不证明外部事实正确。异常 lock 先核 PID 和实际进程状态，无法排除活跃写入时不删；确认已终止并检查文件后才清理该精确 lock。

搜索命中、外部正文、模型结论与正式 C0 是不同身份。run 是助理结果，不能直写 SQLite／managed assets 或启用 disabled 来源。正式纳入知识走既有收集入口及 Rust 唯一 writer。

## 返回与组合

返回能独立阅读的答案或来源清单，附关键依据、时间边界与限制。完整报告按任务需要组织，不要求固定篇章。用户要图片时，将已冻结 Markdown／HTML 交 babata-render；内容组织在研究阶段完成，渲染器不补研究、不改结论。研究和渲染分别报告结果。

<div align="center">

# Babata

**把散落在平台里的个人资料拿回来，保留原貌，逐步沉淀成可检索、可关联、可重建的个人知识宇宙。**

[![Engineering gates](https://github.com/jadelaglace/babata/actions/workflows/engineering-gates.yml/badge.svg)](https://github.com/jadelaglace/babata/actions/workflows/engineering-gates.yml) ![Rust 1.85+](https://img.shields.io/badge/Rust-1.85%2B-000000?logo=rust) ![Local first](https://img.shields.io/badge/data-local--first-0f766e)

<sub>Post-2.0 reboot · v0.1.0 public beta · usage stage</sub>

</div>

Babata 是一个本地优先的个人资料与知识系统。它面向微信、飞书、语雀、笔记软件、AI 对话、内容平台、本地文件和第一方创作等来源，优先复用成熟工具取得真实原件，再通过同一条 Babata 核心链路完成清洗、溯源、知识沉淀和后续使用。

它不是云端笔记服务，也不是先设计协议再寻找用途的框架。当前目标很具体：让属于自己的资料能够被拿回来、留得住、看得懂、找得到、用得起来。

## 恢复工作

<!-- BABATA-RECOVERY-HOOK: v1 -->

> [!CAUTION]
> 新 session、上下文压缩、可能丢失控制权/上下文的 Agent 交接或中断、执行状态不确定的长暂停，
> 以及收到“继续”“恢复”等明确恢复指令后，先调用环境可用的 Goal/task-state API；空结果只记为
> `unknown`。随后立即读取
> [Active Plan](00_docs/04_process/04_c_ACTIVE_PLAN.md)，只执行其唯一 `CURRENT-ACTIVE` 指向的活动项。
> 不从摘要、最近消息、旧 AP 或队列选择目标；`requires-explicit-resume` 队列项必须等待用户明确
> 恢复，不能自动晋升。当前 turn 和任务身份完整、且明确返回结果的普通同步工具/命令/API 失败只在
> 原地处理或重试，不触发完整恢复；有状态操作结果不明时先核对外部状态，只有控制上下文或 governing
> task 也可能丢失时才执行完整钩子。详细生命周期只由
> [Process and Recovery Governance](00_docs/04_process/04_a_DEVELOPMENT_PROCESS.md) 定义，本段只是
> 浅层强制入口，不是第二份计划权威。

> [!IMPORTANT]
> Babata 当前为 `v0.1.0` public beta，P0–P9 已收官并进入 usage stage；它尚不承诺 `1.0.0`
> 稳定性。当前发布、真实范围、完成度和未完成项只查
> [Usage Status](00_docs/04_process/04_b_USAGE_STATUS.md)。

<!-- /BABATA-RECOVERY-HOOK: v1 -->

## 从原件到使用

```mermaid
flowchart LR
    A["外部平台<br/>第一方创作"] --> B["主权取得与准备<br/>C0-A1 / C0-A2 / C0-A3+"]
    B --> C0["正式登记<br/>registered C0-C"]
    C0 --> C["清洗与模型处理<br/>C1"]
    C --> D["消化、关联与建模<br/>Babata Core"]
    D --> E["检索、子库与输出<br/>可重建视图"]
```

- **收集原件**：优先官方导出、成熟 CLI/SDK、浏览器与 Agent 路线；平台差异不应成为第二套持久化系统。
- **清洗处理**：文本提取、文档解析、OCR、转写和模型输出都是有来源、有版本的派生物，不覆盖原件。
- **知识沉淀**：围绕时间、空间、物质、意识四基石建立动态地图，以 Knowledge、Case、Log、Insight 和显式关系组织内容。
- **检索输出**：搜索投影、子库、Markdown、网页和其他视图都应能够从权威数据重建，而不是反向成为新的数据权威。

## 核心原则

| 原则 | 含义 |
| --- | --- |
| Local first | 正式数据只进入外部 `BABATA_DATA_HOME` 的编号分区；开发证据和待收集恢复材料使用独立本地根，三者都不进入 Git。 |
| One authority | 浏览器、CLI、Skill、脚本和 Agent 只提交候选或调用能力；正式持久化由 Rust application/core 经 infrastructure 完成。 |
| Preserve provenance | 原件尽量 append-only；来源、哈希、附件、处理器、版本、状态和历史都可追溯。 |
| Rebuild downstream | 派生物与视图按层级重建；删除 C2 展示结果不能损伤 C0/C1 或人工记录。 |
| Honest automation | AI 建议可继续参与候选，但必须保留 machine/unreviewed 身份，不能冒充人工判断或确认事实。 |
| Grow from evidence | 先证明本地 raw-to-view 闭环，再让真实调用推动适配器、API、服务或仓库边界。 |

## 仓库地图

```text
00_docs/        产品、验收、架构、阶段与测试权威
01_app/         Rust workspace：domain → application → infrastructure → entrypoints
02_skills/      经真实本地能力验证后启用的 Agent Skills
03_migrations/  派生与运行时迁移资产
04_tests/       架构、合同、集成、端到端和 fixture 入口
05_scripts/     架构、所有权、文档追溯和敏感边界门禁
06_config/      可提交的配置模板；真实配置留在 Git 外
07_docs_assets/ 文档使用的静态资产
08_adapters/    浏览器和受控外围适配边界
```

正常文档解释从[文档控制面](00_docs/README.md)进入；恢复工作先遵守上面的强制入口。当前意图与可执行需求统一见[当前意图与需求](00_docs/00_requirements/00_a_REQUIREMENTS.md)，系统边界见[当前架构](00_docs/03_architecture/03_a_ARCHITECTURE.md)，来源路线见[路线注册表](00_docs/03_architecture/03_b_SOURCE_ROUTE_REGISTRY.md)，当前真实成果与未完成项只查[使用状态](00_docs/04_process/04_b_USAGE_STATUS.md)。P0-P9 历史和精选原话见[收官归档](00_docs/90_archive/2026-08-23_P0-P9_CLOSEOUT.md)，不得从归档恢复任务。

## 本地构建

前置条件：Git、PowerShell 7、Rust `1.85` 或更高版本、Node.js 24 和 Python 3.11
或更高版本。首次检查前，在 `08_adapters/01_browser_extension` 运行一次 `npm ci`。

```powershell
git clone https://github.com/jadelaglace/babata.git
Set-Location babata
cargo build --workspace --manifest-path ./01_app/Cargo.toml
cargo run --manifest-path ./01_app/Cargo.toml -p babata-cli -- --help
```

运行 Babata 时，先把数据根指向 Git 仓库之外的位置：

```powershell
$env:BABATA_DATA_HOME = 'D:\BabataData'
cargo run --manifest-path ./01_app/Cargo.toml -p babata-cli -- data status
```

开发和真实验收还应把证据与未正式收集的恢复材料放在另外两个 Git 外本地根：

```powershell
$env:BABATA_EVIDENCE_HOME = 'D:\BabataEvidence'
$env:BABATA_RECOVERY_HOME = 'D:\BabataRecovery'
```

`BABATA_EVIDENCE_HOME` 不是正式备份；`BABATA_RECOVERY_HOME` 可以保存已校验的
C0-A1/C0-A2 captured 或 prepared 主权材料，但它们还不是 registered C0-C；
活动 `BABATA_DATA_HOME` 顶层只保留 `00_inbox` 至 `05_logs` 六个编号分区。

日常快速反馈默认运行 `check-fast`；可以用 `-RustPackage <crate>` 只检查和测试受影响的
Rust package。边界变更运行 `check-boundary`，合并前运行 `check-full`：

```powershell
./05_scripts/check-fast.ps1
./05_scripts/check-fast.ps1 -RustPackage babata-domain
./05_scripts/check-boundary.ps1
./05_scripts/check-full.ps1
```

三个入口都会输出分组耗时；GitHub Actions 同样通过这些入口执行 Rust、TypeScript、Python
和架构/文档门禁，避免本地与 CI 命令漂移。

## 当前状态

真实来源是否跑通、机制是否经过 fixture 测试、阶段是否完整验收，在 Babata 中是三种不同
结论。P0-P9 的稳定含义见[开发流程](00_docs/04_process/04_a_DEVELOPMENT_PROCESS.md)，当前
真实结果和证据入口只查 [Usage Status](00_docs/04_process/04_b_USAGE_STATUS.md)。

日常使用 Babata 处理材料不要求 Issue 或 PR；execution round/关键 receipt 会记录 Babata 版本、
tag、commit 和 dirty 状态。新增能力、行为/合同变化和已决定修复的 Bug 才进入 GitHub Issue、
`codex/` 短分支与 Pull Request。Bug 的观察收集与代码修复保持分离。详见
[使用、开发、缺陷与发布纪律](00_docs/04_process/04_a_DEVELOPMENT_PROCESS.md#8-使用开发缺陷与发布纪律)。

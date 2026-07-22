<div align="center">

# Babata

**把散落在平台里的个人资料拿回来，保留原貌，逐步沉淀成可检索、可关联、可重建的个人知识宇宙。**

[![Engineering gates](https://github.com/jadelaglace/babata/actions/workflows/engineering-gates.yml/badge.svg)](https://github.com/jadelaglace/babata/actions/workflows/engineering-gates.yml) ![Rust 1.85+](https://img.shields.io/badge/Rust-1.85%2B-000000?logo=rust) ![Local first](https://img.shields.io/badge/data-local--first-0f766e) ![Phase](https://img.shields.io/badge/phase-P6.2%20complete-2563eb)

<sub>Post-2.0 reboot · Rust-first · single repository · under active development</sub>

</div>

Babata 是一个本地优先的个人资料与知识系统。它面向微信、飞书、语雀、笔记软件、AI 对话、内容平台、本地文件和第一方创作等来源，优先复用成熟工具取得真实原件，再通过同一条 Babata 核心链路完成清洗、溯源、知识沉淀和后续使用。

它不是云端笔记服务，也不是先设计协议再寻找用途的框架。当前目标很具体：让属于自己的资料能够被拿回来、留得住、看得懂、找得到、用得起来。

> [!IMPORTANT]
> Babata 仍处于开发阶段，不是面向普通用户的稳定发行版。当前已经完成 P0-P5、P6.1 核心知识沉淀与 P6.2 检索/关系导航；P6.3 子库与通用输出尚未完成。

## 从原件到使用

```mermaid
flowchart LR
    A["外部平台<br/>第一方创作"] --> B["收集原件<br/>C0"]
    B --> C["清洗与模型处理<br/>C1"]
    C --> D["消化、关联与建模<br/>Babata Core"]
    D --> E["检索、子库与输出<br/>可重建视图"]
```

- **收集原件**：优先官方导出、成熟 CLI/SDK、浏览器与 Agent 路线；平台差异不应成为第二套持久化系统。
- **清洗处理**：文本提取、文档解析、OCR、转写和模型输出都是有来源、有版本的派生物，不覆盖原件。
- **知识沉淀**：围绕时间、空间、物质、意识四基石建立动态地图，以 Knowledge、Case、Log、Insight 和显式关系组织内容。
- **检索输出**：搜索投影、子库、Markdown、网页和其他视图都应能够从权威数据重建，而不是反向成为新的数据权威。

## 已经落地

- 真实 C0 原件与 first-party 内容的追加式保存、修订、附件和完整读回。
- 飞书、语雀、微信、知乎、Bilibili、AI 对话和笔记导出等首批来源的真实收集路径与限制记录。
- PDF、图片、音视频等多模态资料经百炼处理进入可追溯 C1，保留任务、模型、输入输出哈希和失败状态。
- P6.1 知识核心：三级地图、多重归属、统一标签、显式关系、三维相关度评分、模型建议与追加式人工审阅。
- P6.2 发现能力：可删除重建的 C2 搜索投影、多条件检索、评分排序、关系导航，以及说明方向、相关度、时间和关系的内容浮现。
- 高密度 Markdown C2 的生成、校验、篡改拒绝、删除与重建；删除视图不损伤核心记录。
- SQLite schema 迁移、事务写入、隔离恢复和架构门禁；Rust application/core 是正式持久化的唯一入口。

真实来源是否跑通、机制是否经过 fixture 测试、阶段是否完整验收，在 Babata 中是三种不同结论。准确状态以[开发流程](00_docs/04_process/04_DEVELOPMENT_PROCESS.md)为准。

## 核心原则

| 原则 | 含义 |
| --- | --- |
| Local first | 原件、SQLite、模型输出、日志和凭据只进入外部 `BABATA_DATA_HOME`，不进入 Git。 |
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

先从[文档索引](00_docs/README.md)进入。产品意图在[原始需求](00_docs/00_requirements/00_REQUIREMENTS.md)，系统边界在[架构](00_docs/03_architecture/03_ARCHITECTURE.md)，P6 设计在[个人知识宇宙蓝图](00_docs/03_architecture/09_P6_PERSONAL_KNOWLEDGE_UNIVERSE_BLUEPRINT.md)。

## 本地构建

前置条件：Git、PowerShell 7，以及 Rust `1.85` 或更高版本。

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

仓库级验证：

```powershell
cargo fmt --all --check --manifest-path ./01_app/Cargo.toml
cargo clippy --workspace --all-targets --manifest-path ./01_app/Cargo.toml -- -D warnings
cargo test --workspace --manifest-path ./01_app/Cargo.toml
./05_scripts/check-rust-boundaries.ps1
./05_scripts/check-no-secondary-writer.ps1
./05_scripts/check-doc-traceability.ps1
```

## 路线图

| 阶段 | 状态 | 结果 |
| --- | --- | --- |
| P0-P2 | 完成 | 冻结旧系统，建立产品文档链、单仓与全系统骨架 |
| P3-P5 | 完成 | C0 原始资料、真实来源收集、多模态清洗与百炼处理 |
| P6.1 | 完成 | 核心知识沉淀、地图、关系、评分、建议与高密度表达 |
| P6.2 | 完成 | 搜索投影、多条件检索、评分排序、关系导航与可解释浮现 |
| P6.3 | 下一阶段 | 子库与通用输出 |
| P7-P8 | 未开始 | 扩展来源、正式 Skills、受控 Agent、备份恢复与长期加固 |

日常开发从 GitHub Issue 开始，使用 `codex/` 短分支并通过 Pull Request 合并。提交前请阅读[提交与验收纪律](00_docs/04_process/04_DEVELOPMENT_PROCESS.md#13-提交与验收纪律)。

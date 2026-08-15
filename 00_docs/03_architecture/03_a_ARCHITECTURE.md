# Babata 全局技术架构

<!-- DOC-ID: DOC-ARCH -->

<!-- DOC-AUTHORITY-BOUNDARY: architecture -->

## 1. 架构职责与依据

本文承接：

```text
00_c_USER_WORDING_RECOVERY.md（逐字恢复证据）
  -> 00_b_USER_WORDING.md（当前有效意图集）
  -> 00_a_REQUIREMENTS.md（当前需求权威）
  -> 01_a_PRD.md
  -> 02_a_ACCEPTANCE_CRITERIA.md
  -> 03_a_ARCHITECTURE.md（本文）
```

00_c 保留最后关头恢复语境的逐字证据；00_b 提供日常读取的当前有效意图；00_a 决定当前真实目的和不可丢失的约束，
01_a 决定用户可见行为，02_a 决定可观察的完成结果。
本文只回答：信息如何流动、哪些模块承担责任、每类数据由谁拥有、哪些技术边界防止
产品意图再次被实现细节缩窄。

文件清单、精确方法数量、命令树和 endpoint 清单属于架构补充或 P2 骨架蓝图；阶段
顺序和完成状态属于开发流程。它们不得反过来改写本文的四段边界与数据权威。

## 2. 总体架构决策

Babata 当前采用一个本地优先的模块化单体和一个代码仓库，并保持 Rust、TypeScript、
Python 三种现有语言边界；没有真实生命周期或能力缺口前不增加 Go 或第四套工具链。Rust
是领域规则、应用用例、持久化、迁移、处理编排、worker、备份和恢复的默认实现，也是
唯一可以最终写入权威资料的核心。稳定且已经证明适合核心内实现的来源适配器可以继续使用
Rust；易变浏览器、网站和 JSON API 边界不要求默认改写为 Rust。

四段是同一个应用内部的逻辑边界，不是四个仓库、四个网络服务或四套协议：

```text
来源中的窄收集入口 / 第一方创作入口
                    |
                    v
        +---------------------------+
        |       Rust 应用核心       |
        |                           |
        | 收集 -> 清洗 -> 核心 -> 输出 |
        +---------------------------+
             |       |       |
             v       v       v
            C0      C1      C2
             \_______ C3 ______/
                    |
             BABATA_DATA_HOME
```

正常日常体验不是 CLI-first。飞书来源、浏览器页面/书签和未来来源应在用户正在阅读、
收藏或整理资料的上下文中提供候选与确认。命令行是自动化、恢复、诊断和运维入口；
浏览器扩展或确有需要的窄本地 UI 通过受保护的 loopback API 调用同一应用用例。

本地 API 不是第二套业务实现，也不因“未来可能使用”而扩张成对外平台 API。只有
出现真实本地调用者时才启用对应路由。模块、仓库、服务或协议的进一步拆分，需要
独立部署、权限、生命周期、团队或发布节奏等真实证据。

## 3. 端到端信息流

### 3.1 发现候选，不等于收集

```text
已授权来源上下文
  -> SourceAdapter 只读发现
  -> CandidateSummary 列表
  -> 用户选择单条 / 可见集合 / 明确范围
```

候选发现只读取当前授权范围，并产生临时会话状态。连接来源、列出候选或打开页面
不会创建 C0。候选至少携带实际可得的标题、来源位置/层级、类型、更新时间、附件
可得性和限制；缺失字段保持缺失，不由适配器猜测。

### 3.2 被选择的资料进入统一 C0 路径

```text
CollectionSelection
  -> 来源适配器读取被选内容
  -> AcquisitionPackage（原件 + 来源 + 上下文 + 限制）
  -> Rust 核心校验、暂存、计算哈希
  -> 一次权威提交
  -> C0 原件、版本、附件和溯源
```

来源适配器、浏览器扩展、Skill 和 Python 工具只能提交候选或读取被授权输入。ID 分配、
版本判断、资产最终落盘、关系建立和持久化提交全部由 Rust 应用用例完成。

`AcquisitionPackage -> C0` 是来源收集者的终止边界。一次收集不依赖任何 Process/C1
用例即可进入终态；所有来源共享同一 C0 权威和 writer，不存在 `source.onenote.c0`、
`source.doubao.c1` 一类来源私有数据层或必须贯通的专属管道。

### 3.3 清洗从 C0 读取，向 C1 追加

```text
C0 revision
  -> 用户选择或明确范围的处理任务
  -> Babata 清洗编排
  -> 本地解析器 / QianWen Skills / Bailian CLI / 后续可替换或互补的处理 adapter
  -> 处理运行记录
  -> C1 派生物
```

Babata 永远是集成者，拥有范围、路由、候选合同、登记、重试与审计。处理 adapter 不接收
权威数据库写权限；它只读取批准的输入，在受控暂存区产生候选输出，再由 Rust 核心校验和
登记。切换、组合、失败、重试或重跑 adapter 都不修改 C0，也不产生第二 C1 writer。

C1 通过稳定 input revision/asset 引用知道其 C0 输入，并可在收集完成后的任意时间独立
运行、重跑或永不运行。处理器可以按文档、图片、音频、视频等模态选择实现，但来源名称
不成为一条 C0/C1 所有权边界。

### 3.4 核心区完成语义消化并区分机器与第一方内容

```text
C0 原件 + C1 派生物 + 已有关系
  -> Agent 消化、分类、建模、关系、评分和质量校验
  -> C1 机器语义候选进入 P6 核心，不等待逐条审批
  -> 可选的 first-party 评论 / 感悟 / 日志 / 真实作品改写
```

模型可以在 C1 生成建议。建议即使尚未审阅，也可以被读投影、检索、关系导航、子库
候选和输出候选消费，但必须携带机器来源和审阅状态，不能冒充人工判断。用户接受、
修改或拒绝建议只追加 `SuggestionReview` 状态标记；修改形成了新的用户内容时，再创建
引用原建议和证据的 first-party C0。机器记录本身不会被改名为人工判断，缺少审阅也
不会阻塞下游。

评论/批注、感悟、日志切片、附件/证据、Agent 再分析和同一第一方作品的真实改写必须
分开建模：前三者是独立 C0，附件和证据是资产或关系，再分析是新的 C1 run/suggestion，
只有最后一种才追加既有作品的 revision。

第一方新写内容可以直接进入 C0 以保留原味，不要求先经过清洗。之后如需提取结构、
摘要或其他处理，再从该第一方版本产生 C1。

### 3.5 输出只读消费权威资料

```text
C0 + C1 + 可重建读模型
  -> 检索 / 关系导航
  -> 子库定义（人工组织属于 C0）
  -> 子库物化 / 报告 / 网页 / Obsidian / 应用输出（C2）
```

子库的人工选择、排除和组织规则是用户沉淀的一部分，不能随视图删除；根据这些规则
生成的文件、索引和展示结果属于 C2。任何 C2 builder 都只读 C0/C1，不获得反写
权威资料的能力。

## 4. 数据分级与唯一权威

| 级别 | 权威内容 | 允许的写入者 | 删除与重建规则 |
| --- | --- | --- | --- |
| C0 | 外部原件、原始媒体、来源快照、第一方正文及版本、批注、人工判断、人工关系/分类/模型/评分/分析、子库定义 | Rust 应用用例 | 内容不原地覆盖；修改追加版本或新事件；最高优先备份 |
| C1 | C1A 文字证据；C1B 完成内容判断后的清洗本质（文字及按需保留的图片、音频、视频片段或附件），以及处理运行记录 | Rust 应用用例登记处理器输出 | 可并存、比较、删除后重建；记录可得的输入/来源身份，但 C1B 片段不要求复制原件或建立独立原文件回链 |
| C2 | 搜索投影、子库物化、Datasette/Obsidian、网页、报告、卡片、导出包及其他生成视图 | Rust 控制的只读 builder | 可整体删除重建；不拥有 C0/C1，不接受反向编辑 |
| C3 | 候选会话、队列租约、缓存、临时文件、日志、运行指标和能力状态 | Rust runtime；外围仅能持有本地临时会话 | 可清理；终态收集/处理结果需归档到对应 C0/C1 溯源记录 |
| 机密配置 | 来源令牌、API 密钥、本地 API 凭据和隐私授权 | 受保护配置组件 | 不进入 Git、候选包、日志、视图或备份明文清单 |

所有数据类都使用稳定标识和相对数据根的逻辑资产键。真实资料只有一条最终持久化
路径；外围产生的临时副本不因存在文件就成为权威。

### 4.0 C1A/C1B 与 C2A/C2B

C1A 是抽取、OCR、ASR 和结构化文字等确定性文字证据；C1B 是完成内容判断后的清洗本质，
可以复用 C1A 文字，也可以从 PPT、视频、长 PDF 或附件中直接裁剪/抽取必要片段，新增媒体
可以为零。C1B 不承担外部主权库的原件保存和原始目录维护。新的 C2 默认消费 C1B 生成 C2B；
只有用户明确要求纯文字导航时才生成 C2A。C2A 升级到 C2B 必须先完成 C1B 判断（必要时
生成增量），再由只读 builder 重建 C2。

### 4.1 C0 的版本规则

- 收集到的新资料创建资料与首个版本；同一来源的变化创建新版本或新收集事件。
- `unchanged`、`inaccessible` 和 `removed` 是重收集结果，不重写旧版本。
- 第一方修改创建新版本；批注是独立资料并关联目标版本。
- 人工判断的修订创建新版本；人工关系、分类、模型和评分的变化保留历史。
- 已强校验的物理资产按内容哈希保留并只共享不可变字节；紧急本地收集可以使用独立 opaque
  地址和 `size_snapshot_v1` 直接复制，后续再显式补强校验。任何模式都不删除收集事件和来源差异。

### 4.1.1 来源公共字段、观测与版本边界

P6.2 preflight 使用独立版本合同 `babata.c0.common/v1` 保存各来源共同可表达的 title、
authors、language、可靠 UTC 来源时间、类型化 hierarchy/context、structured limitations、
access state，以及 `babata.c0.media/v1` 媒体子结构。provider 原始 metadata 继续独立保存；
公共字段只提取可证明的值，未知 provider key 不因标准化而丢失。缺少年份或时区的来源时间
只保留原值和 limitation，不猜成 UTC。

四类记录承担不同责任：

| 记录 | 责任 | 更新规则 |
| --- | --- | --- |
| item | 稳定来源身份和首次观测事实 | 创建后不因复采静默覆盖 |
| source observation | 每次成功 capture/recollection 所见的公共字段、provider metadata、访问结果和观测时间 | 只追加；禁止更新和删除 |
| revision | 来源正文或原件的真实变化 | 仅 changed 或新的明确采集内容增加 |
| C2 current projection | P6.2 从 observations 计算当前 title/作者/时间/访问状态等查询值 | 可删除重建，不反写 C0 |

`unchanged`、`inaccessible`、`removed` 只追加 observation 和 recollection check，不制造
revision；`changed` 只创建一个新 revision，并由同一次 capture observation 表达新快照。
旧 item、candidate 和缺少公共字段的旧 CandidateEnvelope 通过默认值与 legacy fallback
继续读取。

### 4.2 溯源最小集

每个 C0/C1 结果在实际可得范围内保留：

```text
稳定输入标识
来源链接、导出路径或原生标识
来源平台、账号/作者和上下文层级
来源时间、收集时间或创作时间
原始内容与附件哈希
适配器、工具、模型、pipeline 和版本
运行状态、限制、错误与重试关系
输出标识与哈希
```

缺少关键溯源的资料进入明确的受限/隔离状态，不会被伪装成完整资料或已确认知识。

## 5. 逻辑能力边界

下面是应用内部责任，不是需要网络化的服务。

| 能力边界 | 主要责任 | 明确禁止 |
| --- | --- | --- |
| CollectorSession | 连接已授权来源、发现候选、保存用户选择、显示逐条状态 | 不持久化最终原件，不做内容理解 |
| Capture | 校验 acquisition、落 C0、建立来源/版本/附件、记录重收集结果 | 不生成摘要、分类或知识判断 |
| Process | 选择 pipeline、排队、执行/重试、登记 C1 | 不修改 C0，不自动确认模型建议 |
| Workspace | 新建、修订和批注 first-party 内容 | 不原地改写旧版本 |
| Knowledge | 记录、关联、分类、建模、评分、分析；处理模型建议的接受/修改/拒绝 | 不把 C1 静默升级为人工事实 |
| Explore | 检索、详情、版本/来源/关系导航 | 不成为持久化写入口 |
| Sublibrary | 保存和修订人工子库定义，生成可重建物化请求 | 不复制出第二套权威资料 |
| Output | 按明确范围生成报告、网页、Obsidian、结构化调用结果和 manifest | 不反写 C0/C1 |
| Capability | 报告来源、处理器、Skill 和输出的真实可用性及限制 | 不因有占位文件就标记可用 |
| Ops | 数据根状态、诊断、备份、隔离恢复和完整性报告 | 不把实时数据库目录当同步盘 |

### 5.1 一个总收集 Skill，内部按来源路由

`babata-collect` 是唯一面向用户和 Agent 的收集 Skill；路由职责直接属于该 Skill，不再增加
一个要求用户先调用的“路由 Skill”。它只负责编排，结构如下：

```text
用户给出来源与明确范围
  -> babata-collect 识别 source route
  -> 查询 Rust Capability 状态
  -> 加载该来源 recipe，选择官方导出 / CLI / 浏览器 / 桌面工具
  -> 发现候选并取得原件、上下文、附件和限制
  -> 同一个 CollectorSession -> Capture -> C0
  -> 回读状态、revision、assets 和限制后结束
```

四个概念保持分离：

| 概念 | 责任 | 扩展条件 |
| --- | --- | --- |
| Skill | 一个用户入口、范围控制、能力检查、执行编排和结果汇报 | 收集产品本身出现新的独立用户意图 |
| route/recipe | 某来源的授权范围、工具顺序、内容形态、限制和失败恢复 | 来源增加新形态或新的真实取得路径 |
| adapter | Rust `SourceAdapterPort` 的窄实现，向统一 Collector 提供来源读取 | 现有工具/Skill 反复证明缺稳定批量、重试或恢复能力 |
| case | 一次真实 PDF/MHT、`.notes`、会话或附件范围及其验收证据 | 用于证明或收紧 recipe，不形成产品入口 |

成熟的浏览器、飞书或桌面控制 Skill 是 recipe 可选择的执行依赖，不拥有 C0。recipe 可以
先由 Agent 按真实证据执行，也可以调用已有 adapter；两者最终都必须经 Rust application/core
进入同一 C0。任何 recipe 都不得调用 Process 作为收集完成条件。

路线治理按稳定、准确、真实、速度排序。前三项由真实数据和重采证据证明后，recipe 记录
耗时与唯一主要瓶颈，并删除没有失败证据支撑的并行猜测路线。允许在 acquisition 层复用
浏览器会话、页面导航或网络捕获生命周期；`CollectorItem`/候选仍是完整性判断、失败隔离、
C0 事务和重采的最小边界。整批 convenience command 不能成为一个外部抖动中断全部验证的
故障边界。

应用层的最小内部请求/结果概念如下，名称可以随实现演化，但责任不能越界：

```text
CandidateSummary       展示候选所需的来源上下文、公共字段和限制
CollectionSelection    用户明确选择的范围与授权
AcquisitionPackage     原件、附件、公共/原始 metadata、上下文、限制和适配器身份
CollectionResult       逐条 queued/running/saved/skipped/failed 及引用
RecollectionResult     changed/unchanged/inaccessible/removed 及旧新关系
ProcessRequest/Result  输入版本、pipeline、授权范围、运行和 C1 引用
KnowledgeRecord        人工记录/判断/分类/模型/评分/分析及版本
SuggestionReview       接受/修改/拒绝模型建议的非阻塞状态标记和可选新内容引用
SublibraryDefinition   人工选择、排除、组织规则和版本
OutputBuild            明确范围、格式、生成版本、manifest 和结果引用
```

豆包 Chrome-native 路线已有真实调用者，因此允许一个窄的 acquisition handoff：它只承接
官方页面响应中的会话信息、完整消息链、来源 URL，以及 Agent 已下载到临时目录的原件路径、
文件名、大小、MD5 和 SHA-256。CLI 在 discovery 时把已校验 handoff 作为 prefetched candidate
暂存，使后续独立 `select` 进程仍能走统一 Collector；Capture 前再次核对会话身份、消息唯一性、
分页终点和全部文件字节。临时路径只存在于待选候选，不进入 ready revision metadata；最终
指纹只使用稳定消息内容与附件 SHA-256。新的 handoff 可在 recollection composition root 中
替换旧 prefetched 取得结果，但仍由 application service 决定 `changed/unchanged`。

只在真实调用者出现时固化序列化协议。应用内部 Rust 类型不是承诺给多个未来消费者的
外部协议。

## 6. 代码与依赖架构

Rust 代码继续使用六个 crate 的单 workspace 结构：

```text
01_app/
├── 01_babata_domain/          # 领域类型、状态、版本与权威不变量
├── 02_babata_application/     # 四段用例、请求/结果和 port traits
├── 03_babata_infrastructure/  # SQLite、资产、来源、处理器、视图和备份实现
├── 04_babata_cli/             # 自动化、恢复、诊断和运维 composition root
├── 05_babata_local_api/       # 浏览器/窄本地 UI 的受保护 composition root
└── 06_babata_worker/          # C3 任务领取与处理 composition root
```

依赖方向固定为：

```text
domain <- application <- infrastructure
       ^                ^
       +--- cli / local_api / worker composition roots ---+
```

- `domain` 不依赖文件系统、SQLite、HTTP、provider SDK、CLI 或 UI。
- `application` 定义用例与所需 port，不导入具体数据库、文件系统、HTTP、SDK 或
  进程执行实现。
- `infrastructure` 实现持久化、资产、核心内稳定来源适配、处理 provider、读模型、视图和备份。
- CLI、local API 和 worker 只做鉴权、输入映射、依赖装配与结果映射，不复制业务规则。
- 任何 crate 都不能绕过 application 用例另建 C0/C1 写入路径。

### 6.1 应用 ports

架构需要以下责任边界；精确 trait、方法和文件由 P2 骨架蓝图维护：

| Port | 责任 |
| --- | --- |
| C0RepositoryPort | 来源、上下文、资料、版本、附件引用、第一方记录、人工知识记录、关系和子库定义的事务 |
| C1RepositoryPort | 处理运行、派生物、模型建议及其输入/输出溯源 |
| AssetStorePort | 暂存、按声明的完整性方法复制/哈希、最终落盘、打开和校验不可变资产 |
| JobRepositoryPort | C3 队列、租约、心跳、完成、失败、重试和取消 |
| SourceAdapterPort | 描述能力、只读发现候选、读取被选资料、报告覆盖和限制 |
| ProcessProviderPort | 描述、准备、提交、轮询、取消和获取处理输出 |
| ReadProjectionPort | 从 C0/C1 构建和查询可重建读模型 |
| OutputBuilderPort | 只读生成 C2、manifest 和验证报告 |
| BackupDriverPort | 一致快照、隔离恢复和哈希验证 |
| CapabilityRegistryPort | 返回能力状态、证据、限制和依赖条件 |
| ClockPort | 为可测试用例提供时间 |

来源适配器、处理 provider 和 output builder 都不能获得可写数据库连接或资产最终
落盘权限。

处理 provider 的模型选择必须可复现且可审计：运行记录使用具体可调用模型 ID 或快照，
并同时记录 provider、service/endpoint、地域、thinking/effort、CLI/API 版本、prompt/schema
版本、输入范围与 hash、输出 hash、usage、耗时、成本、fallback 和限制。未解析的泛化别名
只能作为用户界面输入或历史标签，不能作为实验或 C2 版本的唯一模型身份；任何 fallback
必须显式登记，禁止静默替换。模型目录、公开 benchmark 与 Babata 真任务评测分别记录为
能力证据、候选证据和采用决定，不能相互冒充。

处理请求和输出契约必须分开 control context 与 content payload。范围、试点状态、数据权威、
外部主权库职责、provider、成本、审阅状态和验收规则只进入 control context、manifest、YAML
或验证报告；课程概念、公式、框架、案例、问题和来源引用才进入 content payload。C2 builder
在物化前执行元话语污染检查，命中工作要求、系统自述或发布状态说明时拒绝生成知识正文。

C1 到 C2 的混合执行不采用“API 和 Agent 各自完整重读一遍”的默认双跑。批量 API/Skill 可
生成带引用的结构化 map；Agent 审校先读取 map、验证结果和被引用片段，只对失败 claim、
低置信或高价值范围回读对应 C1。课程级 reduce 消费已验证 map。若直接由 Agent 生成，则
该 Agent 是本次 producer，不再预先运行同范围 API；独立 critic 只核验抽样或失败范围。
provider 账分别使用 `qianwen_api`、`qianwen_skill`、`bailian_cli`、`codex_agent` 等实际身份，
不得把 Skill、CLI、服务和底层模型混成一个字段。

成本账至少汇总每次请求和整个 run 的输入、缓存、输出、推理和总 token，记录单价来源、
查询时间、币种、上下文计费阶梯、Batch/缓存折扣、估算金额与实际账单核对状态。没有单价
快照时可以保留精确 usage 并标为 `amount_pending_reconciliation`，但不能称为成本已完整记录；
Codex Agent 的订阅/席位成本与按量 API 费用分账，不从本地耗时或不可见 token 强行估算。

### 6.2 JavaScript / TypeScript 边界

在浏览器环境，或易变网站/JSON API 使用现成 TypeScript 生态能够明显缩短真实路径时使用。
已验证的成熟 CLI 优先于新增定制适配器；TypeScript 可以实现浏览器扩展、user script 或窄的
候选取得工具，但不形成第二套应用核心。例如浏览器边界为：

```text
浏览器扩展 / user script
  -> 读取当前 URL、标题、选区/页面、书签上下文和声明元数据
  -> 展示候选并取得明确选择
  -> 配对后提交 loopback API
```

TypeScript 边界不包含 SQLite driver、数据根写权限、最终资产管理、知识判断或独立权威队列。

### 6.3 Python 例外边界

只有成熟的 Python-only 工具明显优于 Rust crate、Rust 实现或稳定 CLI 时才使用受控
子进程。Python 读取明确授权的输入，只能写 C3 暂存区，并输出待校验的候选或处理
结果；Rust 核心负责哈希、ID、版本、资产最终落盘和 C0/C1 提交。

### 6.4 外围输入信任边界

TypeScript、Python、CLI 和其他外围工具的输出一律是不可信候选。Rust application/core
按当前候选合同校验协议版本、来源身份、内容、hash、路径和资产声明后，才允许进入 C0/C1
权威事务；未知版本、字段缺失、hash 不符、路径越界或不满足来源能力声明的输入必须失败，
不能降级为外围直接写库或落最终资产。

## 7. 数据根与持久化

运行时优先解析 `BABATA_DATA_HOME`，再使用显式本地配置；仓库只保存配置模板。
`BABATA_DATA_HOME` 是唯一活动产品数据根，不是通用项目工作目录或验收归档。其顶层除
最小本地说明外只允许以下编号分区：

```text
00_inbox/     用户明确放入的待处理文件与导出件；未收集前不是 C0
01_raw/       C0 索引、原件、附件、来源快照、隔离区和 manifests
02_derived/   C1 索引、派生文件和处理记录
03_views/     C2 搜索投影、子库物化、Obsidian、网页和导出物
04_runtime/   C3 队列、缓存、暂存、会话和受保护本地配置
05_logs/      C3 收集、处理、输出和运维日志
```

Git 外另有两个非权威辅助根：

```text
BABATA_EVIDENCE_HOME/  开发/验收报告、隔离数据根和必要历史快照；高敏感，不是正式备份
BABATA_RECOVERY_HOME/  已取得但尚未通过 Capture/C0 接管的来源恢复材料
```

辅助根不得位于 Git 仓库或 `BABATA_DATA_HOME` 内。证据根可以保存脱敏报告、迁移清单和
为了复核真实阶段结果而必要的 SQLite/媒体副本，但不能成为第二活动数据库，也不能被
Babata 正常检索或输出消费。恢复根中的资料仍是待收集材料；只有 Rust application/core
经 infrastructure 提交成功并完整读回后才成为 C0。二者的长期加密备份、保留和删除策略
由 P8 完成；在此之前不得把阶段快照冒充正式备份。

模型和本地预处理的可清理工作目录统一位于
`04_runtime/staging/model-workspaces/<task>/`。外围工具可以在这里写 C3 暂存，但正式 C1
仍只能通过 Process 用例进入 `02_derived`；工作目录路径不得登记为正式逻辑资产。

初始实现可以继续使用：

- `raw.sqlite`：C0 来源、上下文、资料、版本、资产引用、第一方记录、人工知识记录、
  关系和子库定义；
- `derived.sqlite`：C1 处理运行、派生物和模型建议；
- `runtime.sqlite` 或等价运行存储：C3 队列、租约、会话和能力运行状态；
- `03_views/search/index/search.sqlite`：P6.2 搜索/浮现 C2，由 `SqliteReadProjection` 从
  `raw.sqlite` 与 `derived.sqlite` 重建；
- 其他 C2 索引/文件：可从 C0/C1 与生成配置重建。

数据库记录只保存相对数据根的逻辑资产键。移动或隔离恢复数据根不需要批量改写
权威行内容。

### 7.1 P6.2 搜索投影

搜索投影统一投影 `raw_item` 与 `semantic_entry`：前者保留来源、当前观测、版本、资产和
C1 派生物导航，后者保留三大界、五类语义、地图/标签、评分 profile 与历史、建议来源和
审阅身份。投影只读附加权威 raw/derived 数据库；clear、populate、文件缺失标记、FTS 和
metadata 写入在同一事务中完成，失败时保留上一个完整投影。delete 只删除该 SQLite 及其
WAL/SHM，不改变 C0/C1 或知识核心。

投影 metadata 保存 schema、构建时间、raw/semantic/relation 行数和 source fingerprint。
fingerprint 覆盖 item、revision、asset、source observation、semantic/map/assignment、score/
profile/review、raw/semantic relation、process run 与 derivative 内容，用于核对重建输入；它
不是新的权威版本号。

`ExploreService` 统一提供 search、show、traverse 与 surface。search 支持正文、来源/provider、
时间、类型、状态/access state、人物、地图、标签、关系、处理状态、origin/review、媒体/
附件/受限/缺失、profile 和三维/综合分条件及排序；结果携带来源定位、版本、资产、派生物、
地图、标签、评分历史、限制、关系状态和 provenance。surface 只主动返回有合格评分的
semantic entry，并为每项返回 direction、relevance、time、relation 四类解释；rejected 或
modified 的原建议仍可搜索和回看，但不再主动浮现。CLI 的 `explore rebuild/delete/status/
search/show/traverse/surface` 与本地 API `POST /v1/explore/search` 调用同一应用服务。

该投影只完成 P6.2 的发现、检索和导航，不包含 P6.3 的 SublibraryDefinition、子库物化或
通用输出 builder。

### 7.2 P6.3 子库与输出视图

版本化 `SublibraryDefinition` 正文通过 Workspace first-party C0 路径进入 `raw.sqlite`；
raw v7 只对 `babata.sublibrary/v1` JSON 生效，校验 first-party 来源、definition version 与
revision ordinal、父版本连续性，并允许 pending/quarantine/ready 状态转换但禁止定义正文
原地改写或删除。应用层按 P6.2 `ReadProjectionPort` 解析查询规则，再应用人工 include/
exclude 和 `include_unreviewed` 策略；exclude 优先，组织规则产生确定顺序和可读 key。

`SublibraryViewStore` 只接收已解析的只读成员文档，生成
`03_views/sublibraries/<id>/v<version>/materialization.json` 与 manifest。`OutputViewStore` 只接收
显式记录集合或固定子库版本，当前只启用 Markdown 与结构化 JSON；每个 output 目录保存
artifact、current manifest 和 rebuild history。manifest 保留权威 item/revision/semantic、
输入 hash、来源定位、机器/人工与审阅身份、builder/template/profile、状态、限制、输出 hash
和 generation 差异。verify 不写权威资料；delete 只移除 artifact 或物化目录；rebuild 重读
同一明确 scope。Web/Obsidian 未实现并由 capability registry 保持 unavailable。

CLI 与 local API 组合相同的 `SublibraryService`、`OutputService`；C2 ports 不持有
`RawRepositoryPort`、`KnowledgeCoreRepositoryPort` 或 `DerivedRepositoryPort`，因此不存在
从生成文件反写 C0/C1 的能力路径。

### 7.3 写入与故障边界

一次 C0/C1 提交遵循：

```text
校验输入和授权范围
  -> 暂存并计算哈希
  -> 开启短事务
  -> 写入版本、关系、溯源和资产清单
  -> 原子最终落盘
  -> 提交终态
```

失败时清理暂存或留下明确可恢复 journal，不把半成品展示为 `saved`/`succeeded`。
SQLite 使用外键、WAL、有限 busy timeout 和短写事务。初始拓扑只有一台活动写入
机器；NAS/云端保存快照或恢复副本，不挂载为实时多写数据库。

运行中的 `queued`/`running` 和 worker 租约属于 C3；最终收集结果、来源变化与处理
运行溯源分别归档到 C0/C1，清理队列不会抹去已完成历史。

## 8. 收集架构

### 8.1 来源能力状态

每条来源路径分开维护证据成熟度和 runtime 状态：

```text
route evidence: E0 -> E1 -> E2 -> E3
runtime status: absent | disabled | enabled | unavailable(reason)
```

代码、fixture 或工具名不能单独达到 E3。只有用户授权的真实路径验证了适用的候选发现、内容、
上下文、附件、限制、失败与重收集，才可能达到 E3；E3 仍不自动把 runtime 切为 `enabled`。
能力状态由核心登记，适配器不能自报成功。当前逐来源证据、状态和缺口只由 `DOC-ROUTES`
维护，实际执行以 `babata --json capabilities list` 为准。

### 8.2 收集路线选择顺序

完成逐来源工具调查后，当前范围的执行路线按以下顺序选择：

1. 官方免费批量迁移或导出；
2. 现有可用插件或脚本导出；
3. Agent 主导的省心导出；
4. 少量开发后的批量导出；
5. 收费会员/VIP；
6. 重开发或复杂工具流；
7. 需要持续人机交互配合的路线；
8. 只能手工操作的路线。

路线序号描述用户实际付出的摩擦和工程成本，不等于具体技术类型：官方 API、CLI、SDK、
Agent 浏览器、MCP 或站点工具要按它们在当前范围内真正提供的能力归入对应层。选用较低层
时，路线证据必须写明更高层为何不可用。收费 VIP 只进入成本收益比较，不获得自动购买或
启用权限。

第三级 Agent 路线已经真实取得当前范围后，不自动生成来源适配器开发任务。需要重复执行
时先把已验证步骤组织为 Skill 或薄调用；只有真实重复使用暴露稳定性、批量、重试或恢复
缺口时，才进入第四级及以后的开发路线。

不为了来源数量先造重型爬虫，不绕过访问控制。稳定架构不写死“首批 enabled 来源”；浏览器
只是工具层，不能替代其他点名平台的 `source_id`、收藏/会话范围和真实证据。当前具体来源、
工具与限制只在 `DOC-ROUTES` 和 runtime capability 中维护。

选择任何路线前必须完成 `03_d_SOURCE_ROUTE_REGISTRY.md` 的逐来源调查并保留官方文档、
项目维护状态、实际调用、最小授权、数据覆盖和限制证据。网页登录来源必须先证明通用
Agent 浏览器路线的可行或不可行，不能未经验证就为每个站点另写爬虫。provider 文件、
候选协议、本地 fixture 和“可能可用”的工具名都不能代替调查。现有工具能够完成时，
adapter 只负责调用和规范化；不能为了统一内部形状重写已有工具能力。

### 8.3 重收集判断

来源原生标识与来源上下文用于找到既有资料，内容/附件哈希与可达状态用于判断
`changed`、`unchanged`、`inaccessible` 和 `removed`。文本相同只代表可能重复，
不删除新的收集事件；来源变化也不覆盖历史版本。

## 9. 清洗与处理架构

处理由可版本化 pipeline 编排。pipeline 可以组合机械提取、本地工具和模型步骤，
但每一步都必须产生独立状态和溯源，且只声明自己能够产生的 C1 类型。

首个多模态 provider 是百炼 CLI；百炼/通义 API 用于后续队列和批处理。两者实现同一
`ProcessProviderPort` 责任，并记录可得的可执行文件/模型版本、规范化参数、provider
任务标识、输入/输出哈希、错误、重试和成本。

隐私策略在任何字节离开本机前执行。未获批准的资料保持 C0，并以 `skipped` 或受限
状态结束，不静默上传。视频和音频处理保留原媒体；关键帧、字幕、转写和视觉描述
分别登记，不能压成一个唯一权威文本字段。

## 10. 核心沉淀架构

核心区不等同于 `create/revise/annotate` 三个写作命令。P6 的完整产品和领域基线见
`03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md`；总架构至少承载以下领域概念：

```text
FirstPartyContent      笔记、草稿、反思和正文版本
Annotation             独立批注及其目标版本
Judgment               人工判断、依据和版本
Relation               人工或建议关系；人工与机器来源可辨别
WorldviewMap           时间/空间/物质/意识 -> 学科 -> 分支的版本化地图
Classification         跨根、跨学科、跨层级的资料归属
Knowledge              经过消化的理论知识单元
Case                   现实中的实践标本，可与 Knowledge 相互印证
Log                    长期/中期/短期/实时的时间切片
Insight                第一方灵感、框架或成熟思考
KnowledgeModel         用户建立的主题、结构或其他模型及版本
RelevanceProfile       兴趣/战略/共识的版本化权重，默认 40/35/25
Score                  三维分量、综合值、依据、时间和 profile 版本
DenseRepresentation    文本脑图、Mermaid、模型、公式、清单、流程或提纲
Analysis               分析记录、引用证据和版本
ModelSuggestion        C1 机器建议
SuggestionReview       用户接受、修改或拒绝建议的非阻塞状态标记
```

这些概念先以简单、可演化的记录和关系实现，不预先设计万能知识图谱或宏大 ontology。
人工内容和机器建议使用不同类型与权威级别；审阅标记不改变机器记录的作者或类型，
只有用户实际写出新内容时才产生新的 first-party C0。

世界观地图采用固定四根和可演进下层：时间、空间、物质、意识是顶层稳定标识，学科
和分支可以增删改并保留版本；同一内容通过独立归属关系连接多个节点。改变四根本身
必须产生新的世界观模型版本并显式迁移归属，不能用目录重命名静默改写历史。

相关度综合分由 versioned profile 计算。首个 profile 使用
`0.40 * interest + 0.35 * strategy + 0.25 * consensus`；原始分量、依据、计算时间和
profile id 分别保存。真实使用
数据可以形成调权建议或新 profile，但不能在后台无痕改权重或重写历史分数。

高密度表达首先保存为可搜索、可 diff、可版本化的文本；脑图图片等渲染结果属于 C2，
可删除重建，不成为内容存在的前提。

核心工作需要读取原件、派生物、来源、版本和关系的聚合详情。读模型可以为体验优化，
但写入仍通过 Workspace/Knowledge 用例进入 C0。具体审阅和建模 UI 保持开放，直到
raw-to-view 闭环证明最有价值的交互。

## 11. 检索、子库与输出架构

Explore 从 C0/C1 构建可重建读投影，支持正文、来源、时间、类型、状态、人物、人工
分类、关系和处理状态。没有 OCR/转写的媒体资料仍通过 C0 元数据、附件和关系进入
索引；索引缺失不等于资料缺失。

Explore 同时支持主动查询与可解释浮现。浮现可以使用三维相关度、当前方向、时间、
地图归属和关系，但结果必须说明主要原因并暴露人工/机器/未审阅状态；它不能成为
隐藏地改写分类或评分的第二个写入者。

`SublibraryDefinition` 是带版本的 C0 人工资料，保存选择范围、人工纳入/排除和组织
规则。子库 builder 根据定义产生 C2 物化结果。删除物化目录不会删除定义或成员的
C0/C1。

Output builder 接收明确范围与输出类型，只读权威资料并生成：

```text
输出文件或结构化结果
生成 manifest
输入资料与版本引用
builder / 模板 / 配置版本
成功、限制和错误状态
```

Obsidian、网页、报告、卡片和应用调用只是 builder 类型，不是新的存储权威。输出
文件的外部修改默认不导回核心；未来若出现真实回写需求，必须作为新的明确输入和
第一方版本重新进入统一链路。

## 12. Skill、自动化与本地入口

Capability registry 统一报告来源、处理、核心、输出、Skill 和运维能力的真实状态。
Skill 规格可以提前存在，但只有底层用例通过对应验收后才成为可用 Skill。

所有自动化请求都携带明确范围、调用身份和确认/授权信息。默认策略是：

- 日常收集由用户在上下文中触发；
- 批处理由用户选择范围后触发；
- 模型建议不自动升级为人工判断；
- 定时任务和 Agent 不自动扩张授权范围；
- 任一入口得到与核心一致的结果状态和引用。

CLI 暴露自动化、恢复、诊断和运维能力，也可作为底层功能调试入口，但不要求普通
日常收集者手填内部路径和元数据。loopback API 只绑定 `127.0.0.1`/`::1`，使用安装
级本地凭据、来源限制和请求大小限制，并直接调用相同 application 用例。

## 13. 备份、恢复与完整性

BackupDriver 通过数据库一致快照机制复制索引，冻结本次资产清单和哈希，再交给加密
增量备份、NAS 或云端副本。优先级是 C0 > C1 > C2/C3；C2/C3 可按策略省略并在恢复
后重建。

恢复必须写入隔离数据根，完成数据库打开、迁移兼容、资产存在性和抽样/全量哈希
验证后，才允许切换为活动数据根。恢复报告区分：

- C0 缺失或损坏；
- C1 可重建但当前缺失；
- C2/C3 尚未重建；
- 凭据需要重新授权。

实时数据库目录不直接作为同步盘；备份系统消费一致快照，避免制造多个活动写入者。

## 14. 验收标准到架构的追溯

| 验收 | 架构责任 |
| --- | --- |
| AC-01 | CollectorSession、SourceAdapter、Selection、Capability registry、窄本地入口 |
| AC-02 | CollectionResult、RecollectionResult、C3 状态与 C0 终态溯源分离 |
| AC-03 | C0/C1/C2 类型边界、不可变资产、输入/输出关系和隔离状态 |
| AC-04 | Process、Job、ProcessProvider、版本化 pipeline 和受控暂存 |
| AC-05 | Knowledge 用例、个人知识宇宙语义、ModelSuggestion 与 SuggestionReview |
| AC-06 | Workspace、first-party 版本图和独立 Annotation |
| AC-07 | Explore 读投影、关系导航、SublibraryDefinition 与 C2 物化 |
| AC-08 | OutputBuilder、明确范围、manifest、只读生成和可重建 C2 |
| AC-09 | Capability registry、统一 application 用例、范围授权和外围无写权 |
| AC-10 | BABATA_DATA_HOME、唯一 Rust writer、C0-C3、BackupDriver 与隔离恢复 |
| AC-11 | 同一 composition root 下贯通收集、清洗、核心、检索/子库、输出和恢复 |

## 15. 架构补充文档的继承关系

- `03_b_P2_SYSTEM_SKELETON.md` 负责 P2 目录、文件、service、port、命令、API、
  worker、工具与测试位置。它必须补齐 CollectorSession、Knowledge、Sublibrary 和
  Output 责任；旧的 8 service/11 port/117 文件清单若与本文冲突，以本文为准并重新
  计算，不为保持数字而漏掉产品能力。
- `03_c_P3_RAW_FOUNDATION.md` 负责 P3 C0 原始入库架构。稳定 phase/gate 定义只在
  `DOC-PROCESS`，可重复验证只在 TC；不得把 C0 永久缩窄
  成“导入表”；后续人工知识记录继续使用同一权威与版本原则。
- `03_d_SOURCE_ROUTE_REGISTRY.md` 负责逐来源现有工具调查、实际证据、最小授权和路线
  决策。没有该证据，不允许用 adapter、协议或手工导出替代来源规划。
- `03_e_PERSONAL_KNOWLEDGE_UNIVERSE.md` 负责 P6 的语义模型、三维相关度、
  三级地图、高密度表达、非阻塞建议、浮现/检索、子库和输出设计基线；实现/状态不由该文维护。
- `03_f_EXTERNAL_SOVEREIGN_NAVIGATOR_CANDIDATE.md` 只保存尚未采用的外部主权 navigator、snapshot
  和 promotion 兼容口候选；外部主权库继续拥有原件和原生结构的已采用原则由 Requirements/本文
  拥有，不能因 candidate 文件存在而降级。
- `03_g_C1B_C2B_MODALITY_LADDER.md` 是已采用的 C1A/C1B、C2A/C2B 架构补充；运行批次和
  当前课程状态不在该文维护。

补充文档不能新增产品决定，也不能以阶段已实现的局部能力覆盖 00–03 的全局边界。

## 16. 保持开放的架构决定

以下决定仍等待足够真实使用证据：

- 核心区审阅、关联、建模和长期管理的具体 UI 技术与交互形态；
- 输出能力是否长期留在同一应用，或在出现独立部署/消费者后拆分；
- 哪些人工触发可以升级为人工确认或受控定时运行；
- 哪些本地 API 路由确实有浏览器扩展或窄 UI 调用者。

这些开放项不能用未验证的复杂协议、空服务或候选工具替用户提前做决定；当前使用状态和
来源优先级由 usage status/source research 管理，不属于架构开放项。

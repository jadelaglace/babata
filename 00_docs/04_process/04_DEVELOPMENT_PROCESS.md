# Babata 开发流程与实时进度

本文是 Babata 唯一的实时阶段状态和交付顺序来源。00–03 定义为什么做、做什么、
怎样算产品完成以及技术边界；架构补充定义文件和阶段设计；本文只维护现在到哪、
下一步是什么、通过哪道交付门才能进入下一阶段。

## 1. 当前状态

**更新时间：2026-07-18**

```text
P0  冻结旧版本                                    已完成
P1  真实需求、PRD、产品验收、全局技术架构           已完成
P2  全系统模块、目录、代码与工具骨架                 已完成
P3  C0 原始资料与第一方版本底座                     已完成
P4  飞书与浏览器首批真实收集路径                     进行中
P5  C1 多模态清洗与百炼处理                         未开始
P6  核心沉淀、检索、子库与输出                      未开始
P7  扩展来源、正式 Skill 与受控 Agent               未开始
P8  备份、恢复、运维与长期加固                      未开始
```

<!-- P2: completed; P2-G1..P2-G7: passed -->
<!-- P3: completed; P3-G1..P3-G6: passed -->
<!-- P4: in-progress; Kimi/Doubao/Bilibili/Feishu/ChatGPT/Zhihu/Xiaohongshu/Yuque small real loops proven; routes disabled -->

当前真实情况：

- P2 已在旧 117 文件基础上补齐 20 个 Rust 责任文件和 3 份 Skill 规格，达到 6 个
  crate、137 个 Rust 源文件；CollectorSession、Knowledge、Sublibrary、Output、
  ReadProjection 和 OutputBuilder 均有明确位置与 unavailable 壳。P2-G1 至 P2-G7
  已全部通过，P2 已完成。
- 逐来源现有工具调查和路线决策已经写入 `03_architecture/08_SOURCE_TOOL_RESEARCH.md`；
  00 点名的来源都有证据等级、最小授权、正常路线、回退和诚实缺口。飞书 `lark-cli`、
  Browser Use、Agent Browser、Playwright CLI、OpenCLI 和 Codex Chrome 均有实际调用或
  连接证据。具体来源 E3 仍属于 P4/P7，不再错误作为 P2 前置。
- Kimi、豆包、Bilibili、飞书 Docx、ChatGPT、知乎回答、小红书收藏和语雀文档已分别完成一个真实小范围的候选、明确选择、C0、逐条状态和
  重收集闭环；Bilibili 另把 44,773,539 字节原视频作为 C0 资产保存并复核 SHA-256。
  飞书样本另保存 3,391 字符 XML 正文和 8 张真实 PNG；ChatGPT 样本保存 2 条角色消息和
  10 个引用。来源仍保持 disabled：Kimi/ChatGPT 当前样本无附件，豆包二进制媒体未闭合，Bilibili 按用户要求只证明一条，飞书嵌入
  Sheet/Base/Slides/画板内部数据仍未覆盖。P4 正在进行中，不把局部闭环扩大成来源 available。
- P3 已按蓝图重新审阅 29 个活跃文件：显式 text/file/export 和 first-party
  create/revise/annotate 通过同一 Rust application/infrastructure 链路进入 C0，返回包含
  来源、上下文、版本、关系、资产状态、哈希和 operation provenance 的 repository read-back。
  P3-G1 至 P3-G6 已全部通过，P3 已完成。
- stage、graph transaction、finalise、hash verify、ready transition、post-ready read-back
  和 cleanup 故障均有负向测试；
  失败不会伪报 ready，跨 SQLite/文件系统故障保留 quarantine、journal/orphan 诊断，
  CLI 错误携带可关联 operation ID，已被 ready 记录引用的 content-addressed bytes 不会被移动。
- 飞书手动导出、书签 HTML、CandidateEnvelope、route evidence fixture 仍只是回退/机制
  证据；飞书官方 `lark-cli` 的 Wiki -> Docx -> 媒体 -> C0 路径已有独立真实证据。
  P4 migration 已与 P3 raw migration 分开；未完整覆盖的 route/capability 继续 disabled。

项目阶段只使用 P0–P8；C0–C3 是数据权威级别，不是项目阶段。

### 1.1 人话进度地图

```text
已经真的收进 Babata，而且重采过
  Kimi      15 个真实候选 -> 选 1 条 -> 1 条资料/1 个版本 -> 重采没变化
  豆包      20 个真实候选 -> 选 1 条 -> 1 条资料/1 个版本 -> 重采没变化
  Bilibili  20 个真实历史 -> 选 BV1ogzsBFE1T
             -> 正文 + 官方字幕 + 官方摘要 + 44.8 MB 视频
             -> 1 条资料/1 个版本/1 个附件 -> 重采没变化
             -> 按用户要求到此闭合，后续按用户选择再收
  飞书      “一堂”10 个根候选 -> “AI分享”6 个子候选
             -> 选 240612AI落地Live21-AMA特别篇
             -> 3,391 字符正文 + 8 张 PNG
             -> 首次媒体结构不兼容而 failed -> 原任务 retry 成功
             -> 1 条资料/1 个版本/8 个附件 -> 重采没变化
  ChatGPT   正式 Chrome 展开最近聊天，看到至少 28 个真实入口
             -> Babata 按 recent:20 列出 20 个候选，只选“开源部署方案对比”
             -> 2 条角色消息 + 10 个引用；页面 favicon 不冒充附件，真实附件为 0
             -> 1 条资料/1 个版本/0 个附件 -> 重采没变化
  知乎      正式 Chrome 登录后列出 16 个自建收藏夹
             -> 最新“我的收藏”页面标称 28 条，分页命令返回 27 个去重候选
             -> 只选最新回答；完整正文 + 原始 HTML + 17 张正文原图（8.41 MB）
             -> 1 条资料/1 个版本/17 个附件 -> 重采没变化
  小红书    正式 Chrome 登录后读取 20 个真实收藏候选
             -> 选“捉住一只小仙兔” -> 正文/标签/互动 + 2 个媒体（10.16 MB）
             -> 1 条资料/1 个版本/2 个附件 -> 重采没变化
  语雀      正式 Chrome 登录后看到 2 个知识库、8 个最近文档
             -> 选“粒界引擎-车辆材质质感提高方式”
             -> 免费官方 Markdown + 渲染正文/HTML + 22 张图片（3.10 MB）
             -> 1 条资料/1 个版本/22 个附件 -> 重采没变化
             -> 会员 OpenAPI/MCP 只登记，全部来源闭环后统一决策

正在往下闭合
  微信收藏/公众号/聊天
             先复核新出现的专用本地 CLI，再决定官方 PC 微信窄适配

后续队列
  OneNote -> 印象笔记

靠后处理
  抖音

最低优先级
  视频号（用户最新明确降到最低）
```

这里的“真的收进”只表示上述明确小范围已经进入 C0 并有重采证据，不表示账号全量、
附件全覆盖或来源已 `available`。真实资料和 SQLite 均在 `BABATA_DATA_HOME`，不进入 Git。

## 2. 状态维护规则

1. 状态只使用“未开始、进行中、已完成、阻塞”；提前代码写在说明中，不改变阶段。
2. 阶段状态变化必须与对应文档、代码和验证证据在同一提交中更新。
3. 局部实现、旧测试通过、文件已经存在或接口能够返回，不自动推动阶段。
4. P2 使用工程交付 gate；P3 以后按 phase gate 和对应 AC/TC 判断，二者不得混用。
5. 产品意图变化按 `00 -> 01 -> 02 -> 03 -> process -> tests -> code` 顺序传播。
6. 架构补充与主架构冲突时先改补充和骨架，再改代码；不为保留旧数字扭曲产品。
7. `AGENTS.md` 只提供本地操作上下文，不是产品、架构或进度权威。

## 3. P0：冻结旧版本

旧版本保留在 `C:\Users\Aiano\Babata-2.0-frozen`，不在 reboot 工作区继续演化。
P0 已完成。

## 4. P1：真实需求到全局架构

P1 交付链：

```text
00_REQUIREMENTS.md（含 append-only 原话证据）
  -> 01_PRD.md
  -> 02_ACCEPTANCE_CRITERIA.md
  -> 03_ARCHITECTURE.md
```

P1 当前已完成：00 恢复真实意图，01 恢复四段产品行为，02 将 PRD-01..10 映射到
AC-01..11，03 明确四段信息流、C0–C3、唯一 Rust writer 和代码边界。

后续若真实意图变化，P1 文档按链路重新打开；不能在 process 或 code 中偷偷新增
产品决定。

## 5. P2：全系统骨架

### 5.1 P2 目的

在单一模块深入实现前，建立修正后全系统的完整位置：

- 6 个 Rust crate、137 个目标 Rust 源文件；
- 12 个 application service、13 个 port；
- 13 个 CLI 命令模块、受保护 local API 路由模块和 worker 生命周期；
- 浏览器/Python 边界；
- 9 份 Skill 规格；
- C0/C1/C3 migration、测试、脚本和配置位置；
- 每个能力的 owner、允许/禁止依赖和激活阶段。

完整清单见 `03_architecture/04_SYSTEM_SKELETON_BLUEPRINT.md`。

### 5.2 P2.1：文档和目标清单

1. 以 00–03 为上游，修正 04–07 架构补充、开发流程和测试映射。
2. 固定旧 117 文件之外新增的 20 个责任文件。
3. 固定 service/port/CLI/local API/worker/Skill 的所有权。
4. 区分产品 AC/TC 与 P2 工程 gate。

完成证据：文档追溯检查覆盖 PRD-01..10、AC-01..11、TC-01..11；下游不存在旧
`AC-11 = 117 文件` 或 `P4 = 导出导入` 的表述。

### 5.3 P2.2：代码与外围骨架对齐

1. 保留现有 117 文件，不 reset、checkout 或盲目删除用户工作。
2. 添加蓝图列出的 20 个责任文件，目标达到 137。
3. 添加 Knowledge、Sublibrary、Output Skill 规格位置。
4. 更新 module export、DTO、capability descriptor 和 unavailable 壳。
5. 不在此步骤实现真实来源、模型、知识算法、搜索排序和输出模板。

完成证据：P2 inventory 检查报告 6 crate、137 文件和外围规格位置齐全。

### 5.4 P2.3：接口和 composition roots

1. 新增 CollectorSession、Knowledge、Sublibrary、Output service 壳。
2. 新增 ReadProjectionPort 和 OutputBuilderPort。
3. 扩展 RawRepositoryPort 的未来责任但不提前实现 P6 SQL。
4. CLI 添加对应模块；local API 只添加路由 owner，不固定没有真实调用者的 endpoint。
5. worker、browser、Python、provider、view/output builder 全部只调用 application 用例。

完成证据：interface ownership 和 Rust boundary 检查使用新清单；无万能 service、
反向依赖或第二 C0/C1 写入者。

### 5.5 P2.4：工程 gate

必须同时通过 `04_SYSTEM_SKELETON_BLUEPRINT.md` 的工程门：

| Gate | 本阶段判定 |
| --- | --- |
| P2-G1 | 6 crate、137 文件和外围规格位置齐全 |
| P2-G2 | service、port、CLI、API/worker owner 完整 |
| P2-G3 | 依赖单向、workspace 可编译 |
| P2-G4 | 未激活能力诚实 unavailable |
| P2-G5 | 只有 Rust application/infrastructure 可最终写 C0/C1 |
| P2-G6 | 文档、蓝图、脚本和测试追溯一致 |
| P2-G7 | 00 列出的来源都有证据等级、最小授权、路线决策和诚实缺口；当前可调用的代表性官方/通用工具有实际证据 |

```text
check-p2-skeleton-inventory.ps1
check-rust-boundaries.ps1
check-interface-ownership.ps1
check-doc-traceability.ps1
test-doc-traceability.ps1
check-no-secondary-writer.ps1
cargo metadata / check / fmt / clippy / architecture tests
```

这些 gate 证明骨架完整、依赖正确、能力诚实和写入边界唯一。它们不证明任何产品
AC 已完成。

### 5.6 P2 完成证据（2026-07-18）

- 6 个 crate、137 个 Rust 源文件、12 个 application service、13 个 port、13 个 CLI
  命令模块、local API route owner、worker 生命周期和 9 份 Skill 规格位置全部存在；
- `cargo check --workspace`、`cargo fmt --all --check`、`cargo clippy --workspace
  --all-targets -- -D warnings` 通过；
- `cargo test --workspace` 通过 41 个测试；
- P2 inventory、interface ownership、document traceability、document traceability mutation、
  Rust boundary 和 no-secondary-writer 检查全部通过；
- 新增 CollectorSession、Knowledge、Sublibrary、Output、ReadProjection 和
  OutputBuilder 只提供边界与 unavailable 壳，没有业务算法；
- 离线 route evidence 可以记录覆盖，但不能单独把飞书/浏览器标记 enabled；来源仍
  等待 P4 真实上下文候选与选择证据；
- 来源工具调查已覆盖 00 点名的全部来源。已实际核验本机 `lark-cli 1.0.68` 的用户
  OAuth 和 Wiki/Docs 只读调用；已安装并运行 `agent-browser 0.32.1` 的
  version/help/doctor，doctor 7 pass、0 warn、0 fail；Browser Use 0.13.6 / Browser
  Harness 0.1.6 已安装并通过正式版 Chrome、daemon 和本地连接 doctor；两者分别连接
  正式版 Chrome 150.0.7871.129、列出真实 tab 并读取当前页。OpenCLI 1.8.6 已实际运行
  命令发现、站点 help 和 doctor，其确定性站点命令降为第二层；
- 已完成无需账号权限的工具准备：全局安装 Agent Browser 0.32.1、Playwright CLI 0.1.17、
  OpenCLI 1.8.6 和 `yuque-dl 1.0.85`，安装
  Microsoft Graph Authentication/Notes 2.38.1、`evernote-backup 1.13.1`、
  `yt-dlp 2026.07.04`，并确认本机已有 ffmpeg 8.1.1；OpenCLI 官方扩展 1.0.22 已按
  release SHA-256 校验并解压，但其 `<all_urls>`/cookies/debugger 权限必须由用户明确
  批准后才能安装；
- 已淘汰飞书手动 Markdown 主路线、已归档的 BBDown/bilibili-api-python 和已被 DMCA
  屏蔽的 `wx-cli`；语雀、OneNote、Evernote、微信和浏览器均已有明确直接使用、组合
  工具或窄适配决策；
- 第二轮豆包搜索、官方/项目文档和 GitHub 元数据交叉核验发现抖音旧路线失真：
  DouK-Downloader 当前加密参数算法已失效，扫码登录失效、浏览器 Cookie 读取弃用，
  因此撤回“扫码即可”的主路线；`F2` 改为待实证首选候选，但本机隔离安装尚未完成，
  抖音明确保持 E0/disabled，不要求用户手抄 Cookie 来冒充落实；
- 29 文件 P3 提前实现及 34 个 raw 功能测试继续可运行，但不作为 P2 产品验收，也不
  代表 P3 已开始。
- Codex Chrome 已在 Kimi 真实运行：历史页当前读取 65 个会话入口，确认
  `FeedService/ListFeeds` 每页 50 条并带 continuation token，确认 `GetChat` 与
  `ListMessages` 可取得完整会话；两条单会话样本已保存到外部 recovery staging，第二条
  包含 10 个结构化内容块、11 条引用和 104,476 字节正文响应。真实内容、签名 URL 和凭据
  均未进入 Git。该证据为 E2，不替代 E3 的附件、逐条状态和重收集。

上述证明 P2-G1 至 P2-G7 全部通过。P2-G7 证明逐来源调查、路线决策、最小授权和代表性
工具实证已经齐全，不证明任何来源已经 available。抖音等具体来源的 E3 缺口、Kimi 的
全历史/附件/重收集缺口继续留在 P4/P7，不能倒灌为 P2 或 P3 的前置条件。

### 5.7 P2 收尾时的阶段交接（历史）

P2 收尾时，Kimi 具体平台练手已证明 Codex 当前手段可用，当时的下一步是进入 P3，
重新审阅提前实现并完成唯一 C0 路径。P3 现已完成；以下存量回收边界继续有效：

1. Codex 先用官方连接器/Skill；没有直接能力时使用正式版 Chrome 已登录会话；只有桌面
   UI 无结构化入口时使用 Computer Use。
2. 回收结果写入 `${BABATA_DATA_HOME}/recovery-staging/<source>/<batch-id>/`，保留原始
   导出件/媒体、manifest、范围、取得时间、工具版本、hash、缺失和限制。
3. P3 核心可用后，经唯一 Capture/C0 链路校验提交；回收成功不把来源标记为 P4 available，
   也不替代逐条状态、重收集和长期自动化验收。

## 6. P3：C0 原始资料与第一方版本底座

前置：P2-G1 至 P2-G7 全部通过。

P3 按 `05_RAW_FOUNDATION_BLUEPRINT.md` 和 `06_RAW_FOUNDATION_EXECUTION_PLAN.md`
重新审阅已有 29 文件提前实现，完成：

1. 显式 text/file/export 的统一 C0 提交；
2. first-party create/revise/annotate；
3. raw SQLite、不可变资产、哈希、版本、关系与 read-back；
4. transaction、journal、orphan/quarantine 和故障补偿；
5. 临时数据根下的工程/恢复 CLI 验证。

P3 gate：

| Gate | 本阶段判定 |
| --- | --- |
| P3-G1 | 外部数据根与编号分区正确 |
| P3-G2 | text/file/export 形成可回读 C0 |
| P3-G3 | first-party create/revise/annotate 版本关系正确 |
| P3-G4 | 失败不产生伪 ready，journal/orphan 可诊断 |
| P3-G5 | DB/资产写入 owner 唯一 |
| P3-G6 | P2 gate 继续成立且未提前激活其他能力 |

### 6.1 P3 完成证据（2026-07-18）

- 全新临时数据根先报告 schema 0/unreachable，显式 text/file/export 后建立 schema 4；
  最终有 2 个哈希寻址原件，pending journal、orphan 和 quarantined revision 均为 0；
- text 的上下文 `manual-smoke`、file/export 的 role、logical path、SHA-256 和 ready 状态
  均从 `RecordDetail` 回读；输出中的 `operation_id` 与该次提交共用同一 operation；
- first-party create/revise 保留 v1/v2、parent 和 `revises` 关系；annotate 形成独立 item，
  并指向被批注的具体 ready revision；外部 revision 不能被 revise 成 first-party；
- 注入 ready transition 失败后，revision/asset 为 quarantined，最终原件仍在哈希路径，
  journal 和 orphan marker 各 1；共享 content-addressed bytes 不被移走；
- Issue #14 closeout 证明 text/create/revise/annotate 无资产失败仍有 operation journal、
  quarantined operation 和相同 CLI operation ID；post-ready read-back 失败返回 durable ready
  outcome 与 warning，不生成 `finalized_uncommitted`；重导入的两次 locator/native/timestamp/
  metadata 可分别从 revision provenance 回读且旧 wording/asset 不覆盖；
- P3 raw migration 只有 `0001..0004`；P4 route evidence 保存在独立 migration 目录且未应用。
  Candidate/provider route 命令返回 `capability_unavailable`，来源保持 disabled；
- `check-p3-raw-inventory.ps1` 报告 29 个活跃文件和 55 个 raw 功能测试；workspace
  共 63 个测试通过。P2 inventory、interface ownership、document traceability、Rust
  boundary 和 no-secondary-writer gate 持续通过；fmt、check、clippy `-D warnings` 通过。

P3 为 AC-03、AC-06、AC-10 提供部分底座，不满足 AC-01、AC-02 或完整 AC-11。

## 7. P4：飞书与浏览器首批真实收集路径

前置：P3 C0 写入和故障边界稳定。

P4 按 `07_P4_FIRST_COLLECTION_PATHS.md` 实现：

1. 飞书官方授权连接、文档/Wiki/知识库候选、层级和附件限制；
2. Browser Use/Agent Browser 复用已登录 Chrome 的自主导航探针，以及长期浏览器扩展
   配对、页面/选区/链接/书签候选；
3. 用户给出单条、可见集合、收藏夹、会话或明确范围一次后，Agent 自主完成范围内收集；
   未给范围或范围有歧义时不写 C0；
4. queued/running/saved/skipped/failed、局部成功和重试；
5. changed/unchanged/inaccessible/removed 重收集；
6. 真实授权证据与 fixture 机制证据分开。

当前已完成的局部真实证据：

- Kimi：验证根 `p4-kimi-20260718-172641`，15 个候选中选 1 条，C0 为 1 item/1
  revision，重采 `unchanged`；
- 豆包：验证根 `p4-doubao-fingerprint-20260718-174826`，20 个候选中选 1 条，选择前
  0/0、选择后 1 item/1 revision，重采 `unchanged`；
- Bilibili：验证根 `p4-bilibili-final-20260718-181500`，20 个观看历史候选中选
  `BV1ogzsBFE1T`，保存元数据、官方字幕、官方 AI 摘要和 44,773,539 字节 MP4；最终
  1 item/1 revision/1 asset，资产 SHA-256 为
  `35551288f33a21c9ea5b75f69dd578521f9f76a2b79b9a2448d4f33bf2f26d22`，重采
  `unchanged` 且版本/资产数量不增加。
- 飞书：验证根 `p4-feishu-20260718-184000`，官方 user OAuth 下发现私有知识空间
  `一堂` 的 10 个根候选和 `AI分享` 的 6 个直接子候选；选择
  `240612AI落地Live21-AMA特别篇` 后，首次因真实 XML 使用 `src/href` 而进入可重试
  `failed`，兼容后对原 item retry 成功。最终保存 3,391 字符 XML 正文、8 张 PNG，
  1 item/1 revision/8 assets；下载件与 C0 资产逐个 SHA-256 一致，重采 `unchanged`，
  版本/资产数量不增加。
- ChatGPT：验证根 `p4-chatgpt-20260718-190000`。正式 Chrome 已登录，展开最近聊天后
  可见至少 28 个真实入口；Babata 以 `recent:20` 发现 20 个候选，只选择“开源部署方案
  对比”，保存 2 条角色消息、10 个引用，当前样本真实附件为 0。最终 1 item/1 revision/
  0 assets，重采 `unchanged` 且版本数不增加；二进制附件下载仍无非零样本，route 保持
  disabled。首次 OpenCLI 瞬时返回非 JSON 时 C0 保持 0，现已将此类响应归为可读的来源
  I/O 失败，不再误报 C0 integrity 损坏。
- 知乎：验证根 `p4-zhihu-final-20260718-203000`。正式 Chrome 登录后读取 16 个自建
  收藏夹；最新“我的收藏”页面标称 28 条，官方分页命令实际返回 27 个去重候选（12 个
  回答、15 篇文章）。只选最新回答，保存完整正文、原始 HTML 和 17 张正文原图；最终
  1 item/1 revision/17 assets，17 个 SHA-256 均不同，总计 8,413,376 字节。首次验证发现
  图片 CDN 域切换会制造伪版本，改用稳定 `data-original-token` 后，干净验证根重采
  `unchanged`。文章、想法、视频和评论线程尚未覆盖，route 保持 disabled。
- 小红书：验证根 `p4-xiaohongshu-final-20260718-210000`。正式 Chrome 读取 20 个真实
  收藏候选，只选“捉住一只小仙兔”；保存正文、标签、互动数据和 2 个不同哈希的媒体，
  共 10,163,846 字节，最终 1 item/1 revision/2 assets，重采 `unchanged`。
- 语雀：验证根 `p4-yuque-official-20260718-225000`。正式 Chrome 看到 2 个知识库和 8 个
  最近文档，实测整库官方导出为 PDF/LakeBook，单篇免费提供官方 Markdown。只选“粒界
  引擎-车辆材质质感提高方式”，保存官方 Markdown、渲染正文/HTML和 22 张不同哈希图片，
  共 3,101,329 字节；最终 1 item/1 revision/22 assets，重采 `unchanged`。个人 OpenAPI
  和官方 MCP 需要超级会员，只登记并等待全部来源闭环后的统一决策。

浏览器仍是当前存量回收首选。Kimi/豆包/ChatGPT/知乎/小红书/语雀的 OpenCLI 薄命令是为了把浏览器已经证明的
读取动作变成任务结束后可调用的重试/重收集；Bilibili 是因为 Codex Chrome 历史页连续
两次超时后才回退 OpenCLI。两类理由均已记录，不把 OpenCLI 当默认绕路。

已有导出、书签 HTML 和 CandidateEnvelope 只作为回退/提前证据。P4 gate：

| Gate | 本阶段判定 |
| --- | --- |
| P4-G1 | 飞书真实上下文候选成立 |
| P4-G2 | 正式 Chrome 中 Kimi 真实会话候选与所选正文成立；通用 Agent 浏览器自主导航及长期扩展配对另行成立，不能用任意页面替代具体平台 |
| P4-G3 | 一次明确范围内可连续收集；未授权范围不写入 |
| P4-G4 | 逐条状态、局部成功和重试成立 |
| P4-G5 | 四种重收集结果不覆盖旧 C0 |
| P4-G6 | 真实证据与 fixture 分开，未验证来源保持 disabled |

只有 P4-G1 至 P4-G6 和 TC-01、TC-02 通过后，才能把首批来源标记 available，并
满足 AC-01、AC-02。

## 8. P5：C1 多模态清洗与百炼

前置：至少一条真实 C0 来源可稳定回看。

依次激活：

1. C1 schema、process run、job 和 derivative；
2. 文档/网页机械提取；
3. 百炼 CLI 的真实 pipeline；
4. 图片 OCR、音频转写、视频字幕/关键帧/视觉描述；
5. 百炼/通义 API 或批处理、隐私、成本、限流和重试；
6. 原件/派生物对照与转换损失说明。

P5 主要交付 AC-03、AC-04 和 TC-03、TC-04。C1 不覆盖 C0，模型输出不自动成为
人工判断。

## 9. P6：核心沉淀、检索、子库与输出

P6 必须按核心价值顺序进行，不能直接跳到 Datasette/Obsidian：

### P6.1 核心人工沉淀

- 聚合查看原件、派生物、来源、版本和关系；
- 人工记录、判断、关系、分类、主题/结构模型、评分和分析；
- ModelSuggestion 与人工接受/修改/拒绝分离；
- first-party 与人工知识记录的版本历史。

交付 AC-05、AC-06 和 TC-05、TC-06。

### P6.2 检索与关系导航

- C0/C1 可重建读投影；
- 正文、来源、时间、类型、状态、人物、分类、关系和处理状态检索；
- 媒体-only、附件-only 和受限资料仍可发现；
- 版本、来源、关系导航。

交付 AC-07 的检索和关系部分。

### P6.3 子库与输出

- 版本化 SublibraryDefinition；
- 可删除重建的子库物化；
- 人类可读和结构化输出；
- manifest、来源/版本回溯和只读 builder；
- Obsidian、网页、报告等在真实用途出现后逐项启用。

交付 AC-07、AC-08 和 TC-07、TC-08。

## 10. P7：扩展来源、正式 Skill 与受控 Agent

按真实价值扩展语雀、OneNote、印象笔记、微信、知乎、Bilibili、小红书、豆包/Kimi/GPT、
本地文件等来源；当前未闭合队列先处理微信收藏/公众号/聊天、OneNote 和印象笔记，
抖音靠后，视频号最低优先级。每条来源继续优先 Codex 浏览器、官方能力和现有工具，
不先造重型爬虫。

对应底层能力通过自己的 AC/TC 后，P2 Skill 规格才转成真实 `SKILL.md`。Agent 默认
人工触发或确认，批处理携带明确范围，不自动扩张授权或把模型判断升级为事实。

P7 主要交付 AC-09 和 TC-09。

## 11. P8：备份、恢复、运维与长期加固

实现一致快照、加密增量备份、NAS/云端副本、隔离恢复、hash 验证、日志轮转、doctor、
成本与故障监控。恢复报告区分 C0 损坏、C1 可重建、C2/C3 未重建和凭据重授权。

P8 完成 AC-10、TC-10，并与 P4–P7 的真实路径共同完成 AC-11、TC-11 的完整本地
raw-to-view 闭环。

## 12. 阶段与验收映射

| 阶段 | 主要产品验收 | 说明 |
| --- | --- | --- |
| P2 | 无产品 AC；P2-G1..G7 | 工程骨架与现有工具路线门，不冒充产品完成 |
| P3 | AC-03/06/10 的底座部分 | C0、first-party、单一写入；无真实来源/清洗/恢复 |
| P4 | AC-01、AC-02 | 首批真实上下文收集 |
| P5 | AC-03、AC-04 | 原件/派生物与忠实清洗 |
| P6 | AC-05、AC-06、AC-07、AC-08 | 核心先于检索/视图/输出 |
| P7 | AC-09 | 扩展来源、Skill、Agent |
| P8 | AC-10、AC-11 | 一致恢复与完整系统闭环 |

## 13. 提交与验收纪律

Babata 即使由单人开发，也使用 GitHub Issue 和 Pull Request 保留问题、范围、决策、验证
与合并记录。`main` 是可集成基线，不作为日常直接开发分支。

### 13.1 标准工作流

1. 先建立 Issue，写清背景、范围、非目标、验收条件和对应 phase gate/AC/TC；调查任务
   也要在 Issue 中写明要取得的证据，不能只有一个模糊标题。
2. 从最新 `main` 建立短生命周期分支，名称包含 Issue 编号和主题，例如
   `codex/issue-12-browser-probe`。一个分支服务一个可审阅目标，不混入无关改动。
3. 开发过程中按可恢复、可审阅的安全点多次 commit。提交说明具体结果；P2 按文档/清单、
   责任文件、接口、composition roots 和工程检查分组，不混入业务算法。
4. 推送分支并建立 PR。PR 必须引用 Issue，使用 `Closes #N` 或 `Fixes #N`，并写明变更、
   验证、风险、数据/凭据影响、文档影响和未完成项；草稿未完成时使用 Draft PR。
5. PR 中审阅实际 diff，完成适用的文档追溯、编译、测试和边界检查。检查失败、验收证据
   不足、混入真实数据/凭据或范围漂移时不得合并。
6. 检查和审阅结论成立后合并到 `main`，删除工作分支，由 PR 自动关闭 Issue。合并后才
   更新后续 Issue；不得用直接推送 `main` 绕过记录。

### 13.2 纪律

- 小型文档修正、研究结论、依赖升级和紧急修复同样走 Issue/PR；真正需要立即止损的
  紧急修复可以先开短 Issue 和最小 PR，但不能事后没有记录。
- P3 以后每项功能的 Issue、commit 和 PR 引用对应 phase gate 与 AC/TC。
- 功能阶段发现接口不对，先在同一 Issue/PR 更新 03 架构补充与 P2 蓝图，再改代码。
- 真实数据、授权信息、数据库、模型输出和日志不进入 Git 或 Issue/PR 附件。
- 未通过当前阶段门，不提前激活或宣告下一阶段。
- 工作树中已有用户改动默认保留；不 reset、checkout、删除或盲目提交。

# Babata 开发流程与实时进度

本文是 Babata 唯一的实时阶段状态和交付顺序来源。00–03 定义为什么做、做什么、
怎样算产品完成以及技术边界；架构补充定义文件和阶段设计；本文只维护现在到哪、
下一步是什么、通过哪道交付门才能进入下一阶段。

## 1. 当前状态

**更新时间：2026-07-21**

```text
P0  冻结旧版本                                    已完成
P1  真实需求、PRD、产品验收、全局技术架构           已完成
P2  全系统模块、目录、代码与工具骨架                 已完成
P3  C0 原始资料与第一方版本底座                     已完成
P4  飞书与浏览器首批真实收集路径                     已完成
P5  C1 多模态清洗与百炼处理                         已完成
P6  核心沉淀、检索、子库与输出                      进行中
P7  扩展来源、正式 Skill 与受控 Agent               未开始
P8  备份、恢复、运维与长期加固                      未开始
```

<!-- P2: completed; P2-G1..P2-G7: passed -->
<!-- P3: completed; P3-G1..P3-G6: passed -->
<!-- P4: completed; P4-G1..P4-G6 and TC-01..TC-02 passed; representative small real loops proven; incomplete routes remain disabled -->
<!-- P5: completed; TC-03A and TC-04 passed; AC-04 passed; full AC-03/TC-03 awaits P6 TC-03B -->
<!-- P6.1: completed; AC-05..AC-06 and TC-05..TC-06 passed; P6.2..P6.3 not started -->

当前真实情况：

- P2 已在旧 117 文件基础上补齐 20 个 Rust 责任文件和 3 份 Skill 规格，达到 6 个
  crate、137 个 Rust 源文件；CollectorSession、Knowledge、Sublibrary、Output、
  ReadProjection 和 OutputBuilder 均有明确位置与 unavailable 壳。P2-G1 至 P2-G7
  已全部通过，P2 已完成。
- 逐来源现有工具调查和路线决策已经写入 `03_architecture/08_SOURCE_TOOL_RESEARCH.md`；
  00 点名的来源都有证据等级、最小授权、正常路线、回退和诚实缺口。飞书 `lark-cli`、
  Browser Use、Agent Browser、Playwright CLI、OpenCLI 和 Codex Chrome 均有实际调用或
  连接证据。具体来源 E3 仍属于 P4/P7，不再错误作为 P2 前置。
- Kimi、豆包、Bilibili、飞书 Docx、ChatGPT、知乎回答、小红书收藏、语雀文档和微信收藏中的公众号文章已分别完成一个真实小范围的候选、明确选择、C0、逐条状态和
  重收集闭环；Bilibili 另把 44,773,539 字节原视频作为 C0 资产保存并复核 SHA-256。
  飞书样本另保存 3,391 字符 XML 正文和 8 张真实 PNG；ChatGPT 样本保存 2 条角色消息和
  10 个引用。来源仍保持 disabled：Kimi/ChatGPT 当前样本无附件，豆包二进制媒体未闭合，Bilibili 按用户要求只证明一条，飞书嵌入
  Sheet/Base/Slides/画板内部数据仍未覆盖。P4 已按代表性首批路径收尾，不把阶段完成扩大成
  全部点名来源完成或来源 available。
- P4-G1 至 P4-G6、TC-01 和 TC-02 已通过。P4 完成只证明飞书与正式 Chrome 点名平台的
  首批流程、选择范围、逐条状态、失败重试和重采边界成立；OneNote、微信聊天、视频号、
  抖音和书签自动遍历等仍未闭环。印象笔记已经证明官方整库 `.notes` 可用固定算法解密，
  但只完成首条正文校验，尚未全量生成 ENEX、正式进入 Babata 或重采。前述扩展来源转入
  P7，书签最后单独收集；抖音和视频号按用户决定暂时不处理。它们都保持 disabled，不阻塞
  P4，也不冒充已有样本或自动化。
- 微信样本使用官方 PC 微信 4.1.11.55 的“全部收藏”窄 UI，读取 8 个最新可见候选并选择
  “爬虫-这20个仓库教会什么叫降维打击”；保存 2,946 字符结构化正文、2,597 字节
  Markdown 和 2,331,350 字节原始 HTML。首次因候选白名单缺口进入可重试 `failed`，原
  item retry 后为 1 item/1 revision/2 exports，重采 `unchanged` 且数量不增加。未扫描
  微信进程内存、未解密数据库、未安装代理证书；收藏其他类型、聊天和自动遍历仍未完成。
- 2026-07-19 另完成豆包复杂会话“战略领导力W1”的 Agent 收集：16 条消息、8 轮问答和
  完整脑图已拿回，7 个原始 DOCX 共 111,296,956 字节，逐个大小和 MD5 与豆包消息元数据
  一致，并通过 DOCX 结构检查。对话和脑图已正式归档；P5 TC-03A 又把其中“设立目标”的
  原始 DOCX 与平台预览 PDF 作为同一 item 的新 revision 正式登记，分别标为 `original` 和
  `preview`。其余 6 个 Word 原件仍在 recovery staging，尚未登记为正式附件；该结果仍不冒充
  全部附件归档或长期自动化。
- PR #22 已在 PRD 加入人话词汇表和三层闭环规则。后续界面和阶段汇报先说明实际拿回内容、
  保存位置和缺口，再按需补充 C0、asset、revision 等工程词。
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

- P5 已完成：百炼 CLI（`bl`）的真实多类型试跑、引导 Skill、真实 PDF/图片/视频 C0→C1、受控 C1 文件、删除重建、受限样本和原件/预览边界共同通过 TC-03A。合并后的 `main` 又以真实微信 C0 完成 local extract 和一次 `qwen-plus` 摘要，保留注入的 provider 失败与成功 retry、实际 task/usage/output hash、unavailable 分支，并复核 C0 正文/asset 哈希不变，TC-04 与 AC-04 通过。完整 AC-03/TC-03 仍需 P6 TC-03B；清洗派生物、C1、队列和证据只进 `BABATA_DATA_HOME`，不进 Git。
- P6.1 已完成：真实 C0/C1 可在用户零回复时由 Agent 消化为机器/未审阅的三大界核心，
  三级地图、五类语义、关系、三维评分、地图演进、高密度文本和窄 C2 均可追溯；评论、
  Log、Insight、附件、Agent 再分析和真实作品改写保持不同语义。AC-05、AC-06、TC-05、
  TC-06 已通过；P6.2 检索/浮现和 P6.3 子库/输出尚未开始。

项目阶段只使用 P0–P8；C0–C3 是数据权威级别，不是项目阶段。

### 1.1 人话进度地图

```text
已经真的收进 Babata，而且重采过
  Kimi      15 个真实候选 -> 选 1 条 -> 1 条资料/1 个版本 -> 重采没变化
  豆包      20 个真实候选 -> 选 1 条 -> 1 条资料/1 个版本 -> 重采没变化
             -> 另收“战略领导力W1”：16 条消息 + 完整脑图 + 7 个原始 Word
             -> 7 个 Word 共 111.30 MB，大小/MD5/Word 结构均已校验
             -> 对话和脑图已正式归档；Word 仍在临时回收区，尚未挂为正式附件
             -> Agent 收集已完成；当前不开发专用适配器，需要重复执行时优先整理 Skill
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
  微信      官方 PC 微信 4.1.11.55“全部收藏”读取 8 个最新可见候选
             -> 选“爬虫-这20个仓库教会什么叫降维打击”，微信内复制官方原链接
             -> 2,946 字符正文 + 2.6 KB Markdown + 2.33 MB 原始 HTML；正文图片为 0
             -> 首次白名单缺口 failed -> 原任务 retry 成功
             -> 1 条资料/1 个版本/2 个导出原件 -> 重采没变化
             -> 只形成已知公众号 URL 的重复取得；收藏自动遍历和聊天未形成长期能力

P5 已收尾，下一阶段
  P6         核心沉淀、检索、子库与输出（进行中）

转入 P7 扩展来源，不是 P4 完成证据
  微信聊天/收藏其他类型（官方 PC 微信 UI，后续 Agent 带着走）
  OneNote 官方整本 PDF+MHT
  印象笔记官方整库 .notes + 固定算法解密（已解开首条；待全量 ENEX、C0 和重采）

最后单独收集
  浏览器书签自动遍历正文和可得附件

暂时不处理
  抖音；视频号（均保持 disabled，用户重新启用后再继续）
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
2. Browser Use/Agent Browser 复用已登录 Chrome，自主导航点名平台并取得真实内容；
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
- 豆包复杂样本（2026-07-19）：会话“战略领导力W1”
  (`https://www.doubao.com/chat/21060420230098690`) 共 16 条唯一消息、8 轮问答，消息链
  `has_more=false`，完整 mindmap 文本已进入现有对话记录。Agent 从消息内嵌 JSON 识别
  7 个原始 DOCX 对象键，通过登录态 `get_file_url` 路径取得真正 Word 原件；总计
  111,296,956 字节，实际大小和 MD5 均与豆包元数据一致，SHA-256 已记录，DOCX ZIP 中
  `[Content_Types].xml` 和 `word/document.xml` 均存在。文件和 manifest 位于
  `C:\Users\Aiano\BabataData\recovery-staging\doubao\20260719-w1-complex\`。预览器下载的
  43 页 PDF 只是豆包转换预览件，不是原件。当前结论分别是：Agent 收集闭环已完成；
  对话/脑图已正式归档但 7 个 Word 尚未登记为正式附件；长期自动化未完成。
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
- 微信收藏/公众号文章：官方 PC 微信 4.1.11.55 的“全部收藏”窄 UI 读取 8 个最新可见
  候选，只选择“爬虫-这20个仓库教会什么叫降维打击”；从微信文章窗口“更多 -> 复制链接”
  取得 `https://mp.weixin.qq.com/s/Va9tXvh6qWoOkog9SIbOOg`，OpenCLI 下载正文 Markdown，
  Agent 保存公共原始 HTML。该页没有正文图片或音视频，OpenCLI 作者字段为空，公众号名
  “智能系统实验室”由微信 UI/HTML 证据保留。首次选择因 `source.wechat_articles` 未在 C0
  候选白名单而可重试 `failed`，补齐最薄接线后对原 candidate retry 成功。最终为
  `item_01KXWDRSPMZ8GZMB14SYTQH2H2`、1 ready revision、2 ready exports：Markdown
  2,597 字节，SHA-256 `fcc3858b92013d97a1f9ef69497dba4c3f1d3db993530f648d8a8237a3fbdd5f`；
  HTML 2,331,350 字节，SHA-256 `90c46a5ba584ffc879d0f06024846b7a9f02694e3395cf5c0cb3a660b710eff7`。
  重采为 `unchanged`、无新 revision，资产数仍为 2。资料已拿回并正式登记；已知文章 URL
  可重复取得，但收藏候选仍依赖 Agent 操作官方 UI，不代表收藏自动遍历、聊天或微信全量
  已形成长期能力。来源继续 disabled。

2026-07-19 曾建立 Issue #20 尝试把豆包原附件取得开发成持久适配器。复核后确认 Agent
已经把最复杂样本真实跑通，当前继续开发会偏离“优先现有工具、最少开发”的需求，因此
Issue #20 已按 `not planned` 关闭，实验代码全部撤销且未进入 Git。若后续需要重复执行，
优先把已验证的 Agent/Chrome 流程整理为 Skill；只有真实重复使用证明仍缺稳定能力时，
才重新评估窄适配器。

浏览器和官方客户端仍是当前存量回收首选。Kimi/豆包/ChatGPT/知乎/小红书/语雀的 OpenCLI 薄命令是为了把浏览器已经证明的
读取动作变成任务结束后可调用的重试/重收集；Bilibili 是因为 Codex Chrome 历史页连续
两次超时后才回退 OpenCLI。微信历史样本由官方 PC 微信窄 UI 发现收藏候选，当时 OpenCLI
只固化已知公众号 URL 的下载和重采；该事实保留为历史证据，未来微信路线已改为官方 PC
UI-only，由 Agent 带着走。三类理由均已记录，不把 OpenCLI 当默认绕路。

实验性 `Babata Collector 0.2.0` 只完成手动当前页/选区剪藏和 locator-only 书签提交，
正式 Chrome 实测仍要求用户逐项点击，不能自动遍历书签正文。按用户最新纠偏，该入口
冻结、保持 disabled、排到最后优先级，不作为 P4 gate 或当前存量回收完成证据。浏览器
书签后续正常路线必须由 Agent 在一次明确范围后自动遍历网址并取得正文和可得附件。

已有导出、书签 HTML 和 CandidateEnvelope 只作为回退/提前证据。P4 gate：

| Gate | 本阶段判定 |
| --- | --- |
| P4-G1 | 飞书真实上下文候选成立 |
| P4-G2 | 正式 Chrome 中 Kimi 真实会话候选与所选正文成立；冻结的手动剪藏器不计入 gate，不能用任意页面替代具体平台 |
| P4-G3 | 一次明确范围内可连续收集；未授权范围不写入 |
| P4-G4 | 逐条状态、局部成功和重试成立 |
| P4-G5 | 四种重收集结果不覆盖旧 C0 |
| P4-G6 | 真实证据与 fixture 分开，未验证来源保持 disabled |

2026-07-19，P4-G1 至 P4-G6 和 TC-01、TC-02 已通过，AC-01、AC-02 的代表性首批路径
成立，P4 完成。来源 `available` 仍按每个来源的内容形态、附件、限制和重采证据单独判断；
阶段完成不会自动翻转任何 disabled route。00 点名来源并未全部跑通，长期自动化也仅在
部分已验证薄命令/Agent 流程成立。OneNote、微信其余范围，以及印象笔记从已解密样本到
全量 ENEX/C0/重采的剩余工作转入 P7；抖音、视频号暂时延期。

P4 收尾验证：`cargo test --workspace` 共 94 个测试通过；`cargo fmt --all -- --check`、
`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 通过；P2
inventory、P3 raw inventory（29 个活跃文件、59 个 raw 功能测试）、document traceability
及 mutation、interface ownership、Rust boundary 和 no-secondary-writer 检查全部通过。

## 8. P5：C1 多模态清洗与百炼

前置：至少一条真实 C0 来源可稳定回看。

状态：**已完成**。

### 8.1 完成证据

1. 百炼 CLI（`bl`）可安装、鉴权，并作为首个多模态处理路径使用。
2. 对本机课程样例做了每类型最小验证：图 OCR、PDF/DOCX/XLSX/PPTX 结构化摘要、视频截帧与 ASR 转写（含时间戳；说话人字段在单讲师样本中可见）。
3. Agent 引导 Skill 已入库：`02_skills/babata-bailian-clean/`（原件只读、本地规范化、百炼路由、派生物契约、**正式 C1 登记步骤**）。
4. C1 登记路径已激活：`derived.sqlite`（`process_runs`/`derivatives`）、`babata process list-pipelines|register|show-run|list-runs|delete-result`；只有 failed run 可重试，新 attempt 不覆盖旧结果；逻辑删除保留失效时间/理由，重建创建新 run。
5. Skill 默认用 `pipeline=agent_import` 把 staging 结果 `process register` 进 C1；`references/c1-register.md` 定义字段映射与核验口径。
6. AC-03 的 P5 C0/C1 子责任与 TC-03A 已通过；Provider 作业队列的
   `enqueue/run-once/status/retry/cancel`、本地 UTF-8 asset 提取和百炼文本摘要已实现。
7. Issue #48 已补上媒体 kind 强制 asset、run target kind/asset 身份、受控 C1 文件 staging
   与恢复证据、输出表示哈希一致、失败父 run 身份一致、provider/tool/version 与 JSON 校验；
   真实 PDF/图片/视频重登记、v3 实库 migration 修复、旧结果失效、可读 transcript、C1
   删除重建均已完成。真实受限 revision 已重复完成 C1 删除/重建；独立 verification 根以
   真实平台 PDF 验证了 source unavailable 时只有 `preview`、没有 `original` 的等价负向分支。
   ASR provider 响应中的临时签名 URL 已从 active C1 和普通 staging 脱敏，Skill 增加同一
   禁止规则。其他 C0 字段审计继续由 Issue #43 跟踪，不反向阻塞已通过的 P5 证据。
8. 2026-07-20 在合并 `main` `0de2858` 上，以真实微信文章 revision
   `rev_01KXWDRSPMR023M5038FNK2DBG` 完成 TC-04：local extract 绑定真实 Markdown asset，
   删除旧 C1 后重建；注入的 Bailian provider 失败形成 failed job/run，retry 新建 attempt 2，
   再由真实 `qwen-plus` 成功生成摘要。实际 task ID、1,739 tokens usage、输出哈希和 loss notes
   可读；`bailian_ocr` queue 返回 unavailable；C0 正文和 asset 哈希前后不变。证据位于
   `BABATA_DATA_HOME/verification/p5-tc04-20260720-0015/TC04_PROVIDER_QUEUE_E2E.md`。
9. P5 合并实现通过 132 项 workspace 测试、fmt、clippy `-D warnings`、P2/P3 inventory、
   文档追溯及 mutation、接口所有权、Rust boundary、no-secondary-writer 和 Skill validator；
   GitHub PR #55 的 Rust 与 Architecture/docs checks 均通过。

### 8.2 完成口径与后续边界

P5 已完成以下责任：

1. C1 schema、process run/derivative、受控文件、失败/重试、逻辑删除/重建和真实输入绑定；
2. Agent 多模态 Skill → `agent_import` → C1，以及 C3 job queue → 同一 `ProcessService` → C1；
3. 本地文本 asset 提取与真实百炼文本摘要可由 queue 调用；图片 OCR、视频 ASR/视觉等真实
   多模态结果由 Skill 路线取得并正式登记；
4. provider identity、task、usage、错误、输出哈希、预处理和 loss notes 可检查，凭据与签名
   URL 不进入普通 C1；
5. TC-03A 和 TC-04 通过，AC-04 通过。

`bailian_ocr`、`bailian_transcript`、`bailian_visual_description` queue provider、百炼 API 和
长期批处理 worker 尚未实现，继续明确 unavailable；以后有真实重复调用需求时再扩展，不把
它们冒充为 P5 已有自动能力，也不把未启用能力反向变成 P5 阻塞项。

P5 主要交付 AC-03 的 C0/C1 子责任、AC-04、TC-03A 和 TC-04。P6 交付 AC-03 的 C2
子责任与 TC-03B。AC-03 和 TC-03 整体尚未通过；C1 不覆盖 C0，模型输出不自动成为人工判断。

## 9. P6：核心沉淀、检索、子库与输出

2026-07-20 已从 1.0 原始归档恢复 P6 的“个人知识宇宙”产品基线，并由
`09_P6_PERSONAL_KNOWLEDGE_UNIVERSE_BLUEPRINT.md` 集中说明。2026-07-21，Issue #65 / PR #66
完成 P6.1 纵向闭环并通过完整门禁。P6 整体仍未完成；P6.2/P6.3 尚未开始，不能把 P6.1
完成扩大成检索、浮现、子库或通用输出已经可用。

P6 必须按核心价值顺序进行，不能直接跳到 Datasette/Obsidian，也不能用简单文件夹分类
或全文搜索代替核心：

### P6.1 核心语义沉淀

- 聚合查看原件、派生物、来源、版本和关系；
- 由 Agent 把 C0/C1 继续消化为来源齐全、结构可校验的机器语义候选；未审阅候选进入
  核心继续工作，不要求用户逐条确认；
- 建立第一大界的时间/空间/物质/意识 -> 学科 -> 分支三级地图和跨节点归属；
- 建立第二大界的知识/案例与第三大界的长期/中期/短期/实时日志、感悟；
- 建立知识/案例互证、统一标签、双向关系、主题/结构模型、分析与高密度文本表达；
- 建立兴趣/战略/共识三维评分，默认 `40/35/25`，保留 profile、依据和历史；
- ModelSuggestion 与 SuggestionReview 分离；未审阅不阻塞下游，审阅只追加状态标记；
- 区分独立评论、感悟、日志切片、附件/证据、Agent 再分析和少数真实 first-party 改写。

交付 AC-05、AC-06 和 TC-05、TC-06。

2026-07-20，Issue #59 / PR #62 首切片形成以下有效实现与证据：

1. 正式应用 #43 的 raw integrity/collection migration 前先做真实 SQLite 一致性快照；
   迁移前后 13 张业务表内容摘要与行数一致，C0/C1 活跃引用异常为 0；
2. `knowledge review` 在同一上下文读取 ready C0 的来源、版本、资产、关系及全部 C1
   run/derivative，并复核 active C1 的 item/revision/asset/input hash 与受控文件 hash；
3. 临时数据根贯通了 C0+C1 review 和 active C1 文件篡改拒绝；
4. `knowledge create/revise/show` 和线性 `knowledge_versions` 虽通过夹具测试，但后来确认
   它把评论、感悟、日志、附件、Agent 再分析和真实改写错误压成同一手工版本主流程；
5. 真实微信文档样本已通过 review，聚合 1 个 C0 revision、2 个 assets、4 个 process
   runs，以及 extracted text/summary/失败历史；没有把其中模型摘要自动写成 first-party
   Knowledge，真实 knowledge 表仍为 0 行。

Issue #63 在继续 P6.1 前纠偏：保留真实审阅和完整性校验；撤下误导性的手工
`knowledge create/revise/show`；恢复三大界；把自动语义消化、独立评论/感悟/日志、
附件/证据、Agent 再分析和少数真实修订分开。旧 migration 只为兼容保留，不作为新模型
权威。AC-05、AC-06、TC-05、TC-06 和 P6.1 继续保持未通过/未完成。

Issue #63 的真实 migration 预检先在线备份 `raw.sqlite`，确认 v1 的 knowledge records/
versions 均为 0 行；Rust 应用 v2 后将旧表无损隔离为 `deprecated_manual_*`，sources、items、
revisions、assets、relations、capture operations、collections 和 route evidence 行数不变，
SQLite `quick_check=ok`，同一真实微信 C0/C1 仍通过 `knowledge review`。备份位于外部数据根
`verification/p6-1-correction-20260720-233608/snapshot`，不进入 Git。

2026-07-21，Issue #65 开始 P6.1 正式主流程，并形成第一条无需用户回复的真实纵向证据：

1. migration 0003 在同一 `raw.sqlite` 建立三大界、三级地图、多重归属、标签、显式关系、
   高密度文本、评分/profile、ModelSuggestion/SuggestionReview 和 first-party Log/Insight
   语义登记；旧 0001/0002 checksum 不变；
2. `knowledge digest` 聚合 C0 与 active C1，真实调用已鉴权 `bl 1.10.0` / `qwen-plus`，将
   `p6-semantic-candidate/v1` 先登记为 `structured_result` C1，再由核心校验 derivative
   ID/output hash 和全部 evidence ID/hash 后事务化规范写入；
3. 真实微信样本 `item_01KXWDRSPMZ8GZMB14SYTQH2H2` /
   `rev_01KXWDRSPMR023M5038FNK2DBG` 形成 suggestion
   `suggestion_01KY2A6TKXYG1HF3NWRWB3JNSZ`：3 个机器语义条目（Knowledge、Case、
   Map/Direction）、5 个动态地图节点、11 个归属、10 个标签归属、3 个关系、3 个高密度
   表达和 3 个默认 profile 评分，状态保持 unreviewed；
4. 迁移前 SQLite 在线备份位于外部数据根
   `verification/p6-1-semantic-core-20260721-202350/snapshot`；迁移前后原有
   `27 items / 30 revisions / 7 assets / 1 relation` 不变，真实微信仍只有 1 个 revision，
   两库 `quick_check=ok`，raw foreign key check 为 0；
5. 临时数据根测试覆盖 Knowledge/Case 双向关系、跨两个基石的多重归属、Log/Insight
   first-party C0 正文一致性、默认与新 profile 的评分历史、accepted/modified/rejected
   追加审阅，以及评论/新 C0 不会制造来源资料 `v2`。

同日，Issue #65 后续切片补齐地图演进和窄 C2 证据：

1. knowledge migration 0004 为地图节点增加 active/inactive/merged 生命周期，并为节点、
   父边、内容归属和地图标签建立追加式事件；应用层提供学科/分支新增、改名、停用、合并、
   父级调整、内容多重归属和节点标签操作，数据库 trigger 锁住 P6 baseline 四基石；
2. 节点和内容共用评分入口；读回包含 target、profile、分量、综合分、依据、作者身份和时间。
   未审阅 suggestion 明确可进入后续候选，但 `human_judgment=false`、
   `confirmed_fact=false`；rejected/modified 原建议保留历史但不再进入主动候选；
3. 高密度表达可生成受控 `03_views/p6_dense/<semantic_id>/preview.md` 与 manifest；临时纵向
   测试覆盖篡改拒绝、重建、删除和再次重建，删除视图后核心文本仍完整；
4. 临时 CLI 纵向测试还覆盖动态学科/分支双父级、改名、父级迁移、标签增删、内容归属、
   节点评分、分支合并及历史读回，并验证同一 C0 的第二次 Agent 分析形成新 C1
   suggestion，源 item 不产生 `v2`；四基石修改被拒绝；
5. 应用真实库前创建在线 SQLite 快照
   `verification/p6-1-map-evolution-20260721-213222/snapshot`。Rust 入口完成 knowledge
   `v3 -> v4` 后，原有 `5 sources / 27 items / 30 revisions / 7 assets / 1 relation`、
   `1 suggestion / 3 semantic entries / 9 map nodes / 6 edges / 11 assignments` 均不变，
   回填 `9/6/11` 条节点/父边/内容归属事件，`quick_check=ok`、foreign key 异常为 0；
6. 同一真实机器 Knowledge 完成 C2 build/verify/delete/rebuild/verify，删除后目录确实不存在，
   核心高密度表达仍为 1 项；完整证据位于外部数据根同目录的
   `P6_1_MAP_EVOLUTION_E2E.md`，不进入 Git。
7. 最终审查发现 0004 的 update trigger 尚可被“先在其他 map version 创建 foundation，再
   更新进入 baseline”绕过。已保持进入真实库的 0004 不变，新增 0005 同时检查 UPDATE 的
   旧、新 map version；负向迁移测试证明该路径也被数据库层拒绝。真实库 v4 -> v5 前另建
   在线快照并重新核对业务行数、migration checksum、`quick_check` 与 foreign key；证据位于
   `verification/p6-1-foundation-guard-20260721-220641/P6_1_FOUNDATION_GUARD_E2E.md`。
8. AC-06 反向审计发现旧 `capture attach-assets` 会为只补附件复制正文并增加 revision，
   `workspace revise` 也接受正文完全相同的请求，均与本轮明确纠偏冲突。现已新增 raw
   migration 0005：补附件作为独立 operation 追加到既有 ready revision，保留 reason、metadata、
   asset membership、状态和失败；finalise、校验或 ready transition 失败只隔离本次附件，
   原正文仍为 ready。临时应用/CLI/SQLite 测试证明 revision 数量不变、相同正文修订被拒绝、
   跨 revision 挂附件被拒绝。真实 raw 库通过 Babata Rust 入口从 v4 升到 v5，迁移前在线
   快照后原有 `5 sources / 27 items / 30 revisions / 7 assets` 及全部知识业务行数不变，
   新表为空，checksum 匹配、`quick_check=ok`、foreign key 异常为 0；未为验证制造真实附件。
   证据位于 `verification/p6-1-attachment-semantics-20260721-223511/`
   `P6_1_ATTACHMENT_SEMANTICS_E2E.md`。

该证据证明自动语义候选已真实进入核心，不再只是 review 准备；但不得把模型输出冒充用户
确认，也没有替用户制造真实评论、Log、Insight 或审阅决定。旧 P5 附件登记事实保留为历史
操作证据，不再作为规范语义；后续补附件不制造正文版本。PR #66 合并后，AC-05、AC-06、
TC-05、TC-06 已通过，P6.1 已完成；P6.2/P6.3 尚未开始。

### P6.2 检索与关系导航

- C0/C1 可重建读投影；
- 正文、来源、时间、语义类型、状态、人物、地图归属、分类、关系、处理状态和三维
  相关度检索；
- 媒体-only、附件-only 和受限资料仍可发现；
- 版本、来源、地图归属、知识/案例证据、日志/感悟引用和其他关系导航；
- 至少一种基于当前方向、相关度、时间和关系的可解释内容浮现入口。

交付 AC-07 的检索和关系部分。

### P6.3 子库与输出

- 版本化 SublibraryDefinition；
- 可删除重建的子库物化；
- 人类可读和结构化输出；
- manifest、来源/版本/profile/建议状态回溯和只读 builder；
- Obsidian、网页、报告等在真实用途出现后逐项启用。

交付 AC-03 的 C2 子责任、AC-07、AC-08 和 TC-03B、TC-07、TC-08。

## 10. P7：扩展来源、正式 Skill 与受控 Agent

按真实价值扩展 OneNote 官方整本 PDF/MHT、印象笔记官方整库 `.notes` 解密接入、微信
官方 PC UI 中的聊天/收藏其他类型，以及已有小样本来源的更多内容形态。微信由 Agent
后续带着操作 UI，不再等待内存扫描、数据库解密、代理或历史 CLI。抖音和视频号暂时不
处理，只有用户重新启用时才回到队列。浏览器书签排在最后，作为独立收集项自动遍历正文
和附件，不与本阶段其他来源扩展混做。

2026-07-19 已有两个只读 E2 导出解析探针：OneNote 整本 MHT 可读，包含 1 HTML、30 张
图片和 1 个 XML 清单，但没有明确页面边界；印象笔记 `.notes` XML 含 163 条笔记和 349
个资源，163 条正文虽为 `base64:aes`/`ENC0`，但公开的固定算法不需要用户密码。真实文件
首条已通过 HMAC 校验并解密为 381 字节 ENML；网页 DOM 和单篇 MHT 也已验证为备选。
两者都尚未正式进入 Babata、没有重采，不算 E3 或来源可用。

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
| P5 | AC-03（C0/C1 子责任）、AC-04 | 真实输入/派生物与忠实清洗；TC-03A/TC-04 |
| P6 | AC-03（C2 子责任）、AC-05、AC-06、AC-07、AC-08 | TC-03B；核心先于检索/视图/输出 |
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

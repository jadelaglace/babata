# Babata P4 首批真实收集路径

## 1. P4 目标

P4 在 P3 C0 底座之上证明两条正常日常收集路径不是空架子：

1. 飞书文档、Wiki、知识库中的上下文候选与选择性收集；
2. 正式 Chrome 登录态中的首个具体点名平台（当前为 Kimi 对话）候选与选择性收集。

P4 的重点不是“能把一个导出文件塞进数据库”，也不是“能提交一个任意网页 envelope”，
而是用户在正在阅读、收藏或整理
资料的地方发现候选、确认明确范围、看到逐条状态，并能够重收集。

P4 不激活清洗、模型、Knowledge、搜索、子库、输出、远程后台全量收集或第二写入者。

## 2. 共同产品流程

```text
用户连接并授权来源
  -> 只读发现当前上下文候选
  -> 展示标题、位置/层级、类型、更新时间、附件可得性和限制
  -> 用户选择单条 / 可见集合 / 明确范围
  -> CollectorSessionService 建立逐条任务
  -> SourceAdapter 读取被选内容
  -> CaptureService 统一提交 C0
  -> 展示 saved/skipped/failed 与重试
```

连接、授权、候选发现和列表浏览都不创建 C0。只有明确选择后的内容进入 CaptureService。
一个集合局部失败不会回滚已成功项，也不会把失败项伪装成已保存。

外围入口不分配最终 ID、不打开 SQLite、不 finalise 资产、不建立 C0 版本和关系。

## 3. 共同内部契约

### 3.1 CandidateSummary

候选只携带展示与选择所需的信息：

```text
candidate_id（会话内稳定）
route_id
source_native_id / locator（实际可得时）
title
source_location / hierarchy
content_type
source_updated_at
attachment_availability
known_limits
selection_capabilities
```

候选不是最终原件，不能被其他入口当作已经收集的资料引用。

### 3.2 CollectionSelection

```text
session_id
selected_candidate_ids 或明确 scope
user_confirmation
authorised_account/context
requested_attachments
```

范围必须可以向用户解释。`all` 不能成为隐式默认；只有用户明确选择并能预览范围时
才允许大集合操作。

### 3.3 AcquisitionPackage

SourceAdapter 对被选候选返回：

```text
原始内容或原件流
附件清单与实际取得结果
来源链接/原生标识
平台、账号/作者、层级与收藏上下文
来源时间和本次读取时间
访问限制、缺失字段和 adapter 版本
```

Rust 核心负责暂存、哈希、版本判断和最终提交。adapter 读取失败返回结构化原因，
不返回“空内容成功”。

## 4. 飞书路径

### 4.1 正常路径

直接复用用户授权的官方 `lark-cli`，不再先写飞书专用 API 客户端，也不把手动导出
Markdown 当正常路径。工具调查、认证证据和命令覆盖见
`08_SOURCE_TOOL_RESEARCH.md` 6.1：

```text
官方应用配置 + 用户 OAuth
  -> `drive +search` / `wiki +space-list/+node-list` 浏览候选和层级
  -> 显示文档候选、类型、更新时间、附件/子节点可得性和权限限制
  -> 用户选择文档、当前层级可见集合或明确节点范围
  -> `docs +fetch`、`docs +media-download`、`drive +download/+export` 按选择读取
  -> CaptureService
```

实现必须处理 Wiki node 到实际文档的解析、`my_library` 特例、分页、权限不足、
已删除/移动、附件不可得、内嵌 Sheet/Base/Slides/画板分流和速率限制。它不遍历用户
未选择的整个账号，也不因获得 token 自动复制全部内容。

### 4.2 重收集

飞书原生标识和来源层级用于定位旧资料。再次收集时：

- 内容或附件变化：`changed`，追加新版本；
- 内容与附件未变：`unchanged`，保留本次检查事件；
- 当前权限/网络不可达：`inaccessible`，保留旧版本；
- 官方明确删除或移除：`removed`，记录来源状态，不删除 C0。

移动层级但内容不变时，来源上下文变化也要有记录，不用覆盖旧上下文。

### 4.3 回退路径

官方 Markdown/PDF/其他导出加本地附件可以作为离线传递、恢复或 API 暂不可用时的
路径。回退路径应显示它无法提供的实时层级、权限、更新时间或附件信息。

已有 `feishu-export` 解析与 synthetic fixture 只证明回退解析和 C0 提交机制，不证明
AC-01 的正常飞书上下文体验已经通过。

## 5. 具体网页登录平台与浏览器路径

浏览器是复用登录态、导航和下载的工具层，不是 `source.kimi`、`source.zhihu`、
`source.bilibili`、`source.xiaohongshu`、`source.douyin`、`source.doubao`、
`source.chatgpt` 或 `source.yuque` 的替代来源。候选、状态和 C0 溯源必须记录具体
`source_id` 与平台上下文；`source.browser_pages` 和 `source.browser_bookmarks` 只代表
普通网页/书签自身。

P4 先把已有真实 Chrome E2 证据的 Kimi 做到 E3，并以飞书、豆包、Bilibili、知乎、
小红书、ChatGPT、语雀和微信收藏中的公众号文章扩充代表性真实证据。P4 的完成条件是
首批正常流程和共同状态/重采机制成立，不是 00 点名来源逐一跑通。微信聊天与收藏其他
类型改为后续 Agent 操作官方 PC 微信 UI；OneNote、印象笔记转入 P7 扩展；书签自动遍历
排到最后单独收集；抖音和视频号按用户 2026-07-19 决定暂时不处理。未单独验证的平台
始终保持 disabled。

### 5.1 正常路径

当前存量回收先由 Codex 直接控制用户正式 Chrome，让 Agent 在一次给定范围内自主导航、
翻页、发现和读取。只有浏览器在该平台被真实证明不稳定，或需要把已验证动作固化为任务
结束后仍可调用的重试/重收集命令时，才调用 OpenCLI，并在证据中记录理由。浏览器书签的
正常路线同样由 Agent 在一次明确文件夹/集合范围后自动遍历网址、取得正文和可得附件；
`chrome.bookmarks` 只负责候选与层级发现，不能把 locator-only 记录冒充正文收集。手动
当前页/选区快速剪藏器属于未来窄入口，当前冻结并排在点名来源和自动存量回收之后。
具体证据见 `08_SOURCE_TOOL_RESEARCH.md` 第 5、9 节：

```text
用户给定当前页面、站点、收藏夹、会话、时间段或书签文件夹范围
  -> Codex 直接操作已登录正式 Chrome，自主发现候选和遍历范围
  -> 浏览器失败或需要任务外重试/重收集时，才调用有版本的 OpenCLI 薄命令
  -> 需要保真页面时调用 SingleFile
  -> 只有范围有实质歧义、会越界或平台要求登录/授权时才再次找用户
  -> 与本机 Babata 配对
  -> loopback API 调用 CollectorSession/Capture 用例
```

候选可包含 URL、标题、选区/页面内容、声明元数据、书签层级、页面更新时间和已知
限制。CDP 探针只在用户一次批准当前 Chrome 实例后运行，并使用只读/导航优先的动作
策略。任何浏览器工具都不持有数据根路径、SQLite 凭据或最终资产权限。浏览器书签不与
P7 的点名来源扩展混做，排到最后作为独立范围收集。

loopback API 只绑定本机，使用安装级凭据、来源限制和请求大小限制。配对失败、核心
不可用或 payload 超限必须在扩展中显示明确失败，不转存成隐藏权威副本。

2026-07-18 已实现实验性 `Babata Collector 0.2.0`：页面/选区只申请 `activeTab + scripting`，
书签使用按需 `bookmarks` 权限，不申请 `<all_urls>`；扩展只保存本机 API 地址、安装标识和
配对 token。Rust API 只接受 `127.0.0.1`、Chrome extension origin、最多 1 MiB/200 个候选，
并只开放 `source.browser_pages` 与 `source.browser_bookmarks`。隔离验证根
`p4-browser-extension-20260718` 的真实监听端口测试已证明：health 成功、发现 1 个候选、
未确认返回 409 且 item 为 0、确认后仅该项 `saved`、重采 `unchanged`。这仍是本机网络和
机制证据。正式 Chrome 配对实测进一步证明它仍要求用户打开弹窗、逐项点击，书签只保存
标题和 URL，不会自动访问正文；这与当前“Agent 操作优先、用户不操心”的目标不符。
因此该扩展冻结、保持 disabled、排到最后优先级，不计入 P4 gate，也不继续以手工操作补证。

### 5.2 书签与页面的差异

- 当前页面/选区：收集本次用户看见或选择的页面内容与 URL 上下文；
- 书签：用户一次给出明确文件夹或集合范围后，Agent 遍历其中网址并收集正文和可得附件；
- `chrome.bookmarks` 可提供标题、URL 和层级候选，但只保存 URL 仍是 locator-only 未完成状态；
- 后续网页变化通过重收集追加版本，不覆盖第一次保存的页面证据。

### 5.3 回退路径

Netscape bookmark HTML、保存的 HTML/PDF、copy 和 screenshot 可以作为导入/恢复路径。
已有 bookmark export 和 CandidateEnvelope 测试证明格式、hash 和 C0 边界，不证明浏览器
扩展的候选、选择、配对和逐条反馈已经完成。

### 5.4 当前真实闭环证据（2026-07-18）

| 具体来源 | Codex 浏览器证据 | OpenCLI 使用理由 | 当前真实结果 |
| --- | --- | --- | --- |
| 飞书 | 不依赖网页登录 DOM；使用用户已授权的官方 `lark-cli` user identity | 不使用 OpenCLI；官方 Wiki/Docs/Media API 是更直接、可分页和可重采的结构化路线 | `一堂` 10 个根候选、`AI分享` 6 个子候选中选 1 篇；保存 3,391 字符 XML 和 8 张 PNG；原任务从真实 failed 定向 retry 成功；C0 为 1 item/1 revision/8 assets；重采 `unchanged` |
| Kimi | 发现 15 个真实会话候选；读取 `GetChat`/`ListMessages`，样本有 3 条结构化消息和引用 | 浏览器已完成探索；薄命令只用于让 Babata 在本任务结束后仍能重试/重收集 | 选 1 条写入 1 item/1 revision；重收集 `unchanged`；样本无附件，保持 disabled |
| 豆包 | 发现 20 个真实近期会话；读取 `recent_conv`/`conversation/info`/`chain/single` | 浏览器已完成探索；现有 OpenCLI detail 不能读取真实状态，补窄命令用于稳定重收集 | 选 1 条写入 1 item/1 revision；重收集 `unchanged`；二进制媒体未闭合，保持 disabled |
| Bilibili | Codex Chrome 读取历史页连续两次超时 | 有真实浏览器失败证据，且需要下载/重采；因此回退到 OpenCLI + `yt-dlp`/ffmpeg | 20 个真实历史候选中只选 `BV1ogzsBFE1T`；保存元数据、官方字幕、官方 AI 摘要和 44,773,539 字节视频；C0 为 1 item/1 revision/1 asset；重采 `unchanged` |
| ChatGPT | 已登录正式 Chrome 中展开最近聊天并看到至少 28 个真实入口；选中“开源部署方案对比”，DOM 读取 2 条角色消息、10 个引用和 0 个附件 | 浏览器先证明候选和正文；版本化 OpenCLI 薄命令只为 Babata 在任务结束后仍可发现、重试和重采。内置命令因后台元素无尺寸失败，不能直接复用 | `recent:20` 发现 20 个真实候选，只选该会话；C0 为 1 item/1 revision/0 assets，metadata 为 2 条消息、10 个引用、0 附件且当前样本附件已覆盖；重采 `unchanged` |
| 知乎 | 登录后“我的收藏”页列出 16 个自建收藏夹；最新收藏夹页面标称 28 条，Codex 浏览器第一页读取 20 条，并确认所选回答有完整正文和 17 张正文原图 | 浏览器先证明真实收藏夹、候选和媒体；已安装 OpenCLI 的 `collection`/`answer-detail` 用于分页与重采，另补窄 `answer-detail-full` 返回原始 HTML 和稳定图片 token | 官方分页命令从标称 28 条中返回 27 个去重候选（12 回答、15 文章）；只选最新回答，保存正文、原始 HTML 和 17 张原图，共 8,413,376 字节；C0 为 1 item/1 revision/17 assets；重采 `unchanged` |
| 小红书 | 正式 Chrome 登录后进入本人收藏页并读取 20 个真实收藏候选；从持久会话顺序选择“捉住一只小仙兔” | 浏览器先证明收藏范围和详情；薄命令只用于稳定候选、媒体下载和重采，并修复无标题收藏候选 | C0 为 1 item/1 revision/2 assets，两项媒体哈希不同，共 10,163,846 字节；重采 `unchanged` |
| 语雀 | 正式 Chrome 登录后看到 2 个知识库和 8 个最近文档；实测整库官方导出为 PDF/LakeBook，单篇提供官方 Markdown；选中“粒界引擎-车辆材质质感提高方式” | 薄命令把浏览器已证明的官方 Markdown 地址和媒体枚举固化为可重采动作；不使用会员 OpenAPI/MCP，也不要求会话 Token | `recent:8` 选 1 篇；保存官方 Markdown、渲染正文/HTML和 22 张不同哈希图片，共 3,101,329 字节；C0 为 1 item/1 revision/22 assets；重采 `unchanged` |

Bilibili 最终验证根为
`C:\Users\Aiano\BabataData\verification\p4-bilibili-final-20260718-181500`；媒体和 C0
哈希寻址资产的 SHA-256 均为
`35551288f33a21c9ea5b75f69dd578521f9f76a2b79b9a2448d4f33bf2f26d22`。播放、点赞、
投币、收藏、分享、评论和弹幕等实时计数仍完整保存在原始 payload，但不参与正文
`content_fingerprint`，避免每次统计波动制造伪版本；标题、简介、作者、发布时间、时长、
分 P、字幕和官方摘要仍参与变化判断。

以上都是单条/小范围闭环证据，不代表账号全量或整个来源 `available`。Bilibili 按用户
要求只闭合这一条，后续由用户选择具体范围时再收集。

飞书最终验证根为
`C:\Users\Aiano\BabataData\verification\p4-feishu-20260718-184000`。真实 XML 使用
`<img src="资源 token" href="临时签名 URL">`，适配器用结构化 XML 解析器提取稳定
`src/token`，只在用户选择附件时调用官方 `docs +media-download`。原始 `href` 仍保存在
C0 正文；变化判断使用官方 `document_id + revision_id + media token`，签名刷新不产生
伪版本。当前闭合的是 Wiki/Docx/图片附件；嵌入 Sheet、Base、Slides 和画板内部数据仍按
各自官方 reader 下钻，未验证前保持明确限制。

ChatGPT 最终验证根为
`C:\Users\Aiano\BabataData\verification\p4-chatgpt-20260718-190000`。浏览器页面上的 15 个
`img` 都是引用站点 favicon，不计作附件；所选会话真实附件数为 0，因此本样本的附件覆盖
成立，但二进制会话附件下载能力没有得到非零样本验证，route 继续保持 disabled。内容指纹
保留角色、正文、引用和附件名称，但忽略临时签名 URL，避免链接刷新制造伪版本。OpenCLI
瞬时非 JSON 响应按可重试来源 I/O 失败报告，不再误报 C0 完整性损坏。

知乎最终验证根为
`C:\Users\Aiano\BabataData\verification\p4-zhihu-final-20260718-203000`。17 张正文原图均有
不同 SHA-256，总计 8,413,376 字节；作者头像排除。第一次验证发现同一图片会在
`picx/pic1/pica` CDN 域之间切换而制造伪版本，最终改用 HTML 的稳定
`data-original-token` 判断图片身份，下载仍使用当次可用 URL，干净验证根重采为
`unchanged`。当前只闭合回答；文章、想法、视频和评论线程保持明确限制，route 继续
disabled。

小红书最终验证根为
`C:\Users\Aiano\BabataData\verification\p4-xiaohongshu-final-20260718-210000`。第一次发现
无标题收藏候选时保持 C0 为 0，兼容后从持久会话的真实顺序选定目标，不猜笔记 ID；实时
互动计数不参与内容指纹，两个媒体资产参与重采判断。

语雀最终验证根为
`C:\Users\Aiano\BabataData\verification\p4-yuque-official-20260718-225000`。官方整库导出
只提供 PDF/LakeBook；本次小范围使用免费单篇 Markdown 端点，22 个媒体 token 与官方
Markdown 一起参与内容指纹。官方 OpenAPI/MCP 的超级会员门槛只登记，等全部来源闭环后
统一决策；route 因整库通用格式、文件、表格、画板和评论未覆盖继续保持 disabled。

## 6. 收集状态与会话

每条选择项拥有：

```text
queued -> running -> saved
                  -> skipped（带原因）
                  -> failed（带 retryable 与原因）
```

CollectorSession 的实时队列和进度属于 C3；终态收集结果、来源变化、版本和错误摘要
进入 C0 溯源。清理会话/队列不能抹掉已经完成的历史。

重试只针对失败或用户明确选择的项，不重新执行整个来源范围。取消集合后，尚未开始
的项转为 skipped/cancelled 运行状态，已经保存的 C0 不回滚。

并发取消测试会在第一项进入 `running` 后从另一调用取消批次：第一项允许完成为 `saved`，
其余 `queued` 项全部变为 `skipped`，session 保持 `cancelled`，不会被批次收尾覆盖为
`completed`。无范围、重复候选、授权上下文不匹配和未确认都在入队前失败，C0 保持 0。

## 7. 能力状态与真实证据

来源能力从 `scaffolded/disabled` 进入 `available` 必须同时有：

1. 用户授权的真实来源；
2. 候选发现与展示证据；
3. 单条、可见集合或明确范围的选择证据；
4. 正文、上下文、附件和限制覆盖记录；
5. 逐条成功/跳过/失败及重试证据；
6. 至少一次重收集并产生四种结果中实际可验证的结果；
7. 无静默全量复制和无第二写入者的边界证据。

合成 fixture 可以自动验证序列化、分页模拟、hash、状态机和错误映射，但不能替代
真实授权证据。授权记录不得包含 token、真实正文、附件、数据库路径或其他秘密。

## 8. P4 交付门槛

| Gate | 完成证据 |
| --- | --- |
| P4-G1 飞书候选 | 真实授权连接能展示当前文档/Wiki/知识库候选及限制 |
| P4-G2 Kimi 浏览器候选 | 正式 Chrome 中展示真实 Kimi 会话范围并收集所选消息；冻结的手动剪藏器不计入本 gate，也不能替代具体平台 |
| P4-G3 选择性提交 | 未确认不写 C0；确认后只收集所选项 |
| P4-G4 逐条状态 | queued/running/saved/skipped/failed、局部成功和重试成立 |
| P4-G5 重收集 | changed/unchanged/inaccessible/removed 不覆盖旧 C0 |
| P4-G6 真实能力 | 真实证据与 fixture 证据分开，未验证来源保持 disabled |

P4-G1 至 P4-G6 共同对应 AC-01、AC-02 和 TC-01、TC-02。只通过导出解析、书签 HTML、
任意当前页或 CandidateEnvelope fixture，不满足 P4 完成门，也不能把知乎、Bilibili、
小红书、抖音、豆包、Kimi、ChatGPT 或语雀标成 available。

### 8.1 P4 收尾判定（2026-07-19）

P4-G1 至 P4-G6 和 TC-01、TC-02 已通过：飞书与 Kimi 提供两条规定的真实授权主路径；
多个补充来源又证明了候选选择、附件、真实失败、原任务重试、C0 和 `unchanged` 重采；
状态机和其余重采分支由阶段测试证明不会覆盖旧 C0。P4 因此完成，P5 成为下一阶段。

该判定只说明首批代表性收集路径成立，不说明 00 点名的 19 个来源都有真实样本，更不
说明全部可以长期自动运行。Kimi、豆包、Bilibili、飞书、ChatGPT、知乎、小红书、语雀和
微信公众号文章有真实小范围闭环；OneNote MHT 和印象笔记加密 `.notes` 只有 E2 真实导出
解析，微信聊天、视频号、抖音和书签自动遍历等也无对应真实闭环。所有未完整覆盖的来源
继续 `disabled`；抖音和视频号明确延期，不是 P4 阻塞项，也不计作完成证据。

## 9. P4 明确不做

- 不做账号级静默全量复制和远程后台爬取；
- 不在 adapter 或扩展中写 SQLite、最终原件或 C0 关系；
- 不把导出路径、CLI 参数和手填 metadata 作为正常日常流程；
- 不做 OCR、转写、摘要、模型判断、搜索、子库和输出；
- 不因为已存在 P4 测试文件或 migration 就提前标记 P4 进行中/已完成。

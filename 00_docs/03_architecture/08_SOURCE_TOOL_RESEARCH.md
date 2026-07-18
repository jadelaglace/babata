# Babata 来源工具调研与路线决策

## 1. 文档职责

本文是 P2 的来源工具调查证据和路线决策，不是工具愿望清单。它落实
`00_REQUIREMENTS.md` 中最核心的收集要求：面对数量繁多的网站、平台和运营商，先
真实调查并复用已经成熟的 API、CLI、SDK、MCP、浏览器扩展和开源工具，把属于用户的
资料低摩擦拿回来；只有现有工具确实缺关键能力时才写窄适配器；手动导出、复制、截图
和录屏只能是最后回退。

本文直接服务 PRD-01、PRD-02、PRD-09 和 AC-01、AC-02、AC-09。它只决定“从来源
怎样拿到东西”，不在这里设计清洗算法、知识判断或最终输出。

调查日期：2026-07-17。2026-07-17 使用豆包搜索、官方文档、项目 README、GitHub
仓库元数据和本机命令进行了第二轮交叉核验；发现抖音原路线的授权说明已经失效，本文
已据此降级，不沿用旧结论。同日第三轮按用户纠正，优先调查 GitHub 上能供现有 Agent
直接操作真实浏览器的 `browser-use` 与 `agent-browser`，不再把逐站专用 CLI 当作唯一
主线。

## 2. 证据等级与完成口径

| 等级 | 含义 |
| --- | --- |
| E0 文档证据 | 已核官方文档或项目说明，但没有在本机调用 |
| E1 工具证据 | 已核包、仓库、版本、维护状态并实际运行 help/list/doctor 等命令 |
| E2 连接证据 | 已在本机用真实授权身份调用来源；可把明确范围的只读样本保存到外部 recovery staging，但 Git/文档只记录能力结果，不记录真实内容或秘密 |
| E3 路线证据 | 已完成候选发现、用户选择、正文/附件取得、逐条状态和至少一次重收集 |

“工具路线已调研”不等于“来源已支持”。只有 E3 才能把产品能力显示为 available。
P2 可以确定路线和缺口；真实授权样本、逐条状态、C0 提交和重收集仍由对应功能阶段
完成。

P2-G7 的完成口径是：00 点名的来源都有真实调查、证据等级、最小授权、路线决策和诚实
缺口；当前机器可调用的代表性官方工具和通用 Agent 浏览器有实际调用/连接证据。它不要求
每个具体来源先达到 E3，否则会把 P4/P7 的产品验收错误地变成 P3 的前置条件。

## 3. 调查和决策顺序

```text
官方 API / 官方客户端能力
-> GitHub 上成熟的通用 Agent 浏览器 CLI / Skill / MCP / CDP 层
-> 能提供稳定结构、官方导出或媒体取得能力的站点专用工具
-> 只补已证明缺口的窄适配器
-> 手动导出、PDF、复制、截图、录屏（最后回退）
```

统一限制：任何工具只产生候选、临时导出件或读取结果。它不能写 Babata SQLite、分配
最终 ID 或把下载目录变成第二条权威持久化路径。Babata 核心确认选择后，才把工具结果
作为 C0 候选接入唯一收集链路。

## 4. 路线总表

| 来源 | 调研结论 | 推荐正常路线 | 用户最少提供 | 当前证据 |
| --- | --- | --- | --- | --- |
| 飞书文档、Wiki、知识库、云文档 | 路线已定 | 官方 `lark-cli` 直接调用，Babata 只包授权、范围选择和结果接入 | 一次飞书应用配置与用户 OAuth；以后选择文档/节点/范围 | E2 |
| 语雀 | 路线已定 | 已登录浏览器中的批量导出扩展；Agent 批处理可用 `yuque-dl` | 登录语雀；勾选知识库/文档。CLI 路线另需受控会话 token | E1 |
| OneNote | 路线已定 | Microsoft Graph OneNote API + 官方 Graph PowerShell SDK/薄调用 | 一次 Microsoft OAuth，最小 `Notes.Read` | E0 |
| 印象笔记 / Evernote | 路线已定 | `evernote-backup` 明确账号级范围后同步并导出 ENEX | Evernote OAuth；印象笔记需账号登录；明确选择“整个账号” | E1 |
| 微信收藏 | 缺口已识别 | Windows 微信官方收藏 UI + 窄 UI Automation 适配器；按类型调用网页/媒体工具 | 已登录 PC 微信，打开收藏并选择可见集合/范围 | E0，需 P7 实证 |
| 微信公众号文章 | 路线已定 | 已知链接用 OpenCLI `weixin download`；批量公众号用 `wechat-article-exporter` | 链接；批量历史需扫码登录一个公众号后台 | E1 |
| 微信视频号 | 组合路线已定 | 收藏 UI 发现候选，选中后用 `res-downloader` 捕获原媒体 | 已登录微信；显式启用本地代理/证书并播放所选项 | E1 |
| 微信聊天记录 | 路线已定但高风险 | 微信官方先迁移/备份到电脑；显式同意后用 `wechatDataBackup` 本地导出 | 同网手机确认或 PC 已有记录；明确选择会话/范围 | E1 |
| 知乎收藏与内容 | 路线待浏览器实证 | `browser-use`/`agent-browser` 复用已登录 Chrome 发现并遍历收藏；OpenCLI 稳定命令作为第二层 | Chrome 已登录；首次批准本机远程调试 | Agent Browser E1；真实来源 E0 |
| Bilibili 收藏与媒体 | 组合路线待实证 | 通用 Agent 浏览器列收藏和范围；`yt-dlp` 下载所选媒体，OpenCLI 补字幕/元数据 | Chrome 已登录；首次批准远程调试；媒体工具已安装 | Agent Browser/媒体工具 E1；真实来源 E0 |
| 小红书收藏 | 组合路线待实证 | 通用 Agent 浏览器发现收藏和详情；OpenCLI/MCP/媒体下载作为第二层 | Chrome 已登录；首次批准远程调试 | Agent Browser E1；真实来源 E0 |
| 抖音收藏 | 路线未定，错误主路线已撤回 | 先用通用 Agent 浏览器遍历已登录收藏页面；所选媒体再验证 `F2`/现有下载器 | Chrome 已登录；首次批准远程调试；给出收藏范围 | E0，明确缺口 |
| 浏览器书签 | 路线已定 | 窄 Babata 扩展直接调用官方 `chrome.bookmarks` API | 安装扩展；按需授予 bookmarks；选择文件夹/集合 | E0，需 P4 实证 |
| 浏览器当前页面、选区和网页收藏 | 路线已定 | 通用 Agent 浏览器负责导航/读取；SingleFile 保存保真页面；长期入口再用窄扩展 | Chrome 原生远程调试一次批准；给出页面/站点/范围 | Agent Browser E1；真实页面 E0 |
| 豆包对话 | 组合路线待实证 | 通用 Agent 浏览器遍历侧边栏/历史页；OpenCLI 稳定命令与会议 transcript 作为第二层 | Chrome 已登录；给出会话或时间范围 | Agent Browser/OpenCLI E1；真实来源 E0 |
| Kimi 对话 | Codex 路线已有真实正文实证 | 当前优先 Codex Chrome 复用正式版登录态并调用 Kimi 自身结构化历史/会话接口；长期再由 Skill 触发通用 Agent 浏览器，OpenCLI 只作第二层 | Chrome 已登录；给出会话、时间或数量范围 | E2：真实历史分页、2 条会话正文和外部 staging 已验证；未达 E3 |
| ChatGPT 对话 | 组合路线待实证 | 日常范围用通用 Agent 浏览器；OpenCLI 补结构化读取；账号级首次回收用官方 Data Export | Chrome 已登录；全量时只在 Data Controls 确认 | Agent Browser/OpenCLI E1；真实来源 E0 |
| 本地文件 | 路线已定 | Babata 核心文件选择器/受控目录扫描直接读取 | 选择文件、目录或明确监视范围 | 设计内置 |
| 第一方创作 | 路线已定 | Babata 创作入口直接提交同一核心链路 | 新建、修订或批注的明确动作 | 设计内置 |

### 4.1 用户到底最少要给什么

下面是面向实际执行的清单。用户只给一次来源/范围，并完成平台无法替代的登录、扫码、
OAuth 或首次浏览器权限确认；命令、浏览器导航、候选发现、分页、下载、重试、格式接入、
hash、状态和 staging 管理由 Agent/Babata 自主完成，直到范围结束或出现真实阻塞。用户
不需要把密码、Cookie、token、导出路径或元数据粘贴到聊天框，也不需要逐条替 Agent 点。

| 来源 | 用户一次性动作 | 每次只需选择 | Agent/Babata 负责 | 当前还差什么 |
| --- | --- | --- | --- | --- |
| 飞书 | 已完成官方应用配置和用户 OAuth；过期时重新确认 | 文档、搜索结果、Wiki 节点或明确范围 | 列候选、分页、正文、附件、版本、重收集和状态 | 一次真实正文+附件 E3 样本 |
| 语雀 | 优先在已登录 Chrome 安装语雀批量扩展；CLI 路线才授权本机会话 | 知识库、文档或全账号 bootstrap | 目录、图片、附件、断点续传、增量和 staging 接入 | 扩展真实样本；禁止让用户手抄 Cookie |
| OneNote | Agent 发起 `Notes.Read` device OAuth，用户在微软页面确认 | notebook、section、page 或范围 | Graph 分页、HTML、资源、更新时间和删除比对 | 一次 OAuth；官方模块已安装 |
| Evernote | Agent 启动 OAuth；印象笔记由用户在本地工具窗口输入账号/OTP | 明确“整个账号” bootstrap，之后选 notebook/note | sync、增量、expunged、ENEX 和附件 | 一次账号授权；CLI 已安装 |
| 微信收藏 | PC 微信登录并打开收藏；不提供数据库密钥 | 当前可见集合、分类或时间范围 | UI 枚举、类型分流、链接/图片/文件/视频取得 | Windows 窄 UI 适配器和真实样本 |
| 公众号文章 | 单篇无授权；批量历史时扫码登录自己的公众号后台 | 链接、公众号、合集或文章范围 | 正文、图片、音视频、发布时间和重收集 | OpenCLI Bridge 或批量工具登录 |
| 微信视频号 | PC 微信登录；首次明确同意本地代理证书，仅收集时开启 | 收藏中的视频号条目 | 打开所选项、捕获媒体、下载/解密、恢复代理 | 高风险工具需单独确认和实证 |
| 微信聊天 | 用微信官方功能把所选手机记录迁移/备份到电脑；另行同意本人数据库读取 | 会话和日期范围 | 本地导出、媒体、检索、增量和 staging 接入 | 版本兼容与法律/安全确认，最后验证 |
| 知乎 | Chrome 已登录；首次批准当前实例 remote debugging | 收藏夹、条目或时间范围 | 自主列收藏夹、分页、详情、图片和页面快照；必要时调用 OpenCLI | Browser Use/Agent Browser 真实探针 |
| Bilibili | Chrome 已登录；首次批准 remote debugging | 收藏夹、页、视频或分 P | 自主候选、翻页、元数据、字幕、媒体和附件；按需调用 `yt-dlp` | 通用浏览器探针；`yt-dlp`/ffmpeg 已就绪 |
| 小红书 | Chrome 已登录；首次批准 remote debugging | 收藏列表、时间或数量范围 | 自主列收藏、正文、评论、图片/视频和重收集；必要时调用 OpenCLI/MCP | 通用浏览器只读低频探针 |
| 抖音 | 目标路线为：Chrome 已登录，用户明确批准本机读取该 profile 的会话；不手抄 Cookie | 收藏/收藏夹、数量或时间范围 | 枚举、视频/图集、评论、增量和去重 | `F2` 安装/命令/浏览器会话探针尚未完成；完成前保持缺口 |
| 浏览器书签 | 安装 Babata 窄扩展，按需批准 `bookmarks` | 书签、文件夹或可见集合 | 读取层级、显示数量、逐条提交，按需抓网页 | P4 扩展实现和真实 Chrome 样本 |
| 当前页/选区 | 首次批准 Chrome remote debugging；长期入口再按需安装 Babata 窄扩展 | 页面、站点或链接范围 | 自主导航/读取；SingleFile 保真 HTML、元数据和缺失报告 | 通用浏览器探针；后续 P4 扩展/SingleFile 接入 |
| 豆包 | Chrome 已登录；首次批准 remote debugging | 会话、时间或数量范围 | 自主遍历历史、读取消息/附件；OpenCLI 补会议 transcript | 通用浏览器与附件覆盖探针 |
| Kimi | Chrome 已登录；首次批准读取当前实例 | 会话、时间或数量范围 | 自主列历史、读取长对话和附件；长期工具只在需要时由 Skill 触发 | Codex Chrome 已完成真实历史分页和长正文；全历史、附件/深研产物、状态和重收集留给 P7 |
| ChatGPT | Chrome 已登录；首次批准 remote debugging；全量时在 Data Controls 确认 | 单会话、时间范围或明确全账号 | 自主遍历选择性范围；全量解析官方导出 JSON/资产 | 通用浏览器探针；工作区资格按账号验证 |
| 本地文件 | 选择文件、目录或监视范围 | 同左 | 列候选、stream/hash、不可变复制和变更判断 | P3/P4 核心接入 |
| 第一方创作 | 明确点击新建、修订或批注 | 本次草稿/版本/批注 | 同一核心链路新增资料、版本或关系 | 后续创作入口，无第三方授权 |

### 4.2 当前机器已经替用户准备好的工具

截至 2026-07-18，本机已完成以下准备：

| 工具 | 当前状态 | 用户下一动作 |
| --- | --- | --- |
| `lark-cli 1.0.68` | 已安装，user/bot verified，真实 Wiki/Docs 只读调用成功 | 暂无；真实收集时选择范围 |
| `agent-browser 0.32.1` | 已安装；doctor 7 pass；已通过只读 action policy 连接正式版 Chrome 150.0.7871.129，列 27 个页面并读取当前页 snapshot | 长期独立收集时由 Skill 调用；不阻塞当前 Codex 回收 |
| `browser-use 0.13.6` / Browser Harness 0.1.6 | 已用本机 CPython 3.11 安装；正式版 Chrome、daemon 和本地连接 doctor 通过，列 28 个含内部页的 tab 并读取当前页 page info | 长期独立收集时由 Skill 调用；云认证不是本地前置 |
| `@playwright/cli 0.1.17` | 已全局安装并实际运行 version/help/attach/list | 只作比较与回退；真实 attach 前同样需批准浏览器控制 |
| `opencli 1.8.6` | 已全局安装；daemon `127.0.0.1:19825` 正常 | 暂无；只在通用浏览器层需稳定站点命令时再决定是否启用 Bridge |
| OpenCLI Browser Bridge 1.0.22 | 官方 ZIP 已下载、解压并按 release SHA-256 校验 | 暂不安装；其高权限扩展不再是下一步前置 |
| Microsoft Graph PowerShell 2.38.1 | `Authentication` 和 `Notes` 已安装；当前无登录 context | 在微软 device OAuth 页面批准 `Notes.Read` |
| `evernote-backup 1.13.1` | 已通过 `uv tool` 安装并核 help | 批准 OAuth；印象笔记在本地工具窗口登录 |
| `yuque-dl 1.0.85` | 已全局安装并核 command/options | 优先走扩展；CLI 批处理时授权本机会话 |
| `yt-dlp 2026.07.04` + ffmpeg 8.1.1 | 已安装，Bilibili 媒体工具链就绪 | Bridge 登录后选择视频 |
| `F2` 抖音候选 | 仓库、权限模型和收藏命令已核；本机隔离安装探针未完成，不能算已安装 | 暂无；先由 Agent 修复安装并跑无内容 help/doctor |

OpenCLI 扩展权限不是轻量权限：manifest 包含 `debugger`、`tabs`、`cookies`、`activeTab`、
`downloads` 和 `<all_urls>`。源码核到它只连接本机 `127.0.0.1:19825` daemon，但它能
读取和控制已登录页面，因此必须由用户在安装时明确批准；不能把“想调研来源”解释为
静默授予整个浏览器 profile。产品长期日常收集仍应使用权限更窄的 Babata 扩展。

## 5. 通用 Agent 浏览器导航层

网页登录来源先验证这一层，不能再默认为每个站点单独寻找爬虫。它的职责是让当前
Codex/Agent 复用真实浏览器和已有登录态，自主完成一个已给定范围内的导航、滚动、翻页、
候选发现、结构化读取、网络观察、下载和恢复；它只把结果交给 Babata staging/核心，
不直接写 SQLite 或最终原件。

### 5.1 Browser Use：当前第一实证顺位

[browser-use/browser-use](https://github.com/browser-use/browser-use) 为 MIT，未归档，GitHub
API 于 2026-07-17 核到约 105k stars、同日仍有推送，当前 PyPI 版本为 0.13.6。需要区分
两种用法：Babata 当前优先调查的是给 Codex 等既有 Agent 使用的 **Browser Use CLI/Skill**，
不是再嵌入一个由 Browser Use 自己调用大模型的 Python Agent。

官方 CLI/Skill 已明确证明：

- 本地正常模式直接连接运行中的 Chrome/Chromium CDP endpoint，保留当前 tabs、cookies、
  extensions 和 logins；Chrome 未开放时，工具打开 `chrome://inspect/#remote-debugging`，
  用户只需一次批准当前浏览器实例的远程调试；
- 现有 Agent 通过 `browser-use` 执行少量 Python，已有 `page_info`、tab、JS、raw CDP、
  accessibility tree、坐标点击、截图、下载、上传和 network request 等能力；
- 登录墙、密码、MFA 或含糊账号选择属于真正阻塞；已经登录且范围明确时，不应逐步把
  导航交回用户；
- 本地 Chrome 能访问本机和私网可达页面；云浏览器是并发、隔离或反风控的另一个可选
  运行环境，不是 Babata 本地优先路线的默认依赖。

本机已用现成 CPython 3.11 完成 `browser-use 0.13.6` 和 Browser Harness 0.1.6 安装。用户
在正式版 Chrome 150.0.7871.129 原生页面批准 remote debugging 后，Browser Harness doctor
显示 Chrome、daemon 和 1 个本地连接均正常；云认证失败是可选项，不影响本地。它通过
同一正式版 WebSocket 列出 28 个含 `chrome://newtab/` 的 tab，并只读取得当前页 page info。
这证明工具和真实浏览器连接，不证明任何平台来源已经完成候选、附件和重收集。

### 5.2 Agent Browser：并列首选候选

[vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser) 为 Apache-2.0，未归档，
GitHub API 于 2026-07-17 核到约 38.6k stars、2026-07-16 仍有推送；npm 当前版本 0.32.1。
本机已全局安装并实际运行 version/help/doctor，doctor 为 7 pass、0 warn、0 fail，
headless `about:blank` 启动通过，达到 E1。它是 Rust 原生 CLI，以 Chrome/Chromium CDP
和 accessibility snapshot/ref 为核心，不是 Playwright 多浏览器封装。

与 Babata 直接相关的能力包括：

- `--auto-connect` 或 `connect <port>` 连接 Chrome 144+ 原生远程调试，复用已有登录态；
- snapshot/read/tab/network requests/HAR/download/upload/record/PDF/截图和结构化 JSON；
- action policy、下载/上传/eval 确认、页面内容边界、最大输出、加密 auth/session 和
  state expiry，适合把“只收集”约束成可执行策略；
- `skills get core --full` 提供与安装版本一致的 Agent 工作流，不依赖记忆过时命令。

必须记录的限制：安全功能默认不是全部开启；复用现有 Chrome 的 CDP/auto-connect/profile
模式不能同时启用严格 `--allowed-domains`，因为旧页面和已保存来源可能在策略接管前运行。
因此真实登录 profile 的首次探针只能在用户一次批准后进行。本机已使用
`06_config/agent-browser-read-only-policy.example.json`，默认拒绝，只允许连接控制面、
tab list、snapshot、read、get 和 wait；导航、点击、填写、脚本、网络修改、下载、上传和
state 均未开放。它通过正式版 WebSocket 列出 27 个页面并读取当前页 snapshot。内置 Chrome
for Testing 150.0.7871.24 只用于无账号 doctor，不读取用户数据。

### 5.3 当前选择

当前必须区分三个层次：

1. **P2 收尾**：Kimi 已用真实历史和长正文证明 Codex Chrome 能落到具体平台，代表性实证
   已取得；按用户最新顺序立即回到 P2，不继续用无限扩大的存量回收拖延系统阶段。
2. **按需存量回收**：用户给出具体来源和范围时，优先用当前 Codex 已有的官方连接器/Skill、
   Codex Chrome 正式版登录态，以及只在桌面 UI 无结构化入口时使用 Computer Use；不等待
   Babata 长期自动化完成。
3. **脱离 Codex 的长期收集**：Browser Use 与 Agent Browser 均已证明可连接正式版 Chrome，
   后续由 Skill 触发；最终主次由长期真实来源 E3 证据决定。

Microsoft `playwright-cli` 作为比较组保留。OpenCLI 不承担唯一通用浏览器主线，只在有
确定站点命令时列结构化候选、读详情或下载，补充 Codex/通用浏览器层的稳定性和媒体能力。

### 5.4 站点命令层：OpenCLI

OpenCLI 是本次调查中覆盖面最大的 Agent 入口，不能再忽略。已实际运行：

```text
npx -y @jackwener/opencli@1.8.6 --help
npx -y @jackwener/opencli@1.8.6 list
npx -y @jackwener/opencli@1.8.6 <site> <command> --help
npx -y @jackwener/opencli@1.8.6 doctor
```

核验结果：

- [jackwener/OpenCLI](https://github.com/jackwener/OpenCLI) 未归档，Apache-2.0，
  2026-07-12 仍有提交；本机核到 npm 版本 1.8.6；
- 包内有 160+ 站点适配器和结构化 JSON/YAML/Markdown 输出，能复用用户浏览器登录态；
- 与本项目直接相关的现成命令覆盖 Bilibili、知乎、小红书、ChatGPT、Kimi、豆包、
  公众号文章和通用网页；
- OpenCLI 1.8.6 已全局安装，本机 daemon 已运行，但 Browser Bridge 扩展尚未连接。
  `doctor` 明确报告 extension missing，因此当前只能证明命令和连接机制，不能宣称
  这些真实账号路线已通过；
- 官方扩展 1.0.22 已从 v1.8.6 release 下载到用户工具目录，SHA-256 与 release digest
  `9d2e3d053948beab5d97124aa79b1532d2122e33e461eca56cac113afd33207a`
  一致；Chrome Web Store/扩展安装页禁止脚本自动确认，仍需用户批准安装；
- 最小补充动作是从 OpenCLI release 安装 Browser Bridge 到用户选择的 Chrome profile，
  用户在相应站点保持登录。Babata/Agent 不要求用户复制 Cookie；
- OpenCLI 列出的外部 `wx-cli` 已于 2026-07-15 被 GitHub 以 DMCA 屏蔽，本机自动安装也
  失败。该条目是陈旧注册信息，不能作为微信聊天路线。

Babata 对 OpenCLI 只需要薄调用：列候选、读取所选条目、接收结构化输出和临时文件，
再交给核心。不得复制它的 160 个站点适配器重新造一遍。

## 6. 文档和笔记来源

### 6.1 飞书

推荐：直接使用官方 [Lark CLI](https://github.com/larksuite/cli)；官方介绍见
[Lark CLI: Put your AI to work in Lark](https://open.larkoffice.com/document/mcp_open_tools/feishu-cli-let-ai-actually-do-your-work-in-feishu)。

本机证据：

- `lark-cli 1.0.68` 已安装；仓库为 MIT、官方维护，2026-07-17 仍有提交；
- `lark-cli auth status --json --verify` 显示 user/bot 身份均 verified，用户身份具备
  docs、drive、search、wiki 等 scope；
- 真实 user 身份调用 `wiki +space-list` 和 `docs +search` 均返回 `ok: true`；
- `drive +search` 可按标题、文档类型、时间、owner、文件夹或 Wiki space 列候选；
- `wiki +space-list`、`wiki +node-list`、`wiki +node-get` 负责空间和节点层级；
- `docs +fetch` 读取 Docx/Wiki 正文；内嵌 Sheet/Base 需继续调用对应官方 CLI 域；
- `docs +media-download`、`drive +download/+preview/+export` 取得图片、附件、原文件或
  官方导出；`drive +version-history/+version-get` 取得版本；
- Wiki URL 必须先解包为真实对象类型和 token，不能把 wiki token 当文档 token。

正常体验：Babata 调 CLI 列出用户当前搜索、Wiki 节点或文件夹候选，用户选单条、可见
集合或明确节点范围后才读取正文和附件。连接成功不执行账号全量复制。

最少授权：首次 `config init` 配置官方应用；用户身份按 docs/drive/wiki/search 最小范围
OAuth。当前机器已有可刷新登录态，实际产品仍要支持过期重授权。

限制：`wiki spaces list` 不返回个人 `my_library`，需要单独解析；评论、历史版本、
Sheet/Base/Slides/画板分属不同域；权限、密级和附件下载限制必须逐项报告。删除/移动
需要以稳定 token、重新列表和来源事件判断，不能凭正文为空判断。

决策：**直接用**。现有“手动导出 Markdown”只保留为 API/CLI 故障、权限不允许或离线
恢复的最后回退。

### 6.2 语雀

用户日常路线优先使用已登录浏览器中的批量扩展
[ouyangfeng2022/yuque-exporter](https://github.com/ouyangfeng2022/yuque-exporter)：

- Manifest V3，本地运行，2026-06-24 仍有提交，MIT；
- 不要求用户手抄 token；直接复用当前语雀页面登录态；
- 能加载知识库列表、让用户勾选知识库、保留 TOC 目录，并调用语雀官方导出接口取得
  Markdown/Lake/Word/PDF/JPG；
- 支持进度、分批限流和私有知识库，只要当前账号有访问权；
- 它产出的浏览器下载文件只能进入 Babata staging，再由核心提交，不能直接成为 C0。

Agent 批处理路线使用 [gxr404/yuque-dl](https://github.com/gxr404/yuque-dl)：npm
1.0.85，2026-06-27 更新，支持单文档、多知识库、当前账号全部知识库、图片、附件、
断点续传和 `--incremental`。它需要 `_yuque_session` 或企业实例的 cookie key/value，
因此只能在用户明确允许批处理并把会话秘密放入受控 secret store 时使用，不能要求用户
每次打开 DevTools 手抄 token。

补充候选：桌面工具 `ydhawesome/yuque-exporter` 能导出小记和知识库，但要求在第三方
程序中输入账号密码，不作为首选。

限制：评论、协作修订历史、部分表格图表/画板和“收藏但无导出权”的覆盖仍需真实样本
验证。官方导出件应与页面快照/媒体附件分别记录，不能把格式转换结果冒充唯一原件。

决策：**扩展直接用 + CLI 受控批处理**。不再规划逐篇手动 Markdown。

本机已全局安装 `yuque-dl 1.0.85` 并核对 `doc/batch/user`、图片、附件、TOC 和
`--incremental` 参数。它现在只缺授权；正常产品路线仍优先浏览器扩展，不能因为 CLI
已经装好就要求用户手抄 `_yuque_session`。

### 6.3 OneNote

推荐官方 Microsoft Graph OneNote API，使用官方 Graph PowerShell SDK 或很薄的 HTTP
调用，不先写专用爬虫：

- [Get OneNote content and structure](https://learn.microsoft.com/en-us/graph/onenote-get-content)
  可列 notebooks、section groups、sections、pages，并取得页面 HTML；
- [Get resource](https://learn.microsoft.com/en-us/graph/api/resource-get) 可取得页面里的
  图片和文件二进制；
- 最小 delegated permission 为 `Notes.Read`；用户通过一次 Microsoft 浏览器/device
  login 授权，Agent 后续调用；
- 使用页面/分区/笔记本稳定 ID、`lastModifiedDateTime` 与内容 hash 重收集；删除通过
  重新列表比对，不假定 Graph 提供完整 delta。

Windows 离线回退可用
[alxnbl/onenote-md-exporter](https://github.com/alxnbl/onenote-md-exporter)，其仓库未归档，
2025-12 仍有提交，可导出笔记本层级、图片和附件；但依赖 Windows 桌面 OneNote/Word，
只作为 Graph 受限或一次迁移时的后备。

限制：外部组织分享笔记本、21Vianet、嵌入对象和历史版本需单独验证。HTML 是官方 API
表示，不等同于原生 `.one` 文件；可取得原文件时应另存 C0。

决策：**官方 API/SDK 薄调用**。

本机已安装 `Microsoft.Graph.Authentication` 和 `Microsoft.Graph.Notes` 2.38.1，已核到
`Connect-MgGraph`、notebook/section/page/content/resource 命令；`Get-MgContext` 当前
为空。下一步由 Agent 执行 `Connect-MgGraph -Scopes Notes.Read
-UseDeviceAuthentication -ContextScope CurrentUser`，用户只在微软页面确认，不需要
自己运行 PowerShell。

### 6.4 Evernote / 印象笔记

推荐 [vzhd1701/evernote-backup](https://github.com/vzhd1701/evernote-backup)：MIT、未归档，
2025-04 仍有提交；支持 Evernote 和印象笔记，能同步全部 notebooks/notes/resources 到
本地 SQLite，之后离线导出每个 notebook 或每条 note 的 ENEX，并记录新增、更新和
expunged 项。

最少授权：Evernote 首次 `init-db` 走浏览器 OAuth；印象笔记目前不支持 OAuth，需要
账号登录和一次性验证码。密码/token 只能由工具自己的受控认证流程或 secret store
使用，不能进入 Git、日志或 Babata 数据记录。

重要范围限制：该工具的 sync 是账号级同步，不提供连接后先远端枚举、再只下载部分
notebook 的正常路径。因此它只能在用户明确选择“回收整个 Evernote/印象笔记账号”时
运行，不能因完成登录就自动开始。同步后可以从本地索引展示 notebook/note 候选，再由
用户选择哪些提交 Babata C0；未选择的工具缓存仍属 staging/C3，可清理。

官方客户端 ENEX/HTML 导出见
[Evernote export guidance](https://evernote.com/learn/how-to-export-your-notes)，只作为无法
授权 CLI 时的回退。

限制：tasks/reminders 不在公开 API，工具需要额外桌面 token；ENEX 保留资源但不等同于
所有客户端视觉状态；账号级初次同步可能很大。

决策：**现成 CLI，必须显式账号级确认**。

本机已安装并运行 `evernote-backup 1.13.1 --help`。下一步只缺 `init-db` 的账号授权；
用户不需要安装 Python、pipx 或编写命令。

## 7. 微信来源

### 7.1 微信收藏

没有找到一条在 Windows 上同时满足“仍维护、完整覆盖收藏类型、Agent 可调用、无需
解密私有数据库”的成熟工具。Mac 的
[zhuyansen/wx-favorites-report](https://github.com/zhuyansen/wx-favorites-report) 活跃但不
适用当前 Windows 工作区；Windows 搜到的工具多只处理收藏中的聊天记录、公众号链接或
表情，不能冒充完整收藏路线。

因此允许的窄适配器是：基于 Windows UI Automation 操作官方 PC 微信收藏窗口，只读
枚举用户当前打开的收藏列表和可见范围，显示类型/标题/时间/来源，再由用户选择。不同
类型继续分流：

- 公众号链接 -> OpenCLI `weixin download` 或 SingleFile；
- 普通网页 -> SingleFile；
- 图片/文件 -> 官方 UI 的打开/另存能力，进入 staging；
- 视频号 -> `res-downloader`；
- 收藏中的聊天记录/笔记 -> UI 可复制结构；保留截图/原附件作为原貌证据。

最少用户动作：PC 微信已登录，打开收藏并选择当前可见集合、分类或时间范围。不得扫描
内存、解密数据库或静默全量遍历。只有用户另行明确同意本地数据库处理时，才评估更深
路线。

重收集缺少可靠公开 native ID 时，以来源 UI 定位信息、类型、时间、来源 URL 和内容
hash 组合识别，并诚实标注可能重复。

决策：**窄 UI 适配**，是本轮调查确认的真实缺口，不再用“手工一条条复制”代替设计。

### 7.2 微信公众号文章

单篇/已知链接优先用 OpenCLI：

```text
opencli weixin download --url <mp.weixin.qq.com/s/...> --download-images
```

它输出标题、作者、发布时间、Markdown 和本地图片；收藏来源先由微信收藏适配器拿 URL。

批量公众号历史使用
[wechat-article/wechat-article-exporter](https://github.com/wechat-article/wechat-article-exporter)：
MIT、未归档，2026-07-15 仍有提交；支持扫码登录公众号后台、搜索公众号/文章、合集，
导出 HTML/JSON/Excel/TXT/Markdown/DOCX，包含图片和内嵌音视频；阅读量/评论需要额外
credentials，必须单独说明。

最少用户动作：单篇只提供链接；批量时扫码登录自己有权限的公众号后台并选择账号、
文章/合集范围。不得登录后自动抓取所有公众号。

决策：**直接用现成工具**。

### 7.3 微信视频号

OpenCLI 的 `wechat-channels` 当前只有登录、whoami 和发布，不具备个人收藏发现/下载，
不能误用。媒体取得使用
[putyy/res-downloader](https://github.com/putyy/res-downloader)：Apache-2.0、未归档，
2026-06 仍有提交，支持视频号、小程序、抖音、小红书等资源，通过本地代理捕获视频、
音频、图片和 m3u8，并提供视频号解密下载。

正常组合：微信收藏 UI 先列出视频号候选 -> 用户选择 -> 打开/播放所选内容 ->
`res-downloader` 捕获原媒体 -> Babata 接收媒体和同时取得的标题/作者/来源上下文。

最少用户动作：PC 微信已登录；首次显式同意安装本地代理证书和启用捕获；选择并播放
所选视频。代理必须默认关闭，失败后恢复系统代理，不能作为常驻后台窃听器。

限制：资源 URL 可能短时有效，代理/证书有安全风险；候选发现、作者信息和收藏层级不由
下载器提供，必须由微信 UI 上下文补齐。

决策：**现成媒体工具 + 窄候选适配**。

### 7.4 微信聊天记录

先用微信官方路径把手机记录带到电脑。官方说明：
[如何透过电脑备份/还原 WeChat 聊天记录？](https://cs.help.wechat.com/hc/zh-cn/articles/11917889397775-%E5%A6%82%E4%BD%95%E9%80%8F%E8%BF%87%E7%94%B5%E8%84%91%E5%A4%87%E4%BB%BD-%E8%BF%98%E5%8E%9F-WeChat-%E8%81%8A%E5%A4%A9%E8%AE%B0%E5%BD%95)。
用户在同一 Wi-Fi 上从手机确认全部或所选会话，官方备份只供微信恢复，不是 Babata
可读格式。

通用导出候选使用
[git-jiadong/wechatDataBackup](https://github.com/git-jiadong/wechatDataBackup)：Apache-2.0、
未归档，2026-01 仍有提交，支持文字、图片、视频、语音、文件、链接、小程序、视频号等
消息，支持多账号、检索和增量导出。

淘汰项：OpenCLI 注册的 `wx-cli` 仓库已经 DMCA blocked；PyWxDump 仓库当前说明为
“删库”、无明确许可证且长期未提交，都不能列为正常主路线。

最少用户动作：先让官方 PC 微信拥有目标记录；明确选择会话/日期范围，并单独同意本地
数据库读取。工具只在本机运行，导出目录是 staging。未同意时只能使用官方 UI 做当前
会话的窄选择性复制/附件下载。

限制：微信版本升级可能破坏数据库兼容；数据库解密和进程读取具有账号、法律和安全
风险；必须限定本人数据、显式触发、版本白名单、离线运行和可撤销。

决策：**官方迁移 + 明确同意后的成熟本地导出工具**。

## 8. 内容平台

### 8.1 知乎

OpenCLI 已有完整候选链：

```text
opencli zhihu collections
opencli zhihu collection <collection_id> --offset ... --limit ...
opencli zhihu answer-detail <answer_id>
opencli zhihu download --url <article_url> --download-images
```

输出含收藏夹 ID、标题、数量、条目类型、作者、摘要、票数和 URL。用户可先选收藏夹和
条目，再读取回答/文章正文；文章 Markdown 下载可带本地图片。当前页保真副本可追加
SingleFile。

最少用户动作：Chrome 已登录知乎并一次批准当前实例 remote debugging；用户给出收藏夹、
条目或时间范围。Agent 不需要用户安装高权限 Bridge 或复制 Cookie；OpenCLI 只在其
确定性命令确实更稳定时作为第二层。

限制：`download` 专门覆盖文章；回答、问题、想法需对应 detail 命令或网页快照；评论、
视频和公式保真度要用真实样本验证。

决策：**Browser Use/Agent Browser 负责发现和遍历，OpenCLI 确定性命令作为第二层**。
知乎专用导出扩展只作为 UI 备选。

### 8.2 Bilibili

OpenCLI 已实际核到：

```text
opencli bilibili favorite --fid <folder_id> --page ... --limit ...
opencli bilibili video <bvid>
opencli bilibili subtitle <bvid>
opencli bilibili download <bvid> --quality ... --page ...
```

`favorite` 列标题、作者、播放量、URL；媒体下载调用成熟的
[yt-dlp](https://github.com/yt-dlp/yt-dlp)，字幕和元数据由 OpenCLI 提供。Babata 先展示
收藏夹/页面候选，再按所选 bvid 收集，不因登录就下载全部收藏。

最少用户动作：Chrome 已登录 B 站并一次批准 remote debugging；需要会员/已购画质时
使用该登录态。用户给出收藏夹、页或视频范围一次；`yt-dlp` 和 ffmpeg 由 Agent 调用。

淘汰项：`nilaoda/BBDown` 与 `Nemo2011/bilibili-api` 当前均已归档，不能作为长期主路线。

限制：付费、充电、地区限制、失效视频和版权限制必须返回 inaccessible/removed；弹幕、
评论、封面和多 P 是独立附件，不得只存合并视频。

决策：**Browser Use/Agent Browser 发现范围 + OpenCLI 补结构化信息 + `yt-dlp` 下载媒体**。

本机已安装 `yt-dlp 2026.07.04`，并发现现有 ffmpeg 8.1.1；媒体工具链不再需要用户
配置。当前连接缺口是通用 Agent 浏览器尚未在用户自己的 B 站登录态做真实探针。

### 8.3 小红书

OpenCLI 已实际核到：

```text
opencli xiaohongshu saved --limit ...
opencli xiaohongshu note <full-url-with-xsec_token>
opencli xiaohongshu comments <url>
opencli xiaohongshu download <url-or-xhslink>
```

`saved` 返回笔记 ID、标题、作者、点赞、类型和 URL；`note` 取得正文和互动数据；
`download` 取得图片/视频。备选
[xpzouying/xiaohongshu-mcp](https://github.com/xpzouying/xiaohongshu-mcp) 未归档、
2026-07 仍有提交，支持二维码登录、搜索、笔记详情和互动，但是否完整列出当前用户收藏
仍需按版本验证，因此只作为第二层登录/结构化备选。

最少用户动作：Chrome 已登录小红书并一次批准 remote debugging，或 MCP 首次扫码；
用户给出收藏、时间或数量范围一次，Agent 自主读取详情和媒体。

限制：`xsec_token` 可能失效；无官方开放 API，页面/接口变化和风控风险高；只读低频，
不自动点赞、评论、关注或发布。

决策：**Browser Use/Agent Browser 主导航，OpenCLI/MCP 作为稳定命令和登录备选**。

### 8.4 抖音

OpenCLI 当前抖音适配器偏创作者后台，没有本人收藏命令。第一轮曾把
[JoeanAmier/TikTokDownloader](https://github.com/JoeanAmier/TikTokDownloader)
（DouK-Downloader）定为主路线；第二轮核验后撤回：

- 项目仍未归档，GPL-3.0，2026-07-14 仍有提交，也确实列出收藏、收藏夹、增量和
  CSV/XLSX/SQLite；
- 但当前 README 明确警告其加密参数算法已经过期且不再维护，部分功能需要使用者自己
  提供参数生成代码；
- “扫码登录获取 Cookie”已经标记失效，“从浏览器读取 Cookie”已经标记弃用；当前
  可操作说明回到了手动复制 Cookie/剪贴板；
- 这与 Babata 的最低摩擦原则冲突，不能再写成“用户只扫码，Agent 全部完成”。

新的首选候选是 [Johnserf-Seed/f2](https://github.com/Johnserf-Seed/f2)：Apache-2.0、
未归档，GitHub API 核到 2026-04-13 仍有推送；README 明确列出抖音收藏作品、收藏夹
作品、收藏原声和相应 CLI，并使用 `browser_cookie3` 从本机浏览器会话取得授权。
目标交接应是：用户在选定 Chrome profile 登录抖音并明确批准本机读取该 profile 的
会话，选择收藏/收藏夹、数量或时间范围；Agent 负责安装、命令、分页、下载、去重、
临时凭据和 staging 接入，不要求用户打开 DevTools 复制 Cookie。

但 `F2` 当前仍只有 E0：本机隔离安装尝试没有完成到可运行 help/doctor，不能据 README
宣称可用。后备候选 [Johnserf-Seed/TikTokDownload](https://github.com/Johnserf-Seed/TikTokDownload)
也声明 `--auto-cookie`、收藏与扫码，但主仓库最后代码推送为 2024-06-28，只能在 `F2`
失败后再做兼容性探针。`anYuJia/better-douyin` 的公开源码明确不包含真实连接器、签名、
Cookie 或下载解析，不能作为可执行路线。

最少用户动作的目标没有变化：只登录、批准必要的本机会话读取并选范围。若所有现成
工具都无法做到，才允许为已登录抖音页面写窄候选发现适配器；所选条目的媒体可继续用
`res-downloader` 等现有工具取得。任何要求用户手抄 Cookie 或自行提供签名算法的路线
只能列为受限回退，不是正常产品体验。

限制：抖音无稳定公开收藏 API，非官方路线会受页面变化、Cookie 失效、验证码、签名
和风控影响；工具 SQLite 只是 staging/C3，不是 Babata 权威；下载权限和版权必须尊重
平台与内容所有者。

决策：**旧主路线撤回；`F2` 为待实证首选候选，达到 E1/E2 前保持未定和 disabled**。

## 9. 浏览器来源

### 9.1 书签

官方 Chrome `chrome.bookmarks` API 已提供 `getTree/getChildren/getSubTree/search` 和事件，
只需扩展 manifest 的 `bookmarks` 权限。官方参考：
[chrome.bookmarks](https://developer.chrome.google.cn/docs/extensions/reference/api/bookmarks)。

通用 Agent 浏览器可以操作 Chrome 书签管理页面，但官方 `chrome.bookmarks` API 对完整
层级和事件更稳定。这里仍允许窄 Babata 扩展；不是因为没有 Agent 浏览器，而是因为
书签树有更窄、更确定的官方能力。扩展不读取 Chrome profile 的 `Bookmarks` 文件：

```text
用户按需授予 bookmarks
-> 扩展读取树并显示文件夹/数量/标题/URL/层级
-> 用户选择单条、文件夹或可见集合
-> 只把选择结果提交本地 CollectorSession
```

书签本身只是 locator。用户给出书签文件夹或范围一次后，Agent 可让 worker 后续用
SingleFile、Browser Use/Agent Browser 和站点工具自主取得网页原貌，不要求逐条确认。
重收集以 bookmark node ID、URL、层级和页面 hash 组合判断；书签删除不等于已收 C0 删除。

决策：**官方扩展 API 的窄适配器**。

### 9.2 当前页面、选区和网页收藏

候选发现由窄扩展使用 `activeTab` + `scripting`：只有用户点击收集时读取当前 tab、选区、
标题、URL 和声明元数据，不要求永久 `<all_urls>`。完整网页原貌复用
[SingleFile](https://github.com/gildas-lormeau/SingleFile) 和
[single-file-cli](https://github.com/gildas-lormeau/single-file-cli)：前者 AGPL-3.0、未归档，
能保存当前 tab、选区、多个 tab、书签页面及其图片/CSS/font/frame 为单 HTML；CLI 通过
Chrome DevTools Protocol 适合公开页和批量 URL。

Agent 操作已登录网站时先验证 Browser Use CLI 和 Agent Browser 对当前 Chrome 的原生
CDP 连接，不导出 Cookie；已有稳定站点命令时再调用 OpenCLI。SingleFile 扩展负责保真
页面，Babata 清洗阶段再用 Readability/正文提取，不用正文 Markdown 覆盖 HTML 原件。

最少用户动作：第一次允许 Chrome 当前实例远程调试；给出当前页、站点、书签文件夹或
其他明确范围。范围内的导航、滚动、翻页和下载由 Agent 完成。长期日常入口需要时再安装
权限更窄的 Babata 扩展；页面需要登录时复用当前真实浏览器，不手抄 Cookie。

限制：DRM、跨域 iframe、无限滚动、懒加载、canvas/WebGL、临时下载链接和站点 CSP
可能导致不完整；必须显示附件/媒体缺失和当前捕获范围。

决策：**Browser Use/Agent Browser 导航 + SingleFile 保真 + 站点工具补充；窄 Babata
扩展作为长期触发入口，不作为当前调研前提**。

## 10. AI 对话来源

### 10.1 豆包

OpenCLI 已核到 `doubao history --limit`、`detail <id>`、`read`，以及会议对话的
`meeting-summary`、`meeting-transcript`。它能从侧边栏列候选 ID/标题/URL，再读取用户
选中的会话；不需要手动复制每轮消息。

最少用户动作：Chrome 已登录豆包并一次批准 remote debugging；给出会话、时间或数量范围。

限制：当前命令未证明能下载普通对话附件、图片、引用网页和全部历史；需要用真实样本
补充 DOM/网络附件覆盖，必要时对当前会话追加 SingleFile 页面快照。会议 transcript 可
下载，但摘要是派生物，不能替代原会话/音频。

决策：**Browser Use/Agent Browser 主导航，OpenCLI 补稳定命令；附件缺口实证后再窄补**。

### 10.2 Kimi

OpenCLI 已核到 `view-all-history`、`history --limit`、`detail <id>` 和 `read --conv <id>`。
OpenCLI 可先进入完整历史页，列出标题、ChatId 和 URL。主路线改为通用 Agent 浏览器
在用户给定范围内自主滚动和读取；Kimi 专用导出用户脚本、OpenCLI 和通用 AI Chat
Exporter 作为结构化或 UI 备选。

最少用户动作：Chrome 已登录 Kimi 并一次批准 remote debugging；给出会话/时间范围。

限制：附件、引用来源、深度研究产物、超长对话的懒加载和删除状态尚未实证。Kimi Code
CLI 的 `/export` 只覆盖 Kimi Code session，不能冒充 Kimi 网页聊天全量路线。

决策：**Browser Use/Agent Browser 主导航，OpenCLI 补稳定会话标识和读取命令**。

### 10.3 ChatGPT

选择性日常收集先用 Browser Use/Agent Browser 在用户给定范围内遍历真实 ChatGPT 页面；
OpenCLI 的 `history`、`detail <id>`、`read` 和 `deep-research-result` 作为确定性结构化补充。

账号级首次回收使用 OpenAI 官方 Data Export：
[How do I export my ChatGPT history and data?](https://help.openai.com/en/articles/7260999-how-do-i-export-my-chatgpt-history-and-data)。
用户在 Settings -> Data Controls -> Export Data 确认，邮件 ZIP 可能最长等待 7 天；包含
`conversations.json`（大导出可能分片）、对话资产和元数据。它适合用户明确选择“全账号
首次回收”，不适合日常单条收集。

最少用户动作：日常为 Chrome 已登录 ChatGPT、一次批准 remote debugging 并给出会话/
时间范围；首次全量只在 Data Controls 确认。邮件等待、下载、解压和解析由 Agent 继续
处理。Business/Enterprise/Edu 的导出资格受工作区策略影响，不能承诺可用。

限制：OpenCLI 当前未证明能取得全部附件、语音、画布、项目文件和完整历史；官方 ZIP
是异步全量且不是增量 API。对话稳定 ID + 更新时间/内容 hash 用于重收集。

决策：**Browser Use/Agent Browser 选择性路线 + OpenCLI 补充 + 官方导出全量 bootstrap**。

## 11. 本地和第一方来源

### 11.1 本地文件

不需要外部爬虫。Babata 核心通过文件选择器、拖放、明确目录扫描或用户授权的 watched
folder 直接列候选：路径、相对层级、类型、大小、mtime、可读性和可能附件。用户确认后
由 Rust 核心流式读取、hash、复制到 C0；目录扫描只覆盖用户选定范围。

重收集使用操作系统文件 ID（可得时）、规范路径、mtime/size 和内容 hash；同名替换、
移动、删除和权限不足分别记录，不原地覆盖。快捷方式只作为 locator，需明确是否跟随。

决策：**核心内置直接读取**。

### 11.2 第一方创作

自己的新笔记、草稿、反思、批注和人工判断不是“输出回写”，而是 first-party 来源：

- 新写 -> 新资料；
- 修订 -> 新版本；
- 批注 -> 独立资料并关联目标；
- 导入外部编辑器文件 -> 本地文件来源；
- Skill/Agent 只能提交用户确认的草稿候选，不能直接改历史版本。

最少用户动作就是明确的新建、修订或批注。创作 UI、CLI 和 Skill 调同一个 Rust 核心用例，
不需要另建作者数据库或 Obsidian 双写。

决策：**核心内置同链路**。

## 12. 已淘汰或降级的候选

| 候选 | 结论 |
| --- | --- |
| 飞书手动 Markdown 作为正常路线 | 淘汰；官方 `lark-cli` 已安装且真实调用成功 |
| OpenCLI `wx-cli` | 淘汰；仓库 DMCA blocked，自动安装失败 |
| PyWxDump | 不作主路线；仓库当前“删库”、无明确许可证、长期未提交 |
| BBDown | 降级；仓库已归档，可作用户自选旧工具，不作长期依赖 |
| bilibili-api-python | 降级；仓库已归档 |
| DouK-Downloader 作为抖音正常路线 | 降级；签名算法失效，扫码失效、浏览器 Cookie 读取弃用，现状要求手抄 Cookie/自备参数生成器 |
| `better-douyin` 公开源码 | 淘汰；仓库明确不包含真实平台连接器、签名、Cookie 或下载解析 |
| OneNote 手动导出 | 降级；Graph API/SDK 是正常路线 |
| Evernote 手工逐本导出 | 降级；账号级明确确认时用 `evernote-backup` |
| 通用“万能爬虫” | 淘汰；已有站点 CLI/扩展的来源不得重新造重型爬虫 |

## 13. 后续实证顺序

Kimi 已完成第一次具体平台练手：Codex Chrome 在正式版登录态下读取真实历史页，识别
`FeedService/ListFeeds`、`ChatService/GetChat` 和 `ChatService/ListMessages`，并把两条
明确范围的会话原始 JSON、manifest 和 SHA-256 写入外部 recovery staging。其中第二条为
104,476 字节的长正文响应。该结果证明当前 Codex 路线可用，但不包含全历史、附件、逐条
状态或重收集，因此保持 E2，不能把 Kimi 或 P4 标为 available。

后续顺序是：

1. 完成 P2 收尾并进入 P3，建立 recovery staging 进入唯一 C0 路径的底座；
2. 用户再次给出具体来源/范围时，按“官方 API/导出 -> Codex Chrome -> Computer Use ->
   已验证站点工具”的顺序继续存量回收；
3. 原始导出件和媒体仍先进入外部 `BABATA_DATA_HOME` recovery staging，保留来源、范围、
   时间、工具和 hash，不进入 Git，不冒充已经提交 C0；
4. P3 稳定后把 recovery staging 通过唯一核心链路提交 C0，校验成功后清理重复 staging；
5. 长期 Browser Use/Agent Browser、P4 扩展和更多来源 E3 验证转为后续 Skill/自动化工作。

## 14. P2-G7 完成判断

所有 00 已点名来源都已有逐站调查、证据等级、最小授权、正常路线、回退与明确缺口；
Browser Use、Agent Browser、Playwright CLI 与 OpenCLI 的通用方向已经实际调用。Browser Use
与 Agent Browser 已连接同一个正式版 Chrome；飞书 `lark-cli` 已完成真实用户授权调用；
Codex Chrome 又在 Kimi 完成真实历史分页、长正文读取和外部 staging 样本。

因此 **P2-G7 已通过**。抖音等具体来源仍缺 E3，Kimi 也没有完成附件、逐条状态和重收集；
这些来源必须继续保持 disabled，缺口进入 P4/P7。E3 未完成不再错误阻塞 P2，也不能因为
P2-G7 通过就显示任何未验收来源 available。

<!-- P2-G7: passed -->
<!-- P2-G7-SOURCES: source.feishu source.yuque source.onenote source.evernote source.wechat_favorites source.wechat_articles source.wechat_channels source.wechat_chats source.zhihu source.bilibili source.xiaohongshu source.douyin source.browser_bookmarks source.browser_pages source.doubao source.kimi source.chatgpt source.local_files source.first_party -->
<!-- P2-G7-TOOLS: tool.lark_cli tool.agent_browser tool.browser_use tool.codex_chrome tool.opencli -->

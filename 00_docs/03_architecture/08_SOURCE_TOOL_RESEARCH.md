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

### 3.1 工具调查顺序

```text
官方 API / 官方客户端能力
-> GitHub 上成熟的通用 Agent 浏览器 CLI / Skill / MCP / CDP 层
-> 能提供稳定结构、官方导出或媒体取得能力的站点专用工具
-> 只补已证明缺口的窄适配器
-> 手动导出、PDF、复制、截图、录屏（最后回退）
```

### 3.2 调查后的实际路线优先级

```text
1 官方免费批量迁移或导出
-> 2 现有可用插件或脚本导出
-> 3 Agent 主导的省心导出
-> 4 少量开发后的批量导出
-> 5 收费会员/VIP
-> 6 重开发或复杂工具流
-> 7 持续人机交互配合
-> 8 只能手工操作
```

3.1 回答“要调查什么”，3.2 回答“调查后真正选哪条”。前一级能覆盖当前范围时不无理由
降级；`normal_route` 采用较低级路线时，必须在证据或缺口中写明更高级路线不可用的原因。
收费 VIP 排在重开发之前，但购买、订阅和启用仍需用户明确授权。

第三级 Agent 路线真实跑通一次后，当前范围可以完成，不自动进入适配器开发。重复执行先
整理为 Skill 或薄调用；只有实际复用证明仍缺稳定批量、重试或恢复能力时，才记录开发缺口。

统一限制：任何工具只产生候选、临时导出件或读取结果。它不能写 Babata SQLite、分配
最终 ID 或把下载目录变成第二条权威持久化路径。Babata 核心确认选择后，才把工具结果
作为 C0 候选接入唯一收集链路。

## 4. 路线总表

这张表是 P2-G7 的机器可检验权威结构。`source_id` 稳定且唯一；允许以后追加来源，但
00 已点名的 19 个来源不得缺失。证据未到 E3 时，当前状态必须保持 `disabled`。

<!-- P2-G7-SOURCE-TABLE -->
| source_id | source | normal_route | minimum_authorization | current_evidence | current_gap | current_status |
| --- | --- | --- | --- | --- | --- | --- |
| source.feishu | 飞书文档、Wiki、知识库、云文档 | 官方 `lark-cli` 直接调用，Babata 只包授权、范围选择和结果接入 | 一次飞书应用配置与用户 OAuth；以后选择文档/节点/范围 | E3：10 个根候选和 6 个子候选中选 1 篇，正文/8 PNG、真实 failed 后定向 retry 和 `unchanged` 重采已验证 | 嵌入 Sheet/Base/Slides/画板内部数据及其他文档类型未覆盖 | disabled |
| source.yuque | 语雀 | Codex Chrome 发现范围，单篇用语雀官方 Markdown 导出端点；整库可用官方 PDF/LakeBook；`yuque-dl` 仅作受控批处理候选 | 登录语雀并选择知识库/文档；会员 API/MCP 暂不启用，不要求手抄会话 Token | E3：8 个真实候选选 1 篇，官方 Markdown、22 张图片、C0 和 `unchanged` 重采已验证 | 整库通用格式、文件/表格/画板/评论未覆盖；OpenAPI/MCP 需要超级会员，留待统一决策 | disabled |
| source.onenote | OneNote | 官方桌面客户端导出 PDF+MHT 配对或显式 MHT 列表；Rust 窄 adapter 校验每个实际导出，经唯一核心链路保存 C0，并只记录非事实重叠提示 | 客户端已登录；选择明确笔记本/子本范围并完成官方导出 | E3：用户实际交付的 1 对 626 页 PDF/MHT 与 6 个 MHT 全量进入 registered/C0-C，全部重采 unchanged，已知重叠对子无正式 relation | 没有原生 page/section ID；跨新导出来源身份未启用；可选 C1 分段不属于来源缺口 | available |
| source.evernote | 印象笔记 / Evernote | 官方客户端整库 `.notes` 导出；Babata Rust adapter 逐条认证解密为 ENEX/ENML；网页 DOM 和单篇 MHT 为回退 | 客户端已登录并选择一个明确导出范围；不需要用户密码、Cookie 或第三方账号授权 | E3：用户实际交付的 78,711,776 字节整库 `.notes` 中 163 条正文和 349 个资源全量验证，1 batch + 163 notes 全部进入 registered/C0-C 并 164/164 unchanged | `.notes` 没有 note GUID、updated 或笔记本层级；身份限于 immutable export hash + ordinal，跨新导出匹配未启用 | available |
| source.wechat_favorites | 微信收藏 | 官方手机记录迁移到 PC 后，用已验证的 WeChatDataAnalysis 本地恢复/导出取得收藏原库与可读分页；正式登记仍须走 Babata Rust Collector | 已登录官方 PC 微信；用户完成手机迁移并明确收藏范围 | Recovery E3：5,025 条全类型收藏随微信第一阶段全量达到 C0-A1；另有 7 条收藏代表样本进入 registered/C0-C 并重采 unchanged | 全量收藏附件与 URL 正文尚未达到 C0-A2；P8.2 再筛选和推进 | disabled |
| source.wechat_articles | 微信公众号文章 | 公开文章 URL 与已验证只读 Recovery 目录；Agent 保存直接可得目录/响应，后续再按需取正文/媒体 | 公开 URL 无额外授权；禁止默认操作微信 UI | E3：P8.1 已保存 233 条真实公众号文章目录达到 C0-A1；另有 1 篇正文/Markdown/HTML 进入 registered/C0-C 并重采 `unchanged` | 其余文章正文、图片/音视频与批量重采不属于 P8.1 | disabled |
| source.wechat_channels | 微信视频号 | non-plan；保留来源身份，不排入 P8.1/P8.2/P8.3 | 当前无需动作 | E1：候选工具和权限模型已核；2026-07-27 用户明确标记非计划 | 只有用户以后明确重新规划才继续；不安装代理证书或捕获工具 | disabled |
| source.wechat_chats | 微信聊天记录 | 官方手机记录迁移到 PC 后，用已验证的 WeChatDataAnalysis 本地恢复/导出会话、消息和可得媒体；正式登记仍须走 Babata Rust Collector | 同网手机确认迁移或 PC 已有记录；明确选择会话和范围 | Recovery E3：746/746 个会话、216,449 条消息随微信第一阶段全量达到 C0-A1；文件传输助手另有 5 条代表样本进入 registered/C0-C 并重采 unchanged | 单聊/私聊和群聊/其他的筛选、去噪与登记排在 P8.2 | disabled |
| source.zhihu | 知乎收藏与内容 | Codex Chrome 发现范围，OpenCLI 分页/详情/媒体；`Zhihu-Collections-MCP` 仅作后续批量候选 | Chrome 已登录并选择收藏范围；MCP 候选另需实测其登录方式 | E3：27 个候选选 1，正文/HTML/17 原图和 `unchanged` 重采已验证 | 文章、想法、视频、评论及 MCP 候选尚未实证 | disabled |
| source.bilibili | Bilibili 收藏与媒体 | Codex Chrome 先尝试候选；真实超时后用 OpenCLI + `yt-dlp`/ffmpeg 收所选一条 | Chrome 已登录；选择单条视频范围 | E3：20 个候选选 1，正文/字幕/摘要/视频和 `unchanged` 重采已验证 | 按用户要求只闭合一条，后续收藏范围另选 | disabled |
| source.xiaohongshu | 小红书收藏 | Codex Chrome 发现范围，OpenCLI 详情/媒体重采；`XHS-Downloader` 仅作后续批量候选 | Chrome 已登录并选择收藏范围；不要求手抄 Cookie | E3：20 个候选选 1，正文/2 媒体和 `unchanged` 重采已验证 | 其他内容形态未覆盖；专用下载器的浏览器 Cookie 读取已失效 | disabled |
| source.douyin | 抖音收藏 | non-plan；保留来源身份，不排入 P8.1/P8.2/P8.3 | 当前无需动作 | E0：错误主路线已撤回，候选路线已核；2026-07-27 用户明确标记非计划 | 只有用户以后明确重新规划才继续；`F2` 和真实样本均未实证 | disabled |
| source.browser_bookmarks | 浏览器书签 | 最后单独收集；届时由 Agent 按一次明确文件夹范围读取候选并自动遍历网址、收正文/可得附件；实验性窄扩展冻结 | 暂无；最终收集时给出文件夹或集合范围一次 | E1：扩展候选、loopback 和唯一 C0 writer 机制已验证；正式 Chrome 证明当前实现只会手动提交 locator | 延到所有点名来源之后；缺 Agent 自动遍历正文、附件、逐条状态和新鲜重采 | disabled |
| source.browser_pages | 浏览器当前页面、选区和网页收藏 | 当前存量由 Codex Chrome 自主读取历史/页面；未来快速剪藏入口仅作低优先级补充，保真页面再评估 SingleFile | 给出历史、页面或站点范围一次 | E3：P8.1 从 Chrome 返回的 50 条真实近期网页记录中直接保存 5 条 C0-A1；实验性扩展证据不计 | 正文、附件、保真页面和新鲜重采不属于 P8.1，后续按使用需要推进 | disabled |
| source.doubao | 豆包对话 | Codex Chrome 一次展开和选择真实历史；官方 `chain/single` 从 anchor 0 按 `next_index` 逐页读取；已登记范围逐 item 重采 | Chrome 已登录；给出会话、时间或数量范围 | E3：P8.2 第二阶段闭合 377 个非主范围；第三阶段再闭合主对话和其中 37 个长残余，38/38 到 `HasMore=false`；W1 的 7 个原始 DOCX 已进入统一 C0 | 8 个图片仅有全分辨率转码派生物、1 个 PDF 原件缺失；Recovery 不冒充正式 C0 | available |
| source.kimi | Kimi 对话 | 当前优先 Codex Chrome 调用 Kimi 结构化历史和会话接口；OpenCLI 薄命令只用于任务外重试/重采 | Chrome 已登录；给出会话、时间或数量范围 | E3：15 个真实候选选 1，结构化消息、逐条状态、C0 和 `unchanged` 重采已验证 | 当前样本无附件；全历史和深研产物未覆盖 | disabled |
| source.chatgpt | ChatGPT 对话 | 日常范围用 Codex Chrome；OpenCLI 薄命令固化已证明的结构化读取；账号级首次回收可用官方 Data Export | Chrome 已登录；全量时只在 Data Controls 确认 | E3：20 个真实候选选 1，2 条角色消息/10 引用、逐条状态、C0 和 `unchanged` 重采已验证 | 当前样本附件为 0，二进制附件和工作区全量资格未验证 | disabled |
| source.local_files | 本地文件 | Babata 核心文件选择器、拖放或受控目录扫描直接读取 | 选择文件、目录或明确监视范围 | E2：P3 显式 file/export 已通过唯一 C0 提交、资产哈希、回读和故障补偿 | 缺日常文件选择器/目录候选、逐条状态和重收集 | disabled |
| source.first_party | 第一方创作 | Babata 创作入口直接提交同一核心链路 | 明确执行新建、修订或批注 | E2：P3 create/revise/annotate、版本关系、回读和故障补偿已验证 | 缺日常创作入口；不以来源收集状态机替代第一方版本语义 | disabled |

### 4.1 用户到底最少要给什么

下面是面向实际执行的清单。用户只给一次来源/范围，并完成平台无法替代的登录、扫码、
OAuth 或首次浏览器权限确认；命令、浏览器导航、候选发现、分页、下载、重试、格式接入、
hash、状态和 staging 管理由 Agent/Babata 自主完成，直到范围结束或出现真实阻塞。用户
不需要把密码、Cookie、token、导出路径或元数据粘贴到聊天框，也不需要逐条替 Agent 点。

| 来源 | 用户一次性动作 | 每次只需选择 | Agent/Babata 负责 | 当前还差什么 |
| --- | --- | --- | --- | --- |
| 飞书 | 已完成官方应用配置和用户 OAuth；过期时重新确认 | 文档、搜索结果、Wiki 节点或明确范围 | 列候选、分页、正文、附件、版本、重收集和状态 | 一次真实正文+附件 E3 样本 |
| 语雀 | 优先在已登录 Chrome 安装语雀批量扩展；CLI 路线才授权本机会话 | 知识库、文档或全账号 bootstrap | 目录、图片、附件、断点续传、增量和 staging 接入 | 扩展真实样本；禁止让用户手抄 Cookie |
| OneNote | 官方桌面客户端已登录 | 一个明确笔记本或子本范围 | 取得同次 PDF/MHT，或接收用户明确列出的 MHT 导出；Babata 校验结构和 manifest/hash，分别保存实际原件，并仅把确定性重叠作为非事实证据 | 已实跑一对 626 页 PDF/MHT 和 6 个显式 MHT；总 Skill recipe 已建立，来源侧还缺跨导出身份确认和真实受控 Agent 执行；可选 C1 切分/去重独立处理 |
| Evernote | 官方客户端已登录 | 一个明确 `.notes` 导出文件 | Rust adapter 验证原件 hash，生成解密 ENEX，列出 batch/note 候选并经核心提交 C0 | 已实跑 163 notes/349 resources；跨导出匹配未覆盖 |
| 微信收藏 | PC 微信登录，并在需要补历史时完成一次手机迁移 | 收藏集合、分类或时间范围 | Agent 用已验证本地工具导出原库、可读记录与可得资源，校验 hash 后再交统一 Collector | 第一阶段 5,025 条已进 Recovery；还未走 Rust C0/重采及附件逐项对账 |
| 公众号文章 | 单篇无授权；批量历史时扫码登录自己的公众号后台 | 链接、公众号、合集或文章范围 | 已知 URL 的正文、Markdown/HTML、可得媒体和重收集 | 单篇已闭合；还缺带媒体样本、批量历史和更多形态 |
| 微信视频号 | 当前无需动作 | 暂无 | 保留 UI-only 边界，不安装代理证书或捕获工具 | 用户已决定暂时不处理 |
| 微信聊天 | 用微信官方功能把所选手机记录迁移到电脑 | 会话和日期范围 | Agent 用已验证本地工具导出消息、结构和可得媒体，记录缺口并交统一 Collector | 第一阶段文件传输助手已进 Recovery；P8 再处理单聊/私聊和群聊/其他 |
| 知乎 | Chrome 已登录；首次批准当前实例 remote debugging | 收藏夹、条目或时间范围 | 自主列收藏夹、分页、详情、图片和页面快照；必要时调用 OpenCLI | Browser Use/Agent Browser 真实探针 |
| Bilibili | Chrome 已登录；首次批准 remote debugging | 收藏夹、页、视频或分 P | 自主候选、翻页、元数据、字幕、媒体和附件；按需调用 `yt-dlp` | 通用浏览器探针；`yt-dlp`/ffmpeg 已就绪 |
| 小红书 | Chrome 已登录；首次批准 remote debugging | 收藏列表、时间或数量范围 | 自主列收藏、正文、评论、图片/视频和重收集；必要时调用 OpenCLI/MCP | 通用浏览器只读低频探针 |
| 抖音 | 当前无需动作 | 暂无 | 保留历史路线，不继续安装或探针 | 用户已决定暂时不处理 |
| 浏览器书签 | 当前无需动作 | 最终单独收集时给出书签文件夹或集合 | Agent 读取层级并自动遍历网址、正文和附件 | 排到最后；当前不继续扩展实现 |
| 当前页/选区 | 首次批准 Chrome remote debugging；长期入口再按需安装 Babata 窄扩展 | 页面、站点或链接范围 | 自主导航/读取；SingleFile 保真 HTML、元数据和缺失报告 | 通用浏览器探针；后续 P4 扩展/SingleFile 接入 |
| 豆包 | Chrome 已登录；首次批准 remote debugging | 会话、时间或数量范围 | 自主遍历历史、读取消息/附件；OpenCLI 补会议 transcript | 通用浏览器与附件覆盖探针 |
| Kimi | Chrome 已登录；首次批准读取当前实例 | 会话、时间或数量范围 | 自主列历史、读取长对话和附件；长期工具只在需要时由 Skill 触发 | Codex Chrome 已完成真实历史分页和长正文；全历史、附件/深研产物、状态和重收集留给 P7 |
| ChatGPT | Chrome 已登录；首次批准 remote debugging；全量时在 Data Controls 确认 | 单会话、时间范围或明确全账号 | 自主遍历选择性范围；全量解析官方导出 JSON/资产 | 通用浏览器探针；工作区资格按账号验证 |
| 本地文件 | 选择文件、目录或监视范围 | 同左 | 列候选；默认原生逐文件复制并记录前后快照，强 SHA-256 显式后置 | P7 已用 247 文件/14.7 GB 真实批次证明 |
| 第一方创作 | 明确点击新建、修订或批注 | 本次草稿/版本/批注 | 同一核心链路新增资料、版本或关系 | 后续创作入口，无第三方授权 |

### 4.2 当前机器已经替用户准备好的工具

截至 2026-07-18，本机已完成以下准备：

`tool_id` 是回归检查使用的稳定标识；工具证据只描述已经实际核验的状态，不替代具体
来源的 E3。

<!-- P2-G7-TOOL-TABLE -->
| tool_id | tool | current_evidence | next_user_action |
| --- | --- | --- | --- |
| tool.lark_cli | `lark-cli 1.0.68` | E2：已安装，user/bot verified，真实 Wiki/Docs 只读调用成功 | 暂无；真实收集时选择范围 |
| tool.agent_browser | `agent-browser 0.32.1` | E1：doctor 7 pass；只读策略连接正式版 Chrome，列 27 个页面并读取 snapshot | 长期独立收集时由 Skill 调用；不阻塞当前 Codex 回收 |
| tool.browser_use | `browser-use 0.13.6` / Browser Harness 0.1.6 | E1：Chrome、daemon 和本地连接 doctor 通过，列 28 个 tab 并读取 page info | 长期独立收集时由 Skill 调用；云认证不是本地前置 |
| tool.codex_chrome | Codex Chrome | E2：复用正式版登录态读取 Kimi 历史分页和两条真实会话正文 | 暂无；用户给出来源和范围时继续当前存量回收 |
| tool.playwright_cli | `@playwright/cli 0.1.17` | E1：已全局安装并实际运行 version、help、attach 和 list | 只作比较与回退；真实 attach 前同样需批准浏览器控制 |
| tool.opencli | `opencli 1.8.6` | E1：已全局安装，daemon `127.0.0.1:19825` 正常 | 只在通用浏览器层需稳定站点命令时再决定是否启用 Bridge |
| tool.opencli_bridge | OpenCLI Browser Bridge 1.0.22 | E0：官方 ZIP 已下载、解压并按 release SHA-256 校验，尚未安装或调用 | 暂不安装；其高权限扩展不再是下一步前置 |
| tool.graph_powershell | Microsoft Graph PowerShell 2.38.1 | E1：`Authentication` 和 `Notes` 已安装；当前无登录 context | 无；OneNote 改走官方整本 PDF/MHT，不再作为待办 |
| tool.evernote_backup | `evernote-backup 1.13.1` | E1：已通过 `uv tool` 安装并核 help | 无；印象笔记改走官方 HTML，不再作为待办 |
| tool.yuque_dl | `yuque-dl 1.0.85` | E1：已全局安装并核 command 和 options | 优先走扩展；CLI 批处理时授权本机会话 |
| tool.yt_dlp | `yt-dlp 2026.07.04` 加 ffmpeg 8.1.1 | E1：已安装，Bilibili 媒体工具链就绪 | 真实收集时选择视频范围 |
| tool.f2 | `F2` 抖音候选 | E0：仓库、权限模型和收藏命令已核；本机隔离安装探针未完成 | 无；抖音已延期，用户重新启用前不继续 |

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

2026-07-18 在用户已登录的正式 Chrome 中直接核验官方能力：

- 知识库“更多设置 -> 导出”免费可见，整库只提供 PDF 和语雀私有 `.lakebook`；
- 单篇阅读页免费提供“复制为 Markdown”，同一登录态可访问文档的官方
  `/markdown?plain=true&linebreak=true&anchor=true` 端点；
- 所选私密文档通过该端点一次取得完整 Markdown 和 22 个媒体 URL，不需要逐图滚动；
- 官方 OpenAPI 的个人 Token 明示为“超级会员专享”，官方
  [yuque/yuque-mcp-server](https://github.com/yuque/yuque-mcp-server) 只是 19 个 API 工具的
  MCP 封装，同样必须提供该 Token。会员能力只登记，等全部来源闭环后统一决策。

批量扩展 [ouyangfeng2022/yuque-exporter](https://github.com/ouyangfeng2022/yuque-exporter)
仍是候选：Manifest V3、MIT、2026-06-24 更新，README 声称可借当前登录态批量调用官方
Markdown/Lake/Word/PDF/JPG 导出并保留 TOC。但该项目当前规模小，且代码通过
`document.cookie` 传认证并在扩展 worker 设置 `cookie` 请求头，尚无本机真实调用证据，
不能再把 README 声明写成已证明的日常正常路线。

Agent 批处理路线使用 [gxr404/yuque-dl](https://github.com/gxr404/yuque-dl)：npm
1.0.85，2026-06-27 更新，支持单文档、多知识库、当前账号全部知识库、图片、附件、
断点续传和 `--incremental`。它需要 `_yuque_session` 或企业实例的 cookie key/value，
因此只能在用户明确允许批处理并把会话秘密放入受控 secret store 时使用，不能要求用户
每次打开 DevTools 手抄 token。

补充候选：桌面工具 `ydhawesome/yuque-exporter` 能导出小记和知识库，但要求在第三方
程序中输入账号密码，不作为首选。

限制：评论、协作修订历史、部分表格图表/画板和“收藏但无导出权”的覆盖仍需真实样本
验证。官方导出件应与页面快照/媒体附件分别记录，不能把格式转换结果冒充唯一原件。

决策：**Codex Chrome 发现和选择 + 语雀官方 Markdown 端点 + 薄重采命令**。官方整库
PDF/LakeBook 用作恢复或原生备份；会员 OpenAPI/MCP、未实证批量扩展和要求会话 Token 的
`yuque-dl` 都登记为后续批量候选，不阻塞当前免费小范围闭环。

本机已全局安装 `yuque-dl 1.0.85` 并核对 `doc/batch/user`、图片、附件、TOC 和
`--incremental` 参数。它现在只缺授权；正常产品路线不能因为 CLI 已安装就要求用户手抄
`_yuque_session`。

### 6.3 OneNote

用户已确认官方桌面客户端可以整本导出 PDF 和 MHT。按八级路线，这属于第一级官方免费
批量导出，不需要再用 Graph 或第三方 CLI 绕路；Babata 只增加了真实缺口所需的窄配对
adapter，用来结构校验、建立确定性 manifest 并进入唯一 C0 链路。

正常执行是：用户只选择一次笔记本范围，Agent 跟随官方桌面 UI 导出整本 PDF 和 MHT，
保留笔记本名称、导出时间、客户端版本、文件大小和 SHA-256，再通过 Babata 唯一 C0
路径登记。同一次导出的 PDF 与 MHT 属于同一来源、只形成一个 archive item；PDF 更接近
OneNote 的渲染和划分，MHT 更适合保留图片、文字和格式。二者都作为独立官方 export
保存并标明 representation role，不能互相替代，也不能把 PDF 页码冒充平台 page/section ID。

Graph PowerShell 2.38.1 和 `onenote-md-exporter` 的历史调研记录不再作为当前待办。只有
将来真实重复执行证明官方整本导出缺少必要的批量、重试或恢复能力，且用户重新要求时，
才重新评估 Skill 或薄调用。

2026-07-19 已对一个真实整本 MHT 做只读局部解析：文件为 17,161,246 字节，SHA-256
`3873ECBB2B099E380A0A06465BA020A2838F258583237CD731081E5B61CF2A97`；标准
`multipart/related` 中包含 1 个 1,124,064 字节 HTML、12 PNG、18 JPEG 和 1 个 XML
文件清单，正文可直接提取。导出把整本内容串在单一 HTML 中，没有明确 title/page ID/
section ID 边界，因此原 MHT 可作为整本 C0 导出件，逐页切分属于后续 C1 清洗，不能在
收集阶段猜测。

同次配对 PDF 为 11,189,470 字节，SHA-256
`FB00DB70FBBEC77EB28469886DF4190CC3B7DE529AC33BEFA703D3397FBB1DD5`；由 Microsoft
OneNote 2021 生成，未加密，共 626 张 A4 页面。首、中、末页渲染抽样均可读；归一化文本
对比得到约 83.53% 的 MHT 8-gram 覆盖率和约 70.01% Jaccard，证明二者内容高度相关但
用途不同。

2026-07-25，Issue #84 将同一对真实导出归入 `BABATA_RECOVERY_HOME`。Rust
`OneNoteExportAdapter` 的配对路线只接受 `pair:<absolute-mht>|<absolute-pdf>`，拒绝相对路径、
跨目录/异名配对、缺失或重复 HTML root、MIME 路径穿越、畸形/加密 PDF，以及 discovery
后发生变化的文件。它发现一个 archive 候选，manifest 同时绑定 pair/MHT/PDF/HTML hash、
MIME 结构、PDF 页数/版式/生成器和互补角色；没有 adapter cache，也不推断页面身份。

隔离库与活动库均保存 1 item、1 ready revision 和 2 个 ready export，随后同一 session 重采
为 unchanged、0 新 revision。OneNote 自身有 1 source、2 observations；活动库最终为
`7 sources / 193 items / 196 revisions / 360 assets / 1 relation`，schema v7、
`quick_check=ok`，外键异常和所有 pending/quarantine/journal/orphan 为 0。证据位于
`BABATA_EVIDENCE_HOME/runs/p7-2-onenote-20260725-071439/`，不进入 Git。

决策：**官方桌面整本 PDF + MHT 是同一来源的互补表示；停止继续调研 Graph/第三方 CLI**。
基于上述 E3，`source.onenote` 现为 `enabled`。Issue #90 已把它接入唯一总收集 Skill 的
内部 recipe；来源侧仍缺跨导出匹配和真实受控 Agent 执行。可选 C1 逐页切分是收集结束后
独立消费 C0 的通用处理，不是来源缺口或启用条件。
该切片不代表 P7、AC-09 或 TC-09 整体通过。

2026-07-25，Issue #86 处理用户另行导出的六个真实 MHT，其中有的可能是子本。用户同时
明确 OneNote 会把许多独立内容段放在一起。`OneNoteExportAdapter` 新增
`mht-list:<absolute-mht>|...`，只接受用户显式列出的绝对路径、同一目录、互不重复的常规
MHT 文件，不扫描目录，也不要求为这些导出补造 PDF。它同时接受
`multipart/related` 与单体 `text/html`，使用 HTML5 parser 验证 `OneNote.File` 和 Microsoft
OneNote generator，记录根 HTML、可见文本、MIME part、原件与批次 hash。

六个导出各自形成独立 archive C0、1 ready revision 和 1 ready export。批次内稳定 12 字符
n-gram 比较只在“灵感消化”与“猫与月季花”之间产生两条双向提示：较小导出的覆盖率为
100%，较大导出的覆盖率约 95.83%。提示明确保持 `human_judgment=false`、
`confirmed_fact=false`，没有写 OneNote page/section ID、父子关系或正式 C0 relation。
这证明 C0 能保留可复算的初步重叠证据，但不能证明哪一个是子本。若以后确有使用需要，
段落切分、语义去重和层级判断由通用 C1 独立读取这些 C0；它不是 OneNote 收集的后续必经
步骤，也不影响本次 C0 完成。

隔离库与活动库均保存 6/6，随后同 session 重采 6/6 unchanged、0 新 revision。六份 C0
资产与 Recovery 原件逐字节 hash 一致；活动库最终为 `7 sources / 199 items / 202 revisions /
366 assets / 1 relation / 342 observations`，schema v7、`quick_check=ok`，外键异常和所有
pending/quarantine/journal/orphan 为 0。旧配对 manifest 继续使用
`onenote-paired-export/1`，新列表 manifest 独立使用 `onenote-mht-export/1`，避免适配器升级
让既有配对产生伪 revision。证据位于
`BABATA_EVIDENCE_HOME/runs/p7-3-onenote-mht-20260725-090516/`。Issue #90 后总 Skill recipe
已完成，真实受控 Agent 执行仍未完成；P7、AC-09、TC-09 不提前标记通过。可选 C1 分段/层级另行立项，不计作
`source.onenote` 的未完成收集范围。

### 6.4 Evernote / 印象笔记

官方客户端可以一次导出完整 `.notes` XML 合集。按八级路线，这是第一级官方免费批量
导出，优先于逐篇 HTML/MHT、网页遍历、账号同步 API 和专用适配器。用户只选择一次
笔记本或账号范围；Agent 保留原 `.notes`、客户端版本、导出时间、文件大小和 SHA-256，
再把解密后的 ENEX 作为可追溯派生物交给 Babata 唯一 C0 路径。原导出件不能被解密结果
覆盖。

2026-07-19 的真实 `.notes` 文件为 78,711,776 字节，SHA-256
`625FCB7533EAFE00DE490F2F61E4477D513099EC5AAE0A97A6D3B1644D6DC9A5`；根结构兼容
Evernote ENEX，共 163 条 note、349 个 resource。163 条 content 全部标记
`base64:aes`，Base64 解码后以 `ENC0` 开头；标题、创建时间、资源 MIME、文件名和尺寸
本来就可读。

该加密不是用户密码保护。快速 GitHub 代码核验找到了三套相互一致的公开实现：

- [HNIdesu/YinxiangbijiDecrypt-Go](https://github.com/HNIdesu/YinxiangbijiDecrypt-Go)；
- [HNIdesu/YinxiangbijiDecrypt-Cpp](https://github.com/HNIdesu/YinxiangbijiDecrypt-Cpp)；
- [HNIdesu/YinxiangbijiConverter](https://github.com/HNIdesu/YinxiangbijiConverter)。

它们从每条 `ENC0` 正文中的两个 nonce 派生 AES/HMAC 密钥，先用 HMAC-SHA256 验证包体，
再以 AES-128-CBC 解密正文；使用的是应用固定种子，不要求用户提供密码。对真实导出件的
第一条 `Chocolate Arrest(1)` 做了独立只读复现：`ENC0` 签名成立、HMAC 校验通过，成功
得到 381 字节 ENML/XML 正文。该验证没有改写原文件，也没有生成全量输出。

备选路线也已实测，但不再作为主线：

- 正式 Chrome 可直接从当前笔记 DOM 读取标题、笔记本、稳定 GUID、正文 HTML、原图和
  原附件资源地址；样本 `Monster Fight For They were UnComfort` 得到 1,257 字符 HTML、
  一张 800x450 图片和一个页面标称 1.9 MB 的 `.spd` 附件；
- 官方客户端可逐篇导出 MHT；样本 MHT 为 239,809 字节，内含一张 450x800 JPEG、一个
  300x60 PNG 预览图和一个 63,126 字节 `.spd` 原附件。该路线 UI 步骤较多，只作回退。

决策：**官方整库 `.notes` + 已验证固定算法解密为正常路线；网页 DOM 为补充，客户端
单篇 MHT 为回退**。

2026-07-24，Issue #82 将同一真实导出归入 `BABATA_RECOVERY_HOME`，再次校验大小与 SHA-256
不变。Rust `EvernoteNotesAdapter` 只接受 `notes:<absolute-file>` 的单文件明确范围；拒绝相对
路径、目录、错误扩展、XML entity declaration、畸形资源、截断包和 HMAC 篡改。它以应用
固定种子经 50,000 轮 HMAC-SHA256 分别派生认证/AES key，先做常量时间 HMAC 校验，再用
AES-128-CBC 解密；163/163 正文全部通过，349 个资源均完成 base64、字节数、MIME、文件名
和 SHA-256 映射。

同一导出发现 1 个 batch export 和 163 个 note 候选；批次 C0 保存原 `.notes` 与全量解密
ENEX，note C0 保存 ENML，明确请求附件后保存 349 条 attachment 记录。活动库 164/164
`saved`、0 failed/skipped；随后 session 级重采为 164/164 `unchanged`、0 新 revision。最终
Evernote 为 `1 source / 164 items / 164 revisions / 351 assets / 328 observations`，资产角色
为 `349 attachment + 1 original + 1 export`；全库 `quick_check=ok`、外键异常和所有
pending/quarantine/journal/orphan 均为 0。证据位于
`BABATA_EVIDENCE_HOME/runs/p7-1-evernote-20260724-224315/`，不进入 Git。

真实 `.notes` 没有 note GUID 或 updated，163 条 created 也只有 63 个不同值，因此不能
伪造平台级稳定 ID。当前身份诚实限定为 immutable export SHA-256 + note ordinal；同一原件
可稳定重采，跨新导出自动对齐仍未启用。基于上述 E3，`source.evernote` 现为 `enabled`；
这只代表该明确整库导出路径可用；Issue #90 后它已进入总 Skill recipe，但不代表 P7、
AC-09 或 TC-09 已整体完成。

## 7. 微信来源

### 7.1 微信收藏

2026-07-26 的真实恢复结果覆盖此前的 UI-only 决定。手机记录仍先通过微信官方功能迁移到
PC；随后 WeChatDataAnalysis 1.18.5 已在当前 Weixin 4.1.11.55 上稳定读取本地账户数据，
不需要逐条 UI 操作，也不安装代理证书。它在本机生成解密后的数据库、可读 API/导出和
资源归档；这些产物只属于 Recovery/acquisition，不绕过 Babata Rust Collector，也不自动
启用 `source.wechat_favorites`。

2026-07-19 使用 `gh` 重新核验后，当前最可信专用候选是
[r266-tech/wechat-cli](https://github.com/r266-tech/wechat-cli)：MIT、未归档，最新 release
为 `v1.6.20`（2026-07-15），发布包包含 Windows amd64。它声明可读取会话、消息、媒体、
收藏和搜索，但 Windows 路线需要扫描当前登录的 `Weixin.exe` 进程内存并解密本地数据库。

该仓库仍开放的 [Issue #8](https://github.com/r266-tech/wechat-cli/issues/8) 已由维护者确认：
Windows 新版 Weixin `4.1.10.52` 的 session/message 新 salt 无法取得 raw key，导致
`sessions/timeline/context` 不可用；这不是用户配置或登录时序问题。`v1.6.20` 发布说明
没有声称修复，Issue 仍开放。本机版本更高，为 `4.1.11.55`，因此没有理由在用户未批准
高风险权限的前提下安装或尝试 `init`；更不能把较低版本已确认的缺口当作高版本兼容证据。

官方替代也已复核。腾讯发布的 npm 包 `@tencent-weixin/openclaw-weixin 2.4.6` 是
ClawBot/iLink 的 OpenClaw 微信渠道，官方 README 公开的接口只有扫码授权、`getUpdates`
长轮询新消息游标、回复、输入状态和媒体上传下载；没有历史收藏、历史聊天或账号存量读取
接口。因此它适合用户今后主动发给 Bot 的新资料，不替代 Babata 当前的存量回收。

旧 `jackwener/wx-cli` GitHub API 当前返回 HTTP 451/DMCA；此前候选 WeFlow、WeChatMsg、
wechatDataBackup 等也已清空或不再提供可审计的可用代码，不能继续写成正常路线。此前调查
同样没有找到一条在 Windows 上同时满足“仍维护、完整覆盖收藏类型、Agent 可调用、无需
解密私有数据库”的成熟工具。Mac 的
[zhuyansen/wx-favorites-report](https://github.com/zhuyansen/wx-favorites-report) 活跃但不
适用当前 Windows 工作区；Windows 搜到的工具多只处理收藏中的聊天记录、公众号链接或
表情，不能冒充完整收藏路线。

UI Automation 仍可用于小范围发现或公共文章链接回退，但不再是全量收藏的唯一执行路线。
正常全量路线是：官方手机迁移 -> WeChatDataAnalysis 本地恢复/导出 -> Recovery 校验 ->
以后由统一 Rust Collector 正式登记。用户给一次收藏分类、时间或数量范围后，Agent 可从
恢复工具的本地 API/导出继续：

- 公众号链接/普通网页 -> UI 打开或复制公开 URL，Agent 在微信之外保存公共 HTML/媒体；
- 图片/文件 -> 保留数据库中的原始元数据和本地可得资源，缺失原件如实记录；
- 收藏中的聊天记录/笔记 -> 保留可读结构和附件声明；
- 视频号 -> 暂时不处理；若重新启用也只使用官方 UI，不能另存时标记受限。

不安装代理证书，不把 Recovery 工具写成 Babata 的正式 C0 writer。重收集优先保留
`localId`/`serverId`、类型、时间、来源 URL、内容和原件 hash；缺少可靠 native ID 时才
退回组合身份并诚实标注可能重复。

2026-07-19 已在官方 PC 微信 `4.1.11.55` 中真实执行这条路线：进入“全部收藏”，读取 8 个
最新可见候选，选择 1 篇公众号文章并从微信文章窗口复制官方原链接；没有扫描内存、解密
数据库或安装代理。该文章正文、Markdown、原始 HTML、C0 和 `unchanged` 重采已闭合。

2026-07-26 第一阶段已从最新账户整合归档取得 5,025 条收藏，覆盖链接 4,033、视频号 651、
视频 113、图片 90、文本 67、商品 24、文件 21、聊天记录 16、语音 3、笔记 2 和其他 5；
原始 `favorite.db` 为 20,815,872 字节，`quick_check=ok`，26 页可读 JSON 对账为
5,025/5,025 唯一 ID。审计位于
`${BABATA_RECOVERY_HOME}/batches/wechat/20260726-stage1-filehelper-favorites/stage1-audit.json`。
未运行 Rust C0 或 C1。

决策：**官方迁移 + 已验证 WeChatDataAnalysis 本地恢复为全量主路线，官方 PC UI 为小范围
发现/回退；Recovery 不冒充 C0，正式 capability 在 Rust 接入和重采完成前保持 disabled**。

### 7.2 微信公众号文章

公众号文章从官方 PC 微信 UI 打开或复制公开 URL。Agent 可以对 UI 已暴露的公共 URL
保存 HTML、Markdown 和可得媒体，但这只是公共网页取得，不提供微信收藏/历史发现能力，
也不单独提供收藏/历史发现能力。批量公众号文章如果已在收藏或聊天恢复数据中出现，可由
恢复路线先枚举；已知公开 URL 的正文保真仍按普通网页取得，无法取得的部分明确受限。

历史样本曾用 OpenCLI 对 UI 复制出的已知 URL 下载 Markdown，并完成重采；该事实继续
保留，但 OpenCLI 不再作为未来微信路线。

决策：**官方 PC 微信 UI 发现/选择 + 公共 URL 普通网页保存**。

### 7.3 微信视频号

2026-07-19 用户明确决定视频号暂时不处理。以下路线和风险记录只作为未来恢复上下文；
P4 收尾、P5 和 P6 均不安装高风险工具、不做真实样本，也不把延期写成已支持。只有用户
重新启用时才在 P7 或后续阶段继续。

此前调研过 `wechat-channels` 和 `res-downloader`，但前者不具备个人收藏发现/下载，后者
需要本地代理证书。按最新决定，两者都不进入后续执行路线。若用户重新启用视频号，也只
由 Agent 操作官方 PC 微信 UI；UI 无法另存的媒体明确标记受限。

决策：**暂时延期并保持 disabled；重新启用后仍为官方 PC 微信 UI-only**。

### 7.4 微信聊天记录

先用微信官方路径把手机记录带到电脑。官方说明：
[如何透过电脑备份/还原 WeChat 聊天记录？](https://cs.help.wechat.com/hc/zh-cn/articles/11917889397775-%E5%A6%82%E4%BD%95%E9%80%8F%E8%BF%87%E7%94%B5%E8%84%91%E5%A4%87%E4%BB%BD-%E8%BF%98%E5%8E%9F-WeChat-%E8%81%8A%E5%A4%A9%E8%AE%B0%E5%BD%95)。
用户在同一 Wi-Fi 上从手机确认全部或所选会话，官方备份只供微信恢复，不是 Babata
可读格式。

历史上复核过的 CLI 和官方 Bot 通道或不兼容本机版本，或不能读取既有历史；但
WeChatDataAnalysis 1.18.5 已成为本机当前版本的实证例外，不再沿用“所有本地恢复路线均
不可用”的结论。

最少用户动作：先让官方 PC 微信拥有目标记录，再明确选择会话/日期范围。Agent 通过已验证
本地工具导出消息、结构和可得媒体，逐项记录缺失；必要时用官方 UI 做小范围回退。

2026-07-26 的账户恢复已读取 746/746 个会话和 216,449 条消息。第一阶段文件传输助手独立
包包含 1,842 条消息、247 个可得媒体；72 个缺失引用（71 个唯一）集中在文件 61、视频 6、
表情 5，图片和语音缺失为 0，最新账户整合归档也没有这些原件。消息库、资源库与用户给出的
最新整合归档逐库 hash 一致。该包和审计位于
`${BABATA_RECOVERY_HOME}/batches/wechat/20260726-stage1-filehelper-favorites/`，未运行 Rust C0
或 C1。单聊/私聊和群聊/其他的筛选、去噪、对账与正式登记排在 P8。

决策：**官方迁移 + 已验证 WeChatDataAnalysis 本地恢复为批量主路线，官方 PC 微信 UI 为
回退；不安装代理证书，Recovery 不冒充 C0，正式 capability 在 Rust 接入和重采完成前保持
disabled**。

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

补充候选 [JasonJarvan/Zhihu-Collections-MCP](https://github.com/JasonJarvan/Zhihu-Collections-MCP)
在 2026-04 仍有提交，提供私密/公开收藏夹批量 Markdown、图片和 MCP Server。它尚未
实际调用，先登记；当前 27 候选选 1、17 原图和重采闭环保持不动。

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

补充候选 [JoeanAmier/XHS-Downloader](https://github.com/JoeanAmier/XHS-Downloader)
在 2026-07-17 仍有提交，提供收藏/点赞链接提取用户脚本，以及 CLI、API、MCP 和媒体下载。
但其 README 明确标注“从浏览器读取 Cookie”已失效，受登录保护能力会回退到手工 Cookie，
自动滚动也提示有风控风险；因此只登记为后续批量媒体候选，不替换已完成的正式 Chrome +
OpenCLI 小范围闭环。

最少用户动作：Chrome 已登录小红书并一次批准 remote debugging，或 MCP 首次扫码；
用户给出收藏、时间或数量范围一次，Agent 自主读取详情和媒体。

限制：`xsec_token` 可能失效；无官方开放 API，页面/接口变化和风控风险高；只读低频，
不自动点赞、评论、关注或发布。

决策：**Browser Use/Agent Browser 主导航，OpenCLI/MCP 作为稳定命令和登录备选**。

### 8.4 抖音

2026-07-19 用户明确决定抖音暂时不处理。以下候选和风险只保留为未来恢复上下文；P4
收尾、P5 和 P6 不再继续安装、探针或真实样本，只有用户重新启用时才继续。

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

决策：**暂时延期并保持 disabled；旧主路线维持撤回，用户重新启用后再从 `F2` 候选复核**。

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

真实补充证据（2026-07-25，Issue #88）：Agent 此前已从“战略领导力W1”消息内嵌文件对象
取得 7 个原始 DOCX，并按消息 MD5、Recovery manifest SHA-256、字节数和 DOCX ZIP 结构
校验。此前只有“设立目标”DOCX 和一个平台 PDF preview 正式登记；本次通过 Babata 通用
附件操作把剩余 6 个 DOCX 附加到既有 ready revision。活动库没有新增 item、正文 revision、
relation、observation 或 C1，W1 最终为 7 个 `original` DOCX 加 1 个 `preview` PDF。该结果
证明这个明确收集范围已经完整进入统一 C0，不建立或触发豆包专属 C1；它不证明其他附件
形态、可执行来源 recipe、受控 Agent 或长期重采可用；Issue #90 的总 Skill 当时对该状态
fail closed，因此在 Issue #92 批量实证之前 `source.doubao` 保持 disabled。

2026-07-26 Issue #98 又把 W1 原件取得的实际主链复现清楚：OpenCLI `detail-full` 返回的
`AttachmentKeyCount=0` 不代表没有附件，7 个 DOCX 位于 `content_type=20` 消息字符串中的
`entity_content.file`。把这 7 个原始 `key` 一次传给登录态页面的
`/alice/message/get_file_url`，并明确 `type=file`，直接返回 7 条 DOCX 签名地址；无需进入
云盘。7 个文件共 111,296,956 字节，大小、MD5、历史 SHA-256 和 DOCX ZIP 结构全部一致。
点击消息卡片的“查看”会把转换后的 PDF URI 交给同一接口，只能得到 preview，不是原件。
因此主链固定为消息 JSON -> 原始 key -> 批量换签 -> 下载校验；云盘文件行“下载”只作为
直取失败后的独立备选，不与主链混跑。

真实批量证据（2026-07-25，Issue #92）：Agent 在已登录 Chrome 中一次展开约 60 个去重
会话，按用户指示排除超大的“主对话”，再把前 40 个非主会话拆成两个显式 20 条批次。
首批 18 saved/2 failed，次批 20 saved/0 failed；两个失败均因平台 `HasMore=true`，adapter
在残缺消息进入 C0 前拒绝。38 个保存项逐 item 重采全部 `unchanged`、0 新 revision；C1
完全不变。活动库新增 34 items 和 38 revisions，其中 4 条为已有 v1 item 首次进入去除临时
block/signature URL 的 v2 稳定指纹时产生的一次性归一化 revision，稳定消息内容没有变化，
后续重采不再增长。以后先按 `source_native_id` 与现有 C0 去重；指纹升级必须走显式兼容/
迁移决策，不能静默冒充来源变化。

两批收集耗时约 408.53 秒，逐 item 重采约 458.27 秒。实测瓶颈不是 Rust C0，而是 OpenCLI
为每条会话重新导航并建立网络捕获。按“稳定、准确、真实、速度”的顺序，前三项已通过，
下一项只优化同一浏览器/捕获生命周期复用；候选完整分页、独立 C0 事务和单 item 故障隔离
保持不变，不继续并列探索多条未证明路线。

真实全量续页证据（2026-07-28，Issue #118）：第三阶段在登录 Chrome 中直接复用官方
`/im/chain/single` 协议，从 `anchor_index=0` 开始，把每次响应的数字型 `next_index` 作为
下一页 anchor；服务端单页实际最多返回 50 条。主对话和 37 个长会话最终 38/38 均闭合于
`HasMore=false`，共 156 页、5,887 条跨会话唯一消息。逐页原始响应、SHA-256 与游标已保存，
不再依赖不稳定的程序化滚动。该实证只完成 Recovery 范围，不把临时采集器升级成持久 adapter，
也不触发 C0/C1。

决策：**总 Skill 内的豆包 recipe 继续由 Chrome 一次发现范围；完整回收用官方游标分页，
已登记范围仍由 Babata 逐候选写 C0、逐 item 重采。OpenCLI 薄命令保留作有界读取备选，
不把第三阶段临时采集器升级为新 adapter；附件按真实形态窄补并保留缺失清单**。

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
| `r266-tech/wechat-cli` Windows 本地解密 | 当前降级；v1.6.20 最新，但 Weixin 4.1.10.52 raw-key 缺口仍开放，本机 4.1.11.55 不试探 |
| 腾讯 ClawBot/iLink 作为存量导出 | 不适用；官方包只收授权后的 Bot 新消息，不读取历史收藏或聊天 |
| PyWxDump | 不作主路线；仓库当前“删库”、无明确许可证、长期未提交 |
| BBDown | 降级；仓库已归档，可作用户自选旧工具，不作长期依赖 |
| bilibili-api-python | 降级；仓库已归档 |
| DouK-Downloader 作为抖音正常路线 | 降级；签名算法失效，扫码失效、浏览器 Cookie 读取弃用，现状要求手抄 Cookie/自备参数生成器 |
| `better-douyin` 公开源码 | 淘汰；仓库明确不包含真实平台连接器、签名、Cookie 或下载解析 |
| OneNote 官方 PDF/MHT 与显式 MHT 导出 | 正常路线；Issue #84 已完成同源互补配对，Issue #86 已完成 6 个 MHT-only 原件及非事实重叠提示的真实 C0 与 unchanged 重采 |
| 印象笔记官方 HTML 作为主路线 | 降为回退；官方整库 `.notes` 已证明可用固定算法解密，网页 DOM 为补充，单篇 MHT 为 UI 回退 |
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
3. 原始导出件和媒体仍先进入外部 `BABATA_RECOVERY_HOME/batches/`，保留来源、范围、
   时间、工具和 hash，不进入 Git，不冒充已经提交 C0；
4. P3 稳定后把 recovery staging 通过唯一核心链路提交 C0，校验成功后清理重复 staging；
5. 长期 Browser Use/Agent Browser、P4 扩展和更多来源 E3 验证转为后续 Skill/自动化工作。

## 14. P2-G7 完成判断

所有 00 已点名来源都已有逐站调查、证据等级、最小授权、正常路线、回退与明确缺口；
Browser Use、Agent Browser、Playwright CLI 与 OpenCLI 的通用方向已经实际调用。Browser Use
与 Agent Browser 已连接同一个正式版 Chrome；飞书 `lark-cli` 已完成真实用户授权调用；
Codex Chrome 又在 Kimi 完成真实历史分页、长正文读取和外部 staging 样本。

因此 **P2-G7 已通过**。抖音等具体来源仍缺 E3，Kimi 也没有完成附件、逐条状态和重收集；
除已在 P7 以真实导出闭环启用的 `source.evernote` 和 `source.onenote` 外，其余未完成来源继续保持 disabled，
缺口进入 P4/P7。E3 未完成不再错误阻塞 P2，也不能因为 P2-G7 通过就显示未验收来源
available。

<!-- P2-G7: passed -->

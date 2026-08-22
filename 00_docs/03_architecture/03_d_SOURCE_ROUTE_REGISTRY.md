# Babata 来源路线与工具证据注册表

<!-- DOC-ID: DOC-ROUTES -->
<!-- DOC-AUTHORITY-BOUNDARY: source-route-research -->
<!-- P2-G7: passed -->

## 1. 文档职责

本文是逐来源路线、最小授权、工具证据、能力状态和诚实缺口的注册表。它回答“这个来源应当
怎样拿、当前 runtime 是否允许正式执行”，不回答“用户资料已经处理多少”。

- 产品来源范围：`DOC-REQ` / `DOC-PRD`；
- 稳定收集与 writer 边界：`DOC-ARCH`；
- 当前用户范围和完成度：`DOC-USAGE`；
- 实际执行入口：`SKILL-COLLECT`；
- runtime 真值：`babata --json capabilities list`。

表中的 `enabled`/`disabled`/`unavailable`/`absent` 是 route capability，不是 usage 完成声明。一次运行的对象数、
日期、Issue、批次路径和结果只进入 usage/receipt，不进入本文。工具版本、仓库活跃度和平台
接口会变化；执行前必须重新核 runtime，必要时补一次真实探针。

## 2. 证据等级

| Level | 路线证据 |
| --- | --- |
| E0 | 只核过官方文档、项目说明或候选方向 |
| E1 | 在本机核过安装、版本、help/doctor 或静态能力 |
| E2 | 用真实授权身份连接并读取明确范围，但未完成正式选择、保存与重收集 |
| E3 | 完成候选发现、明确选择、正文/附件取得、逐项状态和至少一次重收集 |

只有 E3 route 才可能 `enabled`，但 E3 不自动启用 runtime；产品 registry 仍可因适配器、授权、
完整性或维护原因保持 `disabled`。低于 E3 必须 disabled。

## 3. 稳定政策依赖

路线选择顺序、保真优先级和 acquisition/writer 边界只由 Requirements 与 `DOC-ARCH` 定义。本文不
复制该政策，只为每个 source/tool 记录当前证据、最小授权、诚实缺口和 runtime capability；政策
改变时先更新上游，再重新评价本注册表各行。

## 4. 来源路线注册表

<!-- P2-G7-SOURCE-TABLE -->
| source_id | source | normal_route | minimum_authorization | current_evidence | current_gap | current_status |
| --- | --- | --- | --- | --- | --- | --- |
| source.feishu | 飞书 Docs/Wiki/Drive | 官方 Lark API/CLI 读取明确节点或范围，结果交统一 Collector | 用户 OAuth 与所选空间/节点范围 | E3：真实候选、正文、附件、失败重试和 unchanged 重采已证明 | 当前 runtime descriptor 未启用；嵌入 Sheet/Base/Slides/画板需各自保真规则 | disabled |
| source.yuque | 语雀 | 登录态发现范围，优先官方 Markdown/PDF/LakeBook 导出 | 登录并选择知识库/文档；不索取手抄 token | E3：真实库、文档和收藏候选及正文取得已证明 | runtime descriptor 未登记；附件、表格、画板、评论未形成统一合同 | absent |
| source.onenote | OneNote | 官方 PDF+MHT 同源配对或明确 MHT 列表，经 Rust adapter 校验 | 客户端登录并选择一个导出范围 | E3：同源 pair/MHT-only 保存、附件覆盖和 unchanged 重采已证明 | 无可靠原生 page/section ID；跨导出身份不推断 | enabled |
| source.evernote | Evernote/印象笔记 | 官方 `.notes` 导出，经 Rust adapter 认证解密为 ENEX/ENML | 客户端登录并选择一个导出文件 | E3：正文、资源、结构校验、正式保存和 unchanged 重采已证明 | 导出缺稳定 note GUID/updated/notebook hierarchy；跨导出匹配未启用 | enabled |
| source.wechat_favorites | 微信收藏 | 官方手机到 PC 迁移后，本地只读恢复/导出；所选 URL 再取正文和直接媒体 | 用户完成官方迁移并明确收藏范围 | E3：收藏候选、本地恢复和代表正文/媒体取得已证明 | runtime descriptor 未登记；未选收藏不等于完整正文 | absent |
| source.wechat_articles | 微信公众号文章 | 公开文章 URL 或已授权 Recovery locator，保存正文与必要直接媒体 | 公开 URL 或明确本地范围 | E3：真实文章正文、图片和重采路径已证明 | runtime adapter 未启用；图片型/受限页需显式 gap | disabled |
| source.wechat_channels | 微信视频号 | 保留 source identity；重新启用前优先核验官方/本地迁移能力 | 用户明确纳入一个范围 | E1：候选工具与权限风险已核 | runtime descriptor 未登记；无真实完整 route，不使用证书拦截 | absent |
| source.wechat_chats | 微信聊天 | 官方手机到 PC 迁移后，用已验证本地恢复工具导出明确会话/媒体 | 用户完成迁移并选择会话范围 | E3：本地恢复、会话/消息/部分媒体和 C0 handoff 已证明 | runtime descriptor 未登记；媒体缺口必须逐项保留 | absent |
| source.zhihu | 知乎 | 登录态浏览器发现收藏范围，结构化详情/媒体工具作受控 acquisition | 登录并选择收藏夹/内容范围 | E3：真实收藏分页、正文、失败隔离与最后重试已证明 | runtime descriptor 未登记；视频型和不可读条目保持 gap | absent |
| source.bilibili | Bilibili | 保留 Chrome/官方页面/媒体工具候选；启用前重新核验 | 用户明确纳入一个收藏范围 | E3：收藏范围与媒体候选发现已证明 | runtime descriptor 未登记；完整收藏分页、媒体与重采合同未闭合 | absent |
| source.xiaohongshu | 小红书 | 登录态浏览器发现范围，结构化详情/媒体工具作候选 | 登录并选择收藏/专辑范围 | E3：真实可见卡片、详情和媒体 acquisition 已证明 | runtime descriptor 未登记；完整分页与专辑枚举未证明 | absent |
| source.douyin | 抖音 | 启用前先调查官方能力和可维护的登录态工具 | 用户明确纳入一个范围 | E0：旧候选路线已核但未形成可运行真实 route | runtime descriptor 未登记；签名、登录与完整性未实证 | absent |
| source.browser_bookmarks | 浏览器书签 | 官方 Netscape HTML 导出保留层级，再按所选 URL 取得正文/附件 | 一个导出文件或明确书签文件夹 | E3：真实导出解析、层级保存和代表页面正文取得已证明 | runtime adapter 未启用；历史记录/页面正文不是书签文件本身 | disabled |
| source.browser_pages | 当前页/选区 | 用户已登录浏览器内读取明确页面/选区，未来窄入口复用统一 Collector | 页面、站点或 visible-set 范围 | E3：真实页面候选和明确选择 acquisition 已证明 | runtime adapter 未启用；保真页面、附件和重采合同未闭合 | disabled |
| source.doubao | 豆包对话 | 已登录 Chrome 发现明确 conversation IDs，分页到完整并取得声明附件 | 登录并给出会话、时间、数量或 visible-set | E3：显式批次、长对话完整分页、附件校验和逐项重采已证明 | 普通图片/音频/引用页等新 shape 仍需各自实证 | enabled |
| source.kimi | Kimi 对话 | 登录态浏览器读取明确历史/会话，薄命令只固化已证明重采 | 登录并给出会话、时间或数量范围 | E3：真实候选、完整正文和结构化读取已证明 | runtime adapter 未启用；附件、深研产物和长期重采未闭合 | disabled |
| source.chatgpt | ChatGPT 对话 | 日常范围用登录态浏览器；账号 bootstrap 可用官方 Data Export | 登录并给出会话范围；全量需 Data Controls 明确确认 | E3：真实候选、正文和结构化引用读取已证明 | runtime descriptor 未登记；附件/画布/项目和增量重采未闭合 | absent |
| source.local_files | 本地文件/目录 | 核心文件选择器、拖放或受控目录扫描 | 明确文件、目录或 watch scope | E2：显式 file/export 的 hash、保存和回读已证明 | source descriptor 未登记；缺日常候选、逐项状态和 typed recollection route | absent |
| source.youtube | YouTube 明确播放列表/视频 | yt-dlp 取得白名单 metadata 与媒体，验证 cache manifest 后交统一 Collector | 用户明确给出 playlist/video URL 和范围；不扩张到账号/频道 | E3：真实 cache manifest 的候选、选择、正式保存和 3/3 unchanged 重采已证明 | 首版只接受已准备且完整映射的 cache manifest；adapter 不下载、不枚举频道/账号 | enabled |
| source.first_party | 第一方创作 | Workspace create/revise/annotate，不走来源 Collector | 明确新建、修订或批注动作 | E2：版本、关系、回读和故障补偿已证明 | source descriptor 未登记；不得用收集状态机替代作品版本语义 | absent |

`current_status` 是本表对 runtime descriptor 的文档镜像。若命令输出与本文冲突，以 runtime 为
执行真值并立即修本文；不得以 E3 历史证据绕过 disabled。

## 5. 代表工具证据

<!-- P2-G7-TOOL-TABLE -->
| tool_id | tool | current_evidence | next_user_action |
| --- | --- | --- | --- |
| tool.lark_cli | 官方 Lark CLI/API | E2：本机安装、OAuth 与真实文档/节点读取已证明 | 仅在明确飞书范围时完成授权；route enabled 前不正式收集 |
| tool.agent_browser | agent-browser | E1：安装/命令面与登录态复用能力已核 | 需要新 browser route 时再做真实隔离探针 |
| tool.browser_use | browser-use | E1：包、命令和浏览器控制能力已核 | 仅在当前 route 缺口要求时建立最小真实 probe |
| tool.codex_chrome | Codex Chrome control | E3：多个登录来源的候选发现、正文读取和 acquisition 已证明 | 用户只给明确 scope；工具本身不授予 route capability |
| tool.opencli | OpenCLI | E3：已证明来源的结构化读取、分页和附件 acquisition 可复现 | 只作为 source recipe 内依赖，不创建第二 user Skill |

工具证据说明“工具能做什么”，不说明某个 source 已 enabled。工具版本或平台接口发生变化时，
更新本表证据和对应 source gap；完整命令手册留在工具自身 Skill/官方文档，不复制到这里。

## 6. 授权与执行边界

用户通常只需要提供一次来源、明确范围，并完成 Agent 无法替代的登录、扫码、OAuth 或官方导出。
Agent 负责候选发现、分页、下载、hash、失败隔离和 staging，直到范围结束或出现真实阻塞。

必须停止的情况：

- 范围会从单项/文件夹/visible-set 扩张为账号级；
- route disabled/unavailable/absent；
- 登录、扫码、付费或权限提升不可替代；
- 平台返回不完整分页、声明附件缺失或身份不一致；
- 同一公共失败达到重试上限。

连接账号、打开页面、下载文件或 Recovery 成功都不等于 registered C0。

## 7. 淘汰与降级原则

不再维护逐版本候选墓地。候选仅在仍会影响当前路线选择时保留一行结论：

| Category | Decision |
| --- | --- |
| 手工 Markdown/复制作为正常路线 | 淘汰；只作明确恢复回退 |
| 要求手抄 Cookie/token 的路线 | 默认淘汰；除非用户明确接受且无更窄授权方案 |
| 已归档、DMCA blocked、无许可证或签名失效工具 | 不作长期依赖 |
| 通用“万能爬虫”替代已有 source recipe | 淘汰；先复用官方/成熟工具 |
| 浏览器可见即宣称平台支持 | 淘汰；仍须 source identity、完整性、附件和重采证据 |

详细历史调查、版本号、日期、Issue 和真实运行计数可从 Git 历史与 Git 外 receipt 追溯，不在
当前控制面重复保存。

## 8. 维护与关闭口径

P2-G7 通过表示：19 个点名 source 均有稳定 ID、正常路线、最小授权、合法证据等级、当前 gap
和能力状态；代表工具有真实本机证据。它不要求所有 route E3/enabled，也不证明任何用户
资料范围已经全量处理。

更新顺序：runtime probe -> source row -> source recipe/capability tests -> `SKILL-COLLECT` routing
index -> usage/receipt（只有真实运行结果变化时）。

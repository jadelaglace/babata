# Babata P4 首批真实收集路径

## 1. P4 目标

P4 在 P3 C0 底座之上证明两条正常日常收集路径不是空架子：

1. 飞书文档、Wiki、知识库中的上下文候选与选择性收集；
2. 浏览器当前页面、选区、链接和书签上下文中的候选与选择性收集。

P4 的重点不是“能把一个导出文件塞进数据库”，而是用户在正在阅读、收藏或整理
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

## 5. 浏览器路径

### 5.1 正常路径

首个真实探针先用 Browser Use CLI/Agent Browser 通过 Chrome 原生 CDP 复用用户当前
已登录浏览器，让 Agent 在一次给定范围内自主导航、翻页、发现和读取；已有稳定站点命令
时再调用 OpenCLI。长期浏览器扩展负责更窄的当前上下文触发：书签直接用官方
`chrome.bookmarks` API，当前页/选区使用 `activeTab` + `scripting`，完整网页原貌复用
SingleFile。具体证据见 `08_SOURCE_TOOL_RESEARCH.md` 第 5、9 节：

```text
用户给定当前页面、站点、收藏夹、会话、时间段或书签文件夹范围
  -> Browser Use/Agent Browser 复用已登录 Chrome，自主发现候选和遍历范围
  -> 需要稳定站点命令时调用 OpenCLI；需要保真页面时调用 SingleFile
  -> 只有范围有实质歧义、会越界或平台要求登录/授权时才再次找用户
  -> 与本机 Babata 配对
  -> loopback API 调用 CollectorSession/Capture 用例
```

候选可包含 URL、标题、选区/页面内容、声明元数据、书签层级、页面更新时间和已知
限制。CDP 探针只在用户一次批准当前 Chrome 实例后运行，并使用只读/导航优先的动作
策略；长期扩展默认不申请永久 `<all_urls>`，当前页由用户触发临时授权，书签权限按需
申请。任何浏览器工具都不持有数据根路径、SQLite 凭据或最终资产权限。

loopback API 只绑定本机，使用安装级凭据、来源限制和请求大小限制。配对失败、核心
不可用或 payload 超限必须在扩展中显示明确失败，不转存成隐藏权威副本。

### 5.2 书签与页面的差异

- 当前页面/选区：收集本次用户看见或选择的页面内容与 URL 上下文；
- 书签：先展示选中文件夹或集合中的标题、URL 和层级，再由用户确认范围；
- 书签存在不代表网页内容已取得；只保存 URL 时应明确是 locator-only 或待补充状态；
- 后续网页变化通过重收集追加版本，不覆盖第一次保存的页面证据。

### 5.3 回退路径

Netscape bookmark HTML、保存的 HTML/PDF、copy 和 screenshot 可以作为导入/恢复路径。
已有 bookmark export 和 CandidateEnvelope 测试证明格式、hash 和 C0 边界，不证明浏览器
扩展的候选、选择、配对和逐条反馈已经完成。

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
| P4-G2 浏览器候选 | 扩展配对后能展示页面/选区/书签候选和明确范围 |
| P4-G3 选择性提交 | 未确认不写 C0；确认后只收集所选项 |
| P4-G4 逐条状态 | queued/running/saved/skipped/failed、局部成功和重试成立 |
| P4-G5 重收集 | changed/unchanged/inaccessible/removed 不覆盖旧 C0 |
| P4-G6 真实能力 | 真实证据与 fixture 证据分开，未验证来源保持 disabled |

P4-G1 至 P4-G6 共同对应 AC-01、AC-02 和 TC-01、TC-02。只通过导出解析、书签 HTML
或 CandidateEnvelope fixture，不满足 P4 完成门。

## 9. P4 明确不做

- 不做账号级静默全量复制和远程后台爬取；
- 不在 adapter 或扩展中写 SQLite、最终原件或 C0 关系；
- 不把导出路径、CLI 参数和手填 metadata 作为正常日常流程；
- 不做 OCR、转写、摘要、模型判断、搜索、子库和输出；
- 不因为已存在 P4 测试文件或 migration 就提前标记 P4 进行中/已完成。

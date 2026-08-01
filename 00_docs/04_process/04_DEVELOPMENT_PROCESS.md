# Babata 开发流程与实时进度

本文是 Babata 唯一的实时阶段状态和交付顺序来源。00–03 定义为什么做、做什么、
怎样算产品完成以及技术边界；架构补充定义文件和阶段设计；本文只维护现在到哪、
下一步是什么、通过哪道交付门才能进入下一阶段。

## 1. 当前状态

**更新时间：2026-08-01**

### 1.1 当前运行模式

Babata 已从架构和主链建设转入真实数据回收、剩余来源闭合、产品验收和运维收尾。P0-P6
及整体架构默认稳定；当前复用已有边界完成 P7/P8，不借局部问题重开系统设计，也不要求用户
参与普通架构选择或 routine review。

责任分工如下：

- 用户决定需求、优先级、进度是否符合预期和最终产品验收；
- Agent 维护受影响的文档链，承担实现、测试、证据、状态更新和日常技术/架构 review；
- 只有需求歧义会改变实际结果，或决定涉及不可逆模块边界、数据权威、安全、验收口径及其
  阻塞时，Agent 才暂停并升级给用户；一般可逆实现选择由 Agent 直接完成并留下证据；
- 外部讨论和 AI 回答只用于启发候选方案，其工具、模型、语言和流程结论未经用户明确采用，
  不进入 Babata 的需求或架构权威。

执行默认先在清楚的授权和验收范围内批量展开，再快速集中收敛。相关局部检查放在逻辑边界，
全量文档、编译、测试和边界门禁通常集中到一个活跃工作日的两到三个检查点，并在 PR 合并或
阶段验收前执行；高风险改动、共享契约变化和已出现的系统性失败仍立即验证。检查用于及时拦住
问题，而不是让每次小编辑、每个窄功能都重复走全量流程。

```text
P0  冻结旧版本                                    已完成
P1  真实需求、PRD、产品验收、全局技术架构           已完成
P2  全系统模块、目录、代码与工具骨架                 已完成
P3  C0 原始资料与第一方版本底座                     已完成
P4  飞书与浏览器首批真实收集路径                     已完成
P5  C1 多模态清洗与百炼处理                         已完成
P6  核心沉淀、检索、子库与输出                      已完成
  P6.1 核心知识沉淀                                 已完成
  P6.2 发现、检索与关系导航                         已完成
  P6.3 子库与输出                                   已完成
P7  扩展来源、正式 Skill 与受控 Agent               已完成
P8  来源广度、后续阶段、全量 A1 与存量准备             进行中（新增抓取暂停；P8.5 样本 C0-B 已完成）
P9  简单备份同步                                      未开始（本地备份恢复能力已存在）
```

<!-- P2: completed; P2-G1..P2-G7: passed -->
<!-- P3: completed; P3-G1..P3-G6: passed -->
<!-- P4: completed; P4-G1..P4-G6 and TC-01..TC-02 passed; representative small real loops proven; incomplete routes remain disabled -->
<!-- P5: completed; TC-03A and TC-04 passed; AC-04 passed; P6.3 later completed TC-03B and the full AC-03/TC-03 -->
<!-- P6.1: completed; AC-05..AC-06 and TC-05..TC-06 passed -->
<!-- P6.2 preflight: Issue #60 common C0 metadata/observations implemented -->
<!-- P6.2: completed; AC-07 items 1..4,7 and TC-07 steps 1..4 plus search projection rebuild passed; P6.3 later completed the full AC-07/TC-07 -->
<!-- P6.3: completed; AC-03, AC-07, AC-08 and TC-03, TC-07, TC-08 passed -->
<!-- P7: completed; AC-09/TC-09 passed through real source batches and the controlled two-attempt item contract; account-wide recovery belongs to P8 -->

2026-07-26，Issue #104 澄清 C0 的两个独立维度：主权深度 C0-A1/C0-A2/C0-A3+ 与管理就绪
captured/prepared/registered。当前真实回收优先抢救 C0-A1，再补直接依赖 C0-A2；语义引用 C0-A3+
显式、有限、低优先级。只有 Rust/SQL 完成并回读的 registered/C0-C 称为正式 C0；Recovery
中的 captured/prepared 材料可以如实算“已拿回”，但不冒充正式登记。该决定只更新定义、
验收和报告合同，不新增 schema/Rust，不重开 P3，也不撤销 P0-P6 或既有 P7 证据；历史文档
和测试中的“C0”默认按 registered/C0-C 解释。以后 Recovery 汇报同时给出主权深度与管理状态。
prepared/C0-B 与 registered/C0-C 的共同准入门槛是 C0-A2；C0-A1-only 不进入 B/C，C0-A3+ 不阻塞 B/C。

按这一口径，当前微信文件传输助手与收藏第一阶段只达到 C0-A1：记录、URL 和本地可得资源已
保全，但大量 URL 正文/媒体及缺失必要引用尚未形成 C0-A2 完整性闭环。豆包 MBA 第一阶段已按
完整消息链和声明附件逐项覆盖，达到 C0-A2。二者都先按真实主权深度汇报，不抢跑 B/C。

P7 的微信正式能力证明只取 10–20 条有界代表样本，文件传输助手和收藏两类都覆盖，优先选择
URL/文章并带少量本地媒体。Agent 必须逐条取得正文、内嵌媒体和必要附件达到 C0-A2，随后走
prepared/C0-B、registered/C0-C 和 unchanged 重采；其余第一阶段全量继续保持 C0-A1 Recovery。
该小样本足以验证新来源/内容形态与受控 Agent，不把全量微信 C0-A2 错列为 P7 阶段门槛。

当前真实情况：

2026-07-27，Issue #114 按最新用户定义推进 P8.1 来源广度最低覆盖。计数只使用真实活动库、
Recovery 或实际取得的来源响应/导出件；fixture、locator-only 浏览器扩展记录，以及
`prefetched_envelope_json = null` 的 P4 `collection_candidates` 均不计。紧急收集新增飞书、
语雀、Kimi、Bilibili、小红书和知乎各 4 条真实响应，使这 6 个来源从 1 条补到 5 条；外部
Chrome 恢复后又取得 4 条不同 ChatGPT 会话的完整响应并逐条固化到 Recovery。逐来源结果如下：

| 来源 | 已有 C0-A1 或更高数量 | 是否达到 5 条 | 还差多少 | 正常获取路线 |
| --- | ---: | --- | ---: | --- |
| 飞书文档、Wiki、知识库 | 5 | 是 | 0 | 官方 `lark-cli` Docs/Wiki/Media API |
| 语雀 | 5 | 是 | 0 | 登录 Chrome 发现范围，官方 Markdown/整库导出 |
| 豆包对话 | 317 | 是 | 0 | 登录 Chrome 的历史与结构化会话响应 |
| Kimi 对话 | 5 | 是 | 0 | 登录 Chrome 的结构化历史与消息响应 |
| ChatGPT 对话 | 5 | 是 | 0 | 登录 Chrome/OpenCLI；账号级可用官方 Data Export |
| Bilibili 收藏 | 5 | 是 | 0 | 登录 Chrome/OpenCLI；所选媒体再用 `yt-dlp` |
| 小红书收藏 | 5 | 是 | 0 | 登录 Chrome/OpenCLI 详情与媒体路线 |
| 知乎收藏 | 5 | 是 | 0 | 登录 Chrome/OpenCLI 收藏与详情路线 |
| 浏览器书签/网页 | 5 | 是 | 0 | 明确浏览器范围后由 Agent 读取并遍历 |
| OneNote | 7（全量） | 是 | 0 | 官方桌面 PDF/MHT 导出 |
| 印象笔记 / Evernote | 164（全量） | 是 | 0 | 官方整库 `.notes` 导出 |
| 微信收藏 | 5,025（全量） | 是 | 0 | 只读解密 DB、ZIP 与 Recovery 整合 |
| 微信公众号文章 | 233 | 是 | 0 | 公开文章 URL 与只读 Recovery；不操作微信 UI |
| 微信聊天记录 | 216,449 条消息 / 746 个会话（全量） | 是 | 0 | 只读解密 DB、ZIP 与 Recovery 整合 |
| 本地文件与用户第一方资料 | 499 | 是 | 0 | 本地直接复制与 first-party 核心提交 |
| 抖音 | non-plan | 不计 | 不计 | 用户重新规划前不动作 |
| 微信视频号 | non-plan | 不计 | 不计 | 用户重新规划前不动作 |

机器可读账位于
`BABATA_RECOVERY_HOME/recovery/p8-1-source-breadth-20260727/summary.json`；本轮紧急响应包位于
`BABATA_RECOVERY_HOME/batches/p8-1/20260727-emergency-a1/`，普通网页 A1 位于
`BABATA_RECOVERY_HOME/batches/browser/20260727-p8-1-history-a1/history.json`。P8.1 已完成 15/15，
没有为这些数据追加 A2/B/C/C1；P8.2 已完成豆包第二/第三阶段、微信第二阶段和第三阶段其他范围，
群聊按用户优先级暂缓；P8.3 已由 Issue #124 启动，当前全量盘点见 11.3。

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
  10 个引用。来源仍保持 disabled：Kimi/ChatGPT 当前样本无附件，豆包只闭合一个复杂
  会话的 7 个 DOCX 原件、其他附件形态与长期执行未闭合，Bilibili 按用户要求只证明一条，飞书嵌入
  Sheet/Base/Slides/画板内部数据仍未覆盖。P4 已按代表性首批路径收尾，不把阶段完成扩大成
  全部点名来源完成或来源 available。
- P4-G1 至 P4-G6、TC-01 和 TC-02 已通过。P4 完成只证明飞书与正式 Chrome 点名平台的
  首批流程、选择范围、逐条状态、失败重试和重采边界成立；微信聊天在 P4 收尾时尚未闭环，
  后由 2026-07-26 第一阶段 Recovery 取得真实全量源和有界子集；视频号、抖音和书签自动
  遍历等仍未闭环。P4 收尾时 OneNote 与印象笔记都只有 E2 导出解析；现已分别由
  P7 Issue #84/#82 完成真实 C0 与 unchanged 重采并单独启用；Issue #86 又完成六个显式
  OneNote MHT 导出的 C0、非事实重叠提示与 unchanged 重采。其余扩展来源继续留在 P7，
  书签最后单独收集；抖音和视频号按用户决定暂时不处理。
- P4 当时的微信样本使用官方 PC 微信 4.1.11.55 的“全部收藏”窄 UI，读取 8 个最新可见候选并选择
  “爬虫-这20个仓库教会什么叫降维打击”；保存 2,946 字符结构化正文、2,597 字节
  Markdown 和 2,331,350 字节原始 HTML。首次因候选白名单缺口进入可重试 `failed`，原
  item retry 后为 1 item/1 revision/2 exports，重采 `unchanged` 且数量不增加。未扫描
  微信进程内存、未解密数据库、未安装代理证书；当时收藏其他类型、聊天和自动遍历仍未完成。
  2026-07-26 已由最新本地恢复路线覆盖收藏全类型与聊天真实数据，第一阶段范围另见 P7 记录。
- 2026-07-19 另完成豆包复杂会话“战略领导力W1”的 Agent 收集：16 条消息、8 轮问答和
  完整脑图已拿回，7 个原始 DOCX 共 111,296,956 字节，逐个大小和 MD5 与豆包消息元数据
  一致，并通过 DOCX 结构检查。对话和脑图已正式归档；P5 TC-03A 又把其中“设立目标”的
  原始 DOCX 与平台预览 PDF 作为同一 item 的新 revision 正式登记，分别标为 `original` 和
  `preview`。Issue #88 随后把其余 6 个 DOCX 通过通用附件操作附加到该 ready revision；
  全部 7 个原始 DOCX 现已正式进入统一 C0，未新增正文 revision、relation 或 C1。该结果仍
  不冒充豆包其他附件形态、可执行来源 recipe 或长期自动化完成。
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

- P5 已完成：百炼 CLI（`bl`）的真实多类型试跑、引导 Skill、真实 PDF/图片/视频 C0→C1、受控 C1 文件、删除重建、受限样本和原件/预览边界共同通过 TC-03A。合并后的 `main` 又以真实微信 C0 完成 local extract 和一次 `qwen-plus` 摘要，保留注入的 provider 失败与成功 retry、实际 task/usage/output hash、unavailable 分支，并复核 C0 正文/asset 哈希不变，TC-04 与 AC-04 通过。P5 收尾时完整 AC-03/TC-03 仍需 P6 TC-03B，现已由 P6.3 补齐；正式 C1 和队列进入 `BABATA_DATA_HOME`，阶段证据进入 `BABATA_EVIDENCE_HOME`，二者都不进入 Git。
- P6.1 已完成：真实 C0/C1 可在用户零回复时由 Agent 消化为机器/未审阅的三大界核心，
  三级地图、五类语义、关系、三维评分、地图演进、高密度文本和窄 C2 均可追溯；评论、
  Log、Insight、附件、Agent 再分析和真实作品改写保持不同语义。AC-05、AC-06、TC-05、
  TC-06 已通过；P6.2 检索/浮现与 P6.3 子库/输出现均已完成。
- Issue #60 已完成 P6.2 preflight 实现：新增 `babata.c0.common/v1`、
  `babata.c0.media/v1` 和追加式 source observation，raw schema 升为 v6、collection schema
  升为 v5。首次 item 事实与后续观测分离，changed/unchanged/inaccessible/removed 不再因
  metadata 追踪需要制造伪 revision；除来源级 fixture 外，飞书、知乎和微信还在隔离复制
  数据根中完成真实 provider 重采与读回验证。
  该切片只提供稳定输入；P6.2 的正式通过证据由后续 Issue #74 独立完成。
- P6.2 已完成：独立 C2 搜索投影统一发现 raw item 与 semantic entry，支持全文与结构化组合
  检索、三维评分筛选/排序、详情回看、关系遍历和带 direction/relevance/time/relation 原因的
  主动浮现。真实数据根完成构建、搜索、导航、删除、重建和权威库不变审计；AC-07 第 1–4、
  7 项与 TC-07 第 1–4 步及搜索投影重建部分已通过。子库定义/物化当时仍属 P6.3；P6.3
  已补齐剩余子库责任，因此 AC-07、TC-07 现已整体通过。
- P6.3 已完成：版本化 `SublibraryDefinition` 通过 first-party C0 revision 链保存，raw schema
  升为 v7 并在数据库层保护定义来源、版本连续性和不可原地改写。子库物化、Markdown 人读
  输出和结构化 JSON 输出均带 manifest、输入 hash/版本、机器/人工与审阅身份、builder/
  template/profile、状态和限制；CLI/local API 调用同一 application service。真实 machine/
  unreviewed Knowledge 形成 1 项成员的子库和两类输出，篡改检测、delete/rebuild 与只读审计
  通过；Web/Obsidian 保持 unavailable。AC-03、AC-07、AC-08 与 TC-03、TC-07、TC-08 通过。

项目阶段只使用 P0–P8；C0–C3 是数据权威级别，不是项目阶段。

### 1.2 人话进度地图

```text
已经真的收进 Babata，而且重采过
  Kimi      15 个真实候选 -> 选 1 条 -> 1 条资料/1 个版本 -> 重采没变化
  豆包      20 个真实候选 -> 选 1 条 -> 1 条资料/1 个版本 -> 重采没变化
             -> 另收“战略领导力W1”：16 条消息 + 完整脑图 + 7 个原始 Word
             -> 7 个 Word 共 111.30 MB，大小/MD5/Word 结构均已校验
             -> 对话、脑图和 7 个原始 Word 均已正式登记；另保留 1 个平台 PDF 预览
             -> 补附件没有制造新正文版本，也没有启动清洗
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

P5 已收尾；P6 现已完成
  P6         核心沉淀、检索、子库与输出（已完成）

转入 P7 扩展来源，不是 P4 完成证据
  微信聊天/收藏其他类型（现已验证官方迁移 + 本地恢复；第一阶段完成，第二/三阶段排 P8）
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
5. 产品意图先进入 00，再按真实影响更新 PRD、验收、架构、流程、测试和代码；语义未变化的
   文档不机械重写，但要明确检查过并记录无需变化。
6. 架构补充与主架构冲突时先改补充和骨架，再改代码；不为保留旧数字扭曲产品。
7. `AGENTS.md` 只提供本地操作上下文，不是产品、架构或进度权威。

## 3. P0：冻结旧版本

旧版本保留在 `C:\Users\Aiano\Babata-2.0-frozen`，不在 reboot 工作区继续演化。
P0 已完成。

## 4. P1：真实需求到全局架构

P1 交付链：

```text
00_REQUIREMENTS.md（含精选保真的用户原话证据）
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
2. 回收结果写入 `${BABATA_RECOVERY_HOME}/batches/<source>/<batch-id>/`，保留原始
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
  `${BABATA_RECOVERY_HOME}/batches/doubao/20260719-w1-complex/`。预览器下载的
  43 页 PDF 只是豆包转换预览件，不是原件。Issue #88 复核 Recovery manifest、消息内
  MD5、SHA-256 和 DOCX 结构后，把此前缺少的 6 个 DOCX 作为 `original` 附加到既有 ready
  revision。战略领导力W1 现在有 7 个 DOCX original 和 1 个 PDF preview；正文仍为 2 revisions，
  attachment operation/member 为 1/6，没有新 relation 或 C1。当前结论分别是：该明确范围的
  Agent 收集与统一 C0 登记均已完成；豆包其他附件形态和长期自动化未完成。
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
  可重复取得；这是 P4 当时的 UI 样本，不代表收藏自动遍历、聊天或微信全量已形成长期能力。
  2026-07-26 后已有本地恢复实证，但正式 Rust route 仍未启用。来源继续 disabled。

2026-07-19 曾建立 Issue #20 尝试把豆包原附件取得开发成持久适配器。复核后确认 Agent
已经把最复杂样本真实跑通，当前继续开发会偏离“优先现有工具、最少开发”的需求，因此
Issue #20 已按 `not planned` 关闭，实验代码全部撤销且未进入 Git。若后续需要重复执行，
优先把已验证的 Agent/Chrome 流程整理为 Skill；只有真实重复使用证明仍缺稳定能力时，
才重新评估窄适配器。

浏览器和官方客户端仍是当前存量回收首选。Kimi/豆包/ChatGPT/知乎/小红书/语雀的 OpenCLI 薄命令是为了把浏览器已经证明的
读取动作变成任务结束后可调用的重试/重收集；Bilibili 是因为 Codex Chrome 历史页连续
两次超时后才回退 OpenCLI。微信历史样本由官方 PC 微信窄 UI 发现收藏候选，当时 OpenCLI
只固化已知公众号 URL 的下载和重采；该事实保留为历史证据。2026-07-26 后微信批量主路线
已更新为官方迁移 + 已验证本地恢复，UI 只作回退。三类理由均已记录，不把 OpenCLI 当默认绕路。

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
部分已验证薄命令/Agent 流程成立。当时 OneNote、微信其余范围，以及印象笔记从已解密样本到
全量 ENEX/C0/重采的剩余工作转入 P7；其中微信第一阶段 Recovery 已于 2026-07-26 完成，
第二/三阶段转入 P8。抖音、视频号暂时延期。

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
   `BABATA_EVIDENCE_HOME/runs/p5-tc04-20260720-0015/TC04_PROVIDER_QUEUE_E2E.md`。
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
子责任与 TC-03B。P5 收尾时 AC-03 和 TC-03 尚未整体通过；P6.3 完成后两者已整体通过。
C1 不覆盖 C0，模型输出不自动成为人工判断。

## 9. P6：核心沉淀、检索、子库与输出

2026-07-20 已从 1.0 原始归档恢复 P6 的“个人知识宇宙”产品基线，并由
`09_P6_PERSONAL_KNOWLEDGE_UNIVERSE_BLUEPRINT.md` 集中说明。2026-07-21，Issue #65 / PR #66
完成 P6.1 纵向闭环并通过完整门禁。2026-07-22，Issue #74 完成 P6.2 发现、检索与关系
导航。2026-07-23，Issue #76 完成 P6.3 版本化子库、可重建物化和可追溯输出，P6 整体完成。

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
`BABATA_EVIDENCE_HOME/runs/p6-1-correction-20260720-233608/snapshot`，不进入 Git。

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
   `BABATA_EVIDENCE_HOME/runs/p6-1-semantic-core-20260721-202350/snapshot`；迁移前后原有
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
   `BABATA_EVIDENCE_HOME/runs/p6-1-map-evolution-20260721-213222/snapshot`。Rust 入口完成 knowledge
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
   `BABATA_EVIDENCE_HOME/runs/p6-1-foundation-guard-20260721-220641/P6_1_FOUNDATION_GUARD_E2E.md`。
8. AC-06 反向审计发现旧 `capture attach-assets` 会为只补附件复制正文并增加 revision，
   `workspace revise` 也接受正文完全相同的请求，均与本轮明确纠偏冲突。现已新增 raw
   migration 0005：补附件作为独立 operation 追加到既有 ready revision，保留 reason、metadata、
   asset membership、状态和失败；finalise、校验或 ready transition 失败只隔离本次附件，
   原正文仍为 ready。临时应用/CLI/SQLite 测试证明 revision 数量不变、相同正文修订被拒绝、
   跨 revision 挂附件被拒绝。真实 raw 库通过 Babata Rust 入口从 v4 升到 v5，迁移前在线
   快照后原有 `5 sources / 27 items / 30 revisions / 7 assets` 及全部知识业务行数不变，
   新表为空，checksum 匹配、`quick_check=ok`、foreign key 异常为 0；未为验证制造真实附件。
   证据位于 `BABATA_EVIDENCE_HOME/runs/p6-1-attachment-semantics-20260721-223511/`
   `P6_1_ATTACHMENT_SEMANTICS_E2E.md`。

该证据证明自动语义候选已真实进入核心，不再只是 review 准备；但不得把模型输出冒充用户
确认，也没有替用户制造真实评论、Log、Insight 或审阅决定。旧 P5 附件登记事实保留为历史
操作证据，不再作为规范语义；后续补附件不制造正文版本。PR #66 合并后，AC-05、AC-06、
TC-05、TC-06 已通过，P6.1 已完成；在 PR #66 合并时，P6.2/P6.3 尚未开始。

### P6.2 preflight：稳定 C0 来源 metadata 与 observation

Issue #60 在正式搜索实现前完成以下底座：

1. `CandidateSummary -> CandidateEnvelope -> CaptureService -> NewItem/source observation ->
   repository read-back` 贯通版本化公共来源合同，同时保留 provider 原始 metadata；
2. raw migration 0006 为 item 增加首次公共事实，并新增 append-only
   `source_observations`；collection migration 0005 保存 discovery candidate 公共字段；
3. changed 只新增一个真实 revision；unchanged/inaccessible/removed 只追加 observation 与
   recollection check。item 首次事实不被后续结果覆盖；
4. 来源级 fixture 验证飞书 typed title/hierarchy/updated time 与媒体类型、知乎 author 和
   created/updated 的 RFC3339/Unix 兼容映射，以及微信含糊时间的保守处理；
5. legacy envelope、raw v5 -> v6、collection v4 -> v5、幂等和 checksum fail-closed 均由
   临时数据库测试覆盖；P2/P3 ownership/inventory 门继续适用；
6. 主真实 raw 库在在线快照后由 v5 升至 v6、collection v4 升至 v5，原有
   `5 sources / 27 items / 30 revisions / 7 assets / 1 relation` 按旧列逐表比较无差异，
   新 observation 表为空，`quick_check=ok` 且 foreign key 异常为 0。隔离复制数据根上的
   飞书、知乎、微信既有样本均以 unchanged 重采而不增加 revision；飞书另有一次 fresh
   discovery/capture 证明 title、三级 hierarchy、UTC updated time 和 limitation 进入 item
   与 capture observation。知乎 author、双时间和 17 个媒体条目可读；微信 raw
   `6月2日 08:56` 仍未被伪装成 UTC，并记录缺少年份/时区的 limitation。证据保存在
   `BABATA_EVIDENCE_HOME/runs/p6-2-c0-metadata-20260721-003825/`。

本切片自身不创建搜索 projection，不实现多条件检索、评分排序、关系导航、子库或通用
输出；它不能单独作为 AC-07/TC-07 通过证据。正式 P6.2 已由后续 Issue #74 独立实现。

### P6.2 检索与关系导航

- C0/C1 可重建读投影；
- 正文、来源、时间、语义类型、状态、人物、地图归属、分类、关系、处理状态和三维
  相关度检索；
- 媒体-only、附件-only 和受限资料仍可发现；
- 版本、来源、地图归属、知识/案例证据、日志/感悟引用和其他关系导航；
- 至少一种基于当前方向、相关度、时间和关系的可解释内容浮现入口。

交付 AC-07 的检索和关系部分。

Issue #74 的 P6.2 实现与证据：

1. `SqliteReadProjection` 在 `03_views/search/index/search.sqlite` 建立独立 C2，把 27 个真实
   raw item 与 3 个真实 semantic entry 投影到统一检索面；14 条导航关系保留原件、版本、
   派生物、地图、Knowledge/Case 和其他显式关系；
2. `explore rebuild/delete/status/search/show/traverse/surface` 与本地 API
   `POST /v1/explore/search` 调用同一 application service。多条件检索覆盖正文、来源、时间、
   类型/状态、人物、地图/标签/关系、处理与审阅身份、媒体/附件/受限/缺失及 profile/
   三维分数；
3. 未审阅、accepted、rejected 均可搜索；rejected/modified 原建议不进入主动浮现。surface
   只返回有合格评分的 semantic entry，并逐项返回 direction、relevance、time、relation；
4. 投影重建在单一事务中替换数据，失败保留上一版；delete 只删除 projection SQLite 及
   WAL/SHM。source fingerprint 覆盖权威 item/revision/asset/observation、语义/地图/评分/
   审阅/关系、run 和 derivative 内容；
5. fixture 覆盖真实库中不存在且不得伪造的 attachment-only、受限、缺失边界，以及全部
   搜索/导航/浮现资格状态。真实验证中全文 `AI` 返回 23 项、未审阅 Knowledge/profile
   组合查询返回 1 项、media-only 返回 2 项；详情、遍历和 3 项浮现均完整读回；
6. 删除重建后 fingerprint 与行数一致；46 张 raw/knowledge 表和 4 张 derived 表摘要在构建
   前后完全相同，`quick_check=ok`、foreign key 异常为 0。外部证据位于
   `BABATA_EVIDENCE_HOME/runs/p6-2-discovery-20260722-235222/`。

P6.2 已完成；AC-07 第 1–4、7 项和 TC-07 第 1–4 步及步骤 6 的搜索投影部分已通过。
P6.3 已补齐 AC-07 第 5–6 项、TC-07 第 5 步及步骤 6 的子库物化部分，因此两项整体通过。

### P6.3 子库与输出

- 版本化 SublibraryDefinition；
- 可删除重建的子库物化；
- 人类可读和结构化输出；
- manifest、来源/版本/profile/建议状态回溯和只读 builder；
- Obsidian、网页、报告等在真实用途出现后逐项启用。

交付 AC-03 的 C2 子责任、AC-07、AC-08 和 TC-03B、TC-07、TC-08。

Issue #76 的 P6.3 实现与证据：

1. `babata.sublibrary/v1` 保存用途、组合选择规则、人工纳入/排除、组织规则和未审阅策略；
   create/revise 走现有 Workspace first-party C0 写入，raw v7 将 definition version 与 revision
   ordinal/parent 绑定，并拒绝正文 UPDATE/DELETE。fixture 覆盖两版完整读回与旧版保护；
2. `SublibraryViewStore` 在 `03_views/sublibraries/<id>/v<version>/` 生成 materialization 与
   manifest；成员保留纳入依据、权威引用、输入 hash、机器/人工和审阅身份。人工 exclude
   优先，unreviewed 是否纳入由定义显式控制；
3. `OutputViewStore` 对显式记录集合或固定子库版本生成 Markdown 与结构化 JSON；manifest
   保存 scope、输入 ID/version/hash、来源、builder/template/profile、时间、状态、限制、
   output hash、generation 和重建差异。Web/Obsidian 仍返回 unavailable；
4. fixture 纵向测试从真实 Rust CLI 完成 create/revise/list/show、materialize/verify/delete/
   rebuild、两类 output、外部篡改、CLI/API 同服务和 raw DB 负向保护；unreviewed 纳入/排除
   另有应用层正反测试；
5. 主真实库先在线快照再由 raw v6 升至 v7；46 张 raw 表仅 `schema_migrations` 变化，业务
   表不变，`quick_check=ok`、foreign key 异常为 0。随后用已有 1 条 machine/unreviewed
   Knowledge 生成真实子库和两类输出，身份仍为 `human_judgment=false`、
   `confirmed_fact=false`；
6. 子库物化篡改使 verify 失败；人读输出篡改返回 `valid=false`。delete/rebuild 后再次 verify
   通过。C2 操作前后 46 张 raw/knowledge 表和 4 张 derived 表逐表摘要完全相同。证据位于
   `BABATA_EVIDENCE_HOME/runs/p6-3-sublibrary-output-20260723-200352/`。

P6.3 与 P6 整体完成。该结论只启用实际验证的子库、Markdown 和 JSON 能力，不把 P7 的
正式 Skill/Agent、P8 的备份恢复或未实现的 Web/Obsidian 输出提前写成完成。

### 9.4 P6 后活动数据根治理（Issue #78）

2026-07-23 对真实本地目录完成一次边界纠偏，不改变 P6 产品能力或提前完成 P8：

1. `BABATA_DATA_HOME` 顶层收敛为 `00_inbox` 至 `05_logs` 六个编号分区和最小本地说明；
   原有 `verification/`、`recovery-staging/`、`generated/` 不再作为活动根顶层约定；
2. 360 个验收文件、502,861,199 字节迁至 `BABATA_EVIDENCE_HOME/runs/`；19 个尚待正式
   收集或用于来源恢复的文件、160,453,869 字节迁至 `BABATA_RECOVERY_HOME/batches/`；
   50 个模型/预处理工作文件、40,086,007 字节迁至
   `04_runtime/staging/model-workspaces/`；
3. 迁移采用先复制、逐文件 SHA-256 验证、目标完成后才删除旧顶层的顺序；仓库外
   manifest 记录 429 个文件和 703,401,075 字节，迁移后复核错误为 0；
4. 迁移前后 raw、derived、runtime、search 四个 SQLite 均 `quick_check=ok`、foreign key
   异常为 0，所有表行数不变；7 个 C0 与 6 个 C1 内容寻址文件哈希均有效；
5. `migrate-auxiliary-data-roots.ps1` 默认只审计，显式 `-Apply` 才执行；临时夹具覆盖
   audit-only、不丢字节迁移、干净重跑和目标冲突闭锁。CI 门禁阻止旧顶层路径重新进入
   权威文档或 Skill。

完整真实清单和报告只位于 `BABATA_EVIDENCE_HOME/governance/migrations/issue78-20260723/`，
不进入 Git。该工作只完成 TC-10 第 1 步的数据边界证据；一致快照、加密备份和隔离恢复
仍属于 P8，AC-10、TC-10 不提前标记整体通过。

## 10. P7：扩展来源、正式 Skill 与受控 Agent

按真实价值扩展 OneNote 官方整本 PDF/MHT、印象笔记官方整库 `.notes` 解密接入、微信
聊天/收藏其他类型，以及已有小样本来源的更多内容形态。微信先用官方功能把手机记录迁移到
PC，再由 Agent 调用已验证的 WeChatDataAnalysis 本地恢复/导出；官方 PC UI 作为小范围回退。
Recovery 产物仍须以后经 Rust Collector 才成为 C0。抖音和视频号暂时不
处理，只有用户重新启用时才回到队列。浏览器书签排在最后，作为独立收集项自动遍历正文
和附件，不与本阶段其他来源扩展混做。

2026-07-19 已有两个只读 E2 导出解析探针：OneNote 整本 MHT 可读，包含 1 HTML、30 张
图片和 1 个 XML 清单，但没有明确页面边界；印象笔记 `.notes` XML 含 163 条笔记和 349
个资源，163 条正文虽为 `base64:aes`/`ENC0`，但公开的固定算法不需要用户密码。真实文件
首条已通过 HMAC 校验并解密为 381 字节 ENML；网页 DOM 和单篇 MHT 也已验证为备选。
该段是进入 P7 前的 E2 基线；印象笔记和 OneNote 已分别由下述两个切片推进到 E3。

2026-07-24，Issue #82 完成 P7 首个单来源切片：真实 `.notes` 与 P4 记录的
78,711,776 字节/SHA-256 完全一致并归入 Recovery；Rust adapter 对 163/163 正文逐条完成
ENC0、HMAC-SHA256 与 AES-128-CBC 验证，生成全量解密 ENEX，并映射 349 个资源。隔离库和
活动库均发现 `1 batch + 163 notes`，活动库 164/164 saved、0 failed/skipped；随后同 session
重采 164/164 unchanged、0 新 revision。活动库由 `5 sources / 28 items / 31 revisions /
7 assets` 变为 `6 / 192 / 195 / 358`，原有 relation 仍为 1；Evernote 自身为
`1 source / 164 items / 164 revisions / 351 assets / 328 observations`。全库 schema v7、
`quick_check=ok`、外键异常和 pending/quarantine/journal/orphan 均为 0。

`.notes` 不含 note GUID、updated 或笔记本层级，created 也不唯一，因此当前身份明确限制为
immutable export hash + note ordinal；不伪造跨导出稳定 ID。底层 `source.evernote` 已启用，
当时总 Skill、受控 Agent、取消/越界拒绝的完整 TC-09 仍未完成，P7 继续保持进行中。
开发证据位于 `BABATA_EVIDENCE_HOME/runs/p7-1-evernote-20260724-224315/`，不是 P8 备份。

2026-07-25，Issue #84 完成第二个单来源切片。用户明确同次 PDF/MHT 是一个 OneNote 来源：
PDF 更接近平台渲染和划分，MHT 更适合图片、文字和格式。真实 17,161,246 字节 MHT 含
1 HTML、1 XML、12 PNG、18 JPEG；真实 11,189,470 字节 PDF 由 OneNote 2021 生成，未加密，
共 626 页 A4。Rust adapter 严格配对两份导出，生成一个 archive 候选和确定性 manifest；
隔离库与活动库均只保存 1 item、1 ready revision、2 个互补 export，随后重采 unchanged、
0 新 revision。它不把 PDF 页码伪造为平台 page/section ID，逐页切分留给 C1。

活动库由 `6 sources / 192 items / 195 revisions / 358 assets` 变为 `7 / 193 / 196 / 360`，
relation 仍为 1；OneNote 有 2 次 source observation。两份 C0 资产与 Recovery 原件的字节数和
SHA-256 完全一致；全库 schema v7、`quick_check=ok`、外键异常和所有
pending/quarantine/journal/orphan 为 0。`source.onenote` 已启用，但当时跨导出匹配、总 Skill
和受控 Agent 仍未完成。这里的跨导出匹配是来源身份缺口；可选 C1 切分是以后
独立消费 C0 的通用处理，不属于 OneNote 收集缺口或启用条件。开发证据位于
`BABATA_EVIDENCE_HOME/runs/p7-2-onenote-20260725-071439/`，不是 P8 备份。

同日 Issue #86 完成第三个切片：用户明确给出六个新的 OneNote MHT，并提示其中有子本单独
导出、一本内也有许多独立内容段。每个实际 MHT 各自作为完整 C0 保存，不在收集阶段拆分、
合并或猜测层级。Rust adapter 接受显式同目录绝对路径列表，验证单体 HTML 与 multipart MHT
的 OneNote 元数据、MIME 结构、可见文本和所有 hash；确定性重叠只作为机器未审阅、非确认
事实的 manifest 证据。

隔离库和活动库均为 6 ready items/revisions/assets，随后 6/6 unchanged、0 新 revision。
“灵感消化”与“猫与月季花”产生两条双向重叠提示，约 95.83%/100%，没有正式 relation；
其他四个无提示。活动库最终为 `7 sources / 199 items / 202 revisions / 366 assets / 1 relation /
342 observations`，schema v7、`quick_check=ok`、外键异常和所有
pending/quarantine/journal/orphan 为 0。开发证据位于
`BABATA_EVIDENCE_HOME/runs/p7-3-onenote-mht-20260725-090516/`，不是 P8 备份。当时来源侧仍缺
总 Skill 和受控 Agent；按需要进行的 C1 段落切分、语义去重和层级判断另行立项，不计作
本次 OneNote 收集的未完成部分。

同日 Issue #88 完成第四个切片，但不新增豆包专用 adapter。此前 Agent 已从复杂会话
“战略领导力W1”拿回并校验 7 个原始 DOCX，其中 1 个已正式登记；本切片只把剩余 6 个原件通过
通用 `capture attach-assets` 操作附加到既有 ready revision。隔离库与活动库都只变化
`assets`、`asset_attachment_operations`、`asset_attachment_members`；活动库保持
`7 sources / 199 items / 202 revisions / 1 relation / 342 observations`，资产变为 372，战略领导力W1
为 7 个 DOCX original 加 1 个 PDF preview。目标正文 hash、所有既有资产、C1 的 17 个
process runs/16 个 derivatives 均不变，schema v7、`quick_check=ok`、外键异常和所有
pending/quarantine/journal/orphan 为 0。证据位于
`BABATA_EVIDENCE_HOME/runs/p7-4-doubao-w1-assets-20260725-111326/`，不是 P8 备份。
这证明来源收集可在统一 C0 独立结束，不要求或触发 C1；`source.doubao` 仍保持 disabled，
直到可执行来源 recipe/受控 Agent 与其他真实附件形态通过自身验收。

对应底层能力通过自己的 AC/TC 后，P2 Skill 规格才转成真实 `SKILL.md`。Agent 默认
人工触发或确认，批处理携带明确范围，不自动扩张授权或把模型判断升级为事实。

Issue #90 完成第五个 P7 切片：先沿 00→05 权威链确定只有一个用户可见的
`babata-collect`，再实现其内部来源 recipe 和 Agent 引导。首版只把已有真实证据写成能力
边界：OneNote 与印象笔记可用；豆包保留 disabled；浏览器/UI 是执行工具而不是泛化来源。
同一来源新增内容形态以后扩展 recipe、能力声明和真实测试，不新增平台专用收集 Skill。

该切片按以下顺序交付：

1. 写清 Skill、route/recipe、adapter、case 的责任及 C0 终止边界；
2. 生成 `02_skills/babata-collect/`，按需加载来源 reference；
3. 以隔离、只读 forward tests 验证路由、范围、disabled/unknown 和无 C1 编排；
4. 运行既有 Rust、文档、架构、变异、数据根与敏感信息门禁；
5. Skill 格式、三个隔离只读 forward tests、确定性检查与三项负向变异，以及 Rust/文档/
   架构/数据根门禁全部通过；不提前宣称 P7、AC-09 或 TC-09 整体通过。

Issue #92 完成第六个 P7 切片：在唯一 `babata-collect` 内扩展豆包 recipe 和现有窄 adapter，
不创建平台专用 Skill，也不触发 C1。Agent 一次展开真实历史并排除用户指定的超大主会话，
40 个明确候选分成两个 20 条 Collector session；结果为 18+20 saved，2 条因
`HasMore=true` 在残缺 C0 写入前失败。38 个保存项逐 item 重采全部 `unchanged`、0 新 revision；
活动库从 199 items/202 revisions 增至 233/240，C1 仍为 17 runs/16 derivatives，schema v7、
quick check、外键及 pending/quarantine/journal/orphan 全部正常。新增 34 items 与 38 revisions
的差额来自 4 个已有 v1 item 首次迁移到 v2 稳定指纹的一次性归一化历史，不记作来源内容
变化。两批收集 408.53 秒、重采 458.27 秒，下一切片若提速只复用浏览器/捕获生命周期，
不合并逐会话 C0 事务或完整性判断。该切片启用 `source.doubao` 的正常文本批次；普通媒体/
附件形态、真实取消、多个外围入口对照和模型建议越权测试仍未完成，P7/AC-09/TC-09 不提前
宣告完成。

Issue #96 完成 P7.8 高难样本练手，但不重复写正式活动数据：在新的 acquisition 临时目录和
全新 `BABATA_DATA_HOME` 中，用登录态 Chrome 重新取得“战略领导力W1”的 16 条消息、
`has_more=false` 与 7 个 DOCX 元数据；原件经大小、MD5、SHA-256 和 DOCX 结构校验后通过
`--acquisition-handoff` 进入统一 Collector，结果为 1 saved item、1 ready revision、7 original
assets。刷新页面生成新的 handoff 后，typed recollection 为 `unchanged`、0 新 revision；缺附件
和篡改字节均在 C0 前失败，最终 metadata 无临时路径，C1 未启动。此切片把 Chrome 取得与
后续独立 select/recollect 接上，不把战略领导力W1再记作一次正式取回，也不继续围绕该样本扩展
范围。

Issue #98 随后用第五轮限时练习修正原件取得主链，不再次写正式 C0：OpenCLI 读取完整会话
并从 `content_type=20` 的消息内嵌 JSON 提取 7 个 DOCX 原始对象键；登录态页面一次调用
`get_file_url(type=file)` 直接换得 7 个 DOCX 签名地址。下载的 111,296,956 字节逐项通过
大小、MD5、历史 SHA-256 与 DOCX 结构校验，全程未进入云盘。已把该过程固化为
`babata-collect` 的确定性 acquisition 脚本，并修复顶层 `AttachmentKeyCount=0` 掩盖消息内
文件对象时的 fail-open 判断。消息卡片“查看”仍只产生 PDF preview；云盘下载保持独立备选。

2026-07-26，Issue #100 在 P7 中产出豆包 MBA 第一阶段的有界回收，范围按标题关键词覆盖组织行为学、
数据安全、战略领导力、财务管理、决策会计、商务沟通、营销管理和供应链。Recovery 审计确认
34/34 个对话消息链完整，共 220 条消息；对话声明的 219 个附件对象 key 已 219/219 覆盖，
缺失 0，去重并排除一条无效历史路径后为 192 个有效资产 key。战略领导力W1 延续此前已证明的
16 条消息与 7/7 个附件 key 完整链。权威审计位于
`${BABATA_RECOVERY_HOME}/mba-stage1-completion-audit.json`。这些数据先保留在 Recovery，
不自动写入 C0，也未触发 C1；该结果证明了更广的消息与附件形态，不把全账户豆包回收升级为
P7 的阶段门槛。

同日完成微信三阶段计划的第一阶段 Recovery。用户先用微信官方迁移补齐 PC 记录，再提供
WeChatDataAnalysis 1.18.5 从当前 Weixin 4.1.11.55 生成的最新账户整合归档。该归档为
4,892,484,282 字节，SHA-256 为
`1cbbcbd20d125685538dc5dbf7cf1bb81071a6d5a482df15ae934225bc729ff3`，含 24 个数据库和
26,213 个本地可得资源；Recovery 中以同卷硬链接保全，不重复占用一份空间。

第一阶段只选择文件传输助手和收藏：文件传输助手为 1,842 条唯一消息，时间覆盖
2021-01-11 至 2026-07-26，247 个媒体已打包；72 个缺失引用（71 个唯一）为 61 个文件、
6 个视频、5 个表情，最新整合归档同样没有这些原件，图片和语音缺失为 0。收藏原库
`quick_check=ok`，26 页可读 JSON 对账为 5,025/5,025 唯一 ID，覆盖 2013-10-14 至
2026-07-26；其中链接和视频号共 4,684 条，图片/视频/语音 206 条，文本/笔记/聊天记录
85 条，文件/商品/其他 50 条。权威审计位于
`${BABATA_RECOVERY_HOME}/batches/wechat/20260726-stage1-filehelper-favorites/stage1-audit.json`。
本阶段只证明“拿回并校验”，没有运行 Rust C0，也没有触发 C1；`source.wechat_chats` 和
`source.wechat_favorites` 因尚无正式 adapter/recollection 证据继续保持 disabled。

至此 P7 的豆包回收切片收尾，微信第一阶段也已形成 Recovery 实证。P7 下一步回到
`babata-collect` 的正式来源能力证明：可把微信 Recovery 接入 Rust Collector 并证明重采，
或选择另一个仍 disabled 的点名来源，
完成新的来源/内容形态、可重采边界和受控 Agent 证据，再收敛 AC-09/TC-09。P7 不再继续豆包
全量搬运；后续切片同时补齐真实取消、越权拒绝和外围入口对照。抖音和视频号保持延期，
浏览器书签仍排在最后。

Issue #104 随后把真实回收中已经形成的口径正式写回权威链：拿回资料优先于登记；C0-A1 不等待
解析或 SQL，C0-A2 只补齐当前内容所需正文/媒体/附件，C0-A3+ 不作为默认爬取；prepared 处理不
覆盖原始归档。C1/C2 若发现引用缺口，只能提出新的授权取得任务并形成独立记录或关系，不能
直接改写旧 C0。该定义更新不改变本节任何历史计数或阶段结论。

Issue #101 的 P7 收尾又把本地零散资料接入同一 Collector。用户最终决定紧急路径直接复制，
不在热路径按大小、抽样或哈希去重；`source.local_files` 因此默认 `opaque_copy`，每个选择文件
形成独立 `size_snapshot_v1` 资产，`sha256` 留空，强 SHA-256 仅为显式模式。真实 `omba25`
批次 247 个文件、14,728,536,423 字节在 66.961 秒内 247/247 saved，随后 247/247
`unchanged`、0 新 revision；数据库 `quick_check=ok`、外键违规 0。该批次不启动 C1。

同一收尾批次把微信第一阶段代表样本从 Recovery 接入正式 C0：13 个 prepared handoff 中
12 个保存并重采 `unchanged`，其中收藏 7、文件传输助手 5；`favorite:5008` 因同一 revision
内五个异名同字节媒体触发唯一性冲突，第一次失败后只重试一次并最终 skipped。微信来源固定为
解密数据库/Recovery；除非用户明确要求，Agent 不操作微信 UI。两个长豆包对话
`38414453372140034`（60 条消息）与 `38435487680201730`（25 条消息）也已 typed 登记并分别
重采 `unchanged`，两者均声明 0 个附件。

此前计划单独演练的运行中取消、模型建议越权和多个外围入口逐一对照，按用户实际工作方式
收敛为受控执行合同：明确范围后优先跑完整轮，单项最多两次，第二次失败即跳过，零星难例
最后处理；任何外围入口都不得扩张范围、另写最终资产或把模型判断升级为事实。上述真实批次、
确定性门禁和统一 writer 证据已经覆盖该合同，不再制造脱离真实工作的专项演练。

至此 AC-09、TC-09 通过，P7 完成。未取得内容与 disabled 来源继续诚实报告；全账户回收、
微信第二/三阶段和后续新增来源转入 P8 或运营，不倒灌为 P7 阻塞项。

## 11. P8：来源回收闭合

P8 只处理来源取得范围，不把备份恢复或运维扩张混进来源阶段。

### 11.1 P8.1：每个计划内来源至少 5 条 C0-A1

先从活动数据库、Recovery 和真实来源证据读取现有数量；达到 5 条的直接通过，不降级、不
重跑，不足的只补到 5 条。优先让所有来源跑一轮，单项最多两次；同一通用问题在 3 个不同
对象出现时暂停汇报。P8.1 只要求 C0-A1，不触发 C0-A2、C0-B、C0-C 或 C1。OneNote、
印象笔记和微信按已经证明的全量状态直接计入，抖音和视频号为 non-plan。

2026-07-27，Issue #114 纠正了把 locator-only 候选当作 C0-A1 的错误。紧急收集把飞书、语雀、
Kimi、Bilibili、小红书和知乎补到 5 条实际取得响应；Chrome 近期网页、微信公众号与其他
既有全量来源直接按已取得数量计入。2026-07-28，外部 Chrome 恢复后取得 4 条不同 ChatGPT
会话的完整响应，使 ChatGPT 从 1 条补到 5 条。15 个计划内来源现已全部通过，P8.1 完成；
P8.2 已启动并完成豆包第二/第三阶段、微信第二阶段和第三阶段其他范围，群聊按用户优先级暂缓；
P8.3 已由 Issue #124 启动，当前全量盘点见 11.3。

### 11.2 P8.2：豆包与微信第二、三阶段回收

P8 承担豆包全账户回收与运营闭合，不再把它当成 P7 的新来源能力证明：

1. 第二阶段先枚举登录账户的完整历史，排除主对话 `23419482725122`，并扣除 MBA 第一阶段及
   已有 Recovery/C0 对话 ID；其余非主对话优先全跑一轮，失败项只重试一次。简要记录共性
   失败；同一通用问题若出现在 3 个不同对话，立即暂停并汇报，残余难例再由 Agent 逐个处理。
   每个对话以 ID 对应、消息 ID 去重、末页 `HasMore=false` 和所有嵌套声明附件的逐项清单为
   回收终点；原件记录 size/hash/结构，preview 不冒充 original。之后再从容决定 Recovery 到
   C0 的正式登记批次。
2. 第三阶段按官方 cursor/page 分批推进主对话，不追求单次跑完；同时收回第二阶段跳过、
   超时和超长的零星对话。最终核对唯一对话 ID、消息链 `HasMore=false`、附件声明与原件覆盖，
   保留逐段 cursor、响应 hash 和累计唯一消息数；在不触发 C1 的前提下完成 Recovery 到 C0
   的对账。Recovery 原始响应、完整 JSON、附件 manifest 和尝试 ledger 是回收证据，不冒充
   P8 的正式备份；后续 P9 只把已对账数据纳入选定的简单复制或同步目标，不由 P8.2 扩张
   备份工程。

2026-07-28，Issue #116 完成豆包第二阶段。登录账户历史经外部 Chrome 侧栏完整枚举为 726 个
唯一会话；排除主对话，并扣除 315 个已有 C0 ID 与 34 个 MBA 第一阶段 ID 后，去重并集为
348 个，第二阶段实际范围为 377 个非主对话。377/377 均完成首轮尝试，7 个捕获失败项按合同
各重试一次后全部成功，最终命令失败为 0。340 个会话完整结束于 `HasMore=false`，共取得
1,667 条唯一消息，对话 ID 错配与消息 ID 重复会话均为 0；37 个 `HasMore=true` 长会话已带
当前响应、cursor 与 hash 明确转入豆包第三阶段，不记作丢失或第二阶段命令失败。

340 个完整会话声明 54 个唯一资产，其中 46 个原件成功取得，共 220,445,144 字节。8 个 JPEG
候选字节与声明 MD5 不一致，另有 1 个属于长会话的 PDF 在两次来源尝试后仍无匹配原件；这
9 个残余连同候选字节、实际 hash、结构和尝试次数均已进入第三阶段清单，其中 3 个是仅剩
附件问题的会话。权威批次位于
`BABATA_RECOVERY_HOME/batches/doubao/p8-2-stage2-20260728/`，以
`stage2-completion-audit.json` 和 `stage3-residuals.json` 分别证明第二阶段闭合与第三阶段边界。
2026-07-28，Issue #118 完成豆包第三阶段。主对话 `23419482725122` 与第二阶段转入的 37 个
长会话全部沿登录 Chrome 的官方 `chain/single` 分页闭合；请求从 `anchor_index=0` 开始，逐页
使用响应 `next_index`，服务端实际每页最多返回 50 条。38/38 个会话最终均为
`HasMore=false`，合计 156 页、5,887 条跨会话唯一消息，跨会话重复 ID 为 0。两个旧滚动输出
存在 3 条和 6 条分页重叠，均已显式去重且唯一计数与输出账一致；直接分页页的原始响应、
SHA-256、anchor 与 next_index 均逐页保存并复核通过。

第二阶段留下的 9 个附件残余也已逐项闭账：8 个候选实际是官方 `image_ori` 转码得到的
全分辨率 PNG 派生物，结构可打开但 MD5 不等于上传原件，继续保留为 derivative，不冒充
original；唯一 PDF 从完整消息补出真实存储 URI 与 20,556,280 字节声明，但签名已过期，
两次来源尝试后仍无原件。这 9 项按合同诚实列为 `unrecovered-after-two-source-attempts`。
权威批次位于 `BABATA_RECOVERY_HOME/batches/doubao/p8-2-stage3-20260728/`，其中
`stage3-completion-audit.json` 状态为 complete，并明确 Recovery 已闭合、正式 C0 未由本 Issue
启动、C1 未触发。豆包第二/第三阶段均已完成；P8.2 仍进行中，只剩微信第二/第三阶段。

微信沿用第一阶段已经校验的账户整合归档和 WeChatDataAnalysis 路线，不再重复研究采集工具：

1. 第二阶段处理单聊和私聊。先按会话类型排除群聊、公众号/服务号和其他系统入口，对正文、
   时间、参与者、附件声明做粗粒度筛选，明显无价值或纯通知噪声不进入 Babata；保留项先完成
   Recovery 对账，再批量交统一 Rust Collector。第一阶段的文件传输助手和收藏从候选中扣除。
2. 第三阶段处理群聊和其他剩余内容。以全量会话为底，按用户之后给出的去噪导出/规则做完整
   筛选、消息 ID 去重、重复媒体归并和噪声移除；大群不得因消息量大直接冒充高价值。最终逐
   会话核对消息数、时间边界、附件声明与可得原件，保留缺失清单，再进入统一 Rust Collector。
3. 两阶段均优先完整跑一轮，单项失败只重试一次；零星难例最后由 Agent 处理。Recovery、
   C0 和 C1 状态继续分开报告，筛选/去噪不回写或覆盖上游账户整合归档。

2026-07-29，Issue #120 完成微信第二阶段。全量 746 个会话按最新 `contact.db`、`biz_info`、
会话类型和消息来源库分为 498 个真人单聊候选、132 个群聊、115 个公众号/服务号和 1 个
第一阶段文件传输助手；后 248 个对象不进入本阶段。498 个候选中，482 个保留实质内容，
16 个仅含好友验证或系统提示而排除；共保留 115,261 条唯一消息，移除 1,394 条纯通知噪声，
消息 ID 重复为 0。每个保留会话均形成含完整过滤消息、可读 transcript、时间/参与者、媒体
manifest 和缺失清单的独立 Recovery 包。

全聊天导出已有媒体与最新账户整合归档的第二次精确资源 ID 补取合并后，共有 12,436 个可得
媒体包内条目、4,123,754,036 字节；第二次来源尝试补回 8,688 个原先缺失的资源声明。最终
仍缺 544 项，分为表情 234、视频原件 219、文件 91，影响 119 个会话，全部诚实保留为
`unrecovered_after_two_source_attempts`。482 个会话包共 4,149,726,759 字节，随后分 17 个
session 交统一 Rust Collector，最终 482/482 `saved`、正式项/修订/资产各 482、首轮失败 0，
SQLite `quick_check=ok`，且未触发 C1。首次 handoff 把会话误声明为 Collector 不接受的 `text`，
在同一通用问题出现后暂停并记录 17 个失败 session；仅把内容类型纠正为 `document` 后新建
session 登记，Recovery 包和源数据未重抓或改写。权威批次位于
`BABATA_RECOVERY_HOME/batches/wechat/p8-2-stage2-20260729/`，以
`stage2-completion-audit.json` 和 `unrecovered-media-summary.json` 分别证明闭环与实际缺口。
截至 2026-07-29，微信第二阶段完成；P8.2 仍进行中，只剩微信第三阶段。

2026-07-30，Issue #122 按用户最新决定暂缓 132 个群聊、97,232 条消息，先完成第三阶段其他
范围。115 个公众号/服务号会话全部筛完：111 个有内容会话保留 721 条消息，4 个空会话排除，
仅移除 1 条系统消息，重复消息 ID 为 0。逐会话包取得 101 个头像预览、490,897 字节，最终
声明媒体缺失为 0；111 个自包含会话包共 1,124,131 字节，分 4 个 session 交统一 Rust
Collector，首轮 111/111 `saved`、失败和重试均为 0，C1 未触发。权威批次位于
`BABATA_RECOVERY_HOME/batches/wechat/p8-2-stage3-other-20260730/`，以
`stage3-other-completion-audit.json`、`conversation-dispositions.json` 和
`deferred-groups.json` 分别证明正式登记、115/115 处置与群聊暂缓边界。微信第三阶段其他范围
完成；群聊暂缓且不冒充完成，P8.2 仍进行中。

### 11.3 P8.3：其余计划内来源全量 C0-A1

P8.3 推进点名来源按用户给定范围的全量主权回收，目标是每个计划内来源的全量范围至少达到 C0-A1；
具体来源按已验证官方导出/API/Agent 路线批量执行。抖音和视频号为 non-plan，不计入范围、
完成率或阻塞项，只有用户重新规划才恢复。当前实际状态按最高已证范围报告：用户交付的
OneNote 与印象笔记全量导出均为 registered/C0-C；微信全量为 C0-A1，并有 12 条代表样本为
registered/C0-C。P8.1 的 5 条最低覆盖不替代 P8.3 的全量范围。

2026-07-30，Issue #124 建立 P8.3 逐来源全量现状账。权威 Recovery 位于
`BABATA_RECOVERY_HOME/batches/p8-3/20260730-full-a1/`，主账为 `inventory.json`；Recovery、
正式 C0 和 C1 分开报告，本轮没有启动 A2、B、C 或 C1。当前结果如下：

| 来源 | 用户给定全量范围 | 已有 C0-A1 或更高 | 状态 | 实际缺口 | 正常获取路线 |
| --- | --- | --- | --- | --- | --- |
| 飞书文档、Wiki、知识库 | 当前账号 Wiki 全树及 Drive 根目录 | Wiki 151 个节点（142 docx、6 doc、3 file） | 部分完成 | `space:document:retrieve` 增量授权成功；Drive 根目录两次请求均在本地 `--params` 校验失败，未到 API，按两次上限停止 | 官方 `lark-cli` Wiki/Drive API |
| 语雀 | 3 个个人知识库、邀请协作库、全部收藏记录 | 8/8 个人文档；协作库 0；13/13 收藏记录 | 用户范围完成 | 0；收藏按 A1 保存记录，不递归展开 11 个外部知识库内容 | 登录 Chrome 枚举；官方 Markdown 端点 |
| 豆包 | P8.2 用户给定第二、三阶段范围 | 第二阶段 377 个会话首轮；第三阶段 38/38 到 `HasMore=false` | 用户范围完成 | 8 个图片上传原件、1 个 PDF 原件按用户决定留缺，不重跑 | 已有 Recovery 分页响应；正式 DB 317 items 另报 |
| Kimi | 当前历史目录 15 个会话 | 15/15（既有 4，P8.3 新增 11） | 用户范围完成 | 0；另 1 个 P4 旧会话已不在当前目录 | 登录态历史与结构化会话响应 |
| ChatGPT | 当前历史目录 28 个会话 | 28/28（既有 5，P8.3 新增 23） | 用户范围完成 | 0；未触发账号级 Data Export | 登录态历史与会话响应；必要时官方 Data Export |
| Bilibili | 33 个收藏夹全部收藏关系 | 3,167 条收藏关系、3,116 个唯一 URL | `deferred_by_user` | 用户后来决定整体暂缓；既有 Recovery 保留但不再作为主动缺口或完成分母 | 当前不执行；只有用户重新启用才恢复既有路线 |
| 小红书 | 收藏页 386 篇笔记、16 个专辑、0 个文件 | 20 条可读笔记卡片；专辑只取得分母 | 部分完成 | 至少缺 366 篇笔记；16 个专辑内容未枚举；两次浏览器控制超时后停止 | 登录 Chrome/OpenCLI |
| 知乎 | 16 个收藏夹、声明 98 条 | 65 条完整收藏关系 | 部分完成 | 5 个收藏夹共缺 33；3 个失败对象均完成最后一次重试 | 登录态 OpenCLI 手动分页 |
| 浏览器书签/网页 | 当前浏览器用户给定书签范围 | 后发现 UC Browser 2019 备份：1,560 条 HTTP(S) 书签、47 个文件夹；其中 5 条已到 A2 | 部分完成 | 历史备份不能冒充当前全部浏览器书签；当前全量分母仍未知，新增抓取已暂停 | 复用已有 Netscape HTML 只读解析；不主动要求新导出 |
| OneNote | 用户交付的 1 对 PDF/MHT 与 6 个 MHT | 7/7 registered/C0-C | 用户范围完成 | 0 | 官方桌面 PDF/MHT 导出 |
| 印象笔记 / Evernote | 用户交付的整库 `.notes` | 164/164 registered/C0-C | 用户范围完成 | 0 | 官方整库 `.notes` 导出 |
| 微信收藏 | 第一阶段全类型收藏 | 5,025 条 Recovery A1；7 条代表样本 registered/C0-C | A1 用户范围完成 | A1 缺口 0 | 只读 DB、ZIP 与 Recovery；不操作微信 UI |
| 微信公众号文章 | 第一阶段全量目录 | 233 条目录 A1；1 篇正文 registered/C0-C | A1 用户范围完成 | A1 缺口 0 | 公开 URL 与只读 Recovery；不操作微信 UI |
| 微信聊天 | 第一阶段全量 746 会话、216,449 消息 | 746/746 会话、216,449 消息 A1 | A1 用户范围完成 | A1 缺口 0；132 个群聊/97,232 消息的 P8.2 后续处理继续暂缓 | 只读 DB、ZIP 与 Recovery；不操作微信 UI |
| 本地文件和第一方资料 | 现有用户给定范围 | 499 个正式 items | 用户范围完成 | 0 | 本地复制与 first-party 核心提交 |

抖音和视频号继续为 non-plan，不计入完成率。Bilibili 后来由用户整体暂缓，不再作为主动
缺口；知乎和小红书的差额均有可读收藏夹、标题或页面证据，不只保存 hash/ID；飞书 Drive
与浏览器书签也保留失败文件和正常恢复路线。2026-08-01 用户进一步决定暂停全部新增抓取，
因此 Issue #124 保持 OPEN 并诚实保留缺口，但已耗尽路线不重试，也不由后续样本结果覆盖。

P8 后续只在用户重新启用时继续 P8.3 抓取；当前使用已经完成的真实范围推进存量准备与
AC-11、TC-11 的本地 raw-to-view 验证，暂停范围继续保持缺口而不冒充完成。

### 11.4 P8.4：每个纳入来源至少 5 条 C0-A2

2026-07-31，Issue #126 完成 P8.4。用户把 Bilibili 整体标记为暂缓，它不进入目标、分母
或完成率；抖音和视频号继续沿用 non-plan。其余 14 个纳入来源均从现有正式数据库、Recovery
或来源证据中选出至少 5 条真实对象达到 C0-A2 或更高，最终为 14/14 来源、70 条样本、0 个
待补来源。权威批次位于
`BABATA_RECOVERY_HOME/batches/p8-4/20260730-full-a2/`，总账为
`minimum-five-a2-audit.json`，逐条标题、依赖数量、字节、状态、正常路线和证据路径均可回读。

12 个来源直接从已核验素材达到 5 条；最后两个实际缺口按范围修正后闭合：

- 浏览器只统计收藏夹/书签。UC Browser 的 Netscape 书签备份经 SHA-256 固化并解析为
  1,560 条书签、47 个文件夹；从 `Coding/Java` 和 `Coding/C#` 选择 5 篇仍可访问、正文图片
  依赖为 0 的博客园文章，由 Chrome 可见 DOM 保存完整正文文本和 HTML。历史记录与 6 个
  普通打开页继续保留为范围外证据；一篇超过浏览器 HTML 返回上限的长文明确不计入。
- 微信收藏只统计收藏中的公众号文章正文及正文图片。原有 3 条完整；`5016`“室内光影感
  照片这样拍真的很出片呀”和 `5020`“我们差距就是 我的尺码你穿不上”复用 Chrome 已加载
  页面资源各取得 1 张正文图后达到 5/5。旧登录 token 的持久站点会话仍只导出标题/赞赏空壳，
  因此正常回退方法明确为 Chrome 可见页面资源归档；头像、水印重复图和 UI 资源不计正文。

P8.4 只形成 Recovery/captured 样本和既有 registered/C0-C 的准入说明，没有把 Recovery
冒充正式 C0，也没有启动新的 C0-B、C0-C、C1 或 C0-A3+。P8.2 的 132 个微信群聊仍按用户
决定暂缓，P8.3 的全量 A1 真实缺口仍由其自身状态表管理；二者不因 P8.4 样本完成而被改写。

### 11.5 P8.5：存量 A3 判断与 C0-B 准备

2026-08-01，Issue #128 按用户最新决定暂停新增抓取，只处理 P8.4 的 70 条既有 A2 或更高
样本。逐项 A3 判断结果为 70/70 已评估、0 条当前需要 A3：所有正文和直接依赖已满足当前
A2 范围，且没有单独语义引用同时具备明确价值、授权和停止边界；引用继续原样保留，未来
C1/C2 可另提独立 A3+ 请求，但 A3 不阻塞 B/C。

70 条中 25 条本来就是 registered/C0-C，保持原状态且不倒退。其余 45 条来自飞书、语雀、
Kimi、ChatGPT、小红书、知乎、浏览器书签、微信收藏公众号文章和微信公众号文章，各 5 条，
均形成 prepared/C0-B。权威批次位于
`BABATA_RECOVERY_HOME/batches/p8-5/20260801-stock-c0-b/`；`c0-b-manifest.json` 为总账，
`a3-necessity-assessment.json` 为 A3 逐项账，`items/` 保存 45 个准备 manifest。独立复核重新
读取 549 个现有上游文件、283,283,690 字节，文件存在、大小和 SHA-256 错误均为 0。

准备只保存稳定身份、内容形态、P8.4 原字段、路径、字节、hash 和限制，不覆盖、重命名或
原地规范化上游 Recovery。九个来源的 runtime route 仍 disabled，因此本轮停在 prepared，
没有通过外围脚本写 SQLite 或 managed assets，没有新增 registered/C0-C，也未触发 C1。
该完成范围仅是 P8.4 的每来源 5 条样本，不代表全来源 A2/B，亦不关闭 P8.3 的全量 A1 缺口。

## 12. P9：简单备份同步

P9 只从 NAS、云盘或云 Git 中选一个可用目标，对需要保护的本地结果做纯复制或同步。Issue
#112 / PR #113 已经实现并实证本地一致快照、加密 restic 备份和隔离恢复：两次快照各覆盖
1,281 个文件、21,310,423,952 字节，第二次只新增 2,467,774 字节，隔离恢复与篡改拒绝均已
通过，证据仍位于 `BABATA_EVIDENCE_HOME/runs/p8-1-backup-20260727-214924/`。这套能力保留为
P9 可复用前置，但不代表 P8.1；P9 只差选定并跑通一个外部复制/同步目标，不扩张日志轮转、
成本监控、复杂恢复平台或其他运维体系。

## 13. 阶段与验收映射

| 阶段 | 主要产品验收 | 说明 |
| --- | --- | --- |
| P2 | 无产品 AC；P2-G1..G7 | 工程骨架与现有工具路线门，不冒充产品完成 |
| P3 | AC-03/06/10 的底座部分 | C0、first-party、单一写入；无真实来源/清洗/恢复 |
| P4 | AC-01、AC-02 | 首批真实上下文收集 |
| P5 | AC-03（C0/C1 子责任）、AC-04 | 真实输入/派生物与忠实清洗；TC-03A/TC-04 |
| P6 | AC-03（C2 子责任）、AC-05、AC-06、AC-07、AC-08 | TC-03B；核心先于检索/视图/输出 |
| P7 | AC-09 | 扩展来源、Skill、Agent |
| P8 | AC-09（P8.1 最低覆盖）、AC-11 | P8.1 已完成 15/15；P8.2 已完成除 132 个暂缓群聊外的用户范围；P8.3 暂停新增抓取并保留真实缺口；P8.4 已完成 14/14、70 条 A2 样本；P8.5 已判定 70/70 条当前无需 A3，并把 45 条 captured A2 推进到 prepared/C0-B，另 25 条保持 C0-C |
| P9 | AC-10、TC-10 加一个外部复制/同步目标 | 本地备份恢复能力已存在；NAS/云盘/云 Git 三选一尚未执行 |

## 14. 提交与验收纪律

Babata 即使由单人开发，也使用 GitHub Issue 和 Pull Request 保留问题、范围、决策、验证
与合并记录。`main` 是可集成基线，不作为日常直接开发分支。

### 14.1 标准工作流

1. 先建立 Issue，写清背景、范围、非目标、验收条件和对应 phase gate/AC/TC；调查任务
   也要在 Issue 中写明要取得的证据，不能只有一个模糊标题。
2. 从最新 `main` 建立短生命周期分支，名称包含 Issue 编号和主题，例如
   `codex/issue-12-browser-probe`。一个分支服务一个可审阅目标，不混入无关改动。
3. 在 Issue 的明确范围内先批量推进相关工作，再在逻辑边界快速收敛。按可恢复、可审阅的
   安全点 commit，提交说明具体结果；不要求每个小功能单独走一次全量门禁。
4. 推送分支并建立 PR。PR 必须引用 Issue，使用 `Closes #N` 或 `Fixes #N`，并写明变更、
   验证、风险、数据/凭据影响、文档影响和未完成项；草稿未完成时使用 Draft PR。
5. PR 中审阅实际 diff，完成适用的文档追溯、编译、测试和边界检查。检查失败、验收证据
   不足、混入真实数据/凭据或范围漂移时不得合并。
6. 检查和审阅结论成立后合并到 `main`，删除工作分支，由 PR 自动关闭 Issue。合并后才
   更新后续 Issue；不得用直接推送 `main` 绕过记录。

### 14.2 执行与验证节奏

- 开始一个批次前明确授权范围、退出条件和最终验收，不把已清楚的工作逐步交回用户确认；
- 先展开同范围内可并行或同构的资料/功能，再集中处理共性问题、补证据和收口；
- 快速局部检查跟随高风险变化、共享契约和故障修复；普通编辑和窄功能在批次边界合并检查；
- 全量门禁通常每天集中两到三次，并在 PR 合并和阶段验收前执行。若批次短于一天，以批次
  收口和合并前门禁为准，不为了凑次数重复运行；
- 一旦出现可能污染真实数据、破坏权威边界或说明共性假设错误的失败，立即停止扩散并验证，
  不能用“集中检查”拖延已知风险。

### 14.3 纪律

- 小型文档修正、研究结论、依赖升级和紧急修复同样走 Issue/PR；真正需要立即止损的
  紧急修复可以先开短 Issue 和最小 PR，但不能事后没有记录。
- P3 以后每项功能的 Issue、commit 和 PR 引用对应 phase gate 与 AC/TC。
- 功能阶段发现接口不对，先在同一 Issue/PR 更新 03 架构补充与 P2 蓝图，再改代码。
- 真实数据、授权信息、数据库、模型输出和日志不进入 Git 或 Issue/PR 附件。
- 未通过当前阶段门，不提前激活或宣告下一阶段。
- 工作树中已有用户改动默认保留；不 reset、checkout、删除或盲目提交。

# Babata 测试用例

## 1. 测试职责

本文证明 `02_ACCEPTANCE_CRITERIA.md` 的可观察结果。测试可以使用 unit、contract、
integration、end-to-end、人工验证和真实授权样本，但不能用架构文件数量代替产品场景。

测试证据分为：

- **合成/夹具证据**：证明序列化、解析、状态、边界、失败和确定性机制；
- **真实授权证据**：证明来源、附件、上下文、权限和正常交互路径实际可用；
- **工程 gate**：证明文件、依赖、所有权和单一写入边界；不等同于产品 AC。

真实资料、token、数据库、模型输出和日志保留在外部数据根或受保护测试位置，不进入
Git。所有 destructive 测试使用临时或隔离数据根。

## 2. 产品测试用例

### TC-01：来源上下文候选与选择性收集

关联：AC-01，阶段 P4。

场景：分别使用一个用户授权的飞书来源，以及 Codex 直接控制的已登录正式 Chrome 中
真实 Kimi/点名平台会话；长期扩展的页面/选区/书签路径另作独立来源对照，不能用它
替代 Kimi 或其他点名平台。若回退 OpenCLI，记录浏览器真实失败或任务外重试需求。

步骤：

1. 连接来源；用户只给一次飞书节点范围和一个 Kimi 会话/时间/数量范围；页面/书签
   对照另给一次明确范围。
2. 让 Agent 自主导航、翻页、发现候选和下载，不逐条让用户代操作；同时查看候选标题、
   位置/层级、类型、更新时间、附件可得性和限制。
3. 分别测试未给范围、取消、范围有歧义、明确单条、明确集合和明确批量范围。
4. 对一个需要登录/MFA、一个不完整/无权限候选和一个只支持回退导出的来源操作。
5. 检查候选发现前后、一次确认后连续运行期间以及越界尝试前后的 C0 变化。
6. 对 Bilibili 在 20 个真实历史候选中只选择一个视频，检查正文、可得字幕/摘要和用户
   明确要求的视频附件均通过同一 C0 链路保存，未选择的 19 条不写入。
7. 对飞书从一个真实 Wiki 空间下钻到明确父节点，选择一篇含图片的 Docx；验证真实
   `src/href` XML、正文和用户要求的媒体都经同一 CaptureService 写入，不把目录页或
   手工导出件冒充正常路径。
8. 对 ChatGPT 先用正式 Chrome 展开真实最近聊天并读取所选会话，再让 Babata 在
   `recent:20` 的 20 个候选中只收集“开源部署方案对比”；验证 2 条角色消息、10 个引用、
   0 个真实附件进入同一 C0，页面引用 favicon 不计作附件。
9. 对知乎先用正式 Chrome 读取 16 个自建收藏夹和最新收藏夹第一页；记录页面标称 28 条、
   官方分页实际返回 27 个去重候选，只选择最新回答。验证完整正文、原始 HTML 和 17 张
   正文原图进入同一 C0，作者头像排除，未选择的 26 条不写入。
10. 对语雀先在正式 Chrome 核对官方整库与单篇导出，再从 8 个最近文档中只选择一篇；
    验证免费官方 Markdown、渲染正文/HTML 和 22 张图片进入同一 C0。OpenAPI/MCP 的会员
    Token 不作为当前前置，也不要求用户手抄会话 Token。
11. 浏览器书签排到所有点名来源之后，最后单独验收；用户只给一次明确文件夹/集合范围，
    Agent 自动遍历其中网址，
    取得正文和可得附件，并确认未选范围不写 C0。实验性手动当前页/选区/locator-only
    书签扩展只记冻结机制证据，不作为 P4 或书签来源闭环。
12. 为同一真实范围列出可用路线并按八级顺序选择；分别模拟官方免费批量导出可用、只有
    收费 VIP 可完整覆盖、以及只能人机配合/手工恢复的情况。检查使用较低级路线时是否
    记录更高级路线不可用原因、用户动作和信息损失；未经授权不购买或启用收费能力。
13. 让 Agent 在没有平台专用适配器的情况下完成一个真实明确范围并校验正文/附件；确认
    该次结果可标为 Agent 收集完成。随后模拟一次性和重复执行需求，确认前者不创建开发
    前置，后者先验证 Skill/薄调用，只有复用暴露真实缺口时才进入开发路线。
14. 模拟平台将在一小时内不可访问，同时让解析、C0-A2 递归和 Rust 登记不可用；确认 Agent
    仍先完成 C0-A1，把直接可得字节/响应/导出、来源、哈希和限制保存到本地，并准确报告为
    captured 而非正式 C0。
15. 对一篇正文含内嵌图片、必要附件和外部引用文章的页面执行取得；确认正文、内嵌图片和
    必要附件属于 C0-A2，取得失败时逐项记缺口；外部引用默认只记 reference，只有显式授权后
    才以有界 C0-A3+ 形成独立 node/reference，且不会继续无限追取下一层引用。

预期：

- 候选字段真实可得，缺失与限制明确；
- 未给范围、取消和实质歧义不写 C0，不发生账号级静默全量复制；
- 给出一次明确范围后，Agent 自主完成范围内导航、分页、下载、重试和提交，不反复要求
  用户确认；只有无法替代的登录/授权或真实阻塞才暂停；
- 正常路径不要求手填导出路径、内部 metadata 或候选 JSON；
- 实际路线遵循八级优先级；降级理由可见，收费能力不被自动购买或启用；
- 回退路径被明确标识，未验证来源不显示 available。
- P4 只以飞书和至少一个点名网页登录平台的代表性真实路径验收，不要求 00 的 19 个来源
  逐一跑通；阶段完成不自动启用未验证来源。明确延期的抖音和视频号保持 disabled，既不
  作为 P4 阻塞项，也不作为 P4 完成证据。

### TC-02：逐条状态、局部失败、重试与重收集

关联：AC-02，阶段 P4。

场景：选择一个含成功、跳过和可重试失败项的集合；随后修改、保持、限制访问和删除
不同来源项后重收集。

步骤：

1. 观察每项从 queued 到 running 再到终态。
2. 对失败项重试，取消尚未开始的项。
3. 确认已成功项没有因局部失败或取消而丢失/重复覆盖。
4. 重收集并检查 changed、unchanged、inaccessible、removed。
5. 回看旧版本、旧上下文和本次检查记录。
6. 关闭并重新打开数据库，确认附件选择仍为 true；让播放/点赞等实时计数变化但正文、
   字幕和摘要不变，确认重采为 unchanged 且不增加版本或资产；再改变标题验证 changed。
7. 让飞书媒体结构首次解析失败，确认 item 为 retryable failed 且 C0 为 0；兼容真实
   `src/href` 后对原 item 定向 retry，确认 8 个下载件与 C0 资产逐个哈希一致；刷新临时
   href 后重采为 unchanged，版本和资产均不增加。
8. 让 ChatGPT OpenCLI 瞬时返回非 JSON，确认错误保留可读来源信息并按可重试 I/O 失败
   处理，不误报 C0 integrity 损坏；成功收集后重采为 unchanged，仍为 1 revision/0 assets。
9. 让同一张知乎原图在 `picx/pic1/pica` CDN 域间切换，确认稳定图片 token 不变时重采为
   unchanged，仍为 1 revision/17 assets；正文、更新时间或图片 token 改变时才追加版本。
10. 对语雀重复访问同一官方 Markdown 端点，确认 Markdown 和 22 个稳定媒体 token 未变时
    重采为 unchanged，仍为 1 revision/22 assets；临时下载目录重复产生不重复写入 C0。
11. 在第一项已经 running、其余两项仍 queued 时从并发调用取消：第一项允许保存，后两项
    转为 skipped，session 保持 cancelled；确认取消不会被收尾覆盖成 completed，C0 只有
    已开始成功项。
12. 对同一保存项依次制造 changed、unchanged、inaccessible、removed；逐项核对四条
    `collection_recollection_checks`、旧 revision 数量和新 revision 只在 changed 时增加。

预期：逐条状态与原因可见；重试只影响目标项；changed 追加版本；unchanged 保留检查
事件；inaccessible/removed 不删除旧 C0；局部成功始终保留；动态统计保留在原始响应中，
但不制造伪正文版本，unchanged 重采不重复写入同一附件。

### TC-03：C0、C1、C2 与第一方资料可辨别

关联：AC-03，分为 P3/P5 的 TC-03A 与 P6 的 TC-03B；两者都通过后 TC-03 才整体通过。

#### TC-03A：C0 与 C1 的真实溯源和隔离（P3/P5）

场景：收集文本、文档、图片、音频或视频，创建第一方资料，并从真实 C0 输入生成至少
一种 C1；文件派生样本必须绑定对应 asset，不能挂到人为说明文本。

步骤：

1. 记录 C0 原文/原件哈希与来源。
2. 对真实 PDF/图片/视频 asset 运行抽取、OCR 或转写；核对 revision、item、asset、输入
   哈希、pipeline、kind、provider、工具/模型版本、规范化链和输出哈希。
3. 从 C1 回到输入版本、asset、来源上下文和处理记录。
4. 删除并重建允许重建的 C1，确认 C0 内容、asset 与哈希不变。
5. 使用缺附件或缺来源字段的受限样本重复检查。
6. 使用同时提供上传 DOCX 与平台转换 PDF 的真实或等价样本，确认保存并标识 DOCX 为
   源文件，PDF 只作为派生物或回退证据；再模拟源文件不可取得并检查限制说明。
7. 只收集一批真实原件并保持零项新 C1，确认收集已正常结束；随后可选地从同一 C0
   revision/asset 启动通用清洗，确认 C1 引用该输入且来源收集记录、正文和附件不变化。
8. 分别对 C0-A1-only 和 C0-A2-complete 的 captured 上游归档请求解压、稳定命名、格式识别和
   manifest 生成；确认前者被 B/C 准入门拒绝，后者可形成 prepared/C0-B。逐文件比较前后
   字节与 hash，确认准备只新增结果，不覆盖、移动或规范化改写原件。
9. 分别提供 C0-A2+prepared 和 registered 样本；确认只有后者经过唯一 Rust writer，具有
   item/revision/asset/provenance/relation/status 并可回读，且界面只把后者称为正式 C0。
10. 从 C2 发现一个原始引用缺口；确认系统创建新的授权 acquisition 请求，取得结果形成
    独立记录或关系，原有 C0 内容、版本、hash 和关系均未被下游直接反写。

预期：C0 内容与哈希不变；外部原件、first-party 和机器派生物可辨别；C1 删除不会
损伤 C0；每个文件派生结果绑定真实 asset；不完整资料保持明确限制；平台预览/转码件
不冒充源文件；prepared 不覆盖 captured 原件；只有 registered/C0-C 称为正式 C0；C0 可在
没有 C1 时独立完成，C1 知道其 C0 输入但不形成来源专属管道或反向写入路径。

#### TC-03B：C2 视图与子库边界（P6）

场景：基于已通过 TC-03A 的 C0/C1 生成至少一种搜索投影、视图或子库物化。

步骤：

1. 从 C2 回到所读的 C0/C1、来源上下文和生成记录。
2. 在用户可见入口辨别外部原件、first-party、机器派生物和可重建视图。
3. 删除并重建 C2 搜索索引、视图或子库物化。
4. 核对 C2 builder 没有 C0/C1 反向写入路径。

预期：C2 可删除重建；C0/C1 内容、哈希与关系不变；视图不冒充原件、人工资料或机器
派生权威。TC-03B 不替代 TC-03A 的真实 asset 溯源证据。

执行状态（2026-07-23）：TC-03B 通过，因此与已通过的 TC-03A 合并后，TC-03 整体通过。
这批历史测试中的 C0 指 registered/C0-C；Issue #104 的定义澄清不撤销既有通过结论，新增
C0-A1/C0-A2/C0-A3+ 与 captured/prepared 场景留待相应取得与报告实现时补充执行证据。
P6.2 搜索投影和 P6.3 子库物化/输出均可从 manifest 回到 C0/C1 与生成记录；真实
machine/unreviewed Knowledge 在物化、Markdown 和 JSON 中仍明确为机器建议而非人工资料。
删除重建 C2 前后 46 张 raw/knowledge 表和 4 张 derived 表摘要不变，C2 ports 也没有
C0/C1 writer 依赖。该结论不替代 TC-03A 的真实 asset、源文件/预览和 C1 溯源证据。

### TC-04：忠实清洗、失败重试与百炼路径

关联：AC-04，阶段 P5。

场景：对文档/网页、图片、音频和视频中的已启用类型运行真实处理，其中至少一项经
百炼 CLI，至少一项故意失败后重试。

步骤：

1. 检查 input、pipeline、工具/模型、版本、运行状态和输出哈希。
2. 比较派生文本/结构与原件，记录视觉、时序、语气或版式损失。
3. 注入 provider 错误，重试并保留两次运行记录。
4. 删除一个可重建派生物后重新运行。
5. 对未启用处理类型发起请求。

预期：处理可检查、失败可理解、重试不改 C0；多次结果可并存/比较；原媒体可回看；
未启用能力返回 unavailable；百炼输出不被当作原件或人工事实。

2026-07-20，TC-04 在合并 `main` `0de2858` 上通过。真实微信 C0 同时完成 local extract
删除/重建与一次 `qwen-plus` 摘要；注入 provider 失败后新 job/run retry 成功，父子记录、
实际 task、1,739 tokens usage、输出哈希和 loss notes 可读；未启用 OCR queue 返回
unavailable；C0 正文和 asset 哈希前后不变。证据：
`BABATA_EVIDENCE_HOME/runs/p5-tc04-20260720-0015/TC04_PROVIDER_QUEUE_E2E.md`。

### TC-05：三大界、自动语义沉淀与模型建议边界

关联：AC-05，阶段 P6.1。

场景：一条外部资料及其 C1 在没有用户逐条回复的情况下继续由 Agent 消化，形成三大界中的
机器语义候选、关系、三维评分和高密度表达；随后再加入独立的评论、日志或感悟验证边界。

步骤：

1. 在同一审阅上下文校验原件、派生物、来源、必要历史和关系。
2. Agent 提出类型、地图归属、标签、关系、评分、高密度表达和依据齐全的候选；质量校验
   通过后以机器、未审阅身份进入核心，用户没有回复也继续执行。
3. 验证第一大界地图、第二大界 Knowledge/Case 和第三大界 Log/Insight 不被平铺混写；
   Knowledge 与 Case 双向互证，Log/Insight 可以引用第二大界内容。
4. 为一个节点和一项内容写入兴趣、战略、共识分量与依据，用默认 `40/35/25` profile
   计算综合分；创建新 profile 后回看两套权重和历史结果。
5. 为至少一项知识保存文本脑图、Mermaid、模型、公式、清单、流程或提纲，并生成后
   删除一个可选图片/视图渲染。
6. 保留一个模型建议为 unreviewed，同时分别接受、修改和拒绝其他建议；继续执行检索、
   关系导航、子库候选和输出候选。
7. 另建一条 first-party 评论或感悟并关联目标；触发一次 Agent 再分析，验证两者分别是
   独立 C0 和新的 C1 run/suggestion，而不是目标内容的 `v2`。

预期：三大界、三级地图、多重归属、五类语义、关系、评分/profile 和高密度文本均可追溯；
模型建议始终为 C1；审阅只追加状态标记，修改产生新内容时才创建适当类型的 first-party
C0；评论、感悟、日志和再分析不冒充目标修订；拒绝不删除建议；unreviewed 不阻塞下游
且不冒充人工；删除图片/视图不损伤文本表达；任何建议都不覆盖原件或唯一分类。

### TC-06：第一方新内容、关系与少数真实修订

关联：AC-06，阶段 P3/P6.1。

场景：新写一篇内容，对外部资料写一条批注，记录一条日志和一项感悟，增加附件/证据，
并只在确实改写同一篇作品时形成一次修订。

步骤：

1. 创建 first-party 内容并记录原始措辞和附件。
2. 创建独立 annotation，关联外部目标版本；另建 Log 和 Insight 并建立关系。
3. 为目标增加附件或证据，检查没有因此产生正文 `v2`。
4. 触发 Agent 基于更多资料重新分析，检查形成新 C1 而非 first-party 修订。
5. 对第一步的作品做一次明确改写，查看前后版本；删除并重建展示视图。

预期：新写、真实修订、批注、日志和感悟都是 C0；批注/日志/感悟是独立 item，附件和
证据按资产或关系保存，Agent 再分析是 C1；只有明确改写产生作品新版本，旧措辞和附件
不变；人工/模型可辨别；视图删除不影响任何必要历史和关系。

2026-07-20，Issue #59 首切片用真实微信 C0/C1 通过 `knowledge review` 的跨库引用与
active hash 校验。其 Knowledge create/revise/show 两版夹具后来被 Issue #63 判定为错误
产品主流程，不再作为 TC-05/TC-06 证据。自动语义消化、三大界、地图、关系、评分/profile、
建议审阅和上述事件边界仍待真实验证，因此 TC-05、TC-06 均未整体通过。

Issue #63 已验证纠偏 migration 不丢失潜在旧行、真实库业务表行数不变且 review 继续成功；
该证据只证明兼容迁移与审阅准备，不证明语义候选已经进入 P6 核心。

2026-07-21，Issue #65 已补上第一条真实自动语义证据。真实微信 C0/C1 在用户零回复的
情况下由 `qwen-plus` 形成 suggestion `suggestion_01KY2A6TKXYG1HF3NWRWB3JNSZ`，以
machine/unreviewed 身份进入核心；读回包含 Map/Direction、Knowledge、Case、动态学科/
分支、多重归属、标签、3 条关系、默认 `40/35/25` 评分与 Mermaid/流程/提纲，C1 derivative
ID/hash 和 evidence ID/hash 均可追溯，源 item 仍只有 1 个 revision。临时数据根另外覆盖
first-party Log/Insight、Knowledge/Case 双向关系、第二个 profile 和旧/新评分并存，以及
accepted/modified/rejected 追加审阅；workspace 测试继续覆盖独立 annotation、Agent C1
再分析和少数真实 revision 的边界。

同一纵向测试还锁定四个负向边界：重复 ingest 被拒绝且不产生第二套语义记录；总和不为
`100` 的 profile 被拒绝且不留下新配置；外部来源 C0 不能登记成 first-party Log；
Log/Insight JSON 正文与真实 first-party C0 原文不一致时拒绝写入。

因此 TC-05 的“真实自动消化进入核心”不再缺失，TC-06 的事件语义已有机制证据；但本轮没有
代替用户制造真实评论、Log、Insight 或审阅决定。

Issue #65 后续切片在同一临时 CLI 纵向测试中补齐学科/分支新增、双父级、改名、父级迁移、
标签增删、内容归属、节点评分、分支合并及全部历史读回；四基石修改被应用层和数据库层
拒绝。同一 C0 的第二次 Agent 结构化分析形成新的 C1 run/suggestion，源 item 始终只有
一个 revision。未审阅建议读回允许 search/surfacing/relation/sublibrary/output candidate，
同时明确 `human_judgment=false` 和 `confirmed_fact=false`；拒绝不删除历史。

高密度文本的窄 C2 Markdown 预览完成 build/verify、注入篡改拒绝、rebuild、delete 和再次
rebuild；删除后核心表达仍完整。真实库 knowledge `v3 -> v4` 前在线快照，迁移后原业务
行数不变、`quick_check=ok`、foreign key 异常为 0，并对已有机器 Knowledge 完成真实 C2
删除重建。证据：`BABATA_EVIDENCE_HOME/runs/p6-1-map-evolution-20260721-213222/`
`P6_1_MAP_EVOLUTION_E2E.md`。

最终数据库审查另覆盖一条 0004 未封死的负向路径：不能先在其他 map version 创建
foundation，再通过 UPDATE 搬入 P6 baseline。0005 保持 0004 不变并收紧该 trigger；临时
迁移测试与真实库 `v4 -> v5` 前快照、行数、checksum、`quick_check`、foreign key 复核共同
作为数据库级四基石保护证据。真实证据位于 `BABATA_EVIDENCE_HOME/runs/`
`p6-1-foundation-guard-20260721-220641/P6_1_FOUNDATION_GUARD_E2E.md`。

AC-06 反向审计没有沿用旧“补附件就复制正文 revision”的测试。新的应用与 CLI 测试确认：
补原件/预览返回原 revision ID、`reimported=false`、revision 数量保持 1，独立附件 operation
读回 reason、metadata、membership 和 ready 状态；finalise、哈希校验和 ready transition
三类失败只隔离附件 operation/assets，原 revision 仍为 ready；相同正文的 first-party revise
被拒绝，数据库拒绝跨 revision membership。真实 raw v4 -> v5 前在线快照，迁移后原有
`5 sources / 27 items / 30 revisions / 7 assets` 及知识业务行数不变，新表为空，checksum
匹配、`quick_check=ok`、foreign key 异常为 0。没有在真实库制造附件；证据位于
`BABATA_EVIDENCE_HOME/runs/p6-1-attachment-semantics-20260721-223511/`
`P6_1_ATTACHMENT_SEMANTICS_E2E.md`。

这批证据没有冒充真实用户评论、Log、Insight 或审阅决定；这些能力由 first-party C0 临时
纵向测试证明。PR #66 已通过全部仓库门禁并合并，因此 TC-05、TC-06 已整体通过，P6.1
已完成；P6.2/P6.3 的正式搜索、浮现、子库和输出验收不被提前宣称。

### P6.2 preflight：C0 公共 metadata 与来源观测

Issue #60 的测试是 AC-07/TC-07 前置机制证据，不是检索产品验收：

- domain serde/validation 覆盖 `babata.c0.common/v1`、`babata.c0.media/v1` 和缺少新字段的
  legacy CandidateEnvelope；
- raw v5 -> v6、collection v4 -> v5 临时迁移测试证明旧 item/revision/candidate 行保留，
  公共字段安全回填，重复迁移幂等，篡改 checksum fail-closed；
- collector integration 覆盖首次 capture、changed、unchanged、inaccessible、removed，
  核对 item 首次事实不变、每次 capture 有 observation、后三种结果不增加 revision，
  observation update/delete 被拒绝；
- 飞书 fixture 验证 typed hierarchy/limitations 与 UTC update time；知乎 detail fixture 验证
  author、Unix/RFC3339 created/updated 和未知 provider metadata 保留；微信 fixture 验证
  `6月2日 08:56` 不进入 UTC 字段且 limitation 可读；
- 主真实库在线快照后的 raw v5 -> v6、collection v4 -> v5 迁移保留原有
  `5 sources / 27 items / 30 revisions / 7 assets / 1 relation`，旧列逐表比较无业务差异，
  `quick_check=ok`、foreign key 异常为 0，迁移 checksum 与仓库一致；
- 隔离复制数据根执行真实 provider 验证：飞书、知乎、微信既有样本均为 unchanged 且各自
  revision 数不变，新增 typed recollection observation；fresh 飞书候选的 title、三级
  hierarchy、UTC updated time 和 limitation 同时到达 item 与 capture observation；知乎
  author、published/updated 与 17 个媒体条目可读；微信 raw 含糊时间原样保留且 UTC 为
  null。外部证据位于
  `BABATA_EVIDENCE_HOME/runs/p6-2-c0-metadata-20260721-003825/`。

这些前置测试当时没有构建搜索 projection，也没有执行正文/人物/时间多条件检索、评分排序、
关系导航或内容浮现，因此不能单独作为 AC-07、TC-07 或 P6.2 的通过证据；正式 P6.2 状态
见下方 TC-07 执行记录。

### TC-07：检索、关系导航与子库

关联：AC-07，阶段 P6.2/P6.3。

场景：准备含正文、媒体-only、附件-only、受限项、版本、地图归属、五类语义、三维评分、
人工/机器关系和未审阅候选的资料集，创建一个有纳入/排除规则的子库。

步骤：

1. 组合正文、来源、时间、语义类型、状态、人物、地图归属、分类、关系、处理状态、
   三维分量、综合分和 profile 版本检索。
2. 从结果沿原件、版本、派生物、地图归属、知识/案例证据、日志/感悟引用及其他关系导航。
3. 找到没有 OCR/转写的媒体-only、受限项和明确标识的未审阅模型候选。
4. 触发至少一种内容浮现，核对每项结果说明与当前方向、相关度、时间或关系有关的原因，
   并区分人工确认、机器建议和未审阅候选。
5. 创建、修订子库定义并生成物化结果，明确是否纳入未审阅建议。
6. 删除物化目录和搜索/浮现投影后重建。

预期：各类资料均可发现；浮现可解释且不产生隐藏写入；断链显示明确状态；子库人工
定义/版本不随物化删除；重建不复制第二套权威资料，也不改变 C0/C1、评分历史或建议状态。

执行状态（2026-07-23）：TC-07 整体通过。P6.2 已通过步骤 1–4 和步骤 6 的搜索/浮现投影
部分；P6.3 已通过步骤 5 与步骤 6 的子库部分。

- 隔离 fixture 覆盖正文与结构化组合检索、媒体-only、附件-only、受限、缺失、removed、
  first-party/machine、unreviewed/accepted/rejected、多 profile 评分、raw/semantic 关系、
  show/traverse、四类浮现原因、rejected/modified 主动浮现排除、删除重建，以及 CLI/API
  共用应用服务。
- 真实数据根在在线快照后构建 `03_views/search/index/search.sqlite`：投影包含 27 个 raw
  record、3 个 semantic record 和 14 条导航关系；正文 `AI` 返回 23 项，真实未审阅
  Knowledge/profile 组合查询返回 1 项，媒体-only 返回 2 项。真实库没有 attachment-only
  或受限记录，未为验收制造。
- 真实详情读回 1 个 revision、4 个 derivatives、2 个地图归属、评分历史和 3 条未断关系；
  traverse 可达原始 item、Case 与 Map/Direction。surface 只返回 3 个有资格的 semantic
  entry，每项均说明 direction、relevance、time、relation，且机器未审阅身份仍为
  `human_judgment=false`、`confirmed_fact=false`。
- 删除后投影文件消失；重建后 fingerprint 与行数一致。构建前后 46 张 raw/knowledge 表和
  4 张 derived 表摘要完全相同，实时库 `quick_check=ok`、foreign key 异常为 0。证据位于
  `BABATA_EVIDENCE_HOME/runs/p6-2-discovery-20260722-235222/`
  `P6_2_DISCOVERY_E2E.md`。
- P6.3 fixture 通过真实 CLI 创建两版 first-party C0 `SublibraryDefinition`，覆盖组合选择、
  人工 include/exclude、确定性组织规则、unreviewed 纳入/排除、旧版完整读回、数据库
  UPDATE/DELETE 拒绝、物化篡改拒绝和 delete/rebuild；local API 调用同一 application service。
- 真实定义精确选择现有 machine/unreviewed Knowledge，生成 1 项成员的物化并保留
  `human_judgment=false`、`confirmed_fact=false`。物化篡改使 verify 失败，删除重建后通过；
  C2 操作前后 raw/derived 逐表摘要不变。证据位于
  `BABATA_EVIDENCE_HOME/runs/p6-3-sublibrary-output-20260723-200352/`。

### TC-08：可追溯输出与只读边界

关联：AC-08，阶段 P6.3。

场景：对单项和明确集合分别生成人类可读输出与结构化输出，并对生成文件做外部修改、
删除和重建。

步骤：

1. 检查输出 scope、输入版本、builder/template 版本、manifest 和状态。
2. 从输出定位回 C0/C1 与人工记录。
3. 外部编辑生成文件，检查核心是否被反写。
4. 删除并按相同输入范围重建。
5. 请求一个未实现输出类型。

预期：输出可回溯且支持明确批量；外部编辑/删除不改变权威资料；重建差异有记录；
未实现类型不显示可用。

执行状态（2026-07-23）：TC-08 通过。

- fixture 对固定子库版本生成 Markdown，对显式两项集合及单项生成 JSON；两类 builder
  共用 scope 解析，manifest 读回输入 ID/version/hash、来源、审阅身份、builder/template/
  profile、时间、状态、限制与输出 hash；
- 外部修改 Markdown 后 verify 返回 `valid=false`，首次 delete/rebuild 产生 generation 2 和差异
  记录；manifest profile/history 合同收紧后两类输出经正式 rebuild 形成最终 generation 3 并通过
  verify。C0 定义、外部原件、机器语义与 C1 均未反写；
- 真实数据以现有 1 条 machine/unreviewed Knowledge 生成两类输出，Markdown 和 JSON 均可
  定位 semantic/item/revision/source，且没有把建议改成人工事实；
- 可用类型只列 `human_readable`、`structured`；请求 Web 返回 `capability_unavailable`，
  Obsidian 同样保持未启用；
- 全部证据与 TC-07 的 P6.3 证据同处
  `BABATA_EVIDENCE_HOME/runs/p6-3-sublibrary-output-20260723-200352/`。

### TC-09：Skill、脚本、浏览器入口与 Agent 受控

关联：AC-09，阶段 P7。

场景：从 CLI、Skill、浏览器扩展、脚本和 Agent 调用同一可用/不可用能力，并尝试
扩大批处理范围或直接写数据。

步骤：

1. 比较各入口返回的状态、引用、限制和错误。
2. 对 unavailable 能力调用 Skill。
3. 取消一个明确范围的批处理并检查后续项。
4. 让 Agent 在一个已确认范围内连续收集，再尝试越界扩大到未确认账号级全量和自动确认
   模型建议。
5. 扫描 JS/Python/Skill 的数据库和最终资产写入路径。
6. 用同一个 `babata-collect` 分别解析“收集桌面的 OneNote 导出”“收集印象笔记整库导出”
   和“收集豆包战略领导力W1”，检查 route/recipe 与 capability 状态。
7. 输入未知网站，确认 Skill 先报告未知 route 和所需调查，不擅自退化成 generic capture；
   输入“把已连接账号全部收了”，确认在没有明确范围时拒绝执行。
8. 对 OneNote 分别给出 PDF+MHT、MHT-only；对同一来源未来新增附件形态，确认只扩展
   `source.onenote` recipe，不出现新的用户级 Skill。
9. 扫描 Skill 流程和 forward-test 记录，确认主权回收可以诚实停在 captured/prepared；正式
   收集的成功终点只有 registered/C0-C 回读，没有 Process/C1 命令、第二 writer 或“临时拿回
   即正式登记”的伪成功。
10. 在已登录豆包历史中一次发现明确数量范围，排除用户指定的超大主会话，将约 40 条拆成
    不超过 20 条的显式批次；验证分页不完整项不写 C0，保存项逐 item 重采，一个外部命令
    失败不终止其余 item。记录收集/重采耗时并确认提速方案不放松逐项事务与完整性判断。
11. 对已经正式取回的“战略领导力W1”只建立全新临时数据根：从 Chrome 官方响应生成包含
    16 条消息和 7 个 DOCX 原件的 acquisition handoff，选择时请求附件；再刷新来源生成新
    handoff 并走 typed recollection。删除一个附件、修改文件字节或把 handoff 对话 ID 改成
    其他值时，确认 C0 写入前失败。
12. 对同一批结果分别制造 `C0-A1 + captured`、`C0-A2 + prepared` 和 `C0-A2 + registered`；确认 Skill
    汇报同时显示主权深度与管理状态，并只把最后一种称为正式 C0。再从 C2 提交一个 C0-A3
    缺口请求，确认它走新的来源授权与取得任务，不通过 Skill 或 C2 直接改写旧 C0。
13. 用只含收藏 URL/本地资源的微信 C0-A1 范围和消息/声明附件完整覆盖的豆包 C0-A2 范围对照；
    确认微信范围在取得 URL 正文、内嵌媒体和必要附件前不能进入 B/C，豆包范围可继续准备，
    C0-A3 语义引用是否取得不影响两者的 B/C 准入判断。
14. 从微信文件传输助手和收藏中选择 10–20 条代表样本，两类都覆盖，优先覆盖 URL/文章并
    包含少量本地媒体；逐条取得正文、内嵌媒体和必要附件，确认样本达到 C0-A2 后进入 prepared/C0-B、registered/
    C0-C，并在重采时正确报告 unchanged。核对未选全量仍为 C0-A1，汇报不扩大样本结论。
15. 选择一个包含同大小/同内容文件的本地目录，使用默认 `opaque_copy` 走正式 Collector；确认
    每个文件各自复制和登记，全部资产为 `size_snapshot_v1`、`sha256 = null`，没有大小分组、
    抽样哈希或内容复用；再显式选择 `full_sha256` 验证强校验仍可用。
16. 微信样本只读取解密数据库/Recovery handoff；未收到用户明确要求时，任何步骤都不得操作
    微信 UI。单项第一次失败后只允许一次重试，第二次失败转为 skipped/non-retryable。

预期：所有入口调用同一核心结果；unavailable 不伪成功；已确认范围内不因反复人工确认
中断，取消后范围不扩张；未确认越界自动化被拒绝；外围无第二权威写入；Skill 测试不
替代底层能力测试。用户只需要一个收集 Skill；route/recipe、adapter 和 case 保持内部责任，
同一来源的新内容形态不会制造新的用户入口，收集不触发 C1。

豆包批量补充结果（Issue #92）：40 个真实候选分两批执行，38 saved、2 个因
`HasMore=true` 诚实拒绝；38/38 逐 item 重采 `unchanged`、0 新 revision，C1 不变。数据库
新增 34 items/38 revisions，4 个差额经旧新 payload 对照确认为 v1→v2 稳定指纹一次性归一化，
不是来源内容变化。该实测同时证明整批重采不是合适的外部故障边界；Skill 已固定单 item
隔离和一次定向重试，并把唯一后续提速目标限制为复用浏览器/捕获生命周期。

豆包 W1 临时闭环结果（Issue #96）：全新临时根得到 1 item、1 ready revision、7 个 original
DOCX assets；新的等价 handoff 重采为 `unchanged`、0 新 revision，C1 为 0。单元测试另证明
缺附件和来源文件变化会在 C0 前失败，ready revision metadata 不含 acquisition 临时路径。
W1 原正式记录未被重复写入；该练手闭环后不继续扩大同一样本范围。

P7 收尾结果（Issue #101）：真实 `omba25` 共 247 个文件、14,728,536,423 字节，默认
`opaque_copy` 在 66.961 秒内 247/247 saved、0 failed/skipped；247 个资产全部为
`size_snapshot_v1` 且 `sha256 = null`。同 session 重采 247/247 `unchanged`、0 新 revision，
`quick_check=ok`、外键违规 0。微信 13 个 prepared handoff 中 12 个 saved 并 12/12
`unchanged`；唯一 `favorite:5008` 在一次重试后 skipped/non-retryable。两个长豆包对话
`38414453372140034` 与 `38435487680201730` 分别以 60/25 条消息登记，均重采 unchanged。

状态（2026-07-25）：TC-09 部分执行，尚未整体通过。Issue #82 完成首个 extra-source 与底层
能力前置：真实 `.notes` 在隔离库和活动库均发现 `1 batch + 163 notes`；活动库 164/164
saved、0 failed/skipped，349 个资源连同 1 个原 `.notes` 和 1 个解密 ENEX 经唯一核心链路
进入 C0；同 session 重采 164/164 unchanged、0 新 revision。相对路径、错误扩展、XML
entity、畸形资源、截断包和 HMAC 篡改均被拒绝；CLI/local API 复用 application service，
JS/Python/Skill 无新增数据库或最终资产 writer。

Issue #84 又完成第二个 extra-source：真实 OneNote MHT/PDF 在隔离库和活动库都只发现一个
archive 候选，保存 1 ready revision 与 2 ready export 后重采 unchanged、0 新 revision；
manifest 同时绑定两份原件的 hash、MHT 结构、PDF 626 页版式/生成器及互补 representation
role，不把 PDF 页码当成原生 page/section ID。相对路径、跨目录/异名配对、缺失/重复 HTML、
MIME 穿越、畸形/加密 PDF 和 discovery 后变化均被拒绝。真实库最终为
`7 sources / 193 items / 196 revisions / 360 assets / 1 relation`，schema v7、
`quick_check=ok`、外键异常和所有 pending/quarantine/journal/orphan 为 0。证据位于
`BABATA_EVIDENCE_HOME/runs/p7-1-evernote-20260724-224315/` 和
`BABATA_EVIDENCE_HOME/runs/p7-2-onenote-20260725-071439/`。

Issue #86 扩展同一 `source.onenote` 的显式 MHT-only 路线。六个真实导出同时覆盖
`multipart/related` 和单体 `text/html`，隔离库和活动库均保存 6 ready revisions 与 6 ready
exports，随后 6/6 unchanged、0 新 revision。批次内只有“灵感消化”与“猫与月季花”产生
双向两条 12 字符 n-gram 重叠提示，覆盖率约为 95.83%/100%；两条均为
`human_judgment=false`、`confirmed_fact=false`，新 item 的正式 relation 为 0。活动库最终为
`7 sources / 199 items / 202 revisions / 366 assets / 1 relation / 342 observations`，schema v7、
`quick_check=ok`、外键异常和所有 pending/quarantine/journal/orphan 为 0。相对路径、重复文件、
跨目录、非 OneNote 元数据、非法 MIME、discovery 后变化均被拒绝；旧 PDF/MHT 配对 manifest
继续保持 `onenote-paired-export/1`，避免代码升级制造伪 revision。证据位于
`BABATA_EVIDENCE_HOME/runs/p7-3-onenote-mht-20260725-090516/`。

Issue #88 随后完成第四个 P7 切片：对豆包复杂会话“战略领导力W1”已拿回的 7 个原始
DOCX 复核消息 MD5、Recovery SHA-256、大小和 DOCX 结构，再把此前缺少的 6 个通过通用
附件操作附加到既有 ready revision。隔离库和活动库均只改变 `assets` 与两张附件操作表；
活动库最终为 `7 sources / 199 items / 202 revisions / 372 assets / 1 relation / 342 observations`，
W1 为 7 个 original DOCX 加 1 个 preview PDF。正文 hash、既有资产、17 个 process runs/
16 个 derivatives 均不变，`quick_check=ok`、外键异常和所有 pending/quarantine/journal/orphan
为 0。证据位于 `BABATA_EVIDENCE_HOME/runs/p7-4-doubao-w1-assets-20260725-111326/`。
该切片验证的是统一 C0 独立完成与外围无第二 writer，不包含豆包可执行来源 recipe 或受控 Agent。

Issue #90 随后完成唯一总收集 Skill 的 dry-run 对照：OneNote 同一 route 内选择 PDF/MHT
配对与 MHT-only 两个 Collector session；豆包在 disabled 状态停止；未知网站和未确认账号级
全量拒绝执行。确定性门禁及三项负向变异证明豆包伪 enabled、删除无 C1 边界和追加
`babata process` 命令都会失败。尚未执行的 TC-09 部分包括真实受控 Agent 的范围查看与
运行中取消、自动确认模型建议拒绝，以及多个外围入口的完整结果对照。因此当前只启用
已经分别通过真实验收的 `source.evernote` 和 `source.onenote`，不标记 TC-09 或 P7 完成。

状态（2026-07-27）：TC-09 已通过。上段 2026-07-25 的待测项已按真实操作收敛，不再要求
构造脱离收集工作的取消或模型越权演练。确定性门禁证明未知/disabled 来源、未确认账号全量、
伪 enabled、C1 命令和第二 writer 均被拒绝；Issue #101 的真实批次进一步证明明确范围内连续
执行、单项失败隔离、最多一次重试、第二次失败转 `skipped/non-retryable`、其余项继续、统一
C0-C 回读与 unchanged 重采。微信只读解密数据库/Recovery，未操作微信 UI；本地 247 文件
全部直接复制并独立登记。该结果取代旧的部分执行状态，AC-09/TC-09 与 P7 一并闭合。

豆包第二阶段结果（Issue #116）：外部 Chrome 完整枚举 726 个唯一历史会话；排除主对话并
扣除已有 C0/MBA 第一阶段的 348 个唯一 ID 后，377 个非主对话全部完成首轮尝试。7 个捕获
失败项各重试一次后成功，最终命令失败为 0。340 个完整会话均为 `HasMore=false`，共 1,667
条唯一消息，对话 ID 错配与消息 ID 重复会话均为 0；37 个 `HasMore=true` 长会话按阶段边界
带 cursor/hash 转入第三阶段。完整会话声明的 54 个唯一资产中，46 个原件、220,445,144 字节
已取得；8 个声明 MD5 不一致的 JPEG 与 1 个两次来源尝试仍缺失的 PDF 已逐项记入第三阶段，
未把候选字节冒充原件。证据位于
`BABATA_RECOVERY_HOME/batches/doubao/p8-2-stage2-20260728/`；
`stage2-completion-audit.json` 为 complete，`stage3-residuals.json` 保留 37 个长会话、9 个附件
残余和 3 个仅剩附件问题的会话。该结果只闭合豆包第二阶段，不代表 P8.2、豆包第三阶段、
微信第二/第三阶段、Recovery 到正式 C0 或 TC-11 已完成。

豆包第三阶段结果（Issue #118）：主对话与 37 个长会话全部通过登录 Chrome 的官方
`chain/single` 请求按 `next_index` 分页到 `HasMore=false`，合计 38/38 个会话、156 页、
5,887 条跨会话唯一消息，跨会话重复 ID 为 0。两个旧输出的分页重叠 3/6 条已显式去重；
直接分页保存的每页原始响应 SHA-256 与游标链复核通过。9 个附件残余中，8 个候选确认为
全分辨率 PNG derivative 但不匹配上传原件 MD5，1 个 PDF 补出真实存储 URI 和 20,556,280
字节声明但原件仍缺失；全部保留两次来源尝试与完整性证据，不把 preview/derivative 冒充
original。`BABATA_RECOVERY_HOME/batches/doubao/p8-2-stage3-20260728/stage3-completion-audit.json`
为 complete；该结论完成豆包第三阶段的 Recovery，不启动正式 C0、C1、微信阶段或 P8.3。

微信第二阶段结果（Issue #120）：全量 746 个会话按最新联系人和消息来源证据分出 498 个
真人单聊候选，排除 132 个群聊、115 个公众号/服务号和第一阶段文件传输助手。候选中 482 个
保留实质内容、16 个纯通知会话排除；115,261 条保留消息全局 ID 重复为 0，1,394 条好友验证
或系统提示噪声被移除。逐会话包内的可得媒体与最新账户整合归档第二次补取按资源 ID 归并，
最终 544 个缺口为表情 234、视频原件 219、文件 91，均保留两次尝试账。482 个会话包经
17 个修正后 Collector session 全部 `saved`，数据库中正式项、capture 修订和 export 资产各
482，资产合计 4,149,726,759 字节，`quick_check=ok`，C1 未触发。首次 `text` 内容类型被
Collector 合同一致拒绝的 session 另行保留，纠正为 `document` 后未重抓源数据。证据位于
`BABATA_RECOVERY_HOME/batches/wechat/p8-2-stage2-20260729/`；该结果只关闭微信第二阶段，
微信第三阶段和 P8.3 均未启动。

微信第三阶段其他范围结果（Issue #122）：按用户最新优先级，132 个群聊、97,232 条消息进入
可读暂缓清单，不阻塞本批次。115 个公众号/服务号会话全部完成处置，111 个有内容会话保留
721 条消息，4 个源导出空会话排除，只移除 1 条系统消息，消息 ID 重复为 0。111 个逐会话包
包含 101 个可得头像预览、490,897 字节，声明媒体缺失为 0；经 4 个 Collector session 首轮
111/111 `saved`，失败和重试均为 0，C1 未触发。证据位于
`BABATA_RECOVERY_HOME/batches/wechat/p8-2-stage3-other-20260730/`；该结果关闭第三阶段非群聊
其他范围，群聊仍为用户暂缓，P8.2 不冒充完成。

### TC-10：外部数据根、数据级别与隔离恢复

关联：AC-10，阶段 P3/P9。

场景：使用全新数据根通过多个入口产生 C0–C3，创建一致备份，在隔离数据根恢复并
分别删除/重建各级允许内容。

步骤：

1. 扫描 Git、活动数据根、证据根与恢复根，确认真实数据/凭据边界；活动数据根顶层只
   允许 `00_inbox` 至 `05_logs` 及最小本地说明，辅助根不进入 Git，也不成为第二权威。
2. 验证不同入口的最终资料都能被同一核心解析。
3. 删除 C3、C2、可重建 C1，确认 C0 不受影响。
4. 从一致快照恢复到隔离根，打开索引并校验资产/版本/关系哈希。
5. 模拟 C0 损坏、C1 缺失、C2/C3 未重建和凭据缺失。
6. 连续建立两次活动根一致快照并进入同一加密增量仓库，核对第二次复用未变化内容；密码
   不出现在 Git、manifest、日志或命令输出。
7. 把第二次快照恢复到全新隔离根，运行 SQLite quick check/外键检查并按 manifest 验证 C0/C1；
   再在一次独立损坏副本中修改 C0 字节，确认 restore verification fail closed。

预期：只有 C0 损坏阻止权威恢复通过；C1/C2/C3 按级别报告和重建；Git 无真实数据；
不存在多个活动写入者；恢复报告准确区分故障类型。

Issue #78 已完成步骤 1 的真实目录纠偏：429 个辅助文件经逐文件 SHA-256 清单迁出活动
根，迁移前后四个 SQLite 的 quick check、外键和逐表行数一致，C0/C1 内容寻址文件有效；
fixture 另证明 audit-only 不写盘、目标冲突闭锁且源文件保留。该证据不包含步骤 4–5 的
一致备份与隔离恢复，因此不代表 TC-10 或 AC-10 整体通过。

状态（2026-07-27）：TC-10 已通过。Issue #112 连续建立两个 1,281 文件/21,310,423,952 字节
一致快照并进入同一加密 restic 仓库；第二次只新增 2,467,774 字节。第二个 snapshot 恢复到
全新隔离根后，1,281/1,281 文件、21,310,423,952 字节、4 个 SQLite quick check/外键全部
验证通过，C1/C2/C3 缺失为 0，凭据重授权状态单独报告。真实篡改一个 1,150 字节 C0 资产后，
`ops verify-restored` 以不可重试 `integrity_failed` 拒绝并指出准确路径；恢复原字节后的最终
全量复验再次通过。活动根前后 pending/quarantine/journal/orphan 均为 0。证据位于
`BABATA_EVIDENCE_HOME/runs/p8-1-backup-20260727-214924/`。该结果保留为 P9 可复用的现有
本地能力，不再作为 P8.1 的定义或完成证据。

### TC-11：完整本地 raw-to-view 闭环

关联：AC-11，阶段 P4–P8 系统级验收。

场景：使用一个真实授权来源和一条 first-party 内容贯通收集、清洗、人工沉淀、检索、
子库、输出、删除重建和隔离恢复，并注入一次局部失败。

步骤：

1. 在来源上下文选择资料并观察逐条状态；创建 first-party 内容。
2. 回看 C0，对一项执行真实清洗并保留 C1 溯源。
3. 建立人工判断和关系，处理一个模型建议。
4. 检索资料、创建子库并生成可回溯输出。
5. 删除/重建 C2，执行一致备份和隔离恢复。
6. 在收集或处理环节注入局部失败并重试。

预期：四段链路在同一权威体系中成立；C0/C1/人工沉淀/子库定义不因 C2 删除而改变；
恢复后链路可读；成功项不因局部失败丢失；没有半成品伪装成功。

P8.4 来源深度执行结果（Issue #126，2026-07-31）：总审计逐来源验证 14 个纳入来源各 5 条，
共 70 条 C0-A2 或更高样本；Bilibili 为 `deferred_by_user` 并排除在分母外。浏览器书签以
1,560 条/47 文件夹的 Netscape 导出证明身份，选 5 篇正文图片依赖为 0 的文章保存完整文本
和 HTML；微信收藏公众号文章从 3/5 补到 5/5，两条图片型文章各保存 1 张正文图，头像、
水印重复图和 UI 资源均排除。所有计入文件均通过字节/hash/零字节和依赖校验，最终
`pending_sources=0`；Recovery、正式 C0、C1 分离，本轮没有启动 B/C/C1。该结果属于
TC-11 的来源取得证据，不单独替代完整 raw-to-view 步骤 2–6。

P8.5 存量准备执行结果（Issue #128，2026-08-01）：冻结新增抓取后，对同一 70 条样本逐项
检查 A3 必要性，70/70 已判断、0 条当前需要 A3；25 条既有 registered/C0-C 保持不变，45 条
C0-A2 captured 生成逐项 prepared/C0-B manifest。验证器重算 45 个 manifest 及其 549 个
上游引用文件、283,283,690 字节的存在性、大小和 SHA-256，错误为 0；逐来源直接依赖数量和
字节与 P8.4 总账一致。所有路径保持仓库外只读引用，未写 SQLite/managed assets，未新增
C0-C 或 C1。该结果验证 C0-A2 可在不完成 A3 的情况下进入 C0-B，同时不扩大成全量 A2/B。

## 3. P2 工程 Gate 测试

这些测试不映射产品 AC：

| ID | Gate | 验证 |
| --- | --- | --- |
| GT-P2-01 | P2-G1 | 6 crate、137 个目标 Rust 源文件和外围规格位置齐全 |
| GT-P2-02 | P2-G2 | 12 service、13 port、13 CLI 模块、local API/worker owner 唯一 |
| GT-P2-03 | P2-G3 | domain/application/infrastructure/composition root 依赖单向且可编译 |
| GT-P2-04 | P2-G4 | 未激活能力统一 unavailable，不启动真实来源/provider/worker |
| GT-P2-05 | P2-G5 | JS/Python/Skill/provider/view/output 无 C0/C1 直接写入路径 |
| GT-P2-06 | P2-G6 | `-1` 原话证据、00–05、蓝图、脚本、配置和测试标识追溯一致；00 保持当前需求权威并链接独立原话文档 |
| GT-P2-07 | P2-G7 | 19 个必需 `source_id` 在路线总表中存在且唯一，路线、最小授权、E0-E3 证据、诚实缺口和状态非空；代表性 `tool_id` 有真实状态；未到 E3 的来源保持 disabled；允许未来追加来源 |

`check-doc-traceability.ps1` 解析上述真实表格，并检查独立 `-1_USER_WORDING.md` 与 00 的入口，
不再依赖文末来源或工具 token。`test-doc-traceability.ps1` 必须证明删除 `-1` 原话权威、删除
一个必需来源、清空 Kimi 证据、清空 Kimi 最小授权都会失败。`check-doc-provenance.ps1` 从
独立 `-1` 文档校验必需用户原话 hash；对应 mutation test 必须证明改写原话、混淆 Builder/
用户来源、写入本地 UUID 或敏感 token 均 fail closed。

## 4. P3 Phase Gate 测试

这些测试只证明 C0 与 first-party 底座，不把产品 AC 或真实来源提前标记完成：

| ID | Gate | 验证 |
| --- | --- | --- |
| GT-P3-01 | P3-G1 | 全新临时数据根产生编号分区；首次 status 为 schema 0，提交后为 schema 4；Git 无运行数据 |
| GT-P3-02 | P3-G2 | text/file/export 的 JSON outcome 包含 repository read-back；上下文、版本、asset role/state/path/hash 与最终 bytes 一致；重导入各次 locator/native/timestamp/metadata provenance 独立可读 |
| GT-P3-03 | P3-G3 | create/revise 保留旧 wording、ordinal、parent、revision metadata 和 revises；annotate 是独立 item 并指向具体 ready revision；外部 revision 拒绝 revise |
| GT-P3-04 | P3-G4 | stage/graph/finalise/verify/ready/post-ready read-back/cleanup 故障的 CLI、operation、revision/asset、journal/orphan 与 bytes 状态一致；无资产失败可按 operation ID 诊断；共享哈希原件不被移动 |
| GT-P3-05 | P3-G5 | `check-rust-boundaries.ps1` 与 `check-no-secondary-writer.ps1` 证明 DB/最终资产仍只有 infrastructure owner |
| GT-P3-06 | P3-G6 | 全部 P2 gate 继续通过；P3 raw baseline 保持 0001–0004，后续 raw extensions 显式登记；P4 migration 分离，provider/route/candidate 保持 unavailable/disabled |

`check-p3-raw-inventory.ps1` 验证 29 个 P3 活跃文件、55 个 raw 功能测试和 P4 未提前
激活边界。`p3_raw.rs` 使用全新临时根贯通显式提交与 first-party 版本；application 和
infrastructure 负向测试分别注入 stage、graph、finalise、verify、ready transition、
post-ready read-back 与 cleanup 故障，并联合检查 operation/journal/orphan 和 durable C0。

## 5. 阶段测试映射

| 阶段 | 产品测试 | 工程/阶段证据 |
| --- | --- | --- |
| P2 | 无 | GT-P2-01..07 |
| P3 | TC-03A、TC-06、TC-10 的底座部分 | P3-G1..06、raw integration/CLI tests |
| P4 | TC-01、TC-02 | P4-G1..06、真实授权证据 |
| P5 | TC-03A、TC-04 | C1/provider/integrity tests 与真实 asset 证据 |
| P6 | TC-03B、TC-05、TC-06、TC-07、TC-08 | core/read projection/output tests |
| P7 | TC-09 | Skill/Agent/extra-source tests |
| P8 | TC-11 | 来源回收与 end-to-end evidence；P8.3 暂停新增抓取并保留全量 A1 缺口；P8.4 已完成 14/14、70 条 A2 样本；P8.5 已判定 70/70 当前无需 A3，45 条 captured A2 进入 prepared/C0-B，25 条保持 C0-C |
| P9 | TC-10 | 选定一个外部复制/同步目标；本地 backup/restore 能力已存在 |

## 6. Skill 测试规则

Skill 只有在对应本地能力的 TC 已通过后才激活。Skill 测试验证参数路由、授权范围、
状态和结果引用；它不替代来源、处理、核心、输出或恢复的真实测试。

`check-collection-skill.ps1` 固定验证 `babata-collect` 是唯一总收集入口、OneNote/Evernote/
豆包状态与来源 recipe 一致、Collector/取消/重采命令存在、三层汇报完整且没有可执行的
Process/Knowledge 命令。`test-collection-skill.ps1` 必须证明将已实证的豆包伪降为 disabled、
删除分页不完整拒绝或逐 item 故障隔离、删除无 C1 边界或追加 `babata process` 命令都会失败。

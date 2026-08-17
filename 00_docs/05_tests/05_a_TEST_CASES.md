# Babata 测试用例

<!-- DOC-ID: DOC-TC -->

<!-- DOC-AUTHORITY-BOUNDARY: verification-procedure -->

## 1. 测试职责

本文定义可重复的验证场景、步骤和预期结果。它不维护“当前通过/失败”、Issue、PR、日期、批次、
具体课程文件数或 phase 进度；执行结果进入 `../04_process/04_b_USAGE_STATUS.md` 和 Git 外 receipt。

TC 验证 AC，但不重新定义 AC。工程 gate 可证明编译、所有权或完整性，不能替代产品 AC、真实
路径测试或用户验收。试跑、试点、模板和明确范围全量运行都可作为测试输入；结果必须绑定分母，
不得从样本外推。

## 2. 产品测试用例

### TC-01：来源上下文候选与选择性收集

覆盖：AC-01、AC-11。

1. 在一个真实授权来源中给出收藏夹、会话、知识库、目录或时间段范围。
2. 列出真实候选、层级、类型、附件可得性和限制，不要求逐项确认。
3. 只选择其中一部分并完成正文/原件/必要附件取得。
4. 验证未选和越界对象没有进入 C0；缺失/受限对象有明确状态。
5. 用另一个来源 fixture 或 locator 替代真实来源，测试必须拒绝“已支持”声明。

预期：收集范围和结果可核对；一个来源的样本不证明另一个来源或账号全量。

### TC-02：逐项状态、局部失败、重试与重收集

覆盖：AC-02、AC-11。

1. 对多个真实对象执行收集，并在中间对象注入一次可重试失败。
2. 验证成功项保留，失败项单独重试，其他项不重复。
3. 取消运行，确认取消后不继续扩张范围。
4. 对 saved 对象执行 unchanged、changed、inaccessible、removed 四种重收集。
5. 验证 unchanged 不新增 revision/asset；changed 追加版本；inaccessible 不变成 removed。

预期：逐项状态、原因和分母完整，无整批伪成功或重复写入。

### TC-03：C0、第一方、C1、C2 与 C3 边界

覆盖：AC-03、AC-11。

#### TC-03A：C0 与 C1 溯源和隔离

1. 登记真实文本和至少一种二进制原件，read-back item/revision/asset/provenance/hash。
2. 生成一个真实 C1，验证输入 identity/hash、processor、状态和 output hash。
3. 删除并重建 C1，确认 C0 字节、身份和关系不变。
4. 尝试从 C0-A1-only 或外围 SQLite 写入形成 registered C0，必须 fail closed。

#### TC-03B：C2 与子库/视图边界

1. 从真实 C0/C1/semantic entries 生成搜索投影、子库或输出。
2. 从 C2 回到输入 identity/version/hash 和人工/机器身份。
3. 篡改、删除、重建 C2，确认 C0/C1/first-party/关系摘要不变。
4. 验证 C2/Obsidian/publisher 无 canonical writer 能力。

预期：各层可辨别，可重建层不会取得权威所有权。

### TC-04：忠实清洗、C1B 模态判断与失败重试

覆盖：AC-04、AC-11。

1. 用真实文档/网页、PDF/图片和音视频样本执行已启用清洗能力。
2. 核对输入、processor/model、版本、限制、输出和 hash；未启用能力明确拒绝。
3. 注入 provider/processor 失败，再只重试目标处理；C0 不变，多次 C1 可比较。
4. 对完整 C1 执行 C1B 判断：文字足够时不强留媒体；公式/图表保留必要图片；听觉、连续动作、
   二进制属性分别选择音频、视频或附件片段。
5. 验证 C1B 不用摘要替代完整文字，不要求每种模态都出现，媒体可回到载体定位/hash。
6. 删除 C1B 物化并按 recipe 重建；外部主权库/C0 不变。

预期：清洗忠实、可追溯、可重试；C1B 是内容判断，不是固定模态配额。

### TC-05：三大界、机器建议与高密度表达

覆盖：AC-05、AC-11。

1. 从真实 C0/C1 生成知识/案例/日志/感悟候选及三大界地图归属。
2. 创建一个同时关联多个 foundation 的 discipline、一个具有多个合法 discipline 父边的 branch，
   以及跨根/跨学科/跨层级归属的 semantic；验证 DAG 无环、父边只表达 `subfield_of`，新增合法
   assignment 不会被其他 registrar 删除，地图改名/迁移保留必要历史。
3. 分别登记 `intersects_with/draws_from/applies_to/prerequisite_of`，验证它们没有被压成父边；标签
   不能产生继承或权威归属。
4. 登记独立 course identity/version，让一课 `covers` 多 branch、一个 branch 被多课覆盖；创建 MBA
   Sublibrary/lens 聚合跨学科课程。删除/重建 lens 不改变课程、branch、assignment 或验收状态。
5. 登记基石 `primary/secondary/contextual` 或独立强度和单独 confidence；验证不强制合计 `100%`。
   生成归一化 C2 百分比后删除重建，正式关系/强度不变；兴趣/战略/共识字段未被复用。
6. 生成兴趣/战略/共识、综合分、依据和权重版本；改变权重不改写旧分值。
7. 让 Agent 在用户零回复时生成类型、标签、关系、评分和高密度表达，保持 machine/unreviewed。
8. 分别接受、修改、拒绝建议；审阅标记可追溯，只有实际新写/改写才形成 first-party C0。

预期：未审阅不阻塞系统，但永远不冒充人工事实。

### TC-06：第一方创作、版本、批注与再分析

覆盖：AC-06、AC-11。

1. 新建 first-party 笔记并 read-back。
2. 真正改写同一作品，验证新 revision 与旧版本可读。
3. 分别添加批注、感悟、日志、附件和 Agent 再分析，验证它们没有制造作品伪版本。
4. 删除生成视图并重建，确认 first-party 正文、版本和关系不变。

预期：第一方原味与真实修订保留，评论/证据/机器分析语义分开。

### TC-07：检索、浮现、关系与子库

覆盖：AC-07、AC-11。

1. 按正文、来源、时间、类型、状态、人物、分类、关系和处理状态组合查询。
2. 按三维相关度/综合分筛选和排序，并显示原因及权重版本。
3. 对无文本媒体/附件执行 metadata/关系发现。
4. 沿地图、来源、版本、证据和日志双向导航；缺失/受限目标显示状态。
5. 创建两版 SublibraryDefinition，覆盖 query、include/exclude、组织和未审阅策略。
6. 删除/重建子库物化，确认定义及权威资料不变。
7. 用 MBA lens 同时纳入至少两个 discipline 下的课程；验证 lens 是非拥有型集合，不在地图中创建
   `MBA` discipline，也不把成员内容改成单一 MBA 父路径。

预期：检索和子库建立在权威资料上，不形成第二套手工同步库。

### TC-08：可追溯输出、模板/profile 与 Obsidian

覆盖：AC-08、AC-09、AC-11。

#### TC-08A：通用输出合同

1. 从单项和明确范围分别生成 Markdown 人读输出与 JSON 结构化输出。
2. 核对 scope、input identity/version/hash、来源引用、builder/template/profile、状态和限制。
3. 篡改输出后验证失败；删除重建后权威资料不变。
4. 用一个 dry-run 预览动作/限制且不发布；用 pilot 标记有界真实范围和不可外推边界。
5. 对明确分母执行全量运行，逐项保留 success/failed/skipped/gap；故意漏一项必须继续处理其余项，
   并拒绝 full-run 完成声明，不能把完整性拒绝误用为提前停止整批。

#### TC-08B：C2B docs/profile/package 治理

1. 检查 `C2B-DOCS-FIRST-GATE`：下游引用 `DOC-INDEX` 的唯一权威顺序，真实受影响的
   requirements、PRD、AC、architecture/spec/profile、process/TC 先于实现同步；删除、漏掉或
   重排任一必需 authority stage，治理检查 fail closed。未受影响角色不得因门禁被迫制造编辑。
2. 检查 `C1B-FORMAL-HANDOFF-GATE` 与 `C2B-KNOWLEDGE-UNIVERSE-GATE`：materializer 强制读取
   正式 C1B 和知识宇宙账本；staging sidecar、覆盖不足或 hash 不一致必须拒绝。
3. 检查 `C2B-PACKAGE-OWNED-COURSE-MAP`：`.mmd`、Markdown Mermaid 块和 PNG 均先在 package
   生成；publisher 只复制 hash-verified package。
4. 课程 index 与宇宙级大 Index 分离；课程发布不能静默修改大 Index。
5. 新鲜重建不读取旧 C2B；旧 C2B 仅作比较，重复 materialize 不追加视觉块或媒体路径。
6. `C2B-KNOWLEDGE-UNIVERSE-GATE` 拒绝 course identity 与 branch identity 合并、MBA 作为单一
   discipline、singular-only assignment contract，以及 registrar 清除未在本轮撤销的其他归属。
   对历史单路径 accepted instance 保留既有 C1B、内容、媒体、profile、package/live、用户验收和
   closure，不得冒充当前本体 conformance；兼容迁移不得损坏这些独立 accepted 维度。
7. 通过 CLI 用不可变 definition 登记 Course，再用 course key/version 回读；核对 typed `covers`、
   每个 semantic module 的多重 role/strength/confidence assignment、typed map relations 和 MBA lens
   membership。重复同一 definition 必须幂等，不同 definition 冲突；缺失 lens/semantic/map node 拒绝。
8. 对历史单路径课程做追加迁移后，旧 assignment、C1B/package/live、acceptance/closure 全部不变；
   直接 SQLite、Obsidian 或 sidecar 不能替代 core CLI receipt。

#### TC-08C：课程内容、内链和视觉 profile

1. 检查 `C2B-MECE-COURSE-MAP-GATE`：一个主分类轴覆盖课程，学习支持分层，主树无交叉回边。
2. 检查 `C2B-CRASH-COURSE-MAP-GATE`：图中含正文有据的目标、判断、公式/关系和风险边界，
   不能只由分类和文件名组成。
3. 检查 `C2B-MODERN-VISUAL-MAP-GATE` 和 `C2B-RIGHT-GROWING-MINDMAP-GATE`：按 accepted profile
   验证方向、层级、token、可读性、无裁切/遮挡；具体节点数与阈值从 profile 读取而非写死为全局 AC。
4. 实际渲染 SVG，用 profile 声明的 Obsidian selector 精确匹配全部 internal-link labels，并逐项
   对应唯一 Markdown 目标；源码字符串检查不能替代实际 selector。
5. 实际查看 package-owned PNG；用户视觉接受与工程检查分别记录。
6. `C2B-COURSE-OUTLINE-GATE`：分别用 flat 与 sectioned fixture，验证 unit 顺序、section 顺序和
   source-module 恰好一次覆盖；100+ unit 不得因学习支持命名失败。重复、漏绑、空 section、混合两种
   outline shape 或从文件名前缀推断结构必须拒绝。
7. learning support 使用 profile 声明的语义 slot 和文件名，独立于 unit 数字命名空间；注入
   `09/10/11` 新正式文件名或让文件系统顺序覆盖 plan 顺序时 checker 必须拒绝。
8. v1 -> v2 呈现迁移核对非授权正文/媒体 hash 全部不变、旧 plan/manifest 指纹和 rename map 完整、
   新 index/Mermaid 内链唯一可达、package/live exact match；不得改变课程 acceptance/closure。

#### TC-08D：响应式主图和折叠位图

1. 检查 `C2B-RESPONSIVE-MAP-GATE`：Mermaid 启用 `useMaxWidth`，禁止 false。
2. 实际 SVG 根具有 `width="100%"`、非空 `viewBox` 和受控 `max-width`；内链集合不变。
3. Index 只有一个默认展开的 Mermaid；PNG 仍在 manifest，但位于默认折叠 callout。
4. 调整 Obsidian 窗格宽度，主图随容器收缩；不把 fit-to-pane 声称为内建 zoom/pan。

#### TC-08E：发布、用户打开与正式关闭

1. publisher 核对 accepted profile、正式账本、manifest 和全部 package hash 后替换唯一 live。
2. 验证 package/live 文件集合和 hash、Wiki/media link、控制面污染和 source boundary。
3. `OBSIDIAN-HUMAN-VIEW-BOUNDARY`：只启动课程 manifest/usage status 登记的唯一 live URI 并停止；
   用户自己查看/验收，课程在此之前保持 `pending_user_acceptance`。
4. 用户明确接受后，`verify-mba-course-c2b-closure.ps1` 独立验证 C1B/C2B/知识宇宙、package/live
   hash、数据库完整性和输出合同，并写 Git 外 accepted/closed receipt；缺少明确用户证据必须拒绝。

#### TC-08F：完整轮次到终端、缺陷收敛与新轮复验

1. 用 `invoke-babata-execution-round.ps1` 为一个多阶段课程候选冻结 round identity、分母、
   输入/代码/配置 hash、全新 staging、目标终端、阶段列表、验收矩阵和 fail-fast 类别；开轮后
   篡改任一冻结项，runner 必须拒绝继续。
2. 在早期和中期各注入一个不会使后续证据失效的缺陷；验证实现/配置未在轮内改变，后续阶段仍
   执行到声明终端，round ledger 同时保留两个缺陷和所有阶段结果，且终端矩阵不误报通过。
3. 分别注入 authority/授权越界、输入漂移和会使下游结果失真的合同错误；验证本轮立即 aborted、
   后续 writer/publisher 未执行、候选未发布。
4. 在轮后把普通缺陷按共同根因形成一个修复集并运行目标测试；验证目标测试不能把旧轮提升为
   accepted/closed，也不能修改旧 round receipt。
5. 修复后使用新的 round identity 和 staging 完整重跑；验证不读取旧 C2B/package/live/sidecar，
   只在新轮到达终端后运行一次课程 terminal gate 并替换唯一 live。
6. 验证 merge/release/closure 的全仓 gate 与轮内观察点分开；临时 checker 不能冒充阶段验收。

预期：模板/profile 可复用，具体课程结果只进入 usage；Obsidian 是只读输出，不是第二 writer；
`BATCH-ROUND-TERMINAL-GATE` 防止边跑边修和局部测试冒充整轮验收。

### TC-09：Skill、脚本、浏览器入口与受控 Agent

覆盖：AC-09、AC-11。

1. `babata-collect` 根据来源/范围和 capability 选择已登记 recipe；unknown/disabled/unavailable
   返回不同原因和下一动作。
2. 验证收集只调用统一 Collector/Capture，不触发 Process/Knowledge/C2 publisher。
3. 在明确批量范围内执行，注入单项失败、取消和第二次非重试失败，其他项继续。
4. 比较至少两个外围入口的 read-back，确认都没有直接 SQLite/最终资产 writer。
5. 尝试让 Agent 自动接受模型建议、扩张账号范围或把 captured 冒充 registered，必须拒绝。
6. Skill 自测通过但底层产品 TC 未通过时，capability 仍不可标为 enabled。

预期：入口受控、状态诚实、用户无需选择内部模块，外围不能改变权威。

### TC-10：数据根、备份与隔离恢复

覆盖：AC-10、AC-11。

1. 检查 Git、活动数据、evidence、recovery、staging、用户导出和凭据位置边界。
2. 对活动根形成一致快照和加密增量备份；第二次复用未变化内容。
3. 恢复到全新隔离根，不切换/覆盖活动根。
4. 检查 manifest/hash、全部 SQLite quick check/foreign keys、C0/C1/C2/C3 和凭据重授权状态。
5. 篡改一个 C0 文件必须 fail closed；删除 C2/C3 只报告可重建/未恢复，不冒充 C0 损坏。

预期：恢复证明权威资料可用，缓存或视图不被误当成权威。

### TC-11：完整本地 raw-to-view 闭环

覆盖：AC-01 至 AC-11。

1. 从一个真实授权来源选择并收集资料，同时创建 first-party 内容。
2. 对真实资料执行 C1/C1B，保留原件和处理溯源。
3. 形成机器候选、人工判断和关系，保持身份可辨别。
4. 完成检索、关系导航、子库和输出。
5. 删除重建输出并执行隔离恢复，权威资料保持一致。
6. 在任一中间步骤注入局部失败，确认成功项保留、失败可重试、无伪成功。

预期：证明完整产品链路。某个来源/课程全量或单模板接受不能替代此用例；反过来，TC-11 通过
也不声称全部来源或课程已经全量使用完成。

## 3. P2 工程 Gate 测试

以下只证明骨架/所有权，不是产品 AC：

| Gate test | 对应 gate | 验证 |
| --- | --- | --- |
| GT-P2-01 | P2-G1 | 文件/模块 inventory 与 blueprint 一致 |
| GT-P2-02 | P2-G2 | workspace、format、lint、compile |
| GT-P2-03 | P2-G3 | ports/services/CLI/API ownership 可发现 |
| GT-P2-04 | P2-G4 | Rust crate dependency boundary |
| GT-P2-05 | P2-G5 | 单一 writer、外围无 canonical persistence |
| GT-P2-06 | P2-G6 | placeholders/unavailable 不伪装可用 |
| GT-P2-07 | P2-G7 | 每个来源有真实 route evidence、授权、限制和 fallback |

## 4. P3/P4 Phase Gate 测试

- P3-G1..P3-G6：C0/first-party schema、唯一 writer、事务故障、read-back、migration 和边界测试。
- P4-G1..P4-G6：真实候选、明确选择、逐项状态、附件/上下文、局部失败和 unchanged 重采。

这些 gate 的当前执行状态不在 TC 中维护；见 usage status 和对应 receipt。

## 5. 阶段测试映射

| Phase | 主要 TC | 边界 |
| --- | --- | --- |
| P2 | GT-P2-01..07 | 工程骨架，不证明产品可用 |
| P3 | TC-03A | C0/first-party 底座 |
| P4 | TC-01、TC-02 | 首批真实收集路径，不代表所有来源 |
| P5 | TC-03A、TC-04 | C1 清洗，不提前证明 C2 |
| P6 | TC-03B、TC-05..08 | 核心、检索、子库和通用输出 |
| P7 | TC-09 | 统一 Skill 和受控 Agent |
| P8 | TC-01..11 的真实使用扩展 | 每个明确范围单独报告，不写进 PRD/AC/TC 状态 |
| P9 | TC-10 | 外部同步目标和恢复 |

## 6. Skill 测试规则

Skill 只有在对应产品能力的 TC 已通过后才可激活。Skill 测试验证参数路由、授权范围、状态、
结果引用和失败处理；它不替代来源、处理、核心、输出或恢复的真实测试。

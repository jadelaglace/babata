# Babata 测试用例

<!-- DOC-ID: DOC-TC -->
<!-- DOC-AUTHORITY-BOUNDARY: verification-procedure -->

## 1. 文档职责

本文定义可重复验证场景、步骤和预期结果。它不维护“当前通过/失败”、批次结果或用户验收；实际
执行写入 Git 外 receipt，并由 `DOC-USAGE` 摘要当前结论。

## 2. TC-01：来源候选与选择性收集

追溯：`AC-01`。

1. 用真实授权来源发现多个带正文/附件和上下文的候选。
2. 选择其中一个，明确拒绝或不选择其他对象。
3. 执行 capture/register 并回读数据库与 managed asset。

预期：只有选中对象进入 C0；source/native ID、路径/URL、时间、附件和 hash 可回读；未选对象没有
正式身份；路线、授权和限制与 `DOC-ROUTES` 一致。

### TC-01A：一堂复合对象真实路径

1. 对用户明确给出的一个 replay/document URL pair，经登录态浏览器取得安全字段和完整虚拟滚动正文，
   下载回放与正文图片并生成 prepared manifest。
2. 核对 replay sid、document token/ACL pairing、全文末尾、媒体时长以及正文和每项资产 size/SHA-256。
3. 发现并只选择该候选，经 Collector 保存后回读 item、revision、正文和全部 managed assets。

预期：回放、可读全文和声明图片作为一个 `source.yitang` item 保存；API 加密载荷、用户身份、讨论、
点赞和 cookie 不进入 C0；未授权的文稿内链接不被递归收集。

### TC-01B：千问原生范围与媒体

1. 冻结授权可见集合的 history 页/cursor/空页证据，并逐会话取得分页结束的原生消息。
2. 根据直接用户指令标出排除的原生行和继承字段；保留与排除构成原分母的完整、不重叠分区。
3. 获取保留消息声明的全部媒体并核验 hash；长正文经 prefetched Collector 保存后回读，排除部分
   不进入正式正文或附件，多个媒体 URL 对应同一二进制时保留全部映射而不重复登记 managed path。
4. 破坏 hash、遗漏媒体、重复/外来原生行或未结束分页，确认正式写入前拒绝；禁用 route 不能 start。
5. 新 API acquisition 执行逐项重采；无 fresh handoff 时必须 inaccessible，不以存储快照证明 unchanged。

预期：原生消息及范围位置可精确回读，managed hashes 全部一致；history 无显式终止标志时只声明
观察到的可见集合；原生渲染响应变化导致的 changed 如实保留，不声称全部 unchanged；不启动 C1。

千问只读 `qwen detail-full` 入口另外验证：明确 ID/URL 通过官方 GET 消息接口返回未截断原生行；
错误/缺失 HTTP 状态、业务错误、无分页终止标志、未结束分页、空链或外来会话必须拒绝。
`Complete` 只证明该消息页结束，不证明附件下载或 C0 登记。运行
`node --experimental-vm-modules --test 08_adapters/03_opencli/qwen/full.test.js`，
再用授权测试会话验证真实登录态读取；输出留 Git 外，不发送消息或触发 C1/C2。

### TC-01C：元宝显式原生会话适配器

1. 对授权测试会话完成原生 API 读取与分页，保留 `convs` 行、原生消息身份和 `multiMediaInfo`；
   下载声明的媒体并准备 Git 外 handoff。测试登记使用隔离数据根，不处理历史用户库。
2. 用明确 ID/URL 和 handoff 发现候选；只有选中的会话经统一 Collector 保存。回读原生正文、
   来源 URL、时间、revision 与每项 managed 媒体 size/SHA-256；本地媒体路径不进入正式正文。
3. 缺少媒体、hash/size 错误、外来或重复消息、分页未结束和错误来源 URL 均必须拒绝。
4. 重新读取平台后重采；无新 handoff 时 inaccessible，相同 payload/媒体 unchanged 且不增 revision；
   内容改变时追加 revision，损坏输入不得修改既有成功版本。

预期：正文和媒体均可核对，声明上传大小与平台副本大小分别保留；平台转码不冒称上传原字节无损。
只证明实际覆盖的消息/媒体 shape，不扩张到账号全量或自动启动 C1/C2。

### TC-01D：Kimi 显式清单与完整消息读取

1. 在 Git 外准备 `babata.kimi-chat-manifest/v1` 清单：readiness 为 prepared，account 仅含
   display_name；enumeration 包含 method=`ChatService/ListChats`、captured_at、terminated、
   total_listed 与连续的 pages（page/count/has_next）；chats 仅含 chat_id/title/update_time。
   单独留存原列表响应、工具版本、页首尾 ID 与 token hash；清单本身不保存 token/credential。
2. 校验唯一 UUID/XID、非空清单、页数与条目总数一致、最后一页 has_next=false；破坏 schema、
   readiness、字段、计数、页链、ID 或终止证据必须拒绝；manifest 不枚举账号，也不采集未选对象。
3. 运行 `node --experimental-vm-modules --test 08_adapters/03_opencli/kimi/detail-full.test.js`。
   覆盖显式 ID/URL、长结构化消息、完整 token 分页、protobuf 空尾页、错误 HTTP/JSON、401 有界
   重试、404、身份错配、异常页与重复 token；50 页上限仍有 token 时 Complete=false。
4. Rust provider 测试须拒绝多行、外来 chat、消息数不符和未结束分页；同一有效输入产生相同
   payload hash。404 映射 removed，401/403/缺会话映射 inaccessible，不写新正文。
5. 只有运行时明确启用后，才能在隔离数据根进行 discover/select/save/read-back，覆盖旧 XID
   和 UUID；重采未变化响应不得新增 revision/asset；未选择和原已保存对象不被改写或删除。

预期：单条消息分页结束与账号列表穷尽分开验证；仅元数据的附件不声称二进制完整。引用顺序变化
造成的假修订必须保留为已知缺陷，不能用相同字节输入测试宣称已修复；旧真实运行证据也不能
自动启用 capability。本场景不启动 C1/C2、不发送或删除远端消息。

## 3. TC-02：局部失败、重试与重收集

追溯：`AC-02`。

1. 在一个多对象范围中制造一个可重试失败，保留其他对象成功。
2. 只重试失败对象。
3. 对未变化、变化、不可访问和 removed 对象各执行一次 recollection。

预期：成功项不重跑；failed reason 和 retryability 可见；unchanged/inaccessible/removed 只追加
observation；changed 只追加一个 revision；终端矩阵与冻结分母一致。

对复合 manifest 另做三项 negative：破坏一个附件 hash、移除一个必需资产、改变 replay/document
pairing，均必须在正式写入前 fail closed；用未变化真实 manifest 重采必须 `unchanged` 且无新 revision。

## 4. TC-03：C0、第一方、C1、C2 和 C3 边界

追溯：`AC-03`、`AC-06`。

### TC-03A：原件与派生

1. 登记一个带 asset 的 C0 和一个第一方 Log/Insight。
2. 分别生成 C1 和 C2，并建立 provenance/relation。
3. 删除 C2，再删除可重建 C1，然后从原输入重建。

预期：C0、第一方内容和人工决定未改变；C1/C2 identity、processor/profile 和 hash 可追溯；重建
结果符合合同；C3 job/log 不冒充正式内容。

### TC-03B：事务故障

在 stage、database transaction、asset finalize 和 read-back 各制造一次故障。

预期：没有孤立 ready record、丢失原件或不一致 asset；失败可隔离/重试。

## 5. TC-04：忠实清洗与 C1B

追溯：`AC-04`。

1. 选择文本、文档/网页、图片和音视频真实样本。
2. 生成完整 C1A，并执行 C1B 必要模态判断。
3. 至少包含一个“无需新增媒体”和一个“必须保留媒体”的样本。
4. 制造单项 processor failure 并重试。

预期：完整文字未被摘要替代；媒体判断有内容依据而非配额；model/tool/config、usage、限制和 hash
可回读；失败不覆盖原件或其他结果。

## 6. TC-05：个人知识宇宙

追溯：`AC-05`。

1. 登记 Knowledge、Case、Log、Insight、Course、Branch 和 lens。
2. 创建多 foundation assignment、typed relation、`covers`、动态相关度和 confidence。
3. 提交机器建议，分别接受、拒绝和保持未审阅。
4. 用新 evidence 触发一次真实修订，并执行一次不改变原文的再分析。

预期：身份和关系类型不混用；机器/人工状态可辨别；真实修订追加版本，再分析不伪造 revision；
Log/Insight 可回到时间和 evidence。

## 7. TC-06：第一方创作、批注与并发

追溯：`AC-06`。

验证 create、revise、annotate、cancel 和并发冲突。

预期：作者/Agent 参与、时间、revision relation 和原文可回读；普通批注不制造内容版本；取消和冲突
不留下半成品 ready record。

## 8. TC-07：检索、关系和子库

追溯：`AC-07`。

1. 对混合来源执行字段检索、全文检索和语义增强检索。
2. 从知识导航到案例、日志、课程、外部对象和原证据，再反向返回。
3. 将同一内容纳入两个不同 sublibrary/lens，删除并重建其中一个物化视图。

预期：结果解释命中来源；embedding 不替代稳定 ID；多视图不复制权威；重建不改变定义和上游。

## 9. TC-08：Package、发布和用户视图

追溯：`AC-08`。

1. 从冻结 input scope 和 versioned profile 生成 package。
2. 校验 manifest、hash、标题、正文边界、媒体、链接、图源/位图和 rebuild recipe。
3. 向全新 live 发布，重新 hash read-back。
4. 用户在真实阅读工具中打开并验收；随后从同一 package 重建。

预期：publisher 只复制已验证 package；唯一 live 不反写知识；用户可从输出回到输入；重建一致；
Agent 不替代用户视觉验收。

## 10. TC-09：Skill、脚本和 Agent 边界

追溯：`AC-09`。

1. 分别通过 CLI、Skill、脚本和受保护本地入口调用同一 use case。
2. 验证授权内连续执行、局部重试、取消和状态回报。
3. 尝试越权扩张 scope、直接写数据库、自动启动 C1 和自动接受机器建议。

预期：合法入口得到一致结果；非法动作 fail closed；没有第二 writer 或隐藏持久化；需要登录、不可逆、
安全或价值判断时正确暂停。

## 11. TC-10：Backs 记忆档案与隔离恢复

追溯：`AC-10`。

### TC-10A：Backs inventory 和精确去重

1. 在隔离 fixture/授权小范围创建相同字节多路径、同名不同内容、近似图片和不可靠 timestamp 样本。
2. 生成 pre-clean inventory、强 hash、目录树和结构截屏。
3. 运行 duplicate dry-run 和 oldest survivor 决策。

预期：只有强 hash 相同对象进入 exact duplicate group；同名/近似内容只进入审阅候选；每个 occurrence
保留设备/年份/路径；不可靠最旧时间导致显式 unresolved，而不是静默删除。

### TC-10B：垃圾隔离和 before/after

1. 用版本化规则识别明确缓存、临时文件和程序残留，同时放入容易误判的用户文件。
2. 执行 dry-run，再将已批准候选移入可恢复隔离区。
3. 生成 post-clean inventory、结构截屏和 removal receipt，然后恢复一个候选。

预期：未批准/有歧义对象不移动；每个动作有规则和 before identity；恢复后 hash/路径关系正确；永久
删除没有被自动执行。

### TC-10C：年度归档分析

1. 对授权截图/照片/文本执行 OCR/视觉/文本清洗和时间归属。
2. 生成一个年度 package，包含时间线、日志、主题、感悟和证据索引。
3. 检查所有重要陈述的 source/snapshot 回链，并故意注入一个无证据推测。

预期：有证据内容通过；推测保持 machine suggestion 或被 gate 拒绝；删除/重建年度 package 不改变
Backs snapshot 或正式知识记录。

### TC-10D：备份和隔离恢复

从一致加密备份恢复到全新根，验证 repository、数据库、asset/hash、C1/C2/C3 策略和凭据排除。

预期：C0/第一方损坏 fail closed；声明可重建 C2/C3 缺失进入 missing 清单；活动根未切换；凭据需重授权。

### TC-10E：四根、当前集合与交付位置

1. 用临时四根测试重叠／嵌套、Git 内路径、错分区、相对路径逃逸、junction；非法输入必须拒绝。
2. 合规 legacy workspace 原位通过；将未知／付费成果放入 staging，确认纳入，凭据／浏览器登录态
   显式排除并列理由；历史 receipt 的源 hash 不变。
3. 对四根生成一致 SQLite／逐文件 hash 清单并恢复；注入少文件／hash 错误，关键持久内容必须失败。
4. state 指向另一仓库、snapshot 缺失或旧 state 无位置时不得报健康；交付中断不退出旧好备份。
     历史 SQLite／WAL 原字节与 hash 保留，不按活动库做 header 重写；活动 canonical 库仍经一致 API，
     组内漂移拒绝，故意损坏的历史测试证据不冒充活动库健康缺陷。
5. 真实授权集合另记录冻结范围／截止点、数据库资产关联、工作盘产出和独立盘恢复；
   未授权删除、外部自主库和暂缓对象不动，交付后回执另附保护或下一批补入。

预期：fixture 只证明机制；真实集合／容量／中断与恢复另有证据，不因工程测试关闭数据验收。

## 12. TC-11：完整本地 raw-to-view 闭环

文件协议相关验证另见 TC-10E；工程 fixture 不能替代真实当前恢复集合。

追溯：`AC-11`。

对一个新的明确授权范围，从真实来源或外部 snapshot 开始，连续执行适用 C0/snapshot、C1/C1B、
knowledge/log/insight、verified package、唯一 live、证据回链和隔离恢复，并输出全终端矩阵。

预期：AC-01..10 的适用条件全部有真实证据；不适用、失败、暂缓和缺口显式；用户可从最终视图回到
原始证据；没有 fixture、历史课程或 P0-P9 关闭状态被用来替代本次系统级结果。

当前没有单独的 TC-11 系统级执行回执；本文只定义未来执行方法。

## 13. Checker 和 mutation 规则

### TC-08R：独立调研、PNG 与组合

1. 真实搜索一个明确问题：短词起步、拆子问题、读一手正文后核查；冻结查询／工具／时间／来源／
   读取范围。只返回调研结论，不调用 renderer 或写正式 C0；正文不可读显式失败。
2. 用 run helper 保存并重放相同请求及结果；改同键输入、破坏正文或使用越界路径，必须拒绝；
   新 run 表示刷新，不覆盖旧观察。未完成结论不伪装已核事实。
3. 已有内容独立离线转 PNG，核图片尺寸、内容完整性、中文、表格、页界及视觉结果；
   缺资产、远程资源、危险 HTML、超长／过宽内容和输出冲突必须失败。
4. 正式 record 输出 PNG 后运行 status/verify/delete/rebuild；破坏图片／metadata／上游绑定，
   verify 失败；修复／重建后通过且上游 hash 不变。renderer 失败不得制造成功包。
5. 真实研究结论直接调用渲染生成 PNG，不自动入库。分别记录独立两条路径和组合结果，
   研究核证、机械 QC、视觉检查及用户接受保持分离。

- authority/path/checker 变化必须运行文档 traceability、provenance 和 recovery-hook 检查。
- 关键 fail-closed 规则至少有一个 mutation/negative test，证明故意破坏会被拒绝。
- check 成功只证明输入范围和规则实际覆盖的内容；实时 capability、usage 和用户 acceptance 另行核对。

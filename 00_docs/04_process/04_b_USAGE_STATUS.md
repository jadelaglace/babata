# Babata 使用与交付状态

<!-- DOC-ID: DOC-USAGE -->
<!-- DOC-AUTHORITY-BOUNDARY: usage-status -->

## 1. 文档职责

本文是 Babata **唯一的当前使用与交付状态权威**，回答：实际资料跑到哪里、哪些范围已经
完成、哪些仍未开始或暂缓、当前可用成果是什么、证据在哪里。

本文不定义产品应该具备什么能力，不定义产品验收条件，不定义稳定架构，也不保存完整运行
日志。上游产品定义见 requirements/PRD，完成口径见 AC，交付顺序和 phase gate 见
`04_a_DEVELOPMENT_PROCESS.md`，可重复测试步骤见 TC。运行明细、manifest、数据库、媒体和回执
只留在 Git 外数据根；本文只记录足以判断当前状态的摘要和证据定位。

状态更新必须同时说明范围、完成维度、未完成项和证据。不得用某次试跑、代表样本、模板存在、
代码通过或文件数替代产品能力、整阶段完成或更大范围的使用完成。

## 2. 当前阶段总览

| Phase | 状态 | 当前结论 |
| --- | --- | --- |
| P0 | 已完成 | 冻结旧版本 |
| P1 | 已完成 | 当前 requirements、PRD、AC 和稳定架构建立 |
| P2 | 已完成 | 系统骨架与工程 gate 通过 |
| P3 | 已完成 | C0、first-party 与唯一 writer 底座通过 |
| P4 | 已完成 | 首批真实收集路径通过；不代表全部来源可用 |
| P5 | 已完成 | 真实 C0 -> C1 清洗、失败重试与原件不变通过 |
| P6 | 历史 baseline 已完成 | 关闭时适用的核心沉淀、检索、子库与通用输出 gate 通过；后采用 ontology successor 由 P8.9 单独交付，不倒签 P6 |
| P7 | 已完成 | 扩展来源、统一收集 Skill 与受控 Agent 合同通过 |
| P8 | 已完成 | 来源回收已收束；MBA 13 门 C2B 均已发布，用户已统一验收内容与视觉，13 门课程 closure verifier 全部通过并登记 `accepted / closed` |
| P9 | 已完成 | GitHub private Git 外部目标已完成加密备份同步、唯一一次完整 read-back、隔离恢复和 TC-10；用户已明确免除安卓实测并接受单课 Obsidian 试点 |

Phase 完成只表示该 Phase 关闭时适用的 gate 和 AC/TC 有足够证据，不等于后来新增的 adopted
能力自动实现，也不等于所有来源、资料或未来使用范围都已全量处理。关闭后新增能力进入当前
交付阶段并单独报告 conformance，不倒签或抹除历史 gate。P0–P9 是交付阶段；C0–C3 是数据权威
级别，两者不得混用。

截至 2026-08-20，P0–P9 当前主线全部完成并收官。Web 输出、暂缓来源、更多输出格式、新 UI 等
仍是可选后续，不属于本轮主线遗留，也不得据此重开已关闭 Phase。

## 3. P8 当前使用范围

| 子阶段 | 状态 | 已完成范围 | 仍未完成或明确不做 |
| --- | --- | --- | --- |
| P8.1 | 完成 | 15/15 计划内来源达到最低真实 C0-A1 覆盖 | 不代表 A2、registered 或长期自动化 |
| P8.2 | 范围完成 | 豆包和微信用户授权范围已处理 | 132 个微信群聊按用户决定暂缓 |
| P8.3 | 停止扩张 | 已保留真实 A1 缺口和已耗尽路线 | 暂停新增抓取；不重复失败路线 |
| P8.4 | 完成 | 14/14 纳入来源、70 条样本达到 A2 或已有更高状态 | Bilibili 整体暂缓，不进入分母 |
| P8.5 | 完成 | 70/70 完成 A3 必要性判断；45 条形成 prepared/C0-B | 不代表全来源 A2/B |
| P8.6 | 完成 | 同一 70/70 权威存量进入 registered/C0-C | 未触发新增抓取或 C1 |
| P8.7 | 历史试点 | 本地 MBA 小批 C1 和停队列证据保留 | 不是当前 MBA 权威输入 |
| P8.8 | 完成 | MBA 网站权威 C1 为 763/763，课件 369、视频 394 | 真实数据继续留在外部数据根 |
| P8.9 | 完成 | 财务管理、全球供应链已完成知识治理 successor 登记；13/13 MBA C2B 呈现 v2 迁移完成；用户已于 2026-08-18 统一验收全部课程内容与视觉；13 门课程均已登记 `accepted / closed` | 10 门 closure receipt 均为 `status=passed`，v2 published package/live 逐文件 hash 差异为 0 |

“全量”永远绑定本行声明的明确范围。70/70、763/763 或一门课程完整跑通，都只是对应使用范围
的完成事实，不进入 PRD，也不自动扩大为整个产品、全部来源或全部 MBA 已完成。

## 4. 财务管理当前正式成果

- 正式 C1B：37/37 本质判断、76/76 必要视觉片段，均由 managed C1 登记账本回读。
- 正式 C2B：37/37 semantic entries 与 reviews/branch assignments，归属
  `意识 -> 管理学 -> 财务管理`。
- 当前正式批次：
  `D:\BabataData\04_runtime\staging\model-workspaces\mba-finance-c2b-benchmark-20260815-v17-responsive-map`。
- 唯一用户 live：
  `C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\财务管理\index.md`。
- 用户打开 URI：
  `obsidian://open?vault=Obsidian%20Vault&file=Babata%2FMBA%2F%E8%B4%A2%E5%8A%A1%E7%AE%A1%E7%90%86%2Findex.md`；
  Agent 只启动 URI，不代替用户查看。
- Obsidian profile：`semantic-obsidian/v2 / accepted`；课程内容状态：
  `accepted_benchmark / registered`。
- package/live：92/92，逐文件 hash 差异 0；Wiki/媒体悬空 0。
- 课程呈现 v2：`sectioned`，5 sections / 8 units；每个 source module 恰好绑定一个 unit，
  learning-support 独立于 outline 编号；09/10/11 编号已移除且学习正文 hash 保持不变。
- 课程脑图：原生 Mermaid 为唯一默认展开主图并随窗格自适应；12/12 原生内部链接；
  1488x1920 PNG 保留为默认折叠回退。
- 关闭回执：`<current batch>/closure-verification.json`，schema
  `babata.finance-c2b-formal-closure/v2`，status `passed`。

上述 accepted/closed 继续证明课程内容、媒体、profile、package/live 和用户验收；财务管理的 successor
登记已由核心 writer 追加课程身份、多重基石 assignment、typed covers/map relations 与 MBA lens 并经回读确认。
呈现 v2 迁移只改变可重建的 C2B 视图合同，不重写已接受内容、C1B 或 closure 证据。

财务管理证明的是一套可复用能力、profile 和一门课程的真实使用关闭。后续 MBA 课程必须复用
同一合同重新通过各自输入覆盖、内容质量、媒体必要性、知识宇宙归属、重建和用户验收；不得因
模板或 builder 已存在就把其余课程标成完成。

## 5. 供应链当前正式成果

- 正式课程分母与 C1B：101/101 完整 C1，101/101 本质判断，38/38 必要媒体登记。
- 正式 C2B 核心登记：101/101 semantic entries 为 `registered`，review 为 `accepted`，
  assignment 为 `assigned`，归属 `意识 -> 管理学 -> 供应链管理`；用户已明确认可课程内容与视觉。
- 当前候选批次：
  `D:\BabataData\04_runtime\staging\model-workspaces\mba-supply-chain-c2b-20260815-v5`。
- 唯一用户 live：
  `C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\全球供应链和可持续运营\index.md`。
- 用户打开 URI：
  `obsidian://open?vault=Obsidian%20Vault&file=Babata%2FMBA%2F%E5%85%A8%E7%90%83%E4%BE%9B%E5%BA%94%E9%93%BE%E5%92%8C%E5%8F%AF%E6%8C%81%E7%BB%AD%E8%BF%90%E8%90%A5%2Findex.md`；
  Agent 只启动 URI，不代替用户查看。
- package/live：55/55，逐文件 hash 相等；14 个 Markdown、39 个内容媒体、Wiki/媒体悬空 0。
- 课程脑图：五个知识域、12 个内部章节/学习链接；原生 Mermaid 默认展开，1427x1633 PNG
  作为折叠回退。
- 课程呈现 v2：`sectioned`，5 sections / 8 units；每个 source module 恰好绑定一个 unit，
  learning-support 独立于 outline 编号；09/10/11 编号已移除且学习正文 hash 保持不变。
- 当前状态：用户已明确认可，课程实例为 `accepted / closed`。package/live 保留
  `pending_user_acceptance` 的不可变可重建快照状态；accepted/closed 结果由独立 closure receipt
  和本文记录，避免让 Obsidian 导出成为第二权威 writer。
- 关闭回执：v5 `closure-verification.json`，schema
  `babata.mba-course-c2b-formal-closure/v1`，status `passed`，package/live 55/55、hash 差异 0。

上述 accepted/closed 继续证明课程内容、媒体、profile、package/live 和用户验收；全球供应链的 successor
登记已由核心 writer 追加课程身份、多重基石 assignment、typed covers/map relations 与 MBA lens 并经回读确认。
呈现 v2 迁移只改变可重建的 C2B 视图合同，不清除其他有依据的 assignment，也不改变既有用户验收事实。

## 6. 决策会计当前正式成果

- 正式课程分母与 C1B：33/33 完整 C1、33/33 本质判断、30/30 必要视觉片段登记。
- 正式 C2B：33/33 知识条目已注册，归属新建并回读的 `决策会计` branch；branch id
  `mapnode_01M044RHTAGJRFQMTGE3S6FXEM`。
- 学习正文：4 个章节、课程总览、公式与决策工具、案例练习、复习与自测，共 8 份学习文档；
  学习 manifest 保持 `candidate`，作为 package 内不可变重建输入。
- 当前候选批次：
  `D:\BabataData\04_runtime\staging\model-workspaces\mba-decision-accounting-c2b-20260816-v8`。
- 唯一用户 live：
  `C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\决策会计`。
- package/live：43/43，逐文件 hash 差异 0；raw/derived SQLite `quick_check=ok`，外键错误 0。
- 课程脑图：原生 Mermaid 主图与 PNG 回退均已 materialize；package checker、renderer/materializer
  行为测试和完整工程 gate 通过。
- 当前状态：用户已明确验收内容与视觉，课程实例为 `accepted / closed`。package、manifest 和 live
  保留 `pending_user_acceptance` 的不可变可重建快照状态；accepted/closed 由独立 closure receipt
  与本文记录，避免 Obsidian 导出成为第二权威 writer。
- 关闭回执：v8 `closure-verification.json`，schema
  `babata.mba-course-c2b-formal-closure/v1`，status `passed`，course acceptance `accepted`、
  closure `closed`，package/live 43/43、hash 差异 0。

上述 accepted/closed 证明本门决策会计课程在声明的 33 个模块范围内完成内容、媒体、知识归属、
package/live 和用户验收；不自动扩大为全部 MBA 课程完成，也不改变其他课程的历史状态。

## 7. 执行商务沟通当前正式成果

- 正式课程分母与 C1B：19/19 完整 C1、19/19 本质判断、16/16 必要媒体登记。
- 正式 C2B：19/19 知识条目已注册；学习正文为 8 章、课程总览和 3 份课程专属学习工具，共 12 份。
- 当前候选批次：
  `D:\BabataData\04_runtime\staging\model-workspaces\mba-executive-business-communication-c2b-20260816-v3`。
- 唯一用户 live：
  `C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\执行商务沟通`。
- package/live：33/33，逐文件 hash 相等；14 个 Markdown、17 个媒体文件、Wiki/Markdown 链接检查通过。
- 当前状态：用户已于 2026-08-18 统一验收内容与视觉；课程为 `accepted / closed`。
- 终端证据：`D:\BabataData\04_runtime\staging\execution-rounds\mba-executive-business-communication-20260816-v3\round-ledger.json`，status `passed`、actual terminal `pending_user_acceptance`；发布回执位于 `D:\BabataData\04_runtime\receipts\mba-course-c2b\executive-business-communication\`。
- 关闭回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-executive-business-communication-c2b-20260816-v3\closure-verification.json`，status `passed`，package/live `33/33`、hash 差异 `0`。

该状态证明本门课程的正式 C1B、内容、知识登记、呈现迁移、唯一 live 和用户验收均已闭环；不单独扩大
为其他课程或 Babata 全产品完成。

## 8. MBA 先导课当前正式成果

- 正式课程分母与 C1B：119/119 完整 C1、119/119 本质判断、72 个必要视觉登记；其中 6 个 PDF 页面、66 个视频帧。
- 正式 C2B：119/119 知识条目已注册；学习正文为 6 章、课程总览和 3 份学习工具，共 10 份。
- 当前候选批次：
  `D:\BabataData\04_runtime\staging\model-workspaces\mba-primer-c2b-20260816-v8`。
- 唯一用户 live：
  `C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\MBA先导课`。
- package/live：87/87，逐文件 hash 相等；六域课程脑图含 6 个章节和 4 个学习支持 internal-link。
- 当前状态：用户已于 2026-08-18 统一验收内容与视觉；课程为 `accepted / closed`。
- 终端证据：`D:\BabataData\04_runtime\staging\execution-rounds\mba-primer-20260816-v8\round-ledger.json`，status `passed`、actual terminal `pending_user_acceptance`；发布回执位于 `D:\BabataData\04_runtime\receipts\mba-course-c2b\mba-primer\`。
- 关闭回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-primer-c2b-20260816-v8\closure-verification.json`，status `passed`，package/live `87/87`、hash 差异 `0`。

该状态证明本门课程的正式 C1B、内容、知识登记、呈现迁移、唯一 live 和用户验收均已闭环；不单独扩大
为其他课程或 Babata 全产品完成。

## 9. 创造价值的营销管理当前正式成果

- 正式课程分母与 C1B：70/70 完整 C1、70/70 本质判断、26 个必要视觉登记。
- 正式 C2B：70/70 知识条目已注册；学习正文为 4 章、课程总览和 3 份学习工具，共 8 份。
- 当前候选批次：`D:\BabataData\04_runtime\staging\model-workspaces\mba-value-creating-marketing-c2b-20260816-v2`。
- 唯一用户 live：`C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\创造价值的营销管理`。
- package/live：39/39，逐文件 hash 相等；课程脑图含 4 个章节和 4 个学习支持 internal-link。
- 当前状态：用户已于 2026-08-18 统一验收内容与视觉；课程为 `accepted / closed`。
- 终端证据：`D:\BabataData\04_runtime\staging\execution-rounds\mba-value-creating-marketing-20260816-v2\round-ledger.json`；发布回执位于 `D:\BabataData\04_runtime\receipts\mba-course-c2b\value-creating-marketing\`。
- 关闭回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-value-creating-marketing-c2b-20260816-v2\closure-verification.json`，status `passed`，package/live `39/39`、hash 差异 `0`。

该状态证明本门课程的正式 C1B、内容、知识登记、呈现迁移、唯一 live 和用户验收均已闭环；不单独扩大
为其他课程或 Babata 全产品完成。

## 10. 组织绩效的战略领导力当前正式成果

- 正式课程分母与 C1B：42/42 完整 C1、42/42 本质判断；正式判断为 0 个额外必要媒体。
- 正式 C2B：42/42 知识条目已注册；学习正文为 4 章、课程总览和 3 份学习工具，共 8 份。
- 当前候选批次：`D:\BabataData\04_runtime\staging\model-workspaces\mba-strategic-leadership-c2b-20260816-v2`。
- 唯一用户 live：`C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\组织绩效的战略领导力`。
- package/live：13/13，逐文件 hash 相等；课程脑图含 4 个章节和 4 个学习支持 internal-link。
- 当前状态：用户已于 2026-08-18 统一验收内容与视觉；课程为 `accepted / closed`。
- 终端证据：`D:\BabataData\04_runtime\staging\execution-rounds\mba-strategic-leadership-20260816-v2\round-ledger.json`；v1 中已通过的 8/8 学习正文和 42/42 知识登记被精确复用，未重跑。
- 关闭回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-strategic-leadership-c2b-20260816-v2\closure-verification.json`，status `passed`，package/live `13/13`、hash 差异 `0`。

该状态证明本门课程的正式 C1B、内容、知识登记、呈现迁移、唯一 live 和用户验收均已闭环；不单独扩大
为其他课程或 Babata 全产品完成。

## 11. 组织行为学当前正式成果

- 正式课程分母与 C1B：75/75 完整 C1、75/75 本质判断、107 个必要视觉登记。
- 正式 C2B：75/75 知识条目已注册；学习正文为 5 章、课程总览和 3 份学习工具，共 9 份。
- 当前候选批次：`D:\BabataData\04_runtime\staging\model-workspaces\mba-organizational-behavior-c2b-20260816-v1`。
- 唯一用户 live：`C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\组织行为学`。
- package/live：121/121，逐文件 hash 相等；课程脑图含 5 个章节和 4 个学习支持 internal-link。
- 当前状态：用户已于 2026-08-18 统一验收内容与视觉；课程为 `accepted / closed`。
- 终端证据：`D:\BabataData\04_runtime\staging\execution-rounds\mba-organizational-behavior-20260816-v1\round-ledger.json`。
- 关闭回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-organizational-behavior-c2b-20260816-v1\closure-verification.json`，status `passed`，package/live `121/121`、hash 差异 `0`。

该状态证明本门课程的正式 C1B、内容、知识登记、呈现迁移、唯一 live 和用户验收均已闭环；不单独扩大
为其他课程或 Babata 全产品完成。

## 12. 数据安全、道德和风险管理当前正式成果

- 正式课程分母与 C1B：43/43 完整 C1、43/43 本质判断、47 个必要视觉登记。
- 正式 C2B：43/43 知识条目已注册；学习正文为 5 章、课程总览和 3 份学习工具，共 9 份。
- 当前候选批次：`D:\BabataData\04_runtime\staging\model-workspaces\mba-data-security-ethics-risk-c2b-20260816-v1`。
- 唯一用户 live：`C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\数据安全、道德和风险管理`。
- package/live：61/61，逐文件 SHA-256 零差异；11 个 Markdown、48 个媒体文件，课程脑图含 9 个内部链接。
- 当前状态：用户已于 2026-08-18 统一验收内容与视觉；课程为 `accepted / closed`。
- 终端证据：`D:\BabataData\04_runtime\staging\execution-rounds\mba-data-security-ethics-risk-20260816-v1\round-ledger.json`，status `passed`、actual terminal `pending_user_acceptance`。
- 关闭回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-data-security-ethics-risk-c2b-20260816-v1\closure-verification.json`，status `passed`，package/live `61/61`、hash 差异 `0`。

该状态证明本门课程的正式 C1B、内容、知识登记、呈现迁移、唯一 live 和用户验收均已闭环；不单独扩大
为其他课程或 Babata 全产品完成。

## 13. 管理经济学当前正式成果

- 正式课程分母与 C1B：61/61 完整 C1、61/61 本质判断、55 个必要视觉登记。
- 正式 C2B：61/61 知识条目已注册；学习正文为 5 章、课程总览和 3 份学习工具，共 9 份。
- 当前候选批次：`D:\BabataData\04_runtime\staging\model-workspaces\mba-managerial-economics-c2b-20260816-v1`。
- 唯一用户 live：`C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\管理经济学`。
- package/live：69/69，逐文件 SHA-256 零差异；11 个 Markdown、56 个媒体文件，课程脑图含 9 个内部链接。
- 当前状态：用户已于 2026-08-18 统一验收内容与视觉；课程为 `accepted / closed`。
- 终端证据：`D:\BabataData\04_runtime\staging\execution-rounds\mba-managerial-economics-20260816-v1\round-ledger.json`，status `passed`、actual terminal `pending_user_acceptance`。
- 关闭回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-managerial-economics-c2b-20260816-v1\closure-verification.json`，status `passed`，package/live `69/69`、hash 差异 `0`。

该状态证明本门课程的正式 C1B、内容、知识登记、呈现迁移、唯一 live 和用户验收均已闭环；不单独扩大
为其他课程或 Babata 全产品完成。

## 14. 商业分析当前正式成果

- 正式课程分母与 C1B：51/51 完整 C1、51/51 本质判断、30 个必要视觉登记。
- 正式 C2B：51/51 知识条目已注册；学习正文为 5 章、课程总览和 3 份学习工具，共 9 份。
- 当前候选批次：`D:\BabataData\04_runtime\staging\model-workspaces\mba-business-analytics-c2b-20260816-v3`。
- 唯一用户 live：`C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\商业分析`。
- package/live：44/44，逐文件 SHA-256 零差异；11 个 Markdown、30 个 C1B 媒体文件和 3 个课程脑图资产。
- 当前状态：v2 从干净 staging 完成 67 个一级 digest、2 个有界二级归约摘要、9/9 学习正文与 51/51 知识登记；v3 repair round 的 materialize、package gate 和 publish 三阶段无缺陷通过。用户已于 2026-08-18 统一验收内容与视觉；课程为 `accepted / closed`。
- 终端证据：v2 上游证据位于 `D:\BabataData\04_runtime\staging\execution-rounds\mba-business-analytics-20260816-v2\round-ledger.json`；成功终端位于 `D:\BabataData\04_runtime\staging\execution-rounds\mba-business-analytics-20260816-v3\round-ledger.json`，status `passed`、actual terminal `pending_user_acceptance`。
- 关闭回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-business-analytics-c2b-20260816-v3\closure-verification.json`，status `passed`，package/live `44/44`、hash 差异 `0`。

该状态证明本门课程的正式 C1B、内容、知识登记、呈现迁移、唯一 live 和用户验收均已闭环；不单独扩大
为其他课程或 Babata 全产品完成。

## 15. 全球商业环境当前正式成果

- 正式课程分母与 C1B：38/38 完整 C1、38/38 本质判断、32 个必要视觉登记。
- 正式 C2B：38/38 知识条目已注册；学习正文为 5 章、课程总览和 3 份学习工具，共 9 份。
- 当前候选批次：`D:\BabataData\04_runtime\staging\model-workspaces\mba-global-business-environment-c2b-20260816-v1`。
- 唯一用户 live：`C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\全球商业环境`。
- package/live：46/46，逐文件 SHA-256 零差异；11 个 Markdown、32 个 C1B 媒体文件和 3 个课程脑图资产。
- 当前状态：用户已于 2026-08-18 统一验收内容与视觉；课程为 `accepted / closed`。
- 终端证据：`D:\BabataData\04_runtime\staging\execution-rounds\mba-global-business-environment-20260816-v1\round-ledger.json`，status `passed`、actual terminal `pending_user_acceptance`。
- 关闭回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-global-business-environment-c2b-20260816-v1\closure-verification.json`，status `passed`，package/live `46/46`、hash 差异 `0`。

该状态证明本门课程的正式 C1B、内容、知识登记、呈现迁移、唯一 live 和用户验收均已闭环；不单独扩大
为其他课程或 Babata 全产品完成。

## 16. 战略管理当前正式成果

- 正式课程分母与 C1B：74/74 完整 C1、74/74 本质判断、67 个必要视觉登记。
- 正式 C2B：74/74 知识条目已注册；学习正文为 5 章、课程总览和 3 份学习工具，共 9 份。
- 当前候选批次：`D:\BabataData\04_runtime\staging\model-workspaces\mba-strategic-management-c2b-20260816-v1`。
- 唯一用户 live：`C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\战略管理`。
- package/live：81/81，逐文件 SHA-256 零差异；11 个 Markdown、67 个 C1B 媒体文件和 3 个课程脑图资产。
- 当前状态：用户已于 2026-08-18 统一验收内容与视觉；课程为 `accepted / closed`。
- 终端证据：`D:\BabataData\04_runtime\staging\execution-rounds\mba-strategic-management-20260816-v1\round-ledger.json`，status `passed`、actual terminal `pending_user_acceptance`。
- 关闭回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-strategic-management-c2b-20260816-v1\closure-verification.json`，status `passed`，package/live `81/81`、hash 差异 `0`。

该状态证明本门课程的正式 C1B、内容、知识登记、呈现迁移、唯一 live 和用户验收均已闭环；不单独扩大
为其他课程或 Babata 全产品完成。

## 17. MBA C2B 呈现 v2 全量迁移

- 呈现合同：`babata.mba-course-presentation-plan/v2`；Obsidian profile：
  `semantic-obsidian/v2`。大纲只有两种合法形态：`flat` 的 ordered units，或 `sectioned` 的
  ordered sections -> ordered units；每个 source module 恰好绑定一个 unit，learning-support
  独立于 outline 编号。
- 全量结果：13/13 课程、784/784 文件完成从现有 canonical live/package 的呈现层迁移；10 门
  `flat`，3 门 `sectioned`。sectioned 为执行商务沟通、财务管理、全球供应链，均为 5 sections / 8 units；
  其余课程没有可证明的多 unit 上层主题，保持 flat。
- 迁移边界：只做 index、outline、learning-support 命名和呈现结构的确定性变换；正文 hash 不变，
  `C1 -> C1B -> C2B` 内容、来源绑定、知识权威、课程 accepted/closed 语义不变。旧 flat live 已
  归档，staged successor package 和 manifest 保留，可回建且可审计。
- 独立审计：live/manifest hash differences `0`；09/10/11 numbered files `0`；archived predecessor
  lives `13/13`；retained staged packages `13/13`；source content regeneration `0`、C1B
  registration `0`、knowledge registration `0`、closure verifier `0`。
- 终端回执：
  `D:\BabataData\04_runtime\staging\model-workspaces\mba-course-presentation-rollout-20260817-v3\rollout-receipt.json`；
  中间 v1/v2 失败或不合规回执保留在同一 staging 根下，仅用于追溯，不作为当前状态。

## 18. P9 GitHub 外部备份与隔离恢复

- 外部目标：private repository `jadelaglace/babata-p9-encrypted-backup`，default branch `main`，
  commit `a9ac5e36f0b4d732f69448bc30f7b33a4c7865bb`；只包含加密 restic repository 与 5 个 state
  receipt，不包含 restic 密码或其他凭据。
- 外部副本：536 个 Git LFS 对象、9,386,729,434 bytes；全新隔离 clone 的 557 个 tracked files
  worktree clean，`git lfs fsck` 通过。`source-manifest.json` 自身 SHA-256 为
  `ef6a4f72a60eb356464382cd863a00c92a38bdbf3f425ed32d544af3a2c8c1d1`，其登记的 553/553
  备份文件、9,387,316,740 bytes 逐项 SHA-256 差异 0。
- 从外部 clone 内 repository 执行 restic `check`，5/5 snapshots、packs、indexes、trees 和 blobs
  无错误。最新 Babata snapshot `snapshot_01KYZ2ER57QFJRKD8NR1CPDSM9` 对应 restic snapshot
  `5e646938fd631ff9cda50a4286ed5e663e76e58ba787be283c75537fef3f6cb0`。
- 全新隔离恢复到 `D:\BabataRecovery\recovery\p9-github-external-restore-20260820-v1`，未切换或覆盖
  活动数据根；Babata read-back 为 `state=verified`、2768/2768 files、27,418,034,190 bytes、
  4/4 SQLite `quick_check` / foreign keys 通过，C1/C2/C3 missing 均为空，凭据保持需重新授权。
- TC-10 故障语义通过：篡改一个隔离 C0 文件得到非零 `integrity_failed` 且 `retryable=false`；
  临时移走一个 C2 与一个 C3 文件时仍为 `verified`，只进入各自 missing 清单，不冒充 C0 损坏；
  探针还原后再次全量 clean verify 通过。
- 独立的安卓 Obsidian 单课试点 repository `jadelaglace/babata-obsidian-android-pilot` 保持 private，
  commit `1ad3d4539a2459379970c05166a65a3ee37a3cd5`，组织绩效的战略领导力 13/13 文件经隔离
  clone 逐项 hash 差异 0。用户于 2026-08-20 明确免除安卓设备实测并接受试点；状态登记为
  `accepted_by_user_waiver`，不得表述为手机实测成功。该试点不替代 TC-10，也不重开课程。
- Git 外终端回执：
  `D:\BabataData\04_runtime\staging\p9-github-external-backup-restore-20260820-v1.receipt.json`。

## 19. 试跑、试点、模板与全量使用

| 名称 | 在产品文档中的位置 | 在本文中的状态含义 |
| --- | --- | --- |
| 试跑 / dry-run | PRD 定义产品如何预览范围、成本、限制和预期写入且不冒充成功 | 记录某次试跑实际输入、结果和失败 |
| 试点 / pilot | PRD 定义未普遍启用能力如何有界验证、标记成熟度和退出 | 记录某个真实试点是否通过、覆盖什么、不能证明什么 |
| 模板 / profile | PRD 定义可复用输出合同如何选择、版本化和验收 | 记录某 profile 是否已接受、在哪些真实成果上通过 |
| 全量运行 | PRD 只定义系统能对一个明确授权范围完整处理并逐项报告 | 记录某个具体范围是否真的全量完成、缺口和证据 |

一个试跑结果、试点结果或模板验收可以成为能力成熟度证据，但它们的数字、批次名、日期和
当前完成状态属于本文或运行回执，不属于 PRD/AC/TC 正文。

## 20. 证据索引

| 使用事实 | 主要证据位置 |
| --- | --- |
| P8 来源广度、回收和缺口 | `BABATA_RECOVERY_HOME` 下对应 P8 batch/summary |
| P8.4–P8.6 的 70 条权威存量 | `BABATA_EVIDENCE_HOME` / `BABATA_DATA_HOME` 中对应 manifest、receipt 与 SQLite read-back |
| MBA 763/763 C1 | `BABATA_DATA_HOME` 中 MBA C1 覆盖账和 managed derivatives |
| 财务 C1B 正式登记 | `mba-finance-c1b-registration-20260815-v1/c1b-registration-ledger.json` |
| 财务知识宇宙登记 | `mba-finance-knowledge-universe-registration-20260813-v1/knowledge-universe-registration.json` |
| 财务 C2B 当前关闭 | v17 `manifest.json`、`verification.json`、`closure-verification.json` 与唯一 live |
| 供应链 C1B 正式登记 | `mba-supply-chain-c1b-registration-20260815-v1/c1b-registration-ledger.json`；v2 为 RFC3339 时间修正版证据 |
| 供应链知识宇宙登记 | `mba-supply-chain-c2b-knowledge-registration-20260815-v4/knowledge-universe-registration.json` |
| 供应链 C2B 当前关闭 | v5 `manifest.json`、`verification.json`、`closure-verification.json`、publish receipt 与唯一 live |
| 决策会计 C1B 正式登记 | `mba-decision-accounting-c1b-registration-20260816-v1/c1b-registration-ledger.json` |
| 决策会计知识宇宙登记 | `mba-decision-accounting-knowledge-20260816-v2/knowledge-universe-registration.json` |
| 决策会计 C2B 当前关闭 | v8 `manifest.json`、`verification.json`、`closure-verification.json` 与唯一 live |
| 执行商务沟通 C1B 正式登记 | `mba-executive-business-communication-c1b-registration-20260816-v3/c1b-registration-ledger.json` |
| 执行商务沟通知识宇宙登记 | `mba-executive-business-communication-knowledge-20260816-v3/knowledge-universe-registration.json` |
| 执行商务沟通 C2B 当前关闭 | v3 execution `round-ledger.json`、C2B `manifest.json`、`verification.json`、`closure-verification.json`、publish receipt 与唯一 live |
| MBA 先导课 C1B 正式登记 | `mba-primer-c1b-registration-20260816-v1/c1b-registration-ledger.json` |
| MBA 先导课知识宇宙登记 | `mba-primer-knowledge-20260816-v4/knowledge-universe-registration.json` |
| MBA 先导课 C2B 当前关闭 | v8 execution `round-ledger.json`、C2B `manifest.json`、`verification.json`、`closure-verification.json`、publish receipt 与唯一 live |
| 创造价值的营销管理 C1B 正式登记 | `mba-value-creating-marketing-c1b-registration-20260816-v1/c1b-registration-ledger.json` |
| 创造价值的营销管理知识宇宙登记 | `mba-value-creating-marketing-knowledge-20260816-v2/knowledge-universe-registration.json` |
| 创造价值的营销管理 C2B 当前关闭 | v2 execution `round-ledger.json`、C2B `manifest.json`、`verification.json`、`closure-verification.json`、publish receipt 与唯一 live |
| 组织绩效的战略领导力 C1B 正式登记 | `mba-strategic-leadership-c1b-registration-20260816-v1/c1b-registration-ledger.json` |
| 组织绩效的战略领导力知识宇宙登记 | `mba-strategic-leadership-knowledge-20260816-v1/knowledge-universe-registration.json` |
| 组织绩效的战略领导力 C2B 当前关闭 | v2 repair execution `round-ledger.json`、C2B `manifest.json`、`verification.json`、`closure-verification.json`、publish receipt 与唯一 live |
| 组织行为学 C1B 正式登记 | `mba-organizational-behavior-c1b-registration-20260816-v1/c1b-registration-ledger.json` |
| 组织行为学知识宇宙登记 | `mba-organizational-behavior-knowledge-20260816-v1/knowledge-universe-registration.json` |
| 组织行为学 C2B 当前关闭 | v1 execution `round-ledger.json`、C2B `manifest.json`、`verification.json`、`closure-verification.json`、publish receipt 与唯一 live |
| 数据安全、道德和风险管理 C1B 正式登记 | `mba-data-security-ethics-risk-c1b-registration-20260816-v2/c1b-registration-ledger.json` |
| 数据安全、道德和风险管理知识宇宙登记 | `mba-data-security-ethics-risk-knowledge-20260816-v1/knowledge-universe-registration.json` |
| 数据安全、道德和风险管理 C2B 当前关闭 | v1 execution `round-ledger.json`、C2B `manifest.json`、`verification.json`、`closure-verification.json`、publish receipt 与唯一 live |
| 管理经济学 C1B 正式登记 | `mba-managerial-economics-c1b-registration-20260816-v1/c1b-registration-ledger.json` |
| 管理经济学知识宇宙登记 | `mba-managerial-economics-knowledge-20260816-v1/knowledge-universe-registration.json` |
| 管理经济学 C2B 当前关闭 | v1 execution `round-ledger.json`、C2B `manifest.json`、`verification.json`、`closure-verification.json`、publish receipt 与唯一 live |
| 商业分析 C1B 正式登记 | `mba-business-analytics-c1b-registration-20260816-v1/c1b-registration-ledger.json` |
| 商业分析知识宇宙登记 | `mba-business-analytics-knowledge-20260816-v2/knowledge-universe-registration.json` |
| 商业分析 C2B 当前关闭 | v3 repair execution `round-ledger.json`、C2B `manifest.json`、`verification.json`、`closure-verification.json`、publish receipt 与唯一 live |
| 全球商业环境 C1B 正式登记 | `mba-global-business-environment-c1b-registration-20260816-v1/c1b-registration-ledger.json` |
| 全球商业环境知识宇宙登记 | `mba-global-business-environment-knowledge-20260816-v1/knowledge-universe-registration.json` |
| 全球商业环境 C2B 当前关闭 | v1 execution `round-ledger.json`、C2B `manifest.json`、`verification.json`、`closure-verification.json`、publish receipt 与唯一 live |
| 战略管理 C1B 正式登记 | `mba-strategic-management-c1b-registration-20260816-v1/c1b-registration-ledger.json` |
| 战略管理知识宇宙登记 | `mba-strategic-management-knowledge-20260816-v1/knowledge-universe-registration.json` |
| 战略管理 C2B 当前关闭 | v1 execution `round-ledger.json`、C2B `manifest.json`、`closure-verification.json`、publish receipt 与唯一 live |
| MBA 知识治理 successor 登记 | `D:\BabataData\04_runtime\staging\model-workspaces\mba-course-governance-successor-20260817-v1\registration-receipt.json`；核心 writer read-back 与 SQLite integrity 结果 |
| MBA C2B 呈现 v2 全量迁移 | `D:\BabataData\04_runtime\staging\model-workspaces\mba-course-presentation-rollout-20260817-v3\rollout-receipt.json`、各课程 `migration-receipt.json`、source snapshot manifests |
| P9 GitHub 外部备份与 TC-10 | `D:\BabataData\04_runtime\staging\p9-github-external-backup-restore-20260820-v1.receipt.json`；private repo commit `a9ac5e36f0b4d732f69448bc30f7b33a4c7865bb`；隔离 clone 与 restore 根见 §18 |

历史被否决的候选、旧批次和逐次调参记录保留在 Git 外 staging/archive 和 Git 历史中，只用于
追溯，不作为当前状态或产品定义。本文只在当前事实、范围或证据定位发生变化时更新。

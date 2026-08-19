# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

## 1. 职责与维护边界

本文是 Babata **唯一的当前执行计划与进度控制面**，只保存尚在执行、需要恢复或仍有后续义务的
工作。它回答“下一步做什么、为什么、当前到了哪里、最近形成了什么阶段结论”。产品意图由
Requirements/PRD 拥有，实际交付状态由 `DOC-USAGE` 拥有，完成证据由 TC/运行回执拥有；本文不得
成为这些角色的竞争副本。

新治理输入按 docs-first 规则先进入 `DOC-WORDING-RECOVERY`、当前活动项或下次开工队列。会改变
当前路线的输入直接并入当前活动项；不应打断当前工作的通用待办插入“下次开工队列”顶部。Agent
形成足以改变后续路线的阶段结论、完成一个执行轮终端或发现共享阻断后，必须先更新对应活动项的
临时子计划，再继续大量检索或下一轮操作。

每次恢复边界，包括新 session、Agent/任务交接、Agent 或工具中断、长暂停、上下文压缩，以及用户
明确说“继续”“恢复”“接着做”时，任何状态写入前先核对可用的线程/外部 Goal 与当前活动项。
信息缺失只产生 `unknown`，不得改变 active 身份、优先级或 terminal 状态；“继续/恢复”只授权继续
既有 Goal，不授权选择最近可见子话题。没有明确用户覆盖、合法终端晋升或有依据的授权重排时，
当前 active 默认不可变，`resolved/superseded/closed` 默认不得重开。

恢复时同时核对实时对话、Goal 运行态和本文持久态。若实时对话已有对应结果并明确汇报完成，而
Goal 或本文尚未回写，只补终端状态、证据和清理，不重跑业务动作、收尾、测试或发布；否则才从
当前活动项的“当前状态”和“下一步”继续。已标为完成、已有结果、已终态登记或已按成功终端规则
从本文清理的步骤，不能因摘要、最近消息、旧指令或工具断点再次出现而重跑。

活动项结束时，先把长期有效的决定、产品语义、状态、证据和未决义务提升到各自权威；成功且
无遗留义务的临时活动项随后删除。只自动晋升明确允许 `auto-promote` 的队列项；
`requires-explicit-resume` 项继续留在队列。若没有可自动晋升项，写 `CURRENT-ACTIVE: none` 并等待
用户决定。失败、阻塞、中断或仍需交接时保留并标明终态、原因和恢复入口。本文不积累已完成任务
编年史；Git 历史和正式权威承担长期追溯。

## 2. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20260819-01 -->

### AP-20260819-01：P9 GitHub 私有 Git 与安卓 Obsidian 试点

- 来源锚点：`DFC-20260819-01`；2026-08-19 用户明确启动 Goal，要求完成 P9，并选择一门 MBA
  C2B/Obsidian 导出，通过其 GitHub 私有 Git 让用户在安卓 Obsidian 实机验证取得数据。
- Goal 锚点：Goal API `active`，thread `01a01a89-14ee-7070-81bd-32215a744e08`。
- 状态转换类型：`user-explicit-goal-start`
- 状态转换依据：用户明确启动 2026-08-19 P9 Goal，要求用其 GitHub 建立 private Git，并让其在安卓
  Obsidian 验证一门 MBA C2B/Obsidian 试点；Goal API 已建立同 scope active Goal。
- 当前状态：`in_progress / obsidian-pilot-published / pending-user-android-acceptance-and-full-backup-sync`。
- 用户目标：以 GitHub 私有 Git 作为 P9 外部目标；先用一门已闭环 MBA 课程形成最小真实
  Obsidian 试点，让用户在安卓手机确认能够取得并打开数据；随后完成 P9 外部复制和隔离恢复证据。
- 目标终端：GitHub 外部目标保持 private；一门 MBA 课程的 C2B/Obsidian 文件完整推送且可由安卓
  Obsidian 取得；现有加密备份完成外部纯复制/同步，并从外部副本在全新隔离根恢复和通过 TC-10；
  用户完成手机验收；Usage、证据、Issue/PR 和 Goal 均完成终态回写。
- 不改变/保护边界：不重跑或改写已关闭 MBA 课程；Obsidian/Git 仍是只读可重建输出，不成为
  canonical writer；不把单课手机试点冒充 TC-10；凭据和 restic 密码不得进入 Git；不覆盖活动数据根；
  不提交用户现有 `AGENTS.md`、`README.md` 修改；不扩张 Web 输出或通用运维平台。
- 临时子计划与阶段结论：
  1. [完成] 已核对 P9/TC-10、现有本地加密备份、GitHub 账号和课程 live；外部目标为用户 GitHub
     private storage，安卓试点为“组织绩效的战略领导力”。
  2. [完成] 已建立 Issue #185 和 `codex/185-p9-github-obsidian-pilot`；private repository
     `jadelaglace/babata-obsidian-android-pilot` 已推送课程导出，main commit `1ad3d45`。
  3. [完成] GitHub read-back 和全新隔离 clone v2 已验证：13/13 文件、逐文件 SHA-256 差异 0、
     Git worktree clean；首次 clone 的 10 个换行差异由 `* -text` 后重新索引原始 live 字节修复。
  4. [待执行] 将完整加密备份纯复制/同步到可恢复的 GitHub 外部副本；从该副本执行隔离恢复和
     TC-10 检查。单课仓库和完整备份若受移动端体积或 GitHub 限制，保持逻辑隔离，不降低退出条件。
  5. [等待用户] 用户在安卓 Obsidian 实机确认课程内容、内部链接和课程脑图/PNG 可取得并打开。
  6. [待执行] 回写 DOC-USAGE、DFC、Active Plan、证据和 Goal，按适用 gates 完成 PR。
- 已知阶段结论：P9 权威合同要求外部同步和从外部副本隔离恢复；单课 Obsidian 试点只证明用户读取。
  现有 restic repository 约 9.39 GB、548 个文件，最大单文件约 23.3 MB；当前 GitHub Free 账户有
  Git LFS 使用能力但完整容量尚未实证，不能把未上传的本地备份声称为 P9 完成。试点课程 13 个文件、
  约 333 KB，覆盖 Markdown、内部链接、Mermaid 源和 PNG 回退图；仓库经 API read-back 确认为 private。
- 下一步：用户按 private repository 入口在安卓 Obsidian 完成实机检查并回报；同时继续评估/执行
  完整加密 restic repository 的 GitHub 外部同步和从外部副本隔离恢复。
- 证据入口：`D:\BabataData\04_runtime\staging\p9-github-obsidian-pilot-20260819-v1.receipt.json`；
  隔离 clone 为 `D:\BabataRecovery\recovery\p9-github-obsidian-pilot-20260819-v2`；运行回执不进入 Git。

#### AP-20260816-06 terminal record：MBA 全课程逐门闭环

- 来源锚点：2026-08-16 用户明确恢复“mba所有课程闭环，列好了一课一课跑，统一找我确认”；本次与 Goal API 中同名 active Goal 身份一致；当前验收与恢复授权来自用户 2026-08-18 明确指令。
- Goal 锚点：Goal API 已完成，目标覆盖 MBA 全部课程逐门独立闭环；既有课程顺序与已关闭状态保持有效。
- 状态转换类型：`user-explicit-goal-override`
- 状态转换依据：用户明确覆盖并确认全部 MBA 内容与视觉已验收，授权恢复 Goal、完成最终 Obsidian 模板收尾并提交 PR。
- 当前状态：`completed / final-obsidian-template-and-pr / terminal-writeback-complete`。
- 用户目标：MBA 所有课程按既定顺序一课一课执行；所有待做课程都到达 `pending_user_acceptance` 后，只统一请求一次内容与视觉确认。
- 目标终端：全部 13 门课程分别完成 C1 覆盖、C1B、C2B 内容与知识登记、package/live；用户统一验收后，13 门课程 closure verifier 全部通过并由 DOC-USAGE 记录 `accepted / closed`。
- 不改变/保护边界：不重跑已关闭课程；不重做执行商务沟通已完成的 `19/19` C1B 准备；单课到达 `pending_user_acceptance` 后不打断用户、不等待逐门确认，直接按顺序推进下一门；先导课仍计为独立一课。
- 临时子计划与阶段结论：
  1. [完成] 13 门课程顺序、分母与既有 closure 状态已经冻结；决策会计、财务管理、全球供应链和可持续运营已关闭。
  2. [完成] 执行商务沟通 v3 execution round 无缺陷到达 `pending_user_acceptance`：C1B `19/19`、必要媒体 `16/16`、知识条目 `19/19`、学习文档 12 份、package/live 33/33；唯一 live 已发布，当前不运行 closure verifier。
  3. [完成] 通用 builder/materializer 已改为从课程 plan 解析 `09-/10-/11-` 学习文档并通过课程专属名称定向测试；证据整理合同明确至少 2200 字符，v3 已证明修复后的整轮交付。
  4. [完成] 第 1 门“美国加州多明尼克大学 MBA 先导课”已由权威 coverage audit 证明 C1 `119/119`，分为会计学原理 14、商务英语 16、市场营销 10、管理经济学 46、组织行为学 18、运营管理 15（6 份课件 + 113 段视频）；正式 C1B 已登记 `119/119` 本质判断和 72 个必要视觉，10/10 学习文档与 `119/119` 知识登记完成。修复六域脑图与 kebab-case Mermaid internal-link 后，v8 execution round 的 materialize、package gate、唯一 live 全部通过，87/87 package/live hash 相等，到达 `pending_user_acceptance`；按总 Goal 不运行本课 closure verifier、不单独请求验收。
  5. [完成] “25春 OMBA 5402 创造价值的营销管理”权威 C1 `70/70`；通用 prepare 已修复为按 coverage audit active kinds 合法选择 C1，C1B registrar 也补齐隐藏 runner UTF-8 原生 JSON 边界。正式 C1B 为 `70/70 registered`、26 个必要视觉；v2 execution round 的 8/8 学习文档、70/70 知识登记、39/39 package/live 全部通过，到达 `pending_user_acceptance`。按总 Goal 不运行本课 closure verifier、不单独请求验收。
  6. [完成] “25春 OMBA5404 组织绩效的战略领导力”权威 C1 `42/42`，含课件 13、视频 29；正式 C1B 与知识登记均为 `42/42`，必要视觉为 0，学习正文 8/8。v1 已通过阶段被保留；修复 materializer 的全课程 0-media 空集合汇总后，v2 仅运行 materialize、package gate 和 publish-live，三阶段无缺陷通过，13/13 package/live hash 相等并到达 `pending_user_acceptance`。按总 Goal 不运行本课 closure verifier、不单独请求验收。
  7. [完成] “25春 OMBA5409 组织行为学”权威 C1 `75/75`，含课件 56、视频 19；正式 C1B 为 `75/75` 本质判断和 107 个必要视觉，9/9 学习正文与 `75/75` 知识登记完成。v1 execution round 五阶段无缺陷通过，121/121 package/live hash 相等并到达 `pending_user_acceptance`。按总 Goal 不运行本课 closure verifier、不单独请求验收。
  8. [完成] “25春 MBAO 5411 数据安全、道德和风险管理”权威 C1 `43/43`，含课件 35、视频 8；正式 C1B 为 `43/43` 本质判断和 47 个必要视觉，9/9 学习正文与 `43/43` 知识登记完成。共享 C1 candidate helper 已按内容身份折叠同指纹重复 run、继续拒绝不同指纹分叉，并以 source-map run_id 保持冻结身份。v1 execution round 五阶段无缺陷通过，61/61 package/live 逐文件 SHA-256 零差异，唯一 live 已发布并到达 `pending_user_acceptance`。按总 Goal 不运行本课 closure verifier、不单独请求验收。
  9. [完成] “25春 OMBA 5413 管理经济学”权威 C1 `61/61`，含课件 37、视频 24；正式 C1B 为 `61/61` 本质判断和 55 个必要视觉，9/9 学习正文与 `61/61` 知识登记完成。v1 execution round 五阶段无缺陷通过，69/69 package/live 逐文件 SHA-256 零差异，唯一 live 已发布并到达 `pending_user_acceptance`。按总 Goal 不运行本课 closure verifier、不单独请求验收。
  10. [完成] “25春 MBAO5407 商业分析”权威 C1 `51/51`，含课件 26、视频 25；正式 C1B 为 `51/51` 本质判断和 30 个必要视觉。通用 builder 已实现有界分层归约并通过 5 项通用 MBA dedicated tests；全新 v2 完成 67 个一级 digest、2 个二级归约摘要、9/9 学习正文与 `51/51` 知识登记，越过 v1 字符预算阻断。v2 的 materialize 暴露“可视化/图表”同义 grounding 缺口后，renderer 修复及回归测试通过；v3 repair round 的 materialize、package gate、publish 三阶段无缺陷通过，44/44 package/live 逐文件 SHA-256 零差异，唯一 live 已发布并到达 `pending_user_acceptance`。按总 Goal 不运行本课 closure verifier、不单独请求验收。
  11. [完成] “25春 MBAO 5405 全球商业环境”权威 C1 `38/38`，含课件 25、视频 13；正式 C1B 为 `38/38` 本质判断和 32 个必要视觉。通用 prepare/selector 已增加 plan 级显式 run 冻结合同，在不修改全局历史 run 的前提下解决唯一 OCR 分叉；registration 为 38/38 decisions、32/32 media，0 复用。v1 execution round 五阶段无缺陷通过，13 个一级 digest、9/9 学习正文、38/38 知识登记完成，46/46 package/live 逐文件 SHA-256 零差异，唯一 live 已发布并到达 `pending_user_acceptance`。按总 Goal 不运行本课 closure verifier、不单独请求验收。
  12. [完成] “25春 OMBA 5480 战略管理”已完成全新 v1 execution round：权威 C1 `74/74 covered`（课件 46、视频 28）；正式 C1B `74/74` 本质判断、67 个必要视觉，registration `74/74 decisions`、`67/67 media`、0 复用；9/9 学习正文、74/74 知识登记、81/81 package/live 逐文件 SHA-256 零差异，唯一 live 已发布。五阶段无缺陷到达 `pending_user_acceptance`。按总 Goal 不运行本课 closure verifier、不单独请求验收。
  13. [完成 / 用户 / 2026-08-18] 用户明确确认全部 MBA 课程内容与视觉已验收；不重跑课程内容。
  14. [进行中 / 等待用户统一检查] 用户要求两个问题统一修好后再一次性检查，执行顺序固定为：先只读复核财务管理与全球供应链相对最终知识治理合同的兼容性并修复真实治理缺口；再把已确认的课程大纲/学习支持分层应用到全部 MBA C2B 呈现，更新 course plan、builder/materializer/checker、Obsidian profile/template 与现有 MBA 可重建输出。第一项不重跑已关闭课程；第二项只升级 C2B 呈现合同，不改变 `C1 -> C1B -> C2B` 语义链、正文判断、来源绑定或知识权威。全过程保持 Goal API `blocked`，不运行课程、closure verifier、课程发布验收或无关 Rust 检查；两项及定向验证全部结束后只请求一次用户检查。来源：Issue #180、`DFC-20260817-03`（已 resolved）。
     - [阶段结论 / Agent / 2026-08-17] 两门课的 C1B、C2B 正文、媒体、package/live、用户验收与 closure 证据均保持有效；真实治理缺口是 raw core 的 `courses` 及后继关系表当前为零，历史 ledger 只有单一 branch assignment。既有 core 已具备不可变 Course、typed `covers`、module assignment role/strength/confidence、typed map relation 与 lens membership writer，但 CLI/registrar 尚未暴露，不能以 Docs、Obsidian 或直接 SQLite 写入冒充迁移。剩余路线固定为：先暴露并验证 core CLI；建立版本化 MBA lens；用历史 semantic IDs 追加登记财务管理和全球供应链的独立 Course 身份、covers、基石多重 assignment 与迁移回执；不删除旧 assignment、不重写旧 package/closure。
     - [第一项完成 / Agent / 2026-08-17] core CLI 已暴露 `knowledge register-course/show-course`，并修复 Course assignment 对四个固定基石 ID 的合法支持。通过版本化 MBA lens `sublibrary_01M07ZYC2FY7JFCF2QCCJHENK4`，财务管理与全球供应链已从历史 ledger 追加登记 2 个不可变 Course、138 个 semantic module、799 条带 role/strength/confidence 的 assignment、2 条 typed `covers`、4 条 typed map relation 和 2 条 lens membership；两课 read-back 均保持 `accepted/closed`，SQLite quick check/foreign keys 通过，原 774 条历史 assignment 未删除，旧 package/live/closure 未改。回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-course-governance-successor-20260817-v1\registration-receipt.json`。
     - [第二项完成 / Agent / 2026-08-17] `babata.mba-course-presentation-plan/v2`、`semantic-obsidian/v2` 和 flat/sectioned、unit/source-module、独立 learning-support 合同已沿 authority chain 固化；101-unit 正反 mutation 测试通过。13/13 门课程已从 canonical live/package 完成呈现层迁移并原子发布：10 flat、3 sectioned（执行商务沟通、财务管理、全球供应链，均为 5 sections / 8 units），784/784 文件；live/manifest hash differences 0，09/10/11 numbered files 0，旧 live 与 staged successor package 均保留。正文重生成、C1B 登记、知识登记、closure verifier 均为 0 次。回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-course-presentation-rollout-20260817-v3\rollout-receipt.json`。
     - [定向验证完成 / Agent / 2026-08-17] 17 个变更 PowerShell 文件 parser、presentation governance/materialization tests、13/13 presentation-plan 与 migration checker、intent/plan 与 ontology mutation tests、`check-boundary.ps1`、`git diff --check` 全部通过；Rust 只运行本次改动对应的 Course 固定基石与 CLI course command 两项定向测试，均通过，未运行无关 Rust 全量门禁。
     - [第 3 项短名修正完成 / Agent / 2026-08-18] 已去除 `c2b`、`latest` 字段式后缀，并按用户更正去掉 `25春`、`MBAO/OMBA` 和四位课程号前缀；13 门 live 目录与入口现在只保留 `short_name` 课程本名，不改变内部身份和课程内容。两轮 13/13 live 目录逐文件哈希均保持一致；本轮只做目录/入口短名二次重命名，不重生成正文、不运行 C1B/知识登记/closure、不恢复 Goal。回执：`D:\BabataData\04_runtime\staging\model-workspaces\mba-course-live-display-names-20260818-v2\display-name-migration-receipt.json`。来源：`DFC-20260818-02`。
     - [第 4 项完成 / Agent / 2026-08-18] 最终 Obsidian v2 模板和短展示名已落地；10 门 closure verifier 使用 published migration package 与 live 比对，全部 `status=passed`、hash 差异 0；不重做课程内容或命名迁移。来源：`DFC-20260818-03`。
- 证据入口：知识治理 successor 回执 `D:\BabataData\04_runtime\staging\model-workspaces\mba-course-governance-successor-20260817-v1\registration-receipt.json`；呈现 v2 回执 `D:\BabataData\04_runtime\staging\model-workspaces\mba-course-presentation-rollout-20260817-v3\rollout-receipt.json`；短名迁移回执 `D:\BabataData\04_runtime\staging\model-workspaces\mba-course-live-display-names-20260818-v2\display-name-migration-receipt.json`。
- 下一步：无；PR #183 已通过 gates 并合并到 `main`（merge commit `633578f`）。
- 终态原因：用户已明确完成验收并恢复 Goal；13 门课程 closure、最终 Obsidian 模板、状态写回和 PR 交付均已完成。
- 下一授权/决定：不再恢复课程或重跑内容；`CURRENT-ACTIVE: none`，等待新的明确目标。
- 恢复入口：重新恢复边界仍先查 Goal API 和本 Active Plan；仅凭压缩上下文、历史指令或队列不得执行 MBA 课程。
- 证据入口：执行商务沟通终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-executive-business-communication-20260816-v3\round-ledger.json`；先导课终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-primer-20260816-v8\round-ledger.json`；战略领导力终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-strategic-leadership-20260816-v2\round-ledger.json`；组织行为学终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-organizational-behavior-20260816-v1\round-ledger.json`；数据安全终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-data-security-ethics-risk-20260816-v1\round-ledger.json`；管理经济学终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-managerial-economics-20260816-v1\round-ledger.json`；商业分析成功终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-business-analytics-20260816-v3\round-ledger.json`；全球商业环境终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-global-business-environment-20260816-v1\round-ledger.json`；战略管理终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-strategic-management-20260816-v1\round-ledger.json`，唯一 live 为 `C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\战略管理`。

## 3. 下次开工队列（禁止恢复时自动执行）

队列当前无其他可自动晋升项。

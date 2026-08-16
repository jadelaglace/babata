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

<!-- CURRENT-ACTIVE: AP-20260816-06 -->

### AP-20260816-06：MBA 全课程逐门闭环

- 来源锚点：2026-08-16 用户明确恢复“mba所有课程闭环，列好了一课一课跑，统一找我确认”；本次与 Goal API 中同名 active Goal 身份一致。
- Goal 锚点：Goal API 当前为 `active`，目标覆盖 MBA 全部课程逐门独立闭环；既有课程顺序与已关闭状态保持有效。
- 状态转换类型：`user-explicit-goal-start`
- 状态转换依据：用户明确启动已暂停的 MBA 全课程 Goal，满足本项 `requires-explicit-resume` 条件并授权按既定课程顺序继续。
- 当前状态：`in_progress / mba-course-sequence / strategic-management-course-plan`。
- 用户目标：MBA 所有课程按既定顺序一课一课执行；所有待做课程都到达 `pending_user_acceptance` 后，只统一请求一次内容与视觉确认。
- 目标终端：全部 13 门课程分别完成 C1 覆盖、C1B、C2B 内容与知识登记、package/live；所有待验收课程一次性取得用户内容与视觉确认后，分别运行 closure verifier 并由 DOC-USAGE 记录 `accepted / closed`。
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
  13. [待用户统一验收] 所有待做课程均已发布到 `pending_user_acceptance`；只向用户请求一次全课程内容与视觉确认。用户确认后逐课运行 closure verifier、更新 DOC-USAGE 并关闭全量 Goal。
- 下一步：向用户发起一次全课程内容与视觉统一确认；在确认前不得运行任何课程 closure verifier，不得重跑任何已终端课程，不得运行无关 Rust 检查。
- 证据入口：执行商务沟通终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-executive-business-communication-20260816-v3\round-ledger.json`；先导课终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-primer-20260816-v8\round-ledger.json`；战略领导力终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-strategic-leadership-20260816-v2\round-ledger.json`；组织行为学终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-organizational-behavior-20260816-v1\round-ledger.json`；数据安全终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-data-security-ethics-risk-20260816-v1\round-ledger.json`；管理经济学终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-managerial-economics-20260816-v1\round-ledger.json`；商业分析成功终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-business-analytics-20260816-v3\round-ledger.json`；全球商业环境终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-global-business-environment-20260816-v1\round-ledger.json`，唯一 live 为 `C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\global_business_environment_c2b_latest`。
- 证据入口：执行商务沟通终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-executive-business-communication-20260816-v3\round-ledger.json`；先导课终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-primer-20260816-v8\round-ledger.json`；战略领导力终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-strategic-leadership-20260816-v2\round-ledger.json`；组织行为学终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-organizational-behavior-20260816-v1\round-ledger.json`；数据安全终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-data-security-ethics-risk-20260816-v1\round-ledger.json`；管理经济学终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-managerial-economics-20260816-v1\round-ledger.json`；商业分析成功终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-business-analytics-20260816-v3\round-ledger.json`；全球商业环境终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-global-business-environment-20260816-v1\round-ledger.json`；战略管理终端见 `D:\BabataData\04_runtime\staging\execution-rounds\mba-strategic-management-20260816-v1\round-ledger.json`，唯一 live 为 `C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\strategic_management_c2b_latest`。

## 3. 下次开工队列（禁止恢复时自动执行）

队列当前无其他可自动晋升项。

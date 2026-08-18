# MBA C1B/C2B Rollout

<!-- DOC-ID: DOC-MBA-ROLLOUT -->
<!-- DOC-AUTHORITY-BOUNDARY: delivery-plan -->

## 1. 文档职责

本文定义如何把已验证的 C1B/C2B/profile 模式复用于明确授权的 MBA 课程。它是专项交付计划，
不是 requirements、PRD、AC、architecture、当前优先级或当前状态权威。

- 当前完成到哪门课程、分母和证据：`04_b_USAGE_STATUS.md`；
- 通用产品行为：PRD-04、PRD-05、PRD-08；
- 通用完成口径：AC-04、AC-05、AC-08；
- 可重复测试：TC-04、TC-08；
- output/profile 合同：`../../02_skills/00_specs/09_outputs.md` 和
  `../../02_skills/00_specs/templates/semantic-obsidian-profile.md`。

本文不保存某次模型调用、token/cost、批次名、具体 hash、当前 `N/N` 或用户接受状态。它们属于
Git 外 run ledger/receipt 和 usage status。

## 2. 目标与非目标

目标：

1. 每门课程都从正式完整 C1 开始完成 C1B 本质/模态判断；
2. 生成内容充实、可学习、可复习的 C2B，而不是目录和字段膨胀；
3. 在 C2B 阶段正式登记知识宇宙归属；
4. 通过 accepted semantic Obsidian profile 形成唯一 live 导出；
5. 每门课程独立重建、验收、发布和关闭；
6. 固化可复用 builder/checker，不把旧 C2B 当作新课程输入。

非目标：

- 不重新下载已经完整保留的 MBA 原件；
- 不复制外部主权库的网站目录到 C2；
- 不让 C2 builder 直接从原件补写媒体；
- 不要求每门课程保留同样数量或模态的媒体；
- 不因财务管理通过就批量把其余课程标为 accepted；
- 不在本计划中扩大到未授权、暂缓或非 MBA 来源；
- 不把 Backs 强制洗成 Babata C0/C1；兼容层另行验证。
- 不把一门课程永久等同于一个 branch，不把 MBA 建成单一 discipline，也不为单路径登记删除
  已有且有依据的多重归属；

## 3. 稳定输入与权威边界

```text
外部主权库 / 原件 / 原始目录
        ↓
正式完整 C1
        ↓
C1B：完整文字 + 本质判断 + 必要模态片段
        ↓
正式 semantic entries / reviews / 知识宇宙归属
        ↓
C2B package：正文 + 导航 + profile 产物 + manifest
        ↓
hash-verified publisher
        ↓
唯一 Obsidian live（只读、可删除重建）
```

外部主权库拥有原件和原始目录；managed C1/C1B、semantic core、C2B package、Obsidian export
各有唯一 writer。删除 C1B/C2B/Obsidian 不改变外部原件、C0/C1 或知识关系。

## 4. 每门课程的退出条件

一门课程只有同时满足以下维度才可在 usage status 标为关闭：

| 维度 | 退出条件 |
| --- | --- |
| 输入覆盖 | 课程明确分母内的完整 C1 identity/hash 全部可读 |
| C1B | 每项完成本质/模态判断；必要媒体正式登记，无模态配额 |
| 内容 | C2B 正文可独立学习，不以字段、目录或一两句叶文档冒充内容 |
| 核心登记 | course identity/version、covers、多重 map assignments、semantic entries/reviews 正式可回读 |
| profile | accepted profile 的导航、脑图、链接、媒体和响应式规则通过 |
| 重建 | 全新 staging root，从正式输入重建；旧 C2B 只比较不输入 |
| 轮次 | 冻结轮次到声明终端；终端矩阵、缺陷账和 round receipt 完整 |
| 发布 | package manifest/hash 通过，唯一 live 与 package 一致 |
| 用户结果 | 用户在真实 Obsidian 中完成课程级内容/视觉验收 |
| 关闭 | closure verifier 和边界检查通过，Git 外 receipt 完整 |

任一维度未完成就报告具体 gap，不用一个 `complete` 覆盖。

## 5. 课程级实施步骤

### 5.1 冻结课程分母与本轮合同

1. 从外部主权覆盖账确定课程、课次、课件、视频和正式 C1 分母；
2. 记录 missing、restricted、OCR candidate、低质量和不适用对象；
3. 已存在完整 C1 的对象不重跑；hash/identity 不一致时先修输入权威；
4. 按 `DOC-PROCESS` 的完整执行轮合同记录 round identity、代码/配置身份、全新 staging root、输入
   指纹、阶段列表、目标终端、退出矩阵和 fail-fast 类别。技术候选轮的正常终端是 package、terminal
   gate 与唯一 live 全部完成，状态保持 `pending_user_acceptance`。

课程批次使用 `05_scripts/invoke-babata-execution-round.ps1` 及 Git 外
`babata.execution-round-plan/v1`/`babata.execution-round-ledger/v1` 留证；课程内容 plan、presentation plan、source map、
C1B/knowledge ledger、阶段脚本和其他显式输入必须纳入冻结指纹。round runner 不替代课程 builder、
registrar、checker、publisher 或 closure verifier。

### 5.2 C1B 本质与模态判断

1. 阅读完整 C1，保留完整可读文字；
2. 判断公式/图表/板书/UI 是否需要图片片段；
3. 判断听觉、连续动作、附件属性是否需要音频/视频/附件；
4. 为每个 retained asset 记录来源载体和定位/hash；
5. 文字足够时明确 `no_additional_media`，不凑模态；
6. 经正式 registrar 形成可回读 C1B ledger。

### 5.3 C2B 内容组织

1. 先从课程目标、知识对象和决策链形成章节，不复刻网站顺序；
2. 每章正文必须解释概念、关系、公式、判断、边界和案例；
3. 课程总览、章节导航、公式/工具、案例练习和复习问题服务真实学习；
4. 来源/控制面/模型/存储信息进入 manifest/report，不挤占知识正文；
5. 媒体只挂到确实改变理解的章节，链接无悬空且不重复追加。
6. 用 `babata.mba-course-presentation-plan/v2` 显式选择 `flat` 或 `sectioned`：没有真实上层主题时
   平铺 unit；存在稳定主题阶段、大量小节或长课程时使用 section -> unit。每个 source module 恰好
   绑定一个 unit，但 module 不自动等于 unit。
7. 课程总览和 learning support 与 outline 分层；决策工具、案例练习、复习与自测、视觉证据索引
   使用语义名称并由 plan 排序，不再占用 `09/10/11` 章节编号。

### 5.4 知识宇宙登记

1. 课程从 C2B 阶段正式挂入知识宇宙；
2. 先登记可版本化的 course identity，再登记它对一个或多个稳定 `Branch` 的 `covers` 关系；课程
   identity 与 branch identity 不得合并；
3. 每项 semantic entry/review 及适用的多重 foundation/discipline/branch assignment 经核心 writer
   登记和 read-back；registrar 只增删本轮明确决定的关系，不得用“唯一 course branch”清除其他归属；
4. MBA 使用版本化 Sublibrary/lens 聚合课程，不作为单一 discipline 或唯一父节点；
5. 基石程度若需要记录，使用独立等级/强度与单独置信度，不强制凑 `100%`，不复用动态相关度字段；
6. Obsidian frontmatter、目录或 sidecar 不能代替正式归属；
7. 课程 index 只负责本课，可保持课程内单轴 MECE，不修改宇宙级大 Index。

任何课程 registrar 合同都必须支持本节的 course/branch 分离、typed `covers`、多重 assignment、
MBA lens 和独立基石强度/置信度语义；当前实现覆盖与缺口只查 `DOC-USAGE`，不得写回本计划。

### 5.5 Profile materialization

1. 新呈现消费 accepted `semantic-obsidian/v2`；历史 v1 只作为不可变证据或兼容迁移输入；
2. 生成 package-owned Mermaid 源、同源 PNG、导航和 manifest；
3. 课程图采用清楚主分类轴与用户接受的右向思维导图语言；
4. 图中包含可紧急复习的正文有据知识骨架，不只是目录；
5. 原生 internal-link labels 精确对应当前 package 的 Markdown 目标；
6. Mermaid 为唯一默认展开响应式主图，PNG 默认折叠回退。
7. builder/materializer/checker 都读取同一 presentation plan；目录、文件名前缀和 source module
   数量均不得替代 plan 的 outline。
8. v2 用户入口目录与 index 使用短 `short_name`（课程本名，`Babata/MBA/<short_name>/index.md`）；
   学期、项目和课程号以及 `course_key`、`c2b`、`latest` 仅作为内部元数据，不得出现在用户可见课程目录名中。
9. 兼容迁移逐字复用既有章节/学习正文和媒体，只改授权名称、导航、内链和 manifest/hash，并写
   migration receipt；不运行 C1B、知识正文生成、knowledge registrar 或 closure verifier。

### 5.6 全新重建和预发布验证

每轮使用新的 staging root，依次执行并记录以下观察点：

- input/ledger 覆盖和 hash；
- content density、控制面污染、公式/Markdown 语法；
- retained media、重复视觉段、Wiki/media 悬空；
- 实际 SVG responsive root 和 internal-link selector；
- PNG 可读性、裁切/遮挡；
- manifest file set/hash；
- 第二个新 staging root 的确定性差异；
- 外部主权库、C0/C1/核心关系不变。

本节观察遵守 `DOC-PROCESS` 的完整执行轮和失效性终止规则，不在此复制通用状态机。

### 5.7 发布与人工验收

1. publisher 只复制验证过的 package 到该课程唯一 live；
2. 历史导出移到 Git 外 archive，不作为用户入口；
3. 用户要求打开时只启动当前登记的唯一 live URI 并交还用户；验收前实例保持
   `pending_user_acceptance`，不得要求该 URI 预先具有 accepted 状态；
4. 内容、导航、链接和视觉由用户在真实 Obsidian 中判断；
5. 通过后由 `05_scripts/verify-mba-course-c2b-closure.ps1` 读取明确用户验收证据，独立验证
   package/live、登记账本和数据库完整性，写 Git 外 closure receipt 并更新 usage status；未接受则
   关闭本轮为未接受并保留 gap，进入统一修复集；修复后以全新 staging 和 round identity 重跑，
   只在新轮到达终端时替换同一 live，不对当前 live 逐处打补丁。

### 5.8 终端缺陷收敛与新轮复验

通用终端、缺陷聚类、修复和新轮规则只由 `DOC-PROCESS` 拥有。MBA 专项聚类维度是输入、C1B、
正文生成、知识登记、materialize、脑图、publisher 和 closure；新轮仍不得把旧 C2B/package/live/
sidecar 当内容或状态输入，用户接受后的 closure 不反写不可变 pending package 快照。

## 6. MBA 展开与当前优先级边界

本文不维护课程先后、下一门课或 successor 实现状态。唯一当前优先级和恢复入口只查
`DOC-ACTIVE-PLAN`，已完成课程和当前缺口只查 `DOC-USAGE`。每门课程可以共享代码/profile，但不能
共享完成状态；一个执行轮可以覆盖多门明确授权课程，每门仍独立到终态、验收和关闭。

## 7. 试跑、试点、模板与全量边界

通用定义和运行规则只查 PRD 与 `DOC-PROCESS`。MBA 专项只增加一条：课程全量按冻结课程分母逐项
报告，MBA 全量只在纳入范围内每门课分别关闭后成立。具体结果只写 Usage/run ledger。

外部主权 navigator 是独立 candidate，不属于 MBA rollout；其合同与验证条件只查
`DOC-EXT-SOVEREIGN-NAVIGATOR-CANDIDATE`。

## 8. 变更规则

只有交付顺序、课程级退出条件或稳定复用步骤改变时更新本文。当前数量、批次、模型、成本、
accepted/rejected 实例和用户验收结果只更新 usage status/receipt。若 profile 合同改变，先更新
requirements/AC/TC/spec，再回到本文调整实施步骤。

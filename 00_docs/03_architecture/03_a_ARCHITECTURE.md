# Babata 当前架构

<!-- DOC-ID: DOC-ARCH -->
<!-- DOC-AUTHORITY-BOUNDARY: architecture -->

## 1. 文档职责

本文定义实现 `DOC-AC` 所需的长期数据权威、writer、信任、恢复和模块边界。它不保存 P0-P9
历史、当前来源/课程数量、运行批次、用户验收或尚未采用的详细方案。

## 2. 架构原则

1. **本地优先**：知识数据位于 Git 外 `BABATA_DATA_HOME`；证据、恢复和助理资产按 §7 隔离，代码和可审阅合同位于 Git。
2. **唯一 writer**：正式身份、版本、状态和关系只由 Rust application/core 经 infrastructure 写入。
3. **原件优先**：C0、第一方内容和人工决定不可被模型、索引或输出覆盖。
4. **派生可重建**：C1/C2/C3 按各自合同删除、重建和验证，不反向改变上游。
5. **外围不可信**：来源、浏览器、Skill、脚本、Agent 和模型只产生候选/结果，由核心验证后接纳。
6. **从真实调用生长**：没有真实 caller 不创建仓库、service、协议或跨模块 API。

## 3. 端到端信息流

```text
external source / first-party input
  -> route discovery and candidate
  -> explicit selection or external-library scope
  -> application validation
  -> C0 / first-party writer or versioned external snapshot
  -> C1A extraction + C1B modality judgment
  -> knowledge/case/log/insight/course registration
  -> search and sublibrary projections
  -> verified C2 package
  -> read-only publication/live
```

每个箭头都可以在明确 scope 停止；前一阶段成功不自动授权后一阶段。

## 4. 数据权威与允许 writer

| 数据类 | 权威 | 允许 writer | 删除/恢复边界 |
| --- | --- | --- | --- |
| 外部原件和目录 | 外部主权库 | 外部系统；特定清理 profile 需显式授权 | 默认不改；Backs 见 §11 |
| C0 与 managed asset | Babata raw authority | application/core -> infrastructure | append/revision；完整性关键 |
| 第一方内容与人工决定 | Babata core authority | application/core | 真实修订追加，审阅独立 |
| C1A/C1B | processing authority | application service 验证 processor result 后登记 | 可重建，不覆盖 C0 |
| Knowledge/Case/Log/Insight/Course | knowledge authority | application/core | identity/revision/relation 可追溯 |
| 搜索索引和物化子库 | projection | infrastructure projector | 可删除重建 |
| C2 package/live | output artifact | builder + verifier；publisher 只复制 | 可删除重建，不反写 |
| C3 job/log/temp | runtime | worker/runtime | 临时、可清理、不可冒充产品事实 |
| credential/config | local secure config | 用户授权的本地工具 | 不进 Git、receipt 正文或输出 |
| 个人 WBS 任务当前值 | 用户选定的任务源，当前为飞书原生表 | 授权任务源 native writer，经 `babata-wbs` 内部任务源适配调用 | 无本地任务主库，不从汇总反写 |

个人 WBS 的业务规则、汇报模板和任务源适配都由仓内 `02_skills/babata-wbs/` 拥有，仅一个 Skill
入口。内部 `references/task-source.md` 定义选源、身份、读取／差异写入／回读和错误契约；
`references/providers/feishu.md` 实现飞书映射，使用现成 lark-base／CLI，不另建 feishu-wbs Skill。
私有选源与连接在 `BABATA_ASSISTANT_HOME/solutions/wbs/runtime/task-source.local.json`；缺失、
无效或未实现的 provider 停住有关操作，离线汇总不依赖配置。任务身份包含 provider＋数据集＋native ID，
不把不同源的同名或同 ID 当作同一对象。适配返回字段投影／完整性、原始证据和规范化业务值，
接收明确目标、基线和最小差异，逐项报告写响应与最终回读；具体契约只在该 reference 维护。

选择内部 reference 而非独立 Skill：调用者与维护生命周期相同，替换点是存储实现，不是第二套业务。
将来替换只实现同一窄契约并显式改本机选源，模板与业务规则不变；切源不自动迁移、双写或回退，
迁移须另立明确范围及身份映射。本轮只实现飞书，不建插件加载器、Rust capability、新服务或任务库。
私有配置和快照留 Git 外，旧 Hermes 不共用、不自动同步。

## 5. C0、版本、资产和事务

### 5.1 稳定身份

- source item 表达外部对象身份；revision 表达真实内容版本；observation 表达一次发现/重收集结果。
- 公共来源字段包含 source/provider、stable native ID、URL/path、时间、层级、类型和限制。
- `unchanged`、`inaccessible`、`removed` 只追加 observation；`changed` 创建一个新 revision。
- 第一方 create/revise/annotate 使用相同身份原则，但普通评论、附件或再分析不强制制造 revision。

### 5.2 Provenance 最小集

```text
stable input identity and source locator
input hash / integrity method
revision and observation identity
attachment / dependency references
processor or author identity and version
status, limitation and human/machine attribution
output identity and hash
```

### 5.3 事务序列

```text
validate scope -> stage metadata/assets -> write transaction -> finalize assets
-> read back database and bytes -> ready
```

取消、异常、hash mismatch 或 finalize failure 必须保持非 ready，并允许安全重试或隔离清理。

## 6. 代码与依赖边界

```text
01_domain               stable types, invariants, status and policy
02_application          use cases, request/result contracts and ports
03_infrastructure       SQLite, assets, source/process/output/backup adapters
04_cli                  automation and operational composition root
05_local_api            protected browser/narrow-UI composition root
06_worker               C3 job execution composition root
```

- Domain 不依赖 application/infrastructure/entrypoint。
- Application 只依赖 domain，定义 repository、asset、processor、source、projection、output 和 backup ports。
- Infrastructure 实现 ports，但不拥有产品策略或绕过 application transaction。
- CLI、local API 和 worker 组合相同 services；入口不能直接持久化正式记录。
- JavaScript/TypeScript 用于浏览器和现成生态；Python/PowerShell 用于处理器、确定性批处理和检查；
  它们通过稳定 request/result 或文件合同返回候选，不成为 writer。

## 7. 数据根与运行边界

### 搜索调研与 PNG 原子边界

- `babata-research` 由宿主推理／搜索工具执行查询和核证；配套 run helper 只保存 Git 外冻结请求、
  搜索观察、正文快照、结论及 hash，不是搜索 provider 或正式知识 writer。网页正文仍是外部候选，
  综合结论是 machine 衍生物；研究完成不启用 disabled route 或绕过来源接纳。
- `babata-render` 的浏览器 helper 接受冻结 Markdown/HTML 及限定本地资产，提供 long 与显式
  `.page` 结构分页；页面禁用脚本、网络及登录态，资源经白名单提供，拒绝越界路径。它不是强 OS 沙箱。
- 正式 `OutputKind::Png` 复用 `OutputService`/`OutputViewStore`。Rust 将受控内容交可替换 Node
  renderer，并验证返回 PNG/元数据后写同一 C2 manifest；外围 helper 不写正式库／package。
  凭据和浏览器依赖只在本地配置。renderer 不可用明确返回错误，不静默降级成文字。
- PNG verify 验证冻结产物 hash、渲染元数据及当前权威输入绑定，不调用浏览器重新截图做字节比较。
  明确 rebuild 才创建新 generation，旧 manifest 链保留；只在验证成功后返回 succeeded。
- 临时渲染写 Assistant 工作区，持久研究／视觉证据写 Evidence。重放回读已存结果，输入改变
  拒绝同 run；刷新／新内容用新 run。helper 的局部 receipt 不成为第二知识或输出权威。

### 静态素材加工与视觉检查边界

- `babata-render` 增加静态素材加工，`babata-vision` 负责理解与检查；两者共用 Git 内
  `05_scripts/atoms/image_ops.py`（Pillow＋NumPy，OpenCV 非必需）。它只读输入、写新 run 目录，
  manifest 最后写入；同 run 同请求校验后复用，请求不同拒绝。拒绝 symlink／junction 输入和超像素上限图片。
- 加工命令：keying（白底／色幕）、阴影提取、over 合成、保透明缩放。尺寸不一致默认拒绝，`--fit` 才变换；
  不默认 keep-largest。检查命令：probe、measure（按 profile 解释连通域／半透明／边缘）、diff
  （raw RGBA／alpha 与指定背景可见差异分列）、multibg、tiles（长图分片＋坐标回映）、annotate（写副本）。
- 语义视觉与复杂抠图经 `05_scripts/atoms/vision_backend.py` 窄适配：宿主视觉只记录宿主实际回答，
  `bl vision describe` 与本地 rembg 是可替换后端。适配器校验空／截断／坏格式结果，记录真实模型、耗时和可得
  usage；未配置即 unavailable，不扫网、不静默降级。外传只发本次授权的图片，密钥不进命令行、manifest 或 Git。
- 这些 helper 不写 SQLite、不登记 C0／C1、不成为正式输出 writer；需要正式 PNG 仍走 `OutputService`。
  视觉回答与测量是助理候选，正式接纳仍走既有 clean／writer。动画／视频首版不接入。

活动 `BABATA_DATA_HOME` 顶层保持六个编号分区：

```text
00_inbox/  01_raw/  02_derived/  03_views/  04_runtime/  05_logs/
```

- `BABATA_EVIDENCE_HOME` 保存持久 receipt、manifest、snapshot、verification 和决策，不是正式备份。
- `BABATA_RECOVERY_HOME` 保存待消化／未知唯一恢复材料和恢复工具，不冒充 C0-C。
- `BABATA_ASSISTANT_HOME` 保存非知识的助理方案资产：`solutions/<solution>/runtime/{scripts,cli,...}`
  及 `solutions/<solution>/workspaces/<task-or-run>`，按真实调用增量建立；Git 管理的产品代码仍在 Git。
- 四根必须在 Git 外、互不重合／嵌套；以实际解析路径检查，不追随 junction/symlink 越界。
- `04_runtime` 用于知识处理过程数据；既有 `04_runtime/staging/model-workspaces/` 原位 legacy 兼容，
  不强迁，不因路径名把付费结果、持久证据或未知唯一内容判为可删／可排除。新方案脚本外置 Assistant。
- WBS 的权威仍在飞书，证据进 Evidence/wbs，方案工作区进 Assistant；model-value 同样拆分，
  助理文件只有显式 promotion 经原 Rust writer 才成为正式知识，不直接修改 canonical SQLite。
- 薄 run manifest 可聚合记录身份、producer/build、输入、root alias＋相对路径、size/hash（可 unknown）、
  角色／状态／敏感性／retention／备份分类／依赖／可选晋升，不强迫旧文件逐个补 sidecar。
- 历史回执原字节保留；追加路径终态映射，活动入口适配新路径，历史脚本不得盲执行或全局替换。
- staging、模型 workspace、execution ledger、缓存和日志位于 Git 外；仓库只保存 schema、Skill、模板和 checker。
- SQLite migration 由 application/infrastructure 控制；任何兼容 fallback 必须显式、可测试且不静默降级。

### 文件协议的文档归属

- **模块**在自己的维护范围内管理文档链。已有模块可沿用自己的文档入口，新模块可用
  `modules/<module>/README.md` 做入口。模块拥有自己的需求、行为、设计、验收、进度与证据入口，
  可按需要合在一份或拆开。存量模块随下次相关变更归位，迁移时保留旧入口指针，并标明哪一处是当前权威。
- **总体 Babata**维护跨模块已采纳的共识协议，例如本节四根边界与 §13 的共享恢复约定。
  各模块链接到适用条款。知识宇宙的收集、清洗、沉淀、输出和相关验收归知识宇宙所有。
  在总体文档仍承载存量知识宇宙内容的过渡期，按条款归属引用共享约定。
- **原子能力**可在自己的包或实现旁按需维护技术文档，说明输入、输出、副作用、失败／重试
  和验证方式。调用模块引用其契约并维护自身的组合流程和验收。例如 WBS 模块负责定义
  “按目前进度刷新 WBS”的范围、写回与读回验收，读表和写表原子分别说明自身接口和验证。
- 可版本化的文档随各自维护代码或文档目录入 Git。私有快照、原始回执与运行证据按四根
  规则落盘，文档只保留必要的证据引用。

### 交付与运行维护 Skill 边界

`babata-git` 与 `babata-ops` 是宿主 Agent 组合的独立操作能力，不是知识 writer 或新自治 runtime。
Git refs/index/worktree 由 Git 维护；服务／环境当前状态由目标机 native manager 维护。Skill 只在
用户授权范围调用它们，状态未知先读回，不能承诺外部 exactly-once。四件套继续拥有需求、
架构、代码和设计职责，两个操作包按需返回测试／交付／运行证据，不复制其治理合同。
新包的脚本规范源直接在各自 `scripts/`（单一维护点），验证在 `05_scripts/atoms/`；与其他包
独立，不借用尚未合并的打包重构。标准库 Python 按 PATH／uv 调用，无第三方 Python 依赖。
Git 检查是有界只读观察，覆盖不足须显式非通过；Ops probe 不自动重试、不跟随重定向、不输出
响应正文或非白名单头值。运行证据留 Git 外；秘密发现只记录种类／对象位置，不复制命中字节。
目标版本、权限、回滚和状态核对由 Agent 根据目标环境合同执行，窄脚本不持有写入权限或连接配置。

## 8. 来源、能力与自动化

来源证据和 runtime 状态分开：

```text
evidence: E0 -> E1 -> E2 -> E3
capability: absent | disabled | enabled | unavailable(reason)
object result: success | failed | skipped | changed | unchanged | inaccessible | removed
usage: scope-specific pending | accepted | closed | deferred
```

- `DOC-ROUTES` 保存逐来源正常路线、授权、证据和当前 capability；usage 范围不进入 route registry。
- `babata-collect` 是单一收集入口，内部按 source route/recipe 分发。
- 浏览器或 Agent acquisition handoff 必须绑定来源、会话、完整消息/附件和 hash；临时路径不能进入 ready metadata。
- 浏览器登录态可取得受控页面/API 和已渲染正文，但只能生成 Git 外 prepared manifest；manifest 对
  复合 item 的稳定 source pairing、正文和每个必需资产声明 size/hash，Rust adapter 全量复核后才交唯一 writer。
- Agent 可在授权范围内连续导航、轮询和重试，但不能自动扩张 scope、启动 C1 或接受知识建议。

## 9. C1A/C1B 与 C2A/C2B

### 9.1 清洗

- C1A 是完整、忠实、可检查的文字/结构派生。
- C1B 在 C1A 上增加必要模态判断和保留资产，不是摘要，也没有固定媒体配额。
- processor result 进入核心前校验输入 identity、model/tool/config、状态、usage、限制和 output hash。
- 多个结果可并存；删除 C1 不删除 C0，重建读取同一冻结输入。

### 9.2 输出

- C2A 是通用可重建输出；C2B 是经过内容本质、必要模态、知识归属和 profile 验证的高密度输出。
- C1A 可以在明确要求下生成 C2A；默认新工作优先完成 C1B 判断后生成 C2B。
- C2 package 拥有 manifest、媒体、图源/位图、profile identity 和 rebuild recipe。
- publisher 不渲染、不改正文，只把 hash-verified package 复制到唯一 live。
- 课程 index 只拥有课程内导航，不冒充知识宇宙大 Index。

## 10. 个人知识宇宙

### 10.1 语义模型

- 正式类型至少包括 Knowledge、Case、Log、Insight、Course、Discipline、Branch 和 MapNode。
- 时间、空间、物质、意识是可重叠 foundation assignment；assignment 有依据、强度、confidence 和版本。
- Discipline/Branch 使用有类型多父 DAG；Course 与 Branch 身份分离，通过 `covers` 等关系关联。
- MBA 等跨学科集合是 versioned non-owning lens/sublibrary，不重新拥有内容。
- 兴趣、战略和共识是独立动态相关度；机器建议与人工审阅分别存储。

### 10.2 写入和投影

- core service 创建/修订正式语义身份、assignment 和 relation；projection 只消费已登记记录。
- embedding 用于召回增强，不能替代稳定 ID、来源、显式 relation 或审阅状态。
- sublibrary definition 保存 scope/query/order/profile；materialization 可删除重建。
- Log/Insight 与知识、案例、资料的关系保留时间和 evidence，支持认知轨迹而非静态目录。

## 11. 外部主权库与 Backs

### 11.1 通用候选边界

Babata 已采用“外部系统继续拥有原件和 native writer”的原则，但通用 snapshot/diff/navigator/
promotion 仍未正式产品化。未来窄合同至少需要：

- library/provider/native schema identity；
- stable native object ID、revision、relative path、parent/context、time、size、hash 和限制；
- snapshot ID、parent snapshot、scan scope、cursor、tool version；
- `added/changed/moved/removed/unchanged/inaccessible` diff；
- 可重建导航和可选 promotion relation。

精确重复可以复用内容 identity，但路径 occurrence 和历史 snapshot 不能丢失。

### 11.2 Backs profile

Backs 是首个明确需要“留证后可整理”的外部记忆档案 profile，架构顺序固定为：

```text
pre-clean inventory/hash/tree screenshots
  -> exact duplicate groups and oldest survivor decision
  -> reversible quarantine for exact duplicates and rule-proven junk
  -> post-clean inventory/hash/tree screenshots
  -> OCR/vision/text extraction for informative objects
  -> year/topic/log/insight registration
  -> verified Obsidian 归档分析 package
```

- 自动精确去重只接受强 hash 相同；近似匹配只生成审阅候选。
- “最旧”必须声明使用的时间字段和冲突规则，不能用不可靠 filesystem timestamp 静默决定。
- 删除前后 occurrence、原路径、设备/年份上下文和候选 reason 进入 immutable receipt。
- 垃圾规则版本化并先 dry-run；隔离/回收站优先于永久删除。
- 年度结论必须引用对象和 snapshot；模型推测保持建议身份。

## 12. 输出和 capability

- `outputs` 当前通用 Markdown/JSON contract 与实现可以启用。
- Obsidian 课程交付由专用 profile/publisher 真实证明，但不等于通用 `outputs.obsidian` 已实现。
- `outputs.obsidian` 和 `outputs.web` 的 runtime 状态必须以 capability registry 为准；当前状态见
  `DOC-USAGE`，文档不能自行启用。
- 输出 builder、verifier 和 publisher 分离；verification 不写知识权威，publisher 不接受未验证 package。

## 13. 备份、恢复与安全

- 一致 snapshot 声明数据库、资产、C1/C2/C3 策略和 credential exclusion。
- 备份加密并验证 repository/pack/hash；restore 必须进入独立根。
- 恢复验证包括数据库打开/migration compatibility、integrity/foreign keys、资产存在性和声明范围 hash。
- C0/第一方/人工决定损坏 fail closed；声明可重建的 C2/C3 缺失进入 missing 清单而不冒充 C0 损坏。
- 恢复不自动切换活动根；凭据恢复后重新授权。
- 当前集合以 data/evidence/recovery/assistant root alias 记录来源、开始／结束时间和排除项；
  逐文件稳定拷贝和 SQLite backup API 不等于跨库全局事务。真实冻结需停止相关 writer 并验证库／资产关系。
- 仅活动四个 canonical index 数据库使用 SQLite backup API；历史库／测试库是不可变证据，
  原数据库及 WAL／sidecar 逐字节保留，不因 SQLite 文件头自动改写或按活动健康标准拒绝。
  历史库组必须在无 writer 且组内元数据稳定的冻结窗口复制；漂移即失败，不拼接不同时点的 DB／WAL。
- Evidence、未知 Recovery 和 Assistant 独有成果默认保护；Data staging/logs 不再整体排除。
  密码文件、明确凭据和浏览器登录态单列排除，安全配置／用户成果分开保全；`.env` 不是加密。
- `BABATA_BACKUP_HOME` 是设施，不是第五业务根：工作盘准备／加密／check／隔离 restore →
  独立盘交付／独立恢复验证 → 授权后退出旧历史和工作盘成品。程序变量不随交付改到备份盘。
- state 绑定 repository 路径与 snapshot 实际存在性；旧 state 无位置时记 legacy unknown，不能报健康。
  交付记录与生产记录分开，不伪造本地仍有成品。缺空间不能先删唯一好备份；受控替换另需精确授权。
- 一个当前批次可由同批多根 snapshot 构成；无长期历史，未知历史内容先保全。外部自主库及用户暂缓
  范围为显式例外；交付后新生成 receipt 另附保护或下一批补入，不承诺备份包含自身生成过程。

## 14. 验收追溯

| Acceptance | 主要架构保障 |
| --- | --- |
| `AC-01` | candidate/selection 分离、source port、application writer |
| `AC-02` | object result、observation/revision、幂等重试 |
| `AC-03` | 数据分级、唯一 writer、事务和 provenance |
| `AC-04` | processor port、C1A/C1B、多结果与 hash |
| `AC-05` | knowledge identity、assignment/relation、machine review |
| `AC-06` | first-party revision/annotation/concurrency |
| `AC-07` | projection、search、relation 和 sublibrary definition |
| `AC-08` | package/verifier/publisher 与 read-only live |
| `AC-09` | shared application services、外围信任边界 |
| `AC-10` | 外部数据根、Backs snapshot/quarantine、backup/restore |
| `AC-11` | 贯穿上述边界的真实 raw-to-view evidence chain |

## 15. 保持开放的决定

- Backs profile 是否在真实使用后提升为通用 external-library capability；
- 通用 Obsidian/Web 输出何时出现真实 caller；
- 核心区长期 UI、移动端和桌面端形态；
- 多宇宙和 Program 是否有真实需求；
- CAS、小文件 pack、块级去重或透明压缩是否有实测收益。

开放决定不进入恢复队列，也不因文档存在而成为已实现能力。

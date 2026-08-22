# Babata 当前架构

<!-- DOC-ID: DOC-ARCH -->
<!-- DOC-AUTHORITY-BOUNDARY: architecture -->

## 1. 文档职责

本文定义实现 `DOC-AC` 所需的长期数据权威、writer、信任、恢复和模块边界。它不保存 P0-P9
历史、当前来源/课程数量、运行批次、用户验收或尚未采用的详细方案。

## 2. 架构原则

1. **本地优先**：正式数据位于 Git 外 `BABATA_DATA_HOME`，代码和可审阅合同位于 Git。
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

活动 `BABATA_DATA_HOME` 顶层保持六个编号分区：

```text
00_inbox/  01_raw/  02_processed/  03_core/  04_runtime/  05_logs/
```

- `BABATA_EVIDENCE_HOME` 保存开发/验收证据，不是正式备份。
- `BABATA_RECOVERY_HOME` 保存已验证但尚未 registered 的恢复材料，不冒充 C0-C。
- staging、模型 workspace、execution ledger、缓存和日志位于 Git 外；仓库只保存 schema、Skill、模板和 checker。
- SQLite migration 由 application/infrastructure 控制；任何兼容 fallback 必须显式、可测试且不静默降级。

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

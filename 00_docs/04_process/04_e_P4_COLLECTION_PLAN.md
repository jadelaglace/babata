# Babata P4 首批真实收集交付计划

<!-- DOC-ID: DOC-P4-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: delivery-plan -->

## 1. 文档职责

本文定义 P4 如何用少量真实授权来源验证统一收集闭环。它拥有交付步骤和 P4-G1..P4-G6，
不拥有产品需求、当前 route 状态、来源完成度或历史运行结果。

- 产品行为：PRD-01、PRD-02；
- 完成口径：AC-01、AC-02；
- 稳定 writer/data 边界：`../03_architecture/03_a_ARCHITECTURE.md`；
- 当前路线证据：`../03_architecture/03_d_SOURCE_ROUTE_REGISTRY.md`；
- 当前 phase/来源完成状态：`04_b_USAGE_STATUS.md`；
- 可重复验证：TC-01、TC-02。

任何 dated run、Issue、平台样本数或“已经完成”结论只进入 usage/receipt，不回填本文。

## 2. 目标与边界

P4 要证明至少两条真实授权来源可以走通：

```text
source context
  -> read-only candidate discovery
  -> explicit scope and selection
  -> per-item acquisition
  -> unified Collector/Capture writer
  -> registered C0 read-back
  -> changed/unchanged/inaccessible/removed recollection
```

P4 不是任意文件导入测试，也不要求所有点名来源 enabled。它不激活 C1 清洗、Knowledge、
检索、子库、C2 输出、远程静默全量抓取或第二 writer。

## 3. 路线选择

1. 从 `03_d_SOURCE_ROUTE_REGISTRY.md` 选择已有真实证据且 runtime capability 为 enabled 的路线。
2. 优先官方 API/导出、成熟 CLI/SDK/插件或通用 Agent 工具；仅在已证明缺口上增加窄适配器。
3. 浏览器、桌面控制和平台 CLI 只是 acquisition dependency，不能自行成为 C0 writer。
4. disabled、unavailable、缺失或授权不匹配的路线必须 fail closed。
5. 手工导出、复制、截图和录屏只作恢复或明确回退，不冒充正常日常路径。

具体平台、工具版本、授权证据和当前限制只在 source research/runtime registry 维护。

## 4. 候选、范围与授权

候选至少携带稳定候选 ID、source identity、标题/位置、类型、更新时间、附件可得性和限制。
发现候选是只读动作，不创建 C0。

用户必须选择单项、可见集合、文件夹、会话、时间段或其他明确分母。已登录或已连接只提供
上下文，不等于账号级全量授权。确认后只把所选候选加入同一个 Collector session；未选择、
超出范围或授权不清的候选保持未写入。

## 5. 统一写入与逐项状态

每个候选独立经历：

```text
queued -> running -> saved
                  -> failed
                  -> skipped/cancelled
```

- `saved` 必须返回可回读的 item/revision、原件或正文、资产、hash、来源定位、限制和状态；
- 单项失败不得回滚已保存项，也不得伪造 ready revision；
- 取消只停止尚未开始的项；已保存 C0 不回滚；
- adapter、Skill、浏览器、脚本和临时 Recovery 文件不得直接写 SQLite 或 managed assets；
- 唯一正式写入仍由 Rust application/core 的 Collector/Capture use case 完成。

## 6. 重试与重收集

重试只针对真实失败且仍在原授权范围内的候选，不重新执行成功项。重收集按 item 运行并返回：

- `unchanged`：内容身份不变，不创建伪 revision；
- `changed`：追加新 revision，旧 C0 保留；
- `inaccessible`：记录本次访问失败，旧 C0 保留；
- `removed`：记录来源移除，不删除旧 C0。

临时 delivery 字段、签名 URL 或页面 block ID 不得制造内容变化；fingerprint 规则升级必须有
显式兼容/迁移决定，不能静默把规范化变化称为来源变化。

## 7. 保真与失败边界

- 保存实际正文、原始导出、可得嵌入媒体和明确要求的附件；缺失项逐项记录；
- 不推断原生 ID、层级、作者、时间、父子关系或语义重复；
- 不在收集阶段拆分/合并富文档、做 OCR/ASR、摘要、标签或知识归属；
- fixture 可以验证协议、分页和状态机，但不能替代真实授权 route evidence；
- 对明确分母逐项报告 success/failed/skipped/gap，不能用代表样本冒充全量。

## 8. P4 Gates

| Gate | 可重复通过条件 |
| --- | --- |
| P4-G1 真实候选 | 至少一条真实授权上下文可只读发现候选并展示限制 |
| P4-G2 第二条来源 | 另一种来源/授权形态也能发现并收集明确所选范围 |
| P4-G3 选择性提交 | 未确认不写 C0；确认后只写所选项 |
| P4-G4 逐项状态 | saved/failed/skipped/cancelled、局部失败和重试可观察 |
| P4-G5 重收集 | changed/unchanged/inaccessible/removed 不覆盖旧 C0 |
| P4-G6 证据诚实 | 真实证据与 fixture 分开；未验证路线保持 disabled |

P4 只有在 AC-01/02 和 TC-01/02 对真实授权路径通过后才可关闭。当前是否关闭、由哪些来源
证明、证据在哪里，只查 `04_b_USAGE_STATUS.md` 和 Git 外 receipt。

## 9. 明确不做

- 账号级静默全量复制或无限递归；
- 为单个平台、扩展名或样本建立第二套 C0/C1 pipeline；
- 用 browser visibility、临时下载或 Recovery 文件冒充 registered C0；
- 用 phase 局部成功扩大为全部来源、全部资料或后续 phase 完成。

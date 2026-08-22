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

## 3. TC-02：局部失败、重试与重收集

追溯：`AC-02`。

1. 在一个多对象范围中制造一个可重试失败，保留其他对象成功。
2. 只重试失败对象。
3. 对未变化、变化、不可访问和 removed 对象各执行一次 recollection。

预期：成功项不重跑；failed reason 和 retryability 可见；unchanged/inaccessible/removed 只追加
observation；changed 只追加一个 revision；终端矩阵与冻结分母一致。

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

## 12. TC-11：完整本地 raw-to-view 闭环

追溯：`AC-11`。

对一个新的明确授权范围，从真实来源或外部 snapshot 开始，连续执行适用 C0/snapshot、C1/C1B、
knowledge/log/insight、verified package、唯一 live、证据回链和隔离恢复，并输出全终端矩阵。

预期：AC-01..10 的适用条件全部有真实证据；不适用、失败、暂缓和缺口显式；用户可从最终视图回到
原始证据；没有 fixture、历史课程或 P0-P9 关闭状态被用来替代本次系统级结果。

当前没有单独的 TC-11 系统级执行回执；本文只定义未来执行方法。

## 13. Checker 和 mutation 规则

- authority/path/checker 变化必须运行文档 traceability、provenance 和 recovery-hook 检查。
- 关键 fail-closed 规则至少有一个 mutation/negative test，证明故意破坏会被拒绝。
- check 成功只证明输入范围和规则实际覆盖的内容；实时 capability、usage 和用户 acceptance 另行核对。

# 内部飞书任务源实现

仅在任务源配置 provider=feishu 时读取；这是 babata-wbs 的内部实现，不独立发现或反向路由。
遵守 [内部任务源契约](../task-source.md)，个人业务与汇报模板不在此复制。

## 连接与依赖

- 从任务源本机配置取得 connection.base_token／connection.table_id、identity 和 timezone，
  不在 Skill 硬编码私有表坐标；配置路径和校验以内部契约为准。
- 完整读取宿主安装根的 lark-base Skill；认证／权限问题按其指引读
  lark-shared Skill。命令语法以本轮 help／schema 为准。
- 默认 --as user；用户明确指定才用 --as bot。失败不静默换身份或自动登录，
  不沿用历史权限／token 结论。凭据由 CLI 管理，不写入正文、回执或公开仓库。
- 临时导出／payload 和持久回执遵循 babata-wbs 的 Assistant／Evidence 路径。
  CLI 的 output／@file 必须使用本轮工作目录内相对路径，不向仓库搬私有内容。

## Schema 与字段映射

以下只是已观察字典，不代表本轮 live schema。写入、按类型筛选或建立关联前用
base +field-list 核对当前可写类型、选项与关联目标；业务语义见 babata-wbs。

| 业务信息 | Base 字段与编码 |
| --- | --- |
| 业务标识／标题 | 任务ID／标题，text；native record_id 才是唯一写入主键 |
| 优先级／状态／层次／归档 | 同名 select，CellValue 为选项名数组；精确选项来自 live schema |
| 计划日期 | datetime，带用户时区时间或 Base 时区时间 |
| 进度 | number、percentage 样式；底层 0–1，0.05＝5%、0.5＝50% |
| 摘要／详情／本地目录 | 同名 text；不要用字段缺失或目录名猜值 |
| 循环任务／完成次数 | checkbox／number，次数为整数 |
| 上级关系 | 上级 WBS，本表 link，字段名含空格；对象数组 [{"id":"rec_parent"}] |
| 关联项目 | 跨表 link；仅解析用户明确对象，不猜目标 ID |

层次当前序列化选项为 L1 愿景、L2 大目标、L3长期目标、L4任务；它们的业务默认值
由 babata-wbs 决定。已有选项或字段漂移不能自动改整个结构。用户已明确要求的状态不存在时，
可按 lark-base 的 field-update 指引读取完整字段定义，仅追加该选项并保留其余配置，使用
必要确认 flag；回读 schema 和有关记录，不将暂停偷偷映射为等待。其他结构变化先消歧。

进度曾观察到默认值 0.4；创建时显式 null 也曾被应用默认值。新任务进度未知时，
创建后核对实际值；若被注入无依据默认值，仅在确认目标未被他人更新后对该新记录显式 null
清空并回读。不要改整列默认值或清其他任务；仍不一致则报告冲突／未知，不称未登记。

TOP 的物理表示与父 link 见下文“TOP 与关系编码”。

## 读取与完整性

1. 已知 native record_id 用 base +record-get；未知时按必要字段读取，或用业务 ID 定位并检查重复。
   不把标题当唯一键。结构化筛选用 +record-list --filter-json，不滥用非空关键词搜索。
2. 重复 --field-id 投影需要字段，包括汇总所需的状态、归档、优先级、摘要、百分比、
   日期、父关系和循环信息。状态／归档未投影不同于真实空值。
3. 优先 --format ndjson --output ./records.ndjson；stdout 为 manifest，按其中实际
   record_file 读取行。解析数量必须匹配 records_count。NDJSON 缺省／最大 2000，
   JSON 缺省100／最大200；依 has_more／next_offset 持续分页并汇总。
   未结束、失败或范围不符只能交局部结果。任务少于500条时可全表读取后本地筛选。
4. 向 babata-wbs 提供真实字段值、native 身份、查询范围、用户时区、观察时间、已知 rev 和
   完整性；不要在适配层另造汇总口径或模板。空归档按业务规则解释，不能只筛未归档而漏项。

## 最小写入与回读

- 写前读取目标涉及字段及本轮基线。冲突停受影响项，保留当前值；无条件更新能力就不承诺无竞态。
- base +record-upsert 无 --record-id 是创建，有则更新，不按业务 ID 自动 upsert。
  新建的去重范围和内容由 babata-wbs 决定，解析到本轮 native 身份后只提交差异字段。
- --json 顶层是字段映射，不包 fields；不写系统、formula、lookup、attachment。
  同表串行提交；JSON 用序列化工具写 @file，不靠手工拼接或禁止引号。
- 业务 dry-run 只运行读取与带 --dry-run 的写预览。请求构造通过不证明真实鉴权、表中查重
  或写后状态，不自动转实写。
- 写响应先保存；本轮有关变更完成后统一回读目标字段。异步延迟最多有限重查，例如3次。
  成功响应不等于最终可见；失败／超时结果不明先查现状，不重建或重复加计数。
  误写保留前后值，已知目标和无并发时才做窄修正，不盲目回滚覆盖未知原值。
- 批次逐项报告 changed／unchanged／conflict／failed／unknown，区分响应、最终回读与用户验收；
  不假装跨记录事务。返回真实值给 babata-wbs 按统一格式交付。

## CLI 编码要点

- Select：{"状态":["暂停"]}；父 link：{"上级 WBS":[{"id":"rec_parent"}]}。
  shell 写 --field-id '上级 WBS'。native ID 先解析，不猜。
- 日期写带时区值；按天用 ExactDate(YYYY-MM-DD)，闭区间按当前日期过滤协议，
  不把数字 >=／<= 直接套在日期字段。
- filter conditions 保持嵌套元组；PowerShell 检查数组未压平。null 可在允许字段清空，
  select 空数组可清数组；真实结果仍需回读，尤其创建默认值。
- 固定 sleep 不能证明正确；当前 CLI help 和 lark-base 负责完整协议，本文件不复制 CLI 手册。

## TOP 与关系编码

### 摘要前缀

当前人工顺序编码在摘要开头，不新增排序字段：

- 主线：[TOP n/M]；历史记录也有无空格 [TOPn/M]，读取应同时识别。
- 占位前置：[TOP n/M · 前置]。
- 父任务顶置：[TOP n/M · 顶置]。
- 子任务：[TOP n · 子任务]，不带分母，不额外计主线名额。

向业务层返回 n、M 和标记；没有 TOP 不补值。按授权改序时仅替换前缀，
保持原摘要正文；解析模糊时返回原文和缺口，不能猜组成员。

### 关系字段

上级 WBS 才是本表实际父 link，单元格为 native record_id 对象数组。
解析父任务业务 ID 后取得真实 native ID；多父关系保留全部无关成员。
摘要 TOP 标记不自动建立 link。父子显示和跨组去重交给 babata-wbs。

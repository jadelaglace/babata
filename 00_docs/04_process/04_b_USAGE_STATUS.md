# Babata 当前使用状态

<!-- DOC-ID: DOC-USAGE -->
<!-- DOC-AUTHORITY-BOUNDARY: usage-status -->

## 1. 文档职责

本文是唯一当前使用、交付和未完成状态权威。它只保留会影响下一项用户决定的摘要和 evidence
入口，不复制完整 execution ledger、课程级 receipt 或历史调参过程。

详细 P0-P9 历史见 `DOC-P0-P9-ARCHIVE`；具体数字和 hash 以 Git 外 receipt 为准。

## 2. Release 基线

| 项目 | 当前值 |
| --- | --- |
| 生命周期 | `public-beta / usage-stage` |
| 产品版本 | `0.1.0` |
| release tag | annotated `v0.1.0` |
| release commit | `58d5ce62819e82f1506068b6ff0ddbdefc13877a` |
| 唯一版本源 | `01_app/Cargo.toml` `[workspace.package].version` |
| GitHub Release | 不创建；当前只维护 Git tag |

后续 execution round/关键 receipt 必须记录 version、tag、commit、dirty、profile/config、provider/model/
tool version、调用量、价格依据和实际或估算成本。

## 3. 当前能力边界

运行状态必须以 `babata --json capabilities list` 和 `DOC-ROUTES` 为准；本文只记录当前用户决策所需
的汇总。

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| C0/first-party core writer | `enabled` | text/file/export/first-party 的正式身份、版本、asset 和 read-back |
| C1 处理与登记 | `enabled` | provider-neutral 清洗入口和实际文本/多模态使用已证明 |
| Knowledge/search/sublibrary | `enabled` | 核心登记、关系、检索和通用物化能力已有真实使用 |
| `outputs` Markdown/JSON | `enabled` | 通用输出合同与实现存在 |
| 专用 MBA/Cherno Obsidian publisher | `proven-specialized-route` | 已真实交付，但不是通用 capability |
| `outputs.obsidian` | `unavailable / unplanned` | 不因专用 publisher 倒签 |
| `outputs.web` | `unavailable / unplanned` | 未实现 |
| external snapshot/diff/navigator | `candidate / not-adopted` | Backs 可先做有界 profile |

## 4. P0-P9 收官状态

P0-P9 主线已完成并由用户接受，当前不再作为活动 roadmap。它证明 Babata 已建立本地 raw-to-view
能力、真实来源/C1/知识/输出使用和外部隔离恢复；不证明全部来源、通用 Obsidian/Web 或 TC-11
独立系统回执已经完成。

收官归档：`DOC-P0-P9-ARCHIVE`。P0-P9、MBA、Cherno、P9 和 TC-10 不得从恢复流程自动重跑。

## 5. 来源与存量使用摘要

| 范围 | 当前结果 | 仍未完成或不在分母 |
| --- | --- | --- |
| P8 来源最低真实广度 | 15/15 计划内来源达到最低 C0-A1 覆盖 | 不代表 A2、registered 或长期自动化 |
| P8 有界深度样本 | 14/14 纳入来源、70 条样本达到 A2 或更高 | Bilibili 整体暂缓 |
| P8 主权准备/登记 | 70/70 A3 判断；45 prepared/C0-B；70/70 registered/C0-C | 不代表全来源都达到同一深度 |
| 微信/豆包授权回收 | 用户授权范围已处理 | 132 个微信群聊用户暂缓 |
| 新来源扩张 | 已保留真实缺口和已耗尽路线 | 当前暂停，不重复失败路线 |

具体 route evidence、授权和 runtime status 见 `DOC-ROUTES`；具体输入和 receipt 在
`BABATA_RECOVERY_HOME`、`BABATA_EVIDENCE_HOME` 和 `BABATA_DATA_HOME`。

## 6. MBA 当前成果

- 13/13 MBA 课程完成正式 C1/C1B、知识登记、必要媒体、C2B package、唯一 live 和用户验收。
- 全部课程使用 `semantic-obsidian/v2`；用户于 2026-08-18 统一接受内容与视觉，状态为
  `accepted / closed`。
- 呈现 v2 已完成 13/13 课程、784/784 文件迁移；10 门 flat、3 门 sectioned，正文和上游身份未重写。
- 唯一 live 根：`C:\Users\Aiano\Documents\Obsidian Vault\Babata\MBA\`。
- 课程级 package/live 数量、hash、closure receipt 和旧状态的精确详情由冻结 commit `9182a77` 的
  旧 `DOC-USAGE` §4-17 与 Git 外 receipts 恢复，不在当前状态页复制。

专用 MBA route 的成功不把 `outputs.obsidian` 提升为通用 enabled。

## 7. Cherno 当前成果

- 三套课程已由用户验收并关闭：C++ 115、OpenGL 31、Game Engine 123，合计 269/269。
- 6 个真实缺集已补齐；source manifest 为 6 `exact` + 263 `high`，0 unresolved。
- YouTube `video_id` 是稳定身份；用户可见文件名/H1/链接使用 `L###-<可读内容或操作名称>`。
- 269/269 C0 和逐项 SHA-256/大小 read-back 通过；全量 ASR、538 个 C1 derivative、269/269 C1B、
  269/269 知识登记和 3/3 课程计划完成。
- 原 MP4 保留来源帧率且未覆盖、未统一重编码；31 个旧 DOCX 在替代链和删除清单冻结后移入回收站。
- 三套 `semantic-course-obsidian/v1` package 已发布到唯一 live 并由用户接受，状态 `accepted / closed`。
- 唯一 live 根：`C:\Users\Aiano\Documents\Obsidian Vault\Babata\Cherno\`。

成本记录：ASR 按北京地域 0.00022 元/秒估算上限 93.173341 元，provider 未返回实际 usage；Qwen
课程学习为 4,742,450 input + 319,246 output tokens，调用时估算 13.315852 元，免费额度抵扣 unknown。

## 8. P9 与恢复当前成果

- private backup repo：`jadelaglace/babata-p9-encrypted-backup`；外部 commit
  `a9ac5e36f0b4d732f69448bc30f7b33a4c7865bb`。
- 553/553 备份文件 SHA-256 一致；536 个 Git LFS 对象通过 `git lfs fsck`；5/5 restic snapshots 完整。
- 外部隔离恢复 2768/2768 文件、27,418,034,190 bytes；4/4 SQLite quick_check/foreign keys 通过。
- TC-10 的 C0 fail-closed 与 C2/C3 missing 可重建语义通过；活动数据根未被切换或覆盖。
- 安卓 Obsidian 试点是 `accepted_by_user_waiver`：用户免除设备实测，不能表述为手机实测成功。

终端 receipt：`D:\BabataData\04_runtime\staging\p9-github-external-backup-restore-20260820-v1.receipt.json`。

## 9. 当前开放、暂缓和未开始

| 项目 | 状态 | 下一授权条件 |
| --- | --- | --- |
| P0-P9 Docs 收束 | `completed / merged` | PR #194；merge `aaa4d0fe18c378cb5e58751f1eb981608673466e`；旧全文冻结于 `9182a77` |
| Backs 记忆档案与年度归档分析 | `adopted requirement / not-started` | 本轮 Docs 终端后由用户明确启动新的 usage/development scope |
| 132 个微信群聊 | `deferred-by-user` | 用户明确恢复 |
| Bilibili | `deferred-by-user` | 用户明确恢复 |
| 通用 external navigator | `candidate / not-adopted` | Backs 有界试用证据和后续采用决定 |
| 通用 Obsidian/Web | `unavailable / unplanned` | 新产品目标和真实 caller |
| TC-11 独立系统级 receipt | `not-run` | 用户明确提出系统级终验；不能从缺口自动重开主线 |
| 新 UI、桌面/移动端、更多输出 | `open / optional` | 真实使用需要 |

这些项目不是 Active Plan 队列，不能在 session 恢复时自动执行。

## 10. 主要证据入口

| 使用事实 | Evidence 入口 |
| --- | --- |
| 整理前完整 Docs | Git commit `9182a77` |
| P8 来源广度、深度和登记 | `BABATA_RECOVERY_HOME` / `BABATA_EVIDENCE_HOME` 对应 P8 manifests/receipts |
| MBA 13 门课程 | `D:\BabataData\04_runtime\staging\execution-rounds\` 和 `model-workspaces\mba-*` |
| MBA 呈现 v2 | `mba-course-presentation-rollout-20260817-v3\rollout-receipt.json` |
| Cherno C0-C2B | `model-workspaces\cherno-course-*-2026082*-v1`；最终 `cherno-course-c2b-full-20260822-v1` |
| P9/TC-10 | `p9-github-external-backup-restore-20260820-v1.receipt.json` |

旧失败批次和旧 candidate 只用于追溯，不作为当前产品定义或状态。

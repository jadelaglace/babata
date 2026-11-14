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
| 产品版本 | `0.2.0` |
| release tag | annotated `v0.2.0` |
| release commit | 以 `v0.2.0^{}` 解析结果为准；本文不维护第二份 commit hash |
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
| 搜索调研／PNG 双原子（#233） | `implemented / local-verified / unreleased` | 宿主实际搜索＋可恢复研究 run；独立 Markdown/HTML 转图及组合已实测。正式 PNG 接既有 outputs，需本机 renderer／浏览器配置；不是新增搜索服务或自动入库。独立 PR 交付不自动 merge/release，证据见 §11 |
| 专用 MBA/Cherno Obsidian publisher | `proven-specialized-route` | 已真实交付，但不是通用 capability |
| `source.yitang` | `enabled` | 成对回放、渲染全文和正文图片的 prepared-manifest Collector 路线已通过真实样本并随 v0.2.0 发布 |
| `source.qianwen` | `enabled`（当前构建） | 原生对话与媒体 C0 回收及外卖范围修正已验证；新增只读 OpenCLI `qwen detail-full` 适配器（`08_adapters/03_opencli/qwen/full.js`），两个测试会话已由 detail-full 真实读取；入口 PR #222 已合并、Issue #221 已关闭，PR #208 已合并，适配器尚未随新版本发布，不属于 v0.2.0 release 基线 |
| `source.yuanbao` | `enabled`（当前构建） | 显式原生 handoff 接统一 Collector，文字/图片保存与 fresh 重采已在用户提供的两个测试对话验证；PR #218 已合并；不是账号全量任务，尚未随新版本发布 |
| `source.kimi` | `disabled` | 显式 manifest/UUID/XID 读取及分页校验已随 PR #202 合并，Issue #201 已关闭；当前本地 full/boundary 与 GitHub 三类检查通过，未随新版本发布。账号取回与用户验收见 §5/§11；引用顺序假修订、会话稳定性、附件二进制与长期重采缺口由 #229 保留，合并不等于启用 |
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
| 千问授权可见集合 C0 回收 | `completed / C0 registered`；观察到 23 会话，排除外卖后保留 22 会话/42 条原生行；排除 124 条外卖相关行、6 处继承上下文，含后补识别的两条饮品活动二维码及对应图片 | 仅覆盖观察到的可见集合；旧历史版本与完整采集留存，未物理清除；C1 未启动，用户验收不推断 |
| 元宝全量对话取回与正式登记（2026-11-08 复核） | `completed / C0 registered with source fidelity limits`：已取回 API 可见范围 199/199 个会话、790 条原生消息，经既有通用 export/attach-assets writer 登记；199 份原始 JSON、82 份唯一媒体（79 图片、2 PDF、1 TXT）、129 个 URL 映射及 20 份媒体映射清单全部回读核验 | 正文缺口 0；16 个资源 ID（13 张唯一图片）仅证明平台副本，未证明上传原字节无损；不声称服务器未暴露历史完整。该批次使用通用导入；后续适配器开发仅用两个独立测试对话，未重跑此历史批次；未启动 C1/C2，回执见 §11 |
| 新来源扩张 | 已保留真实缺口和已耗尽路线 | 当前暂停，不重复失败路线 |
| Kimi 账号历史取回 | `accepted / closed`：冻结账号清单 339/339 已取回，缺口 0；2578 条消息。另保留 10 条用户取回后从来源删除的旧对话，最终 349 篇阅读笔记、371 文件；2024/2025/2026 为 2/69/278 篇。用户 2026-11-03 已验收，后续仅按指令修正呈现 | 附件二进制未下载，61 处引用未解析，未做来源事实核验/C1B/C2B；已知缺陷不被验收覆盖。原始及修正回执不改写；本次只补工程/docs 收尾，不重跑历史数据 |

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

## 8. 一堂当前成果

- 用户明确给出的《AI×知识管理》开放麦回放/文稿 pair 已完整取得；后续系列仍逐项等待用户授权。
- 完整虚拟滚动得到 666 个原生 block、398 个非空 block、27 个标题和 110 张正文图片；回放为
  9,824.015 秒、2,649,314,076 bytes，来源和全部保留资产 size/SHA-256 回读通过。
- `source.yitang` 将 replay sid + document token/ACL 绑定为一个复合 item；115/115 assets 已保存，
  缺失、hash 漂移、pairing 漂移、加密正文替代或未声明附件均 fail closed。
- MPMux 与 HLS/ffmpeg stream copy 都可作为 acquisition 方式，但必须归一为本地 MP4 + duration +
  size/SHA-256 manifest；本样本已用 ffmpeg 无损取得，不重复下载。
- 大文件整读和跨 session 伪 revision 两个真实缺陷已在 Issue #196 范围内修复并复验；修复后峰值
  工作集低于 20 MB，新 session 和显式 recollect 都保持 revision 数不变。修复前产生的 ordinal 2
  revision 作为缺陷证据保留，不删除或伪装成来源变化。
- 该单课后续使用轮已完成完整 C1/C1B：9,824 秒音频得到 1,571 句 transcript 与脱敏 structured ASR，
  最后一句距音频尾部 30 ms；C1B 从 110 张原生文稿图片中保留 4 张不可由正文等价替代的架构图。
- Qwen 细粒度知识与课程总览建议均已接受；课程 `course_01M0PVXA89E4BSD24HHV0NXRRM`、专用课程镜头和
  materialization 已登记验证，课程状态 `accepted / closed`。
- `semantic-course-obsidian/v1` package 已发布到唯一 live
  `C:\Users\Aiano\Documents\Obsidian Vault\Babata\一堂\AI×知识管理开放麦\`；13/13 文件、
  12 个内部链接、4 个媒体链接及 package/live SHA-256 全部通过。
- 本轮 ASR 按北京地域 0.00022 元/秒估算 2.16128 元；Qwen 返回 17,670 input + 2,655 output
  tokens，按调用时 `qwen-plus` 非思考价格估算 0.019446 元；合计估算 2.180726 元。provider 未返回
  实际扣费，免费额度抵扣 unknown。
- 专用 Obsidian publisher 的成功不把通用 `outputs.obsidian` 提升为 enabled；后续一堂系列仍逐项
  等待用户明确 replay/document pair。

证据：`D:\BabataData\04_runtime\staging\model-workspaces\yitang-ai-knowledge-management-source-20260823-v1`；
闭合终端：`D:\BabataData\04_runtime\staging\model-workspaces\yitang-ai-knowledge-management-clean-c2b-20260823-v1\terminal-receipt.json`。

## 9. P9 与恢复当前成果

- private backup repo：`jadelaglace/babata-p9-encrypted-backup`；外部 commit
  `a9ac5e36f0b4d732f69448bc30f7b33a4c7865bb`。
- 553/553 备份文件 SHA-256 一致；536 个 Git LFS 对象通过 `git lfs fsck`；5/5 restic snapshots 完整。
- 外部隔离恢复 2768/2768 文件、27,418,034,190 bytes；4/4 SQLite quick_check/foreign keys 通过。
- TC-10 的 C0 fail-closed 与 C2/C3 missing 可重建语义通过；活动数据根未被切换或覆盖。
- 安卓 Obsidian 试点是 `accepted_by_user_waiver`：用户免除设备实测，不能表述为手机实测成功。

终端 receipt：`D:\BabataData\04_runtime\staging\p9-github-external-backup-restore-20260820-v1.receipt.json`。

## 10. 当前开放、暂缓和未开始

| 项目 | 状态 | 下一授权条件 |
| --- | --- | --- |
| P0-P9 Docs 收束 | `completed / merged` | PR #194；merge `aaa4d0fe18c378cb5e58751f1eb981608673466e`；旧全文冻结于 `9182a77` |
| #233 搜索调研与 PNG 原子 | `implemented / local-verified / PR-pending` | #210 / Epic #203 第一条；两个原子可独立及组合，本地 .agents 已安装，旧报告入口复用。Rust 1.100.0 全套门禁通过，新增重放边界补回归；远端检查及 PR 状态随后回写。未合并、未发布，宿主自动选择和用户验收不推断 |
| 一堂系列后续内容 | `v0.2.0 route released / future scope not-authorized` | 用户提供下一组明确 replay/document pair |
| Backs 记忆档案与年度归档分析 | `externalized / closed` | 最终自主库为 `E:\Backs`：12 个快照、34,701 个文件、216,126,221,452 个声明字节通过终态审计；`_VAULT\Obsidian` 保存年度阅读导航，`_records` 保存原始目录树和 34,701 条 occurrence，`managed-records` 保存 1,822 个 C0、4,220 个派生记录和 5,936 个逐项 SHA-256 通过的 blob。目录骨架全量演练与 12 快照实物抽样恢复通过。D 盘 Backs 来源、views、quarantine、R0-R6 workspace、执行脚本及专属 C0/C1 blob 已清理；共享 SQLite 仅保留不可变审计元数据，7 个跨来源共享 C0 哈希保留。终态见 `E:\Backs\_VAULT\_records\externalization-terminal-receipt.json`。 |
| MBA 课程库收口 | `in-place / closed` | MBA 不迁移到 E 盘。D 盘 Babata C0 保留 763 个下载项、758 个唯一哈希和 106,583,439,764 字节原件；C1 保留 3,351 条派生记录，C2 保留 763 组 suggestion/review/semantic entry。C 盘最终 Obsidian 保留 13 门课程、784 个文件。763 条原下载路径清单与恢复脚本可重建最初目录结构。184 个 MBA/高顿 workspace、22 个 execution round 及明确的 MBA 旧 archive/receipt/lock/history 已清理，C0/C1/C2 实体与当前 Obsidian 未删除。终态见 `D:\BabataData\04_runtime\receipts\mba-closure-20260919\terminal-receipt.json`。 |
| 132 个微信群聊 | `deferred-by-user` | 用户明确恢复 |
| Bilibili | `deferred-by-user` | 用户明确恢复 |
| 通用 external navigator | `candidate / not-adopted` | Backs 有界试用证据和后续采用决定 |
| 通用 Obsidian/Web | `unavailable / unplanned` | 新产品目标和真实 caller |
| TC-11 独立系统级 receipt | `not-run` | 用户明确提出系统级终验；不能从缺口自动重开主线 |
| Kimi 日常 capability 启用 | `disabled / deferred` | #229：引用顺序导致的假 revision、连续读取稳定性和附件/长期重采实证。须另立有界修复与真实验证，不从已验收历史或 PR 合并推断通过 |
| 豆包完整 C1 | `closed-with-losses / verified` | 按用户‘试着找一下不行拉倒，赶紧闭环 c1’完成有界恢复并收口：保留 322 个文本 revision/1536 消息与前轮 57 份补充 C1；找回 116 个图片引用（115 个唯一图像）和 6 份本地匹配附件，新增 119 份正式 C1 并核验。55 个未恢复/视觉未成功对象与 6 份附件提取限制已逐项留证，不称无损完整 C1，无待用户决策或自动后续义务。详见 §11 恢复闭环回执。 |
| 新 UI、桌面/移动端、更多输出 | `open / optional` | 真实使用需要 |
| #220 薄协议－文件协议 | `8/8 AC verified / authorized retirement complete / closing supplement verified` | PR #224、#226、#227、#228 已合并，未随新版本发布。原主集合 78,817 文件全量 SHA-256、4 活动库及资产关系、历史／敏感附录和 D/E 隔离恢复通过；经精确授权，E 七旧快照与 D 八组冗余／交付／验证副本／旧 state 已退出，原五个组成快照保留，清理后全仓库 read-data 通过。删除后 74,750 原文件、3,821 survivor 再 hash及四活动库复核通过。2026-11-14另按用户授权补备70个归位文件、停用说明及最后收尾证据：300文件／211,497,422字节，D加密完整read-data、D/E独立恢复全文件SHA-256、E结构check通过；仅增量交付21,439,729字节密文。E现为同一当前批次六个组成（原五个＋收尾附录），不是六份历史备份；本轮临时仓库与恢复目录已退出。程序变量仍D，D无本地成品，ops本地degraded与E交付分开记。未知／legacy／外部／暂缓范围不动，干净布局恢复依赖保全映射；最后交付、核验与状态回写晚于冻结截止，留待下一批保护，不递归自包含。 |

这些项目不是 Active Plan 队列，不能在 session 恢复时自动执行。

## 11. 主要证据入口

| 使用事实 | Evidence 入口 |
| --- | --- |
| #233 搜索调研与 PNG 2026-11-14 实施 | `D:/BabataEvidence/research-render-233/`：真实搜索、正文及结论在 `live-research/run/`；独立分页、组合及正式 PNG 实物和目视记录在 `final-round/`；最终 gate／构建／交付身份由 `terminal-receipt.json` 指定。正文和图不进 Git。正式首版沿用现有 record 投影，不声称完整正文／附件；依赖 probe 不等于启动测试，离线请求护栏不是强 OS 沙箱，视觉自审不是用户验收 |
| #220 文件协议实施与真实验收 | `BABATA_EVIDENCE_HOME/issue220-implementation-20261110/REPORT.md`；实际位置和六个组成见同目录 `CURRENT-BATCH.json`；原终验 `post-retirement-audit-20261111/terminal.json`，八项 AC 见 `closure-20261111/AC-MATRIX.json`。有界收尾补备：`BABATA_EVIDENCE_HOME/file-protocol-supplement-20261114/REPORT.md`、`terminal.json`、`inputs.ndjson`、`ACCOUNTING.json`；原请求／授权／清理回执不改。旧 dry 不能替代真实验收。 |
| 千问全量可见集合与外卖剔除 | `D:\BabataData\04_runtime\staging\model-workspaces\qianwen-chat-recovery-20261107-v1\terminal-receipt.json`；4 项试点后完成其余 18 项；原生行、范围与媒体 SHA-256 回读通过，fresh API 重采 13 unchanged / 9 changed（原生 HTML/JS 与渲染字段变化）。当前 C0 及历史保留边界见 §5，逐项矩阵与媒体清单见回执。PR #208 已合并、Issue #204–207 已关闭；终态全套检查通过，日志为同目录 `full-check-closure.log`。 |
| 千问轻量对话适配器 2026-11-09 收尾 | `implemented / real-path verified`：`08_adapters/03_opencli/qwen/full.js` 只读调用官方消息接口，接受显式 ID/URL；校验 HTTP、业务码、会话归属和 `have_next_page=false`，完整返回原生行。两个既有测试会话经此入口真实读取，各 1 个原生会话行，request_messages / response_messages 数组全部保留；不把对话界面气泡数当原生行数。6 个定向测试覆盖长正文、身份、HTTP/业务错误与分页负例，已接工程 gate；OpenCLI validate 通过。仅单页最多 100 行，分页未结束即拒绝，不下载附件或写 C0，未启动 C1/C2；尚未随新版本发布，用户验收不推断。PR #222 已合并、Issue #221 已关闭；本地全套与 GitHub Rust/适配器/文档检查通过；交付与原生读取证据：D:/BabataData/04_runtime/receipts/qianwen-adapter-closure-20261109/terminal-receipt.json。 |
| 整理前完整 Docs | Git commit `9182a77` |
| Kimi 全量交付与验收 | `D:/BabataData/04_runtime/staging/model-workspaces/kimi-account-wide-20261102-r1/terminal-receipt.json` 为 C0 339/339 与 10 条旧记录保留证据；后继 `kimi-account-wide-20261102-c1/terminal-receipt.json` 保存全链结果；`D:/BabataData/04_runtime/receipts/kimi-user-acceptance-20261102/acceptance-receipt.json` 与 `correction-receipt.json` 分别为用户验收和仅呈现修正。修正后 terminal SHA-256 已核对；工程收尾回执：`D:/BabataData/04_runtime/receipts/kimi-adapter-closure-20261111/terminal-receipt.json`。历史技术回执的 pending 不覆盖后继验收。 |
| P8 来源广度、深度和登记 | `BABATA_RECOVERY_HOME` / `BABATA_EVIDENCE_HOME` 对应 P8 manifests/receipts |
| MBA 13 门课程 | `D:\BabataData\04_runtime\staging\execution-rounds\` 和 `model-workspaces\mba-*` |
| MBA 呈现 v2 | `mba-course-presentation-rollout-20260817-v3\rollout-receipt.json` |
| Cherno C0-C2B | `model-workspaces\cherno-course-*-2026082*-v1`；最终 `cherno-course-c2b-full-20260822-v1` |
| P9/TC-10 | `p9-github-external-backup-restore-20260820-v1.receipt.json` |
| Backs 自主库、R0-R6 与清理 | `E:\Backs\_VAULT\_records\externalization-terminal-receipt.json`；R6 证据位于 `E:\Backs\_VAULT\reports\r6`，历史去重证据位于 `E:\Backs\_VAULT\reports\dedupe-20260807`，managed C0/C1 位于 `E:\Backs\_VAULT\managed-records` |
| MBA 就地收口与恢复 | `D:\BabataData\04_runtime\receipts\mba-closure-20260919\terminal-receipt.json`；原下载结构清单为同目录 `original-downloads.jsonl`，恢复脚本为 `restore-original-downloads.ps1` |
| 豆包 2026-10-17 增量 | `completed / C0 registered` | 以 2026-09-19 的 322 个唯一对话为本地基线，核对当前官方 recent_conv 窗口和可见链接；主对话 `23419482725122` ID 未变，新增 12 条消息，形成新 revision `rev_01M540D3S4CX3BFJFABGZ8684S`。新增消息中声明的 4 张原始图片已下载、PNG 尺寸和 SHA-256 验证，并作为该 revision 的 attachment assets 登记。观察窗口未发现新对话 ID；未启动 C1/C1B/C2，旧 revision 和本地历史保留。终态见 `D:\BabataData\04_runtime\receipts\doubao-incremental-20261017\terminal-receipt.json`。 |
| 豆包 C1 2026-10-18 增量 | 复用 320 条最新 revision C1，新增主对话 `rev_01M540D3S4CX3BFJFABGZ8684S` 的正式 `structured_result`（15 条消息，run `run_01M578QEQQ4X86FSVSG7A7V1CZ`）；当前 321 个非 MBA 对话最新 revision 已有 C1，另保留主对话历史 C1，有效 run 总数 322。MBA 总结对话继续 C0-only；2 个空重复 item deferred，其非空 counterpart 已有 C1。C0 快照、输出哈希及 4 张 C0 图片哈希通过；图片仅保留引用，未图像解读、不声称 C1B；未采集、未删除、未推进 C2。终态 `D:\BabataData\04_runtime\receipts\doubao-c1-incremental-20261018\terminal-receipt.json`。 |
| 豆包 C1A 2026-10-18 修复闭环 | `completed / Agent verified`：按用户“形成解决方案并解决 闭环豆包C1”新开外部适配器修复轮；按 parent graph 排除思考子树，并以源 TTS/可见块交叉核对最终正文。143 revision 重建登记、179 复用，143 个旧结果经核心逻辑失效；321 个最新非 MBA 对话 + 1 个主对话历史 revision 均有唯一有效 C1A。全量 1536 消息/1083 个思考块/15 个 artifact 记录核验通过；19 项回归/负例、边界检查和正式文件 hash/readback 通过。C0 行级 snapshot 与 14 个附件 blob 前后相同，322 份旧输出原字节备份保留。MBA C0-only，2 个空重复 item deferred；未采集/删远端、未改产品实现、无模型调用费用、不推进 C1B/C2。图片/外部附件未解读，artifact brief 不称全文。旧增量回执仅证明当时登记/机械完整性，不证明已排除思考；本修复审计替代其正文质量结论。用户验收不预设。方案、formal audit、registered ledger：`D:\BabataData\04_runtime\receipts\doubao-c1-repair-20261018\terminal-receipt.json`。 |
| 一堂真实样本、C1/C1B、知识与 Obsidian | 来源 `model-workspaces\yitang-ai-knowledge-management-source-20260823-v1`；闭合 `model-workspaces\yitang-ai-knowledge-management-clean-c2b-20260823-v1\terminal-receipt.json` |
| 豆包完整 C1 2026-10-24 本地执行 | `partial_source_gaps`，不能称完整 C1 已完成。复用并重新核验 322 个文本 revision/1536 消息；新增正式登记 4 份图片视觉描述、53 份已捕获模态文本（219 条生成提示/视频卡片字幕记录），57 份 managed output/hash/身份及 CLI 回读通过、无重复登记。C0 行及 14 个原件 blob 不变，raw/derived SQLite quick_check 均通过。175 个缺口按 revision/类型/来源键计：146 个图片引用、13 个上传附件引用、1 个生成文件、15 个 artifact 版本全文未核实；154 个外部推荐视频仅保留卡片信息，不称完整转写。只读查得 5 个同名同大小本地附件候选及 1 个源 MD5 匹配候选；尚未追加 C0，其他仅大小相同的候选未冒认。无新远端模型调用；复用 4 次历史 Qwen 视觉结果（12672 returned tokens），实际扣费未返回。MBA 仍 C0-only，未采集/删除/推进 C2，未改产品实现或直接写库。先前脚本回读字段错误已在全新 v2 workspace 纠正并复用首条成功登记。终态、全量 ledger/audit、候选与恢复方案：`D:\BabataData\04_runtime\receipts\doubao-c1-complete-20261024-v2\terminal-receipt.json`。 |

| 豆包 C1 2026-10-24 有界恢复闭环 | `closed-with-losses / verified`：用户明确接受找不到的缺件留损失收口。已从残存链接恢复 116 个图片引用（115 个唯一 SHA-256），并找到 6 份本地附件（5 份同名同大小+内容锚点、1 份源 MD5 匹配；前 5 份不冒称源 hash 证实）。通过核心 append-assets 语义向既有 ready revision 追加 121 个资产；旧 item/revision/asset 行及 14 个旧 blob 不变。正式新增 113 份图片视觉描述/尽力 OCR 与 6 份附件正文，合计 119 份 managed C1；身份、hash、CLI read-back、无重复及 raw/derived quick_check 通过。322 个原文本 revision/1536 消息、前轮 57 份补充 C1 仍有效。剩余 55 个原件/全文/视觉结果缺口及 6 份附件的内嵌媒体、表格布局和来源同一性限制已进入 loss ledger；154 个推荐视频仍仅卡片元数据。不声称无损完整 C1；无待授权、用户决策或自动后续义务。Qwen 成功处理 113 个唯一输入，返回 373158 tokens；实际扣费未返回。未新增远端删除、MBA 处理、C2 或产品实现改动。终态与全量审计/损失清单：`D:\BabataData\04_runtime\receipts\doubao-c1-recovery-20261024\terminal-receipt.json`。 |

| 豆包 C2A 2026-10-24 全文阅读部分成果 | `partial C2 / C2A package verified`：复用正式 C1，交付 321 个当前对话 + 1 个历史版本、1536 条完整消息、176 份补充和 125 个本地媒体/附件文件。498 份正式派生原字节与全部消息核验通过；全文来自带原始身份的正式 C1；全库索引 900 秒超时，独立阅读包未登记 core output ID，此限制如实保留。全文/标题搜索、322 页 DOM、所有本地图片及移动端通过；从冻结 C1 新根重建的 manifest/全部文件 hash 一致，C0/C1 前后逻辑指纹不变，SQLite 检查通过。继承 55 个缺失对象、6 份附件提取限制及 154 个仅卡片信息的视频，不冒称 C2B 或通用 Web capability，未推断用户验收。零新增模型调用；唯一 live 入口 `D:\BabataData\03_views\doubao-c2a-20261024\index.html`；终态与核验 `D:\BabataData\04_runtime\receipts\doubao-c2-closure-20261024\terminal-receipt.json`。用户已否定其代表完整 C2；正式 C1B、知识登记和 C2B 仍待完成，纠正证据见 `D:\BabataData\04_runtime\receipts\doubao-c2-full-closure-20261024\correction.json`。 |

| 豆包第一批 2026-10-26 C2B 闭环 | `closed / machine-verified / user acceptance not-inferred`：第一批 `261026` 已完成 C2B 工程闭环。C1B 为 322/322；Knowledge 登记 316 个 revision、707 个 semantic entries；dense preview 707/707；core output 为 `output_01M5VZ4430X32Q7W7P6PG75N6K`。C2B live 为 `D:\BabataData\03_views\doubao-c2b-20261026`，独立重建为 `D:\BabataData\03_views\doubao-c2b-20261026-independent`；Obsidian live 为 `C:\Users\Aiano\Documents\Obsidian Vault\Babata\Doubao`。终态回执 `D:\BabataData\04_runtime\receipts\doubao-c2-full-closure-20261024\terminal-receipt.json`，Obsidian 回执 `D:\BabataData\04_runtime\receipts\doubao-c2-full-closure-20261024\obsidian-export-receipt.json`。C0/C1 未改变；仍保留 55 个缺失对象、6 个部分附件限制、154 个视频仅卡片元数据。`user_acceptance` 保持 `not-inferred`。 |

| Kimi 2026-10-26 最近 3 条试点 | `pilot-verified-with-route-policy-defect / user acceptance not-inferred`：冻结的 3 个对话取得完整结构化正文，共 15 条消息，经 Rust Collector 登记为 3 个 ready revision；原文身份及 SHA-256、SQL 回读、3/3 unchanged 重采、未新增 revision、SQLite quick_check/foreign_key_check 均通过。数据位于 `D:\BabataData`。20 个 file 字段均为搜索引用 PDF 元数据，未发现上传附件；附件二进制、深研产物、长对话分页及全量范围未验证。实测能力表仍为 `source.kimi: disabled`，但 Collector 接受并保存，已登记为策略执行缺陷；本试点不宣称通用 route 已启用。持久浏览器会话须串行调用。C1 未要求或触发，无新增模型费用。终态与缺陷：`D:\BabataData\04_runtime\receipts\kimi-pilot-20261026\terminal-receipt.json`、同目录 `defect-ledger.json`。 |

| 元宝 2026-11-07 全量主权取回 | acquisition-closed-with-fidelity-limits / verified：新列表 36、旧会话列表 170 的并集为 198，默认历史另补 1 个真实 ID；最终逐 ID 核对仍为 199，199/199 正文与 790 条原生消息已保存并 fresh read-back，分页全部终止。本轮通过 GET /api/conv/{id} 补回 162 个旧 UUID 正文/670 条消息，保留此前 37 个成功正文；先前将 v1/detail 的 21000 解释为旧正文不可恢复的结论已被推翻。129 个有效媒体 URL 映射对应 82 份唯一文件（79 图片、2 PDF、1 TXT），图片解码、PDF 打开与全包 312 文件 hash 回读通过；旧包 449 文件再次校验通过且未覆盖。16 个资源 ID（13 张唯一图片）的普通下载、v1 签名原图与直接 COS 均返回平台 JPEG，和上传元数据声明大小不符；可用内容已存，原始上传字节无损未证实，逐项见 fidelity-ledger.json。2 个 example.com 示例图 URL 原文保留，不计用户附件。当前独立包入口 D:/BabataData/00_inbox/yuanbao-recovery-gap-20261107/README.md；终态 D:/BabataData/04_runtime/receipts/yuanbao-recovery-gap-20261107/terminal-receipt.json，取代前轮 source-losses 判断。仅证明当前账户 API 暴露集合，不声称服务器未暴露历史也已找回。该 acquisition 轮关闭时 source.yuanbao absent、正式 C0-C 登记 0；后续通过既有通用 writer 的正式登记与回读见下一行。原取回包、旧回执与保真限制保留；该轮适配器暂缓，后续用户显式恢复的限定开发结果见后续行，未启动 C1/C2。 |

| 千问与元宝正式 C0 2026-11-08 核验 | `completed / C0 registered / verified`：千问 22 个 item、32 个 ready revision、当前 42 条原生行的范围与全部历史版本正文/资产 SHA-256 已只读核验，既有外卖排除与历史保留边界不变。元宝经现成 capture export --provider yuanbao --native-id 登记 199 个 item/199 个 ready revision；原始 JSON 逐字节保留，790 条消息身份与来源 URL 回读一致。129 个媒体 URL 对应 82 份唯一文件，在所属 revision 下形成 98 个媒体 attachment occurrence；另登记 20 份媒体映射清单，保留转码限制与示例占位引用。全部 managed 文件 size/hash、SQLite quick_check/foreign_key_check 和 pending/orphan 检查通过。该轮 source.yuanbao 尚 absent，采用通用导入；后续适配器限定测试见下一行。该轮未改产品实现、未直接写库、未新增远端访问或启动 C1/C2，零新增模型调用；用户验收不推断。冻结输入、构建身份、逐项矩阵和终态：D:/BabataData/04_runtime/receipts/qianwen-yuanbao-c0-20261108/terminal-receipt.json；独立核验见同目录 qianwen-verification.json 与 yuanbao-verification.json。 |

| 元宝对话适配器 2026-11-08 限定开发验证 | `implemented / real-path verified`：source.yuanbao 已接统一 Collector，仅接受显式 ID/URL 与完成分页的原生 handoff。用户新建的两个测试会话在 Git 外隔离数据根完成候选、选择、4 条原生消息与 1 张图片的保存；消息、来源、时间、全部 managed SHA-256 回读通过；重新读取平台后 2/2 unchanged，缺少 fresh handoff 时 2/2 inaccessible，均未增 revision。已覆盖定向负例与长正文/changed 重采的隔离工程测试。原历史库 199 个会话的 item/revision/asset 行前后逻辑 hash 一致，无历史批次重跑。测试图平台 JPEG 为 79,437 字节，上传元数据为 471,438 字节，保留原声明和保存 hash，不声称上传原字节无损。适配器不内置账号发现/下载；其他媒体 shape、legacy 与长分页的适配器真实路径不外推。未启动 C1/C2，零新增模型调用，用户验收不推断。PR #218 已合并、Issue #217 已关闭；本地全套检查与 GitHub 必需检查通过；测试终态、构建与范围：D:/BabataData/04_runtime/receipts/yuanbao-adapter-20261108/real-v1/terminal-receipt.json；交付回执：同目录上级 terminal-receipt.json。尚未随新版本发布。 |

旧失败批次和旧 candidate 只用于追溯，不作为当前产品定义或状态。

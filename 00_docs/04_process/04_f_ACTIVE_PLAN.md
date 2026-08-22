# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-INTENT-PLAN-GOVERNANCE`](04_g_INTENT_AND_PLAN_GOVERNANCE.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20260821-02 -->

### AP-20260821-02：Cherno 课程缓存治理与 Babata 重洗

- 来源锚点：`DFC-20260821-02`、`DFC-20260822-01`、`DFC-20260822-02`、`DFC-20260822-03`、`DFC-20260822-04`；GitHub Issue #190；draft PR #191。
- Goal 锚点：Goal API 返回的 Cherno 三套课程缓存治理目标，状态 `active`。
- 状态转换类型：`user-explicit-goal-start`
- 状态转换依据：用户明确启动 Cherno 三套课程缓存治理 Goal，并另以 `DFC-20260821-03` 授权
  补取确认缺失的集数、以 `DFC-20260822-01` 恢复百炼/千问处理并授权免逐次费用确认，再以
  `DFC-20260822-02` 明确授权直接升级且不得把常规执行动作反复设为用户 gate；Goal 更新又以
  `DFC-20260822-03` 明确要求在新链路验证后删除旧 DOCX。
- 用户目标：以稳定 video ID 命名视频并由 metadata 保留 YouTube 原始标题、URL、playlist identity
  和顺序；评估 ffmpeg 规范化；排除旧 DOCX/内嵌旧字幕后经 Babata 重建 C1/C1B/C2B，最终发布
  可重建 Obsidian 视图。
- 当前状态：`in_progress / bl-upgrade-authorized / asr-authorized`。
- 目标终端：三门课完整 source manifest 和 legacy 映射可审计；代表性试跑通过后完成全量正式
  C1/C1B/C2B，并由验证后的 C2B package 发布唯一可重建 Obsidian 视图。
- 不改变：`E:\Cherno` 现有 MP4 不覆盖、不删除；旧 DOCX 和内嵌旧字幕不作为新 C1 权威输入。
  31 个旧 DOCX 只在 269 项替代 C1 完整注册/read-back、删除清单冻结后删除；内嵌字幕随只读 MP4
  保留但不消费。全量 ASR/C1 注册、C1B/C2B 和 Obsidian 发布仍须按阶段验证。
- 临时子计划与阶段结论：Agent 已补齐 C++ 6 集并冻结三门课 269/269 source manifest；旧缓存映射
  为 6 `exact` + 263 `high`，0 unresolved。playlist order 会变化，故 video ID 是稳定身份/文件名，
  position 只作 metadata。原件不统一转码；三门课代表样本已生成完整时长 mono/16 kHz FLAC 并
  完成哈希回读。窄 `source.youtube` Collector route 已以真实 cache manifest 完成 3/3 代表 C0
  保存、managed asset 回读和 unchanged 重采，现为 `E3 / enabled`；首版不下载、不遍历频道/账号。
  三项代表 ASR 已用 `fun-asr` 完成 1544.127 秒全长处理、脱敏和 6/6 正式 C1 注册/read-back；
  免费额度前估算 0.339709 元，provider usage 未返回而保持 `{}`。幂等复跑复用 6/6 run、0 新增。
  全量 round 已冻结为 269 项、423515.185 秒，按实时 0.00022 元/秒估算上限 93.173341 元；
  `session_01M0JJ9YA4DTWT3JJMD4V54WPP` 已完成 269/269 C0 和 43,483,878,072 bytes 逐项哈希回读；
  3 个旧代表项被追加同字节 ordinal 2，按重复 capture 如实保留。全量 mono/16 kHz FLAC 与 ASR
  已完成 269/269；脚本复用已终态的 3 项 pilot，只新增调用 266 项并记录累计/增量估算。
  为保留编程课程不可替代的代码/UI/动画证据，derived schema v6
  已在一致性备份后新增 `audio_excerpt`/`video_excerpt`/`attachment_excerpt`，迁移保持 3368 run、
  3367 derivative，workspace 测试和真实 DB quick/FK 检查通过；三样本 Qwen 视觉 C1B 已正式登记
  3 个 essence decision、13 个 key frame 和 2 个 video excerpt，managed hash 回读通过。PowerShell 5
  顶层 JSON 数组兼容问题已修复；幂等探针产生的同哈希重复 run 已逻辑失效并保留审计记录，当前
  C1/视觉 pilot 分别验证为 0 新增、6/18 复用。Agent 复核全量时发现 239/269 集超过 10 分钟、
  最长约 10560 秒，代表样本的整段视频调用不能机械扩展；全量 C1B 改为完整时间轴分块、带绝对
  时间戳的本地联系表/Qwen 视觉判断，再从只读原 MP4 精确回切 key frame/video excerpt。该调整只
  规范化 provider 输入，不替换 C0，也不重跑已终态的 3 项 pilot。
  全量 ASR 已完成 269/269、0 failed；538 个候选结果已正式登记并逐项回读，首轮为 532 新建 +
  6 复用，幂等复跑为 0 新建 + 538 复用。31 个旧 DOCX 已在冻结路径/大小/hash 清单后送入
  Windows 回收站，MP4 未删除、未覆盖。Obsidian 只从验证后的 C2B package 发布。`bl` 保留 1.14.2：npm 报告
  1.17.0，但 `bl update` 和 npm install 均因 `ENOVERSIONS` 明确失败；不再以此阻塞 provider。
  当前全量视觉 session `99728` 已汇总完成 599/599 chunk；恢复后已从 209/599 断点复用缓存，
  语义矛盾归一化生效并完成 599/599、0 failed；汇总兼容性修复已补齐旧成功 chunk 缺失的
  `why_text_insufficient` 字段，不重跑视觉调用。首轮在 9.6 秒尾块发现 FFmpeg 8.1 MJPEG 色域/零帧问题，
  已改为首帧必取的 interval sampler、显式 yuvj420p/strict 参数，并把联系表 SHA-256 纳入请求身份。
  C1B 首轮注册暴露 PowerShell 5.1 嵌套 JSON 参数转义问题，已改为原生参数数组；随后全量注册在单项 Babata
  调用上长时间无 managed-file 增长，已终止并改为可恢复的 Python 按项注册批次；当前已完成并写入 ledger
  65/269 项，后台批次继续推进，单项失败不会拖住全量。课程学习已完成 269/269 单课候选和 3/3 课程计划，0 failed；首批统一缺字段后改为显式 JSON 字段合同
  和单节 pilot-first，课程级 length/分区问题又以提高上限、首次出现去重和小型漏项 repair 收敛。
  实际用量为 4,742,450 input + 319,246 output tokens，按官方公示单价预估 13.315852 元；逐文件/计划 hash
  与敏感字段审计通过。视觉进度继续写入 full stage 的 `progress.json`，不得因普通 provider 重试重跑已成功对象。
- 证据入口：`D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-stage2-20260821-v1\`
  下 `results/source-manifest.json`、`results/representative-samples.json`、`inventory/`、`REPORT.md`；
  缺集证据仍见 `cherno-course-gap-acquisition-20260821-v1`。
- 当前检查点（2026-08-22）：269/269 C1B 已正式注册，课程知识登记 269/269、3 门课程和 lens 已完成；三门 C2B package 均通过
  engineering check，已发布到各自唯一 Obsidian live。发布状态保持 `published_pending_user_acceptance`，不得代替用户做视觉验收。
- 新增用户纠正（2026-08-22）：Cherno Obsidian 课节的用户可见标题/文件名不得把 `video_id` 当标题；应使用
  `L###-<可读内容或操作名称>` 表达本节讲什么、做什么，`video_id` 仅保留为稳定身份、文件关联和 metadata。
  本轮已只重建 C2B package 并原子替换 live，115/123/31 个课节均通过可读标题、H1/文件名一致、ID 不泄漏标题、
  唯一 ID 和 publication hash 回读；未重跑 C0/ASR/C1/C1B/知识登记。证据入口为
  `cherno-course-c2b-full-20260822-v1` 的 package manifest、publication receipt 和 `publication-round.json`。
- 下一步：等待用户对 C++、Game Engine、OpenGL 三个 live 进行内容与视觉验收；验收前不得关闭 Goal 或重发布。不得重跑已终态的缺集、manifest、全量 C0、ASR、C1、C1B 或课程登记。

## 2. 下次开工队列（禁止恢复时自动执行）

队列当前无其他可自动晋升项。

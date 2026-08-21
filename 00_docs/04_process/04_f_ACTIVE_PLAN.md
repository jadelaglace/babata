# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-INTENT-PLAN-GOVERNANCE`](04_g_INTENT_AND_PLAN_GOVERNANCE.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: AP-20260821-02 -->

### AP-20260821-02：Cherno 课程缓存治理与 Babata 重洗

- 来源锚点：`DFC-20260821-02`；GitHub Issue #190；draft PR #191。
- Goal 锚点：Goal API 返回的 Cherno 三套课程缓存治理目标，状态 `blocked`。
- 状态转换类型：`user-explicit-goal-start`
- 状态转换依据：用户明确启动 Cherno 三套课程缓存治理 Goal，并另以 `DFC-20260821-03` 授权
  补取确认缺失的集数。
- 用户目标：以稳定 video ID 命名视频并由 metadata 保留 YouTube 原始标题、URL、playlist identity
  和顺序；评估 ffmpeg 规范化；排除旧 DOCX/内嵌旧字幕后经 Babata 重建 C1/C1B/C2B，最终发布
  可重建 Obsidian 视图。
- 当前状态：`blocked / awaiting-asr-consent`。
- 目标终端：三门课完整 source manifest 和 legacy 映射可审计；代表性试跑通过后完成全量正式
  C1/C1B/C2B，并由验证后的 C2B package 发布唯一可重建 Obsidian 视图。
- 不改变：`E:\Cherno` 现有 MP4 与 DOCX 不覆盖、不删除；旧 DOCX 和内嵌旧字幕不作为新 C1
  权威输入；全量转码、ASR/C1 注册、C1B/C2B 和 Obsidian 发布尚未进入本阶段执行。
- 临时子计划与阶段结论：Agent 已补齐 C++ 6 集并冻结三门课 269/269 source manifest；旧缓存映射
  为 6 `exact` + 263 `high`，0 unresolved。playlist order 会变化，故 video ID 是稳定身份/文件名，
  position 只作 metadata。原件不统一转码；三门课代表样本已生成完整时长 mono/16 kHz FLAC 并
  完成哈希回读。窄 `source.youtube` Collector route 已以真实 cache manifest 完成 3/3 代表 C0
  保存、managed asset 回读和 unchanged 重采，现为 `E3 / enabled`；首版不下载、不遍历频道/账号。
  运行时 `processing.bailian_cli` enabled，但首次 ASR 调用等待用户确认可能计费。旧字幕待新 C1
  完整注册/read-back 后再受控退役；Obsidian 只从验证后的 C2B package 发布。
- 证据入口：`D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-stage2-20260821-v1\`
  下 `results/source-manifest.json`、`results/representative-samples.json`、`inventory/`、`REPORT.md`；
  缺集证据仍见 `cherno-course-gap-acquisition-20260821-v1`。
- 下一步：用户确认百炼可能产生的费用后运行三项代表 ASR，并绑定已回读的正式 C0 身份完成
  C1 注册/read-back；代表样本通过后再冻结全量 269 项处理轮次。
- 终态原因：连续三轮均停在首次百炼 ASR 的费用授权边界；未经用户确认不能调用计费服务，且
  当前没有其他可替代这一步并继续证明完整 C1/C1B/C2B/Obsidian 目标的已授权动作。
- 下一授权/决定：用户明确回复同意百炼 ASR，或明确选择另一条转写路线及其费用/数据边界。
- 恢复入口：用户明确授权后，先按 Goal API 与本 Active Plan 恢复，再执行百炼版本/鉴权 preflight
  和三项代表 ASR；不得重跑已终态的缺集、manifest、代表 C0 或 unchanged 重采。

## 2. 下次开工队列（禁止恢复时自动执行）

队列当前无其他可自动晋升项。

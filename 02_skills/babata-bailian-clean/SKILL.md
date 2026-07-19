---
name: babata-bailian-clean
description: >
  Guide an agent through Babata multimodal cleaning with Aliyun Bailian CLI (`bl`), then formally
  register C1 derivatives via `babata process register` (pipeline agent_import). Use when the user
  wants to OCR, transcribe, summarize, structure, tag, or clean local course/media/documents
  (pdf/docx/xlsx/pptx/png/jpg/mp4/audio), especially omba/course folders or Babata C0 revisions;
  when format/size/resolution must be normalized locally before model calls; or when video should
  become a transcript with timestamps/speakers/paragraphs if the API provides them. Prefer this
  skill over one-off rigid batch scripts. Do not overwrite originals; write staging derivatives,
  then register into C1 only against the real C0 revision and asset when the source is a file.
---

# Babata × 百炼多模态清洗（引导型）

> 仓库位置：`02_skills/babata-bailian-clean/`（P5 进行中）。
> **清洗 + 正式 C1 登记** 一体引导：staging 写 `BABATA_DATA_HOME/generated/`，正式入库走
> `babata process register --pipeline agent_import`（见 [references/c1-register.md](references/c1-register.md)）。

面向 **Agent 决策与执行**，不是固定批处理脚本。  
脚本只在你自己需要时临时写；本 skill 提供路由、门槛、失败回退、交付物契约与 **C1 登记步骤**。

依赖并优先配合 `$bailian-cli`（`bl`）与已安装的 `babata` CLI。本 skill 不重复整本 CLI 手册。

## 不可破的边界

1. **原件 / C0 只读**：不覆盖、不移动用户原资料与 raw assets。
2. **派生物可追溯**：文件来源必须绑定真实 C0 revision + asset + 原件哈希；规范化输入另记哈希和步骤。
3. **本地先规范化，云端再理解**：格式/体积/分辨率不达标时，先用本机 `ffmpeg` / Pillow / 文档库处理。
4. **按样本智能路由**：先看内容再选模型能力；不要对所有文件套同一命令。
5. **能本地免费完成的不要先烧 token**：可抽取的文本优先本地抽；图像/手写/扫描/音视频再上百炼。
6. **最小验证优先**：新目录先每类 1 个样本跑通，再扩大。
7. **staging ≠ 入库**：只有 `babata process register` 成功后才算正式进入 C1。
8. **Provider 响应先脱敏**：临时签名 URL、token、鉴权头和账号凭据不得进入普通 C1；只登记脱敏后的 JSON，并在 params 记录清理动作。

## 何时启用

- 用户说：清洗、OCR、转写、摘要、结构化、标签、C1、多模态、课程资料、入库、register
- 路径含课程导出（如 `omba25`）、混杂 pdf/视频/作业
- 已有 C0 revision，要把 Agent 清洗结果挂到该版本上
- 已装百炼 CLI，要做真实资料闭环而不是空架构

## 开工前检查（失败则先修）

```text
bl --version
bl auth status --output json
babata --help
babata --json process list-pipelines
```

- 无 API Key：引导 `bl auth login --api-key` 或控制台登录（见 `$bailian-cli`）。
- 无 `ffmpeg`：视频/音频先装再继续。
- 无 `babata`：在仓库 `01_app` 构建 CLI 后再 register。
- Python 建议可用：`Pillow`、`pypdf`、`python-docx`、`openpyxl`、`pymupdf`（按实际类型按需装）。

Windows 上 `subprocess` 找不到 `bl` 时，用：

`C:\Users\<user>\AppData\Roaming\npm\bl.cmd`

## 标准流程（每轮都走，但步骤可跳过）

```text
1. 摸清范围     盘点扩展名、体量、代表样本
2. 绑定 C0      文件必须取得真实 revision_id、asset_id、asset sha256；否则先 capture/collector；已有 revision 后找回原件/预览则用 capture attach-assets 追加版本
3. 选定样本     每类先 1 个（优先小文件 / 用户点名）
4. 本地探针     元数据：页数、分辨率、时长、可否抽文本；核对 C0 原件 sha256
5. 规范化       仅当会阻碍模型或成本过高
6. 百炼清洗     按路由表调用
7. 整理并脱敏 staging Markdown/JSON + manifest + REPORT
8. 正式 C1 登记 babata process register（agent_import）
9. 核验         process show-run / list-runs
10. 扩大或停下  用户确认后再批量
```

输出根目录默认（staging）：

`<BABATA_DATA_HOME 或 C:\Users\<user>\BabataData>\generated\<任务名>-bailian-clean\`

子目录建议：

```text
preprocessed/   # 本地规范化产物
results/        # 模型结果原始 JSON + 可读 .md
manifest.json   # 源文件映射（含 revision_id / run_id 一旦登记）
REPORT.md       # 人话汇总
```

正式 C1 落在数据根 `02_derived/index/derived.sqlite`，不经 Git。

## 类型路由（核心智能）

先判 **“已有可机读文本吗？”** 再判模态。

| 形态 | 本地优先 | 百炼 | staging 产物 | 登记 kind |
|------|----------|------|--------------|-----------|
| 纯文本 / 可抽 PDF / DOCX / PPTX 文本 | 抽正文 | `bl text chat` 摘要/结构/标签 | `*-text.md` | `extracted_text` / `summary` / `structured_result` / `tags` |
| 扫描 PDF / 页图 / 手写笔记图 | 渲页或缩图 | `bl vision describe` OCR+说明 | `*-ocr.md` | `ocr_text` / `visual_description` |
| 照片/截图/幻灯片图 | 缩到可传 | `vision describe` | 描述+OCR+标签 | `visual_description` / `ocr_text` / `tags` |
| 表格 XLSX | 抽前 N 行预览 | `text chat` 解释列与任务 | 结构说明 | `structured_result` / `summary` |
| 音频 | 转 mono 16k wav | `bl speech recognize` | 逐字稿 | `transcript` |
| 视频 | 抽音频 + 可选截帧 | ASR 为主；截帧 VL 辅助 | 时间轴逐字稿+摘要 | `transcript` / `summary` / `key_frame` |

详细门槛与命令见：

- [references/media-routing.md](references/media-routing.md)
- [references/bailian-recipes.md](references/bailian-recipes.md)
- [references/output-contract.md](references/output-contract.md)
- [references/c1-register.md](references/c1-register.md) ← **正式入库必读**

## 视频 = 转写优先

用户要的是 **课/会议内容**，不是再生成视频。

1. `ffprobe` 看时长/音轨。
2. 最小验证：先 **1–3 分钟** 音频，再全长。
3. ASR：

```bash
bl speech recognize --url <audio> --language <zh|en|...> --diarization --out result.json --output json
```

4. 能拿尽拿：时间戳、说话人、分句；词级 confidence 有则保留。
   **没有不强求**：API 缺 diarization/段落时，用 `text chat` 做智能分段与小标题，并在 `loss_notes` 写明“模型后处理，非 ASR 原生”。
5. 截帧 VL 只补：板书、PPT 页、讲者画面；不替代全文转写。
6. 全长视频体积大时：本地切片/抽音频，不要一上来传整段原片；在 `--params-json` 记录原视频 asset、切片范围、规范化参数和 provider 输入哈希。
7. 转写结果用 `--kind transcript` register；可读 md 与 raw json 可分两次登记。

## 正式 C1 登记（强制步骤摘要）

有 C0 `revision_id` 或用户要求入库时：

```bash
babata --json process register \
  --pipeline agent_import \
  --revision rev_... \
  --kind <ocr_text|transcript|summary|...> \
  --provider bailian_cli \
  --model <model> \
  --tool-version <bl-version> \
  --input-sha256 <C0-text-or-asset-64hex> \
  --input-asset-id asset_... \
  --text-file path/to/results/....md \
  --output-file path/to/results/....md \
  --params-json '{"provider_input_sha256":"...","preprocessing":["..."]}' \
  --language zh \
  --loss-notes "..."
```

- pipeline 固定优先 **`agent_import`**（Agent 按本 skill 完成清洗）。
- 文件派生结果必须传 `--input-asset-id`，`--input-sha256` 必须是该 C0 asset 的哈希；不能改用 staging/规范化文件哈希。
- 同一来源同时取得上传原文件和平台预览时，先用 `capture attach-assets --original ... --preview ...` 追加 C0 revision；后续清洗绑定 `original` asset，预览件不冒充源文件。
- `--output-file` 把结果复制到受控 `02_derived/files/sha256/`；不得把 `generated/...` 作为 `--logical-path`。
- 同时传 `--text-file`/`--json-file` 与 `--output-file` 时必须指向完全相同的字节，否则登记失败。
- Provider JSON 中的临时签名 URL、token、鉴权头和账号凭据必须先删除或替换；在 `--params-json` 的 `sanitization` 记录动作。完整响应如确需保留，只能进入明确受限、不会被普通检索/输出消费的证据区。
- 只有 failed run 可用 `--retry-of`；revision、item、asset、input hash、pipeline、kind 必须与父 run 一致。
- 登记后：`process show-run` / `list-runs` 核对。
- 完整字段与模板见 [references/c1-register.md](references/c1-register.md)。

## 本地规范化启发式

**需要处理：**

- 图边长 > 2048 或体积过大 → Pillow 等比缩小
- 透明图/奇怪模式 → RGB
- PDF 不可抽字或抽字乱码 → 渲页再 OCR
- 视频很长 → 先切片音频；全长确认后再跑
- 音频非 16k mono → `ffmpeg` 转 wav 提升 ASR 稳定性

**不要过度处理：**

- 已能本地抽干净文本 → 别整页 OCR
- 已是清晰小图 → 别再压画质
- 用户只要某一页/某题 → 别清洗整门课

## 文本清洗提示骨架（按材料改写）

对抽取出的正文，让模型输出中文 Markdown：

1. 一句话摘要  
2. 结构化要点（≤8）  
3. 文档类型  
4. 关键词（约 5）  
5. 若是作业/案例/测验 → 题目/任务清单  
6. 清洗备注（抽取质量、缺口、未验证点）  

约束：**不编造原文没有的事实**。

长文用 `bl text chat --messages-file messages.json`（JSON messages 数组），避免 Windows 命令行截断。

## 失败回退

| 现象 | 动作 |
|------|------|
| No API key | 先鉴权，再重试 |
| vision/ASR 超时 | 缩短输入、降分辨率、切 1–3 分钟 |
| diarization 失败 | 去掉 `--diarization` 重试 |
| PDF 抽字为空 | 转页面图 OCR |
| PPTX 无 python-pptx | zip+XML 抽 `a:t` 或改 VL 渲页 |
| 路径中文/空格/`\xa0` | Python `pathlib`；少在 shell 里手拼 |
| 批量中单文件失败 | 记录错误继续，不整批中止 |
| register 缺 revision | 先 capture/collector；禁止伪造 id |
| provider 失败 | 先 `process register-failure --kind ...`；修复后用同一身份 `--retry-of` |
| register 校验失败 | 不会创建 run；保留 staging，修正 C0/字段后新 register，不伪称 retry |
| 删除并重建 C1 | `process delete-result --run ... --reason ...`；重建创建新 run，不使用 `--retry-of` |

## 汇报口径（对人）

始终分开说：

```text
1. 原件 / C0 是否还在原处且只读
2. 本地做了哪些规范化
3. 百炼实际调用了什么（模型/能力）
4. staging 派生物路径（generated/...）
5. 是否已 process register（run_id / derivative_id / kind）；未登记要明说
6. C0 revision / asset / 原件哈希和正式 `02_derived` 文件是否一致
7. 哪些能力原生具备 / 哪些是后处理
8. 未覆盖范围与下一步建议
```

## 与 Babata 阶段关系

- 本 skill 覆盖 **P5 C1 清洗试跑 + Agent 结果正式登记**。
- `agent_import` 是当前推荐登记 pipeline；`enqueue/run-once` 作业队列仍可能未启用。
- 不要把本 skill 结果假装成 P6 检索库或人工事实。
- 模型摘要/标签永远是 C1，不是 first-party C0。

## 反模式

- 写死“对目录所有 mp4 一律全长 ASR”而不先探针
- 覆盖下载目录里的课程原片
- 只丢 JSON 不给可读摘要
- 只写 staging 就宣称「已进入 Babata」
- 把 bailian-cli 手册整页粘贴进对话
- 未鉴权就连续重试烧时间
- 伪造 revision_id 去 register
- 把抽取 WAV、压缩视频、截图或 staging 文本哈希当成 C0 原件哈希
- 把 `generated/...` 登记成正式 `logical_path`
- 失败登记不写目标 kind，或把 OCR 失败重试成 summary

---
name: babata-bailian-clean
description: >
  Guide an agent through Babata C1-style multimodal cleaning with Aliyun Bailian CLI (`bl`).
  Use when the user wants to OCR, transcribe, summarize, structure, tag, or clean local course/media/documents
  (pdf/docx/xlsx/pptx/png/jpg/mp4/audio), especially omba/course folders or Babata derivatives; when format,
  size, or resolution must be normalized locally before model calls; or when video should become a transcript
  with timestamps/speakers/paragraphs if the API provides them. Prefer this skill over writing one-off rigid
  batch scripts. Do not replace or overwrite originals; write traceable derivatives only.
---

# Babata × 百炼多模态清洗（引导型）


> 仓库位置：`02_skills/babata-bailian-clean/`（P5 进行中）。
> 这是 **Agent 清洗引导** Skill，不是已通过 TC 的 `babata process` 正式能力；派生物默认写 `BABATA_DATA_HOME`，不代替 C1 核心登记。

面向 **Agent 决策与执行**，不是固定批处理脚本。  
脚本只在你自己需要时临时写；本 skill 提供路由、门槛、失败回退与交付物契约。

依赖并优先配合 `$bailian-cli`（`bl`）。本 skill 不重复整本 CLI 手册。

## 不可破的边界

1. **原件只读**：不覆盖、不移动用户原资料。
2. **派生物可追溯**：每次清洗写清 `source_path`、工具/模型、时间、参数、失败原因。
3. **本地先规范化，云端再理解**：格式/体积/分辨率不达标时，先用本机 `ffmpeg` / Pillow / 文档库处理。
4. **按样本智能路由**：先看内容再选模型能力；不要对所有文件套同一命令。
5. **能本地免费完成的不要先烧 token**：可抽取的文本优先本地抽；图像/手写/扫描/音视频再上百炼。
6. **最小验证优先**：新目录先每类 1 个样本跑通，再扩大。

## 何时启用

- 用户说：清洗、OCR、转写、摘要、结构化、标签、C1、多模态、课程资料
- 路径含课程导出（如 `omba25`）、混杂 pdf/视频/作业
- 已装百炼 CLI，要做真实资料闭环而不是空架构

## 开工前检查（失败则先修）

```text
bl --version
bl auth status --output json
```

- 无 API Key：引导 `bl auth login --api-key` 或控制台登录（见 `$bailian-cli`）。
- 无 `ffmpeg`：视频/音频先装再继续。
- Python 建议可用：`Pillow`、`pypdf`、`python-docx`、`openpyxl`、`pymupdf`（按实际类型按需装，不预装全家桶）。

Windows 上 `subprocess` 找不到 `bl` 时，用：

`C:\Users\<user>\AppData\Roaming\npm\bl.cmd`

## 标准流程（每轮都走，但步骤可跳过）

```text
1. 摸清范围   盘点扩展名、体量、代表样本
2. 选定样本   每类先 1 个（优先小文件 / 用户点名）
3. 本地探针   元数据：页数、分辨率、时长、可否抽文本
4. 规范化     仅当会阻碍模型或成本过高
5. 百炼清洗   按路由表调用
6. 整理交付   Markdown/JSON 派生物 + 简短人话汇报
7. 扩大或停下 用户确认后再批量
```

输出根目录默认：

`<BABATA_DATA_HOME 或 C:\Users\<user>\BabataData>\generated\<任务名>-bailian-clean\`

子目录建议：

```text
preprocessed/   # 本地规范化产物
results/        # 模型结果原始 JSON + 可读 .md
manifest.json   # 源文件映射
REPORT.md       # 人话汇总
```

## 类型路由（核心智能）

先判 **“已有可机读文本吗？”** 再判模态。

| 形态 | 本地优先 | 百炼 | 目标产物 |
|------|----------|------|----------|
| 纯文本 / 可抽 PDF / DOCX / PPTX 文本 | 抽正文 | `bl text chat` 摘要/结构/标签/题清单 | `*-text.md` |
| 扫描 PDF / 页图 / 手写笔记图 | 渲页或缩图 | `bl vision describe` OCR+说明 | `*-ocr.md` |
| 照片/截图/幻灯片图 | 缩到可传 | `vision describe` | 描述+OCR+标签 |
| 表格 XLSX | 抽前 N 行预览 | `text chat` 解释列与任务 | 结构说明 |
| 音频 | 转 mono 16k wav（可截断试跑） | `bl speech recognize` | 逐字稿 |
| 视频 | 抽音频 + 可选截帧/短预览 | ASR 为主；截帧 VL 辅助 | 带时间轴逐字稿+摘要 |

详细门槛与命令见：

- [references/media-routing.md](references/media-routing.md)
- [references/bailian-recipes.md](references/bailian-recipes.md)
- [references/output-contract.md](references/output-contract.md)

## 视频 = 转写优先

用户要的是 **课/会议内容**，不是再生成视频。

1. `ffprobe` 看时长/音轨。
2. 最小验证：先 **1–3 分钟** 音频，再全长。
3. ASR：

```bash
bl speech recognize --url <audio> --language <zh|en|...> --diarization --out result.json --output json
```

4. 能拿尽拿：时间戳、说话人、分句；词级 confidence 有则保留。  
   **没有不强求**：API 缺 diarization/段落时，用 `text chat` 做智能分段与小标题，并在备注写明“模型后处理，非 ASR 原生”。
5. 截帧 VL 只补：板书、PPT 页、讲者画面；不替代全文转写。
6. 全长视频体积大时：本地切片/抽音频，不要一上来传整段原片。

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

## 汇报口径（对人）

始终分开说：

```text
1. 原件是否还在原处
2. 本地做了哪些规范化
3. 百炼实际调用了什么（模型/能力）
4. 得到了什么派生物（路径）
5. 哪些能力原生具备 / 哪些是后处理
6. 未覆盖范围与下一步建议
```

## 与 Babata 阶段关系

- 这是 **C1 清洗试跑/技能**，不等于资料已正式入库 C0/C1。
- 正式写入 Babata 核心库需走仓库开发流程（Issue/分支/PR）与产品文档；本 skill 默认只产 data 目录派生物。
- 不要把本 skill 结果假装成 P6 检索库。

## 反模式

- 写死“对目录所有 mp4 一律全长 ASR”而不先探针
- 覆盖下载目录里的课程原片
- 只丢 JSON 不给可读摘要
- 把 bailian-cli 手册整页粘贴进对话
- 未鉴权就连续重试烧时间

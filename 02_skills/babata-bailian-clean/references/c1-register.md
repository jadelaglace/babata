# Agent 清洗 → C1 正式登记

清洗完成后，**staging 派生物不算已入库**。要把结果写入 Babata C1，必须调用：

```text
babata process register ...
```

本 skill 默认用 pipeline **`agent_import`**：表示 Agent 按本 skill 路由完成清洗后，再把结果正式登记。

## 何时必须 register

在以下任一情况为真时，做完清洗后立刻 register（不要只停在 `generated/`）：

1. 用户明确要 C1 / 入库 / process register
2. 样本已经绑定到某个 C0 `revision_id`（来自 capture/collector）
3. 要证明「可追溯派生物」闭环（TC-04 部分证据）

若用户只要目录里的可读 md、尚未有 C0 revision：先 `babata capture ...` 或沿用 collector 已写入的 revision，再 register。

## 字段映射

| register 参数 | 含义 | skill 侧来源 |
|---|---|---|
| `--pipeline agent_import` | Agent 引导清洗登记 | 固定（除非用户指定走内置 pipeline） |
| `--revision` | C0 revision id | capture/collector 输出 |
| `--item` | 可选 item id | 同上 |
| `--kind` | 派生物类型 | 见下表 |
| `--provider` | 处理身份 | 通常 `bailian_cli` 或 `local_extract` |
| `--model` | 模型/工具名 | 如 `qwen-vl-plus`、`qwen-plus`、`paraformer` |
| `--input-sha256` | 输入哈希（64 hex） | 原件或 revision `text_sha256` / 文件 SHA-256 |
| `--text` / `--text-file` | 文本派生物 | `results/*.md` 或整理后的正文 |
| `--json-file` | 结构化 JSON | `results/*.json`（ASR/结构化） |
| `--logical-path` | 数据根内相对路径 | 大文件派生物（可选） |
| `--language` | 语言 | `zh` / `en` / … |
| `--loss-notes` | 已知损失 | 布局/时间轴/说话人缺失等 |
| `--retry-of` | 重试父 run | 失败后再登时使用；会新建 attempt |

### kind 选择

| 清洗结果 | `--kind` |
|---|---|
| 本地抽出的正文 | `extracted_text` |
| OCR | `ocr_text` |
| 音视频转写 | `transcript` |
| 字幕轨 | `subtitle` |
| 摘要 | `summary` |
| 画面/板书描述 | `visual_description` |
| 关键帧（元数据/路径） | `key_frame` |
| 标签 | `tags` |
| 结构化清单/字段 | `structured_result` |
| ffprobe/页数等元数据 | `media_metadata` |

一次清洗可 **多次 register**（不同 kind），共享同一 `revision`；每次生成新 `run_id`。

## 命令模板

### 1) 摘要 / OCR / 可读 Markdown

```bash
babata --json process register \
  --pipeline agent_import \
  --revision rev_... \
  --item item_... \
  --kind summary \
  --provider bailian_cli \
  --model qwen-plus \
  --input-sha256 <64hex> \
  --text-file path/to/results/02-pdf-text.md \
  --language zh \
  --loss-notes "layout and figures not reconstructed"
```

### 2) ASR 原始 JSON + 可读稿

```bash
# 结构化
babata --json process register \
  --pipeline agent_import \
  --revision rev_... \
  --kind transcript \
  --provider bailian_cli \
  --model paraformer-v2 \
  --input-sha256 <64hex> \
  --json-file path/to/results/06-video-asr.json \
  --language zh \
  --loss-notes "speaker diarization from API when present; paragraphs may be post-processed"

# 可读稿（第二次 register）
babata --json process register \
  --pipeline agent_import \
  --revision rev_... \
  --kind transcript \
  --provider bailian_cli \
  --model paraformer-v2 \
  --input-sha256 <64hex> \
  --text-file path/to/results/06-video-asr.md \
  --language zh
```

### 3) 重试（保留旧 run）

```bash
babata --json process register \
  --pipeline agent_import \
  --revision rev_... \
  --kind summary \
  --provider bailian_cli \
  --input-sha256 <64hex> \
  --text-file path/to/results/retry.md \
  --retry-of run_... \
  --loss-notes "retry after empty OCR"
```

### 4) 核验

```bash
babata --json process show-run --run run_...
babata --json process list-runs --revision rev_...
```

## input-sha256 怎么算

优先顺序：

1. C0 revision 的 `text_sha256`（正文 revision）
2. 源文件 SHA-256：`Get-FileHash -Algorithm SHA256`（Windows）或 `sha256sum`
3. 若只有 Agent staging 输入，对**进入模型前的输入字节**哈希，并在 `loss_notes` 写清哈希对象

禁止：把 API Key、完整 prompt 里的密钥写进 params 或 content。

## 与 staging 目录的关系

```text
generated/<task>-bailian-clean/     # Agent 工作区（可清理）
  results/*.md|*.json
  manifest.json
  REPORT.md

derived.sqlite process_runs/derivatives   # 正式 C1（经 register）
```

- staging 可重建；C1 是登记权威。
- 不要把 `generated/` 整目录假装成已入库。
- 大媒体原件仍在 C0 assets；C1 只存派生物正文/JSON/逻辑路径。

## 汇报口径（register 后必须分开说）

```text
1. 原件 / C0 revision 是否仍只读完好
2. staging 派生物路径（generated/...）
3. 是否已 babata process register（run_id / derivative_id / kind）
4. 是否可 list-runs / show-run 回看
5. 未登记范围与下一步
```

## 反模式

- 只写 `generated/` 就说「已进入 Babata」
- 覆盖 C0 原文或 assets
- 重试时覆盖旧 run（应 `--retry-of` 新建）
- 把模型摘要写成 first-party 人工事实（那是 P6）
- 无 revision 时硬编假 id

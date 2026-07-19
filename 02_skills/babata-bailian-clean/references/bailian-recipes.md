# 百炼命令菜谱（清洗用）

鉴权与完整 CLI 以 `$bailian-cli` 为准。这里只收清洗高频。

## 文本结构化

```bash
bl text chat --messages-file messages.json --max-tokens 900 --output json --quiet
```

`messages.json`：

```json
[{"role":"user","content":"...提示词 + 原文..."}]
```

## 图像 / 页图 OCR 与描述

```bash
bl vision describe --image <path> --prompt "<中文任务>" --output json --timeout 180
```

提示词应明确要：OCR 全文 / 摘要 / 标签 / 是否手写校正。

## 语音识别（视频主路径）

```bash
bl speech recognize --url <audio-or-url> --language zh --diarization --out asr.json --output json --timeout 300
```

- 可加 `--language en` 等；以实际口音/课程语言为准。
- diarization 失败则去掉重试。
- 读 `asr.json`：常见字段含 `transcripts`、分句 `begin_time`/`end_time`（毫秒）、`speaker_id`、词级 `words`。

把 JSON 再交给 `text chat` 生成：

- 可读逐字稿（保留时间戳/说话人若存在）
- 智能段落与小标题
- 摘要与关键词

并标注哪些字段来自 ASR 原生、哪些来自后处理。

## 视频理解（辅助）

```bash
bl vision describe --video <path-or-url> --prompt "总结课程内容要点" --output json
```

优先程度：**ASR 全文 > 截帧 OCR > 整段 VL 摘要**。课视频内容以口播为主时不要只用 VL。

## 临时上传

仅当命令不接受本地路径或需 `oss://` 时：

```bash
bl file upload --file <path> --model <target-model>
```

## Windows 注意

- 优先 `bl.cmd` 全路径给 Python `subprocess`
- 含 `&` 的 URL 不要经未转义的 `cmd /c start`
- 输出统一 `--output json` 再本地抽 `choices[0].message.content`

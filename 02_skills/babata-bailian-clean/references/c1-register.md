# Agent 清洗结果登记到 C1

`generated/` 只是可清理工作区。正式 C1 必须由 `babata process register` 写入，并能回到真实 C0 输入。

## 登记前证据

文件来源必须先取得以下四项，缺一项就先 capture/collector，不得创建说明文本占位：

```text
item_id
revision_id
asset_id
asset sha256
```

已有 ready revision 后才找回真实原件或平台预览时，追加新 C0 revision，不修改旧版本：

```bash
babata --json capture attach-assets \
  --revision rev_... \
  --original path/to/uploaded-source.docx \
  --preview path/to/platform-preview.pdf \
  --reason "recover source file and distinguish platform preview"
```

新 revision 中 `original` 与 `preview` 必须分别显示；后续文件清洗绑定 `original` asset。

正文 revision 直接产生 summary/tags/structured_result 时可以没有 asset，但 `input-sha256` 必须等于该 revision 的 `text_sha256`。

以下 kind 必须绑定 asset：

```text
extracted_text
ocr_text
transcript
subtitle
visual_description
key_frame
media_metadata
```

文件经过转码、切片、抽帧或渲页时：

- `--input-sha256` 仍写 C0 原件 asset 哈希。
- `--input-asset-id` 仍写 C0 原件 asset id。
- 在 `--params-json` 写规范化步骤、范围和真正送给 provider 的输入哈希。
- 在 `--loss-notes` 写截断、降采样、缺页、语言或说话人等损失。

## 必填身份

每次成功或失败登记都要有：

```text
--pipeline
--revision
--kind
--provider
--model
--tool-version
--input-sha256
```

文件来源再加：

```text
--item
--input-asset-id
```

`--usage-json` 只在 provider 真实返回用量时填写；没有就保留 `{}`，不得估算。

## 正式文件规则

推荐把可读文本或 JSON 同时作为内联内容和受控文件登记：

```bash
babata --json process register \
  --pipeline agent_import \
  --revision rev_... \
  --item item_... \
  --kind ocr_text \
  --provider bailian_cli \
  --model qwen-vl-plus \
  --tool-version 1.10.0 \
  --input-sha256 <C0-asset-sha256> \
  --input-asset-id asset_... \
  --text-file path/to/results/image-ocr.md \
  --output-file path/to/results/image-ocr.md \
  --params-json '{"provider_input_sha256":"<hash>","preprocessing":[]}' \
  --language zh \
  --loss-notes "layout not reconstructed"
```

`--output-file` 在全部 C0、retry 和输出一致性校验通过后才进入 staging，成功后落到：

```text
02_derived/files/sha256/<prefix>/<output-sha256>
```

禁止：

- 用 `--logical-path generated/...` 登记可清理 staging。
- 同时提供内容不同的 `--text-file` / `--json-file` / `--output-file`。
- 把任意 data-root 路径当作正式 C1。

只有已经位于 `02_derived/files/sha256/` 且可重新核验哈希的文件才可用 `--logical-path`。

## PDF / 图片 / 视频模板

PDF 本地抽取正文：

```bash
babata --json process register \
  --pipeline agent_import \
  --revision rev_... --item item_... \
  --kind extracted_text \
  --provider local_extract --model pypdf --tool-version <version> \
  --input-sha256 <pdf-asset-sha256> --input-asset-id asset_... \
  --text-file results/pdf-text.md --output-file results/pdf-text.md \
  --params-json '{"provider_input_sha256":"<pdf-asset-sha256>","preprocessing":[]}'
```

视频 ASR 可读稿：

```bash
babata --json process register \
  --pipeline agent_import \
  --revision rev_... --item item_... \
  --kind transcript \
  --provider bailian_cli --model fun-asr --tool-version 1.10.0 \
  --input-sha256 <video-asset-sha256> --input-asset-id asset_... \
  --text-file results/video-transcript.md --output-file results/video-transcript.md \
  --params-json '{"provider_input_sha256":"<wav-sha256>","preprocessing":["first 180 seconds","16kHz mono WAV"]}' \
  --language en \
  --loss-notes "source limited to first 180 seconds; final low-confidence segment retained"
```

ASR 原始 JSON 另建一个 run，使用同一个 C0 视频身份：

```bash
babata --json process register \
  --pipeline agent_import \
  --revision rev_... --item item_... \
  --kind structured_result \
  --provider bailian_cli --model fun-asr --tool-version 1.10.0 \
  --input-sha256 <video-asset-sha256> --input-asset-id asset_... \
  --json-file results/video-asr.json --output-file results/video-asr.json \
  --params-json '{"provider_input_sha256":"<wav-sha256>","preprocessing":["first 180 seconds","16kHz mono WAV"]}'
```

## 失败与重试

Provider 真实失败时先登记失败身份：

```bash
babata --json process register-failure \
  --pipeline agent_import \
  --revision rev_... --item item_... \
  --kind transcript \
  --provider bailian_cli --model fun-asr --tool-version 1.10.0 \
  --input-sha256 <video-asset-sha256> --input-asset-id asset_... \
  --params-json '{"provider_input_sha256":"<wav-sha256>","preprocessing":["first 180 seconds","16kHz mono WAV"]}' \
  --error-code provider_timeout \
  --error-message "ASR request timed out"
```

修复后对该 failed run 重试：

```bash
babata --json process register \
  <与失败 run 完全相同的 pipeline/revision/item/kind/input/asset 身份> \
  --retry-of run_... \
  --text-file results/video-transcript.md \
  --output-file results/video-transcript.md
```

只有 failed run 能作为父 run。retry 不得改变 revision、item、input hash、asset、pipeline 或 kind；每次重试创建新 run，父 run 保留。

参数校验失败发生在 run 创建前，不算 provider failed；修正参数后重新 register，不要伪造 `--retry-of`。

## 删除与重建

删除一个可重建 C1 结果时保留审计记录，但把该 run 明确标为失效：

```bash
babata --json process delete-result \
  --run run_... \
  --reason "why this C1 result is no longer authoritative"
```

重建是新的独立处理 run，不是 provider 失败重试，不传 `--retry-of`。旧 run 的
`invalidated_at` / `invalidation_reason` 必须可见；C0 revision、asset 和哈希保持不变。

## 核验

```bash
babata --json process show-run --run run_...
babata --json process list-runs --revision rev_...
```

逐项核对：

```text
run.target_kind == derivative.kind
run.input_asset_id == derivative.input_asset_id == C0 asset_id（文件来源）
run.input_sha256 == C0 text_sha256 或 asset sha256
logical_path 位于 02_derived/files/sha256/
output_sha256 == 正式文件/内联内容实际哈希
provider、model、tool_version、params、usage、loss_notes 与真实调用一致
retry_of 只指向同身份 failed run
有效结果的 invalidated_at 为空；已删除结果有 invalidation_reason
```

API Key、鉴权文件、完整敏感 prompt 不进入 params、content、日志或报告。

## 受控作业队列

对已经由 Babata 正式保存的 C0，可以让核心队列直接执行当前已启用的窄 pipeline：

```bash
babata --json process enqueue --pipeline local_extract_text --revision rev_...
babata --json process enqueue --pipeline bailian_summary --revision rev_...
babata --json process run-once
babata --json process status job_...
babata --json process retry job_...
babata --json process cancel job_...
```

- `local_extract_text` 必须读取该 revision 上真实、UTF-8、文本类 C0 asset，不接受任意外部路径。
- `bailian_summary` 必须读取 ready revision 的 C0 正文；只有 `bl` 可执行且已鉴权时才 enabled。
- queue job 保存在可清理的 C3 `runtime.sqlite`，provider 成功/失败仍由同一个 `ProcessService`
  登记到 C1；job 只引用 `result_run_id`，不成为第二 C1 writer。
- retry 新建 job，并让新的 C1 run 指向旧 failed run；父 job/run 均保留。
- provider task/request ID、实际 usage 和 queue job ID 进入 C1 的非敏感处理元数据；鉴权输出不进入。
- `bailian_ocr`、`bailian_transcript`、`bailian_visual_description` 尚无 queue provider 时明确
  unavailable；继续由本 Skill 完成真实多模态调用并用 `agent_import` 登记。

Provider 原始 JSON 还必须检查并移除：

```text
临时上传/下载签名 URL
URL 查询参数中的 token、Signature、credential
Authorization/Cookie 等鉴权头
账号密钥或可复用会话凭据
```

只把脱敏后的 JSON 作为 `--json-file` / `--output-file` 登记；在 `--params-json` 增加
`sanitization` 数组说明删除了什么。完整响应确需审计时，只进入明确受限且不会被普通
检索、视图或输出消费的证据区，不能把普通 C1 当凭据仓库。

# Provider recipes

Read only the section for the selected adapter. Provider commands create staging candidates; Babata
remains the only formal C1 writer.

## QianWen text and vision skills

Use the installed QianWen skill matching the task:

- `qianwen-text` for summaries, structure, tags, and controlled JSON output.
- `qianwen-vision` for OCR, page layout, screenshots, charts, and visual reasoning.
- `qianwen-model-selector` when model quality, latency, capability, or current price determines routing.

For readable documents, extract text locally first. Send only the required text or chunks. For scans,
OCR only pages without an adequate text layer. Save sanitized response JSON and a readable result in
the task staging directory.

Register QianWen output with `--provider qianwen_skill`, the exact model, and the installed skill or
script version. Put `service: dashscope`, `adapter: qianwen_skill`, and
`credential_source: environment` in `params-json`.

## Bailian CLI

Use the official `bailian-cli` (`bl`) when its command surface is the best route, especially speech
recognition or Bailian-owned platform resources. Read `$bailian-cli` for current flags instead of
copying a full CLI manual here.

Common ASR shape:

```bash
bl speech recognize \
  --url <local-audio-path> \
  --model <supported-asr-model> \
  --language zh \
  --diarization \
  --out results/asr.json \
  --output json
```

The command accepts `--model`; compare candidate ASR models on the same representative input before a
large batch. If diarization fails, one retry without it is allowed. Preserve native transcript fields
before producing a readable Markdown form.

Register CLI output with `--provider bailian_cli`, the exact model, and `bl --version`. Put
`service: dashscope`, `adapter: bailian_cli`, and `credential_source: environment` in `params-json`.

## Local adapters

Use stable local parsers for deterministic extraction and `ffmpeg`/`ffprobe` for media probes and
normalization. Register with `--provider local_extract`, the parser/tool name as the model, and its
exact version. Local output still requires the same C0 binding, managed C1 file, and read-back audit.

## Shared prohibitions

- Do not put API keys or console tokens on command lines, in files, or in reports.
- Do not preserve temporary upload/download signatures in ordinary staging or C1.
- Do not register a provider response before checking that it describes the intended complete input.
- Do not hide adapter changes by reusing an inaccurate provider or tool identity.

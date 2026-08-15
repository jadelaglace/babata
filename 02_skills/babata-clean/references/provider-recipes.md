# Provider recipes

Read only the section for the selected adapter. Provider commands create staging candidates; Babata
remains the only formal C1 writer.

## Authentication

For DashScope-backed adapters, use the user-level `DASHSCOPE_API_KEY`. Do not copy it into repository
`.env` files, provider config files, prompts, manifests or C1 metadata. Check authentication without
printing credentials and record only `credential_source: environment`. Do not make a second model call
merely to prove an adapter after an authenticated task already succeeded.

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

## QianWen direct API

Use the OpenAI-compatible DashScope API for repeatable text batches when direct control over request
JSON, structured output, usage and cost accounting is more useful than the Skill wrapper. The Skill
and direct API may call the same underlying model; do not treat them as two independent quality
sources. Register with `--provider qianwen_api`, the exact callable model ID and the adapter/script
version. Record endpoint region, prompt profile/version/hash, schema version, request ID, input/output
hash, usage and pricing reconciliation status. Never put the API key in params or staging.

## Codex Agent

Use a Codex Agent for high-value semantic organization, relation judgment, independent review and
repair of failed claims. It is not a replacement for deterministic extraction, OCR or ASR. Register
an accepted candidate with `--provider codex_agent`; record the available Codex model/harness version,
task/session/agent identity, prompt profile/hash, input derivative IDs and hashes, output hash, and
usage when the harness exposes it. Use `{}` when usage is unavailable; do not estimate hidden tokens
or convert elapsed time into a fabricated API cost.

For an API/Agent hybrid, do not send the complete C1 input through both routes by default. The Agent
first receives the structured candidate, citations and validator failures; it reads only the cited or
failed C1 spans needed to decide or repair. Full dual runs are explicit comparison experiments and
must keep separate cost and quality records.

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
- Do not place control instructions, pilot status, storage boundaries, provider explanations or review
  policy in a knowledge/content field. Keep them in prompt control context, manifest, metadata or REPORT,
  and reject candidates that echo them into the content payload.

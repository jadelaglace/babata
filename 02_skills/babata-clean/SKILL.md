---
name: babata-clean
description: >
  Clean Babata C0 material into traceable C1 derivatives through a provider-neutral Agent workflow.
  Use when the user wants to extract, OCR, transcribe, summarize, structure, tag, compare processors,
  or register derived results for documents, images, audio, video, course material, or an existing
  Babata revision. Route between deterministic local tools, QianWen Skills, Bailian CLI, and future
  adapters without giving any adapter database authority. Keep C0 read-only, use staging for
  candidates, and register accepted output only through `babata process register`.
---

# Babata Clean

Babata owns the cleaning workflow. Local tools, QianWen Skills, Bailian CLI, and future providers
are replaceable or complementary adapters. None of them owns C1 or writes SQLite or managed assets.

Use `BABATA_DATA_HOME/04_runtime/staging/model-workspaces/<task>/` for working files. A result is
formal C1 only after the Rust application/core accepts it through `babata process register`.

## Non-negotiable boundaries

1. Keep the source C0 revision and assets read-only.
2. Bind every file-derived result to the real C0 item, revision, asset, and asset hash.
3. Treat every adapter output as untrusted until validated and registered by Babata.
4. Preserve adapter, service, model, tool version, preprocessing, usage, output hash, and limitations.
5. Remove temporary signed URLs, tokens, authorization headers, cookies, and reusable credentials
   before ordinary staging or C1 registration.
6. Never put credentials, real model output, media, databases, or runtime logs in Git.
7. Do not reacquire source material merely because C1 exposes a gap. Record a separate acquisition
   request and wait for authorization.

## Workflow

```text
1. Define scope and read the existing C1 coverage
2. Bind each target to its authoritative C0 identity and hash
3. Probe modality, structure, duration, extractability, and obvious limitations locally
4. Reuse a complete active C1 result when it already satisfies the need
5. Select one or more adapters by fidelity, capability, speed, and cost
6. Normalize only the provider input; never replace the C0 original
7. Run a representative sample before expanding a new route
8. Sanitize and validate staging Markdown/JSON plus its manifest
9. Register accepted output through `agent_import`
10. Read the run back and audit C0/C1 integrity
```

Default staging layout:

```text
04_runtime/staging/model-workspaces/<task>/
  preprocessed/
  results/
  manifest.json
  REPORT.md
```

Read [references/output-contract.md](references/output-contract.md) before creating the manifest and
[references/c1-register.md](references/c1-register.md) before any formal registration.

## Adapter selection

Prefer deterministic local work before remote interpretation.

| Need | Preferred adapter | Escalation |
| --- | --- | --- |
| Extract readable PDF/Office/text content | local parser | Remote model only for useful summary or structure |
| Summarize or structure readable text | QianWen text skill | Stronger text model for difficult/high-value material |
| OCR a scan, page image, or screenshot | QianWen vision/OCR skill | General VL model for complex layout or visual reasoning |
| Transcribe audio or video | Bailian CLI speech recognition | Compare supported ASR models on one representative input |
| Inspect model capability, price, or parameters | QianWen model selector or live provider catalog | Record the decision separately from C1 output |
| Unsupported or unavailable route | Another authorized adapter | Keep the same staging and registration contract |

Use [references/media-routing.md](references/media-routing.md) for local probes and
[references/provider-recipes.md](references/provider-recipes.md) only for the selected adapter.

## DashScope authentication

For the current local workflow, QianWen Skills and Bailian CLI share the user-level
`DASHSCOPE_API_KEY`. Do not copy it into `.bailian/config.json`, a repository `.env`, prompts,
manifests, or C1 metadata. Record only `credential_source: environment`.

Check authentication without printing credentials. Do not make a second model call merely to prove
an adapter after an authenticated task has already succeeded.

## Provenance convention

Keep the adapter identity honest in `--provider` and put the underlying service and credential source
in `--params-json`:

```json
{
  "service": "dashscope",
  "adapter": "bailian_cli",
  "credential_source": "environment",
  "provider_input_sha256": "<normalized-input-sha256>",
  "preprocessing": ["16kHz mono FLAC"]
}
```

Examples of adapter identities are `local_extract`, `qianwen_skill`, and `bailian_cli`. The exact
model and adapter version remain separate required fields. Never label a model-selection suggestion
or a staging file as formal C1.

## Registration

For a file-derived result:

```bash
babata --json process register \
  --pipeline agent_import \
  --revision rev_... \
  --item item_... \
  --kind <extracted_text|ocr_text|transcript|summary|structured_result|tags> \
  --provider <local_extract|qianwen_skill|bailian_cli> \
  --model <exact-model-or-parser> \
  --tool-version <exact-version> \
  --input-sha256 <C0-asset-sha256> \
  --input-asset-id asset_... \
  --text-file results/result.md \
  --output-file results/result.md \
  --params-json <sanitized-json> \
  --language zh \
  --loss-notes <honest-limitations>
```

The C0 hash remains the registered input hash even when an adapter receives normalized audio,
rendered pages, chunks, or a resized image. Record the normalized input hash and transformation in
`params-json`.

Use `register-failure` only after a real provider attempt created a stable processing identity. A retry
must reference the failed run and keep the same target identity. Parameter validation failures occur
before run creation and are not provider retries. Rebuilding an invalidated result creates a new run.

## Video and audio

For course or meeting video, prioritize full-duration ASR over generic video summaries:

1. Inspect duration and audio streams with `ffprobe`.
2. Extract a loss-controlled mono 16 kHz provider input and record its hash.
3. Test a short representative segment when the adapter/model route is new.
4. Process the full authorized duration before claiming transcript completion.
5. Preserve native timestamps, speakers, confidence, and segmentation when available.
6. Mark model-created headings or paragraphs as post-processing, not native ASR fields.
7. Use frame OCR or vision only to supplement slides, boards, charts, or visual-only evidence.

## Batch discipline

- Batch straightforward local extraction before remote exceptions.
- Attempt one object through one route at most twice.
- Stop and report when the same common failure reaches three objects.
- Preserve succeeded, failed, deferred, not-applicable, and staged-only as distinct states.
- Do not rerun a complete active derivative solely because adapter policy changed.
- Report Recovery/staging, formal C1, and any later C2 work separately.

## Verification

After registration, use `process show-run` and `process list-runs` to verify:

```text
run and derivative target identity match
input revision/asset/hash match authoritative C0
managed output hash matches actual content
provider/model/tool/params/usage/loss notes match the real operation
temporary credentials and signed URLs are absent
retry parent is a failed run with the same identity
active and invalidated results are distinguishable
```

Do not call C1 complete until the item-level coverage ledger and raw/derived integrity checks agree.

# C1 Staging and Handoff Contract

Use this contract for untrusted processing candidates and the manifest that binds them to C0. Staging is
deletable workspace; it is never formal C1 or a downstream C2 package.

## Contents

- [Manifest](#manifest)
- [C1B retained modality](#c1b-retained-modality)
- [Required task artifacts](#required-task-artifacts)
- [Registration boundary](#registration-boundary)
- [Report shape](#report-shape)

## Manifest

```json
{
  "schema": "babata.c1-staging/v1",
  "task": "<task-id>",
  "created_at": "<ISO-8601>",
  "items": [
    {
      "id": "<staging-item-id>",
      "c0": {
        "item_id": "item_...",
        "revision_id": "rev_...",
        "asset_id": "asset_... or null",
        "input_sha256": "<authoritative-C0-hash>"
      },
      "processing": [
        {
          "provider": "local_extract|qianwen_skill|qianwen_api|bailian_cli|codex_agent",
          "model": "<exact-model-or-parser>",
          "tool_version": "<exact-version>",
          "provider_input_sha256": "<normalized-input-hash>",
          "preprocessing": []
        }
      ],
      "derivatives": [
        {
          "kind": "extracted_text|ocr_text|transcript|summary|structured_result|tags|key_frame",
          "path": "results/<file>",
          "sha256": "<output-hash>",
          "loss_notes": []
        }
      ],
      "essence_judgment": {
        "text_sufficient": true,
        "retained_modalities": ["text"],
        "decision_basis": "<why retained media is or is not necessary>",
        "missing_or_loss_notes": []
      },
      "registrations": [],
      "status": "staged_only|registered|partial|failed"
    }
  ]
}
```

Rules:

- `c0.input_sha256` is always the authoritative C0 text/asset hash, not a rendered page, clip or transcoded file.
- Record normalized provider input and preprocessing separately.
- Omit `essence_judgment` when the task does not perform C1B judgment; never fabricate it as a required field.
- `registrations` contains only core-read-back run/derivative IDs and managed hashes after successful register.
- The manifest does not contain C2 materialization, knowledge-universe assignment, publish or live-export state.

## C1B retained modality

When C1B retains an image, audio, video or attachment excerpt, add a derivative record with:

```json
{
  "c1_variant": "c1b",
  "modality": "image|audio|video|attachment",
  "path": "results/excerpts/<file>",
  "sha256": "<excerpt-hash>",
  "source_locator": {
    "page": 12,
    "time": "00:31:10-00:31:42",
    "crop": [0, 0, 1600, 900]
  },
  "source_locator_status": "available|partial|unavailable",
  "processing": [{"tool": "<tool>", "operation": "<operation>"}],
  "loss_notes": []
}
```

Retain only media that changes understanding; text-sufficient material may remain text-only. Locator metadata is
desirable but not a substitute for excerpt bytes/hash. Complete originals and source directory structure remain
with C0/external sovereign storage, not staging.

## Required task artifacts

- `manifest.json` using the schema above;
- readable Markdown/JSON derivative files actually needed by the task;
- sanitized provider response JSON only when a provider returned useful structured evidence;
- preprocessing files only when they are required to reproduce the provider input;
- `REPORT.md` with scope, results, failures, limitations and registration read-back.

Do not require a provider JSON for deterministic local extraction. Do not save credentials, temporary signed URLs,
authorization headers or cookies in ordinary results, manifest, report or C1.

## Registration boundary

Follow [c1-register.md](c1-register.md). `staged_only` means no formal register occurred. Only after core read-back
may the item be marked `registered` and its run/derivative/managed hashes added to the manifest.

A downstream C2B builder receives complete registered C1 text, registered essence decisions, registered retained
media/rebuild identity, C0/C1 hashes and loss notes. It does not receive permission to reread external originals or
ask this Skill to publish output.

## Report shape

1. explicit scope and C0 identities;
2. local probes/preprocessing;
3. adapter/model decisions and actual outputs;
4. C1B essence/media decisions when applicable;
5. formal registration read-back, separated from staging;
6. failures, gaps and honest limitations.

Never report “entered C1” when only staging exists.

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
8. End a general cleaning task at validated C1. When C1B/C2B is explicitly in scope, end at a formal
   C1B handoff. A C2B builder must not use this Skill to reread external originals, create
   knowledge-universe assignments, render course maps, or publish Obsidian output.

## C1B to C2B handoff

`C2B-DOCS-FIRST-GATE` applies before changing a downstream C2B capability: follow the single authority
order in `00_docs/README.md`, review each affected role, and update only changed semantics or contracts
before changing a builder, registration script, or publisher.

The handoff contains complete validated C1 text, essence judgments, retained modality assets or rebuild
recipes, C0/C1 identities and hashes, and honest loss notes. Downstream C2B consumes only that handoff and
formal core records; it does not ask `babata-clean` to become an output writer and does not reread the
external sovereign original to fill media gaps. Knowledge-universe registration and package-owned course
maps remain responsibilities of the output/core path.

`C1B-FORMAL-HANDOFF-GATE` applies at promotion: staging decisions are not formal C1B. Reuse the unique
active complete-C1 derivative, register each essence decision and retained media derivative through the
Rust core, read back managed paths and hashes, and emit a `registered` ledger. A formal C2B builder must
reject a missing, partial, duplicated or hash-inconsistent ledger.

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
9. Register accepted output through `babata process register --pipeline agent_import`
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

Read [references/c1-staging-contract.md](references/c1-staging-contract.md) before creating the manifest and
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

## Provenance and registration

Keep provider, adapter, model and tool version as separate truthful identities. Always record the
authoritative C0 hash, normalized provider-input hash, preprocessing, usage when actually returned,
sanitization and loss notes. Never label model selection, staging or an unregistered candidate as C1.

Follow [references/c1-register.md](references/c1-register.md) for commands, required identities, managed
paths, failure/retry semantics, deletion and read-back. The authoritative registered input remains the C0
text/asset hash even when the provider receives rendered pages, chunks, clips or transcoded media.

Use `register-failure` only after a real provider attempt created a stable run identity. Parameter validation
before run creation is not a provider failure; rebuilding an invalidated derivative is not a retry.

## Modality completion

Follow [references/media-routing.md](references/media-routing.md). For audio/video, complete the authorized
duration before claiming transcript coverage; preserve native timestamps/speakers/confidence when available,
and label model-created structure as post-processing. Use visual processing only for evidence that text/ASR
does not preserve.

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

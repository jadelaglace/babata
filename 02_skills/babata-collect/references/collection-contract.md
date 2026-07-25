# Collection Contract

## Completion Boundary

Collection ends at `AcquisitionPackage -> Rust application/core -> verified C0`. C0 is complete
without C1. C1 may later read a specific C0 revision or asset, but collection never owns, starts, or
waits for that processing.

The source-specific boundary ends when original bytes or text, source context, attachments, access
state, limitations, and collection identity are handed to the unified Collector/Capture use case.
There is no OneNote C0/C1 pipeline, Doubao C0/C1 pipeline, or other source-private data layer.

## Authority

| Material | Status | May count as formally collected? |
| --- | --- | --- |
| File still in platform or official export directory | source material | No |
| Agent/browser/CLI download in temporary or Recovery storage | acquired candidate | No |
| `collector` result with `state=saved`, `item_id`, `revision_id` | unified C0 | Yes |
| OCR, transcript, summary, structure, tags | C1 derivative | Not part of collection |

Only Rust application/core assigns final IDs, versions, assets, hashes, observations, and managed
paths. Skills, scripts, browser tools, Python, and provider tools must not directly write SQLite or
managed C0 assets.

## Scope and Authorisation

- Record a human-readable scope and a local authorisation reference; never put passwords, cookies,
  tokens, or OAuth secrets in command arguments, manifests, logs, or Git.
- A connected account is context, not consent to collect all data.
- Within a clear authorised scope, continue without per-item confirmation.
- Stop for ambiguous scope, account-wide expansion, unavailable route, or irreplaceable login.
- Cancellation stops queued expansion while preserving already completed C0.

## Fidelity

- Preserve exact original exports/media and their hashes.
- Keep known complementary representations as assets of the same item only when the recipe proves
  they are the same export scope.
- Do not infer missing native IDs, hierarchy, timestamps, authors, or relationships.
- Do not split/merge rich documents, deduplicate semantically, or promote overlap hints to facts.
- Record limitations and absent fields honestly.

## Required Report

Always separate:

1. **拿回**: what the Agent or source tool retrieved and verified outside C0;
2. **正式登记**: session, saved/failed/skipped counts, item/revision references, attachments, and
   limitations committed by the unified C0 path;
3. **长期重复**: whether the same recipe is enabled and proven for recollection, or what remains.

End with an explicit statement that C1 was not required or triggered. If only layer 1 completed,
say so; do not imply layer 2 or 3.

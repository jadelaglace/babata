# Collection Contract

## Completion Boundary

Collection distinguishes sovereignty recovery from formal registration. A1/A2 acquisition may end
successfully before registration when source-loss risk or current capability makes "get the bytes
back now" the correct boundary. Formal collection ends at
`AcquisitionPackage -> Rust application/core -> registered C0-C read-back`. Registered C0 is complete
without C1. C1 may later read a specific registered revision or asset, but collection never owns,
starts, or waits for that processing.

The source-specific acquisition boundary owns A1/A2 retrieval and may stop with an honest
captured/prepared result. When formal registration is available, original bytes or text, source
context, attachments, access state, limitations, and collection identity are handed to the unified
Collector/Capture use case. There is no OneNote C0/C1 pipeline, Doubao C0/C1 pipeline, or other
source-private data layer.

## Sovereignty Depth

| Depth | Required result |
| --- | --- |
| `A1` | Preserve immediately available bytes, responses, exports, source identity, hashes, and limitations without waiting for parsing, recursion, or registration. |
| `A2` | Retrieve the body, embedded media, and required attachments needed to make the A1 material locally complete; record each unresolved dependency. |
| `A3+` | Follow a semantic reference only when explicitly valuable and authorised. Save the result as a separate node/reference and stop recursion by default. |

Body text, embedded media, and required attachments are completeness dependencies and normally
belong to A2. A cited article, paper, or page is normally a reference, not an A2 dependency.

## Management Readiness

| Readiness | Meaning |
| --- | --- |
| `captured` | Locally preserved and verifiable. |
| `prepared` / `C0-B` | After A2 is complete, non-semantic extraction, stable naming, format identification, manifest, and field preservation are complete without overwriting upstream originals. |
| `registered` / `C0-C` | After A2 and preparation, the unique Rust application/core assigned SQL identity, revision, assets, provenance, relations, status, and successful read-back. |

Depth and readiness are reported independently, but readiness has an admission rule: A1 may only be
`captured`; A2 is required before `prepared / C0-B` or `registered / C0-C`. A3+ is optional and is not
an admission requirement. Only `registered / C0-C` may be
reported as "formally collected", "entered C0", or "formal C0". A1/A2 captured or prepared material
may still be a successful sovereignty recovery and must not be reported as if nothing was retrieved.

## Authority

| Material | Status | May count as formally collected? |
| --- | --- | --- |
| File still in platform or official export directory | source material | No |
| Agent/browser/CLI download in temporary or Recovery storage | A1/A2 captured or prepared | No, but it may count as successful sovereignty recovery |
| `collector` result with `state=saved`, `item_id`, `revision_id` and read-back | registered / C0-C | Yes |
| OCR, transcript, summary, structure, tags | C1 derivative | Not part of collection |

Only Rust application/core assigns final IDs, versions, assets, hashes, observations, and managed
paths. Skills, scripts, browser tools, Python, and provider tools must not directly write SQLite or
managed C0 assets.

C1/C2 may identify a missing reference and request a new authorised A3/A4 acquisition. They must not
rewrite an old C0 record directly; each newly acquired result follows this contract as its own record
or reference.

A bounded representative scope may advance to A2/B/C while the rest of a recovered source remains
A1. The report must preserve both scopes. For the P7 WeChat proof, select 10-20 representative items
across both File Transfer Assistant and Favorites, prioritise URL/article items with a small local-media
cross-section, and require body, embedded media, and required attachments before preparation or registration.

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

1. **拿回**: what the Agent or source tool retrieved and verified, including A1/A2/A3+ depth and
   captured/prepared/registered readiness;
2. **正式登记**: session, saved/failed/skipped counts, item/revision references, attachments, and
   limitations committed and read back through registered C0-C;
3. **长期重复**: whether the same recipe is enabled and proven for recollection, or what remains.

End with an explicit statement that C1 was not required or triggered. If only layer 1 completed,
say so; do not imply layer 2 or 3.

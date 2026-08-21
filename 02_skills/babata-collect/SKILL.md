---
name: babata-collect
description: >
  Route one explicitly scoped collection request through Babata's verified source recipes and
  unified Rust Collector/Capture path into registered C0-C when available, while allowing urgent
  C0-A1/C0-A2 sovereignty recovery to complete before registration. Use when the user asks to collect, recover, import,
  export, recollect, retry, or cancel personal material from OneNote PDF/MHT exports, Evernote or
  Yinxiang .notes exports, Doubao or other chat histories, websites, browser state, official apps,
  or local source exports. This is the single user-visible collection Skill: platform recipes are
  internal, capability state must be checked before execution, and formal collection ends at registered C0-C.
  Do not use for OCR, transcription, summarization, C1 processing, knowledge promotion, or output.
---

# Babata 总收集

Use this as the only collection entry point. Identify the source and authorised scope, load one
internal source recipe, use the best verified acquisition tool, and submit through Babata's Rust
application/core. Never create a platform-specific user Skill.

Judge and improve routes in this fixed order: **stable, accurate, real, then fast**. Once the first
three are proven, measure the live run, remove repeated navigation and indecision, and retain one
working route plus one explicit fallback. Do not preserve speculative branches or keep researching
after the authorised scope can be completed honestly.

## Mandatory Contract

Read these two references for every task:

- [references/collection-contract.md](references/collection-contract.md)
- [references/route-catalog.md](references/route-catalog.md)

Then read exactly one relevant source reference:

- OneNote: [references/source-onenote.md](references/source-onenote.md)
- Evernote / 印象笔记: [references/source-evernote.md](references/source-evernote.md)
- 豆包: [references/source-doubao.md](references/source-doubao.md)
- YouTube: [references/source-youtube.md](references/source-youtube.md)
- Website, browser, desktop app, or unknown source:
  [references/source-browser-and-ui.md](references/source-browser-and-ui.md)

## Hard Boundaries

1. Require a source and an explicit item, conversation, export, folder, time, count, or visible-set
   scope. Login or connection alone never authorises account-wide collection.
2. Treat `babata --json capabilities list` as runtime truth. Execute only an `enabled` source route.
   Return the real reason for `disabled`, `unavailable`, absent, or unknown routes.
3. Use browser, Lark, desktop-control, platform CLI, and official export tools only as acquisition
   dependencies. They may produce C0-A1/C0-A2 captured or prepared results, but do not own formal C0.
4. Let only Babata Rust application/core finalise originals, IDs, revisions, assets, hashes, and
   source observations. Never write SQLite or managed asset directories directly.
5. Prefer C0-A1 preservation when source loss is urgent; then obtain C0-A2 body, embedded media, and required
   attachments. Do not prepare or register C0-A1-only material; C0-A2 is required for C0-B/C0-C. Follow
   semantic references only as bounded, explicitly valuable C0-A3+ tasks; C0-A3+ is not a B/C prerequisite. End formal
   collection after every selected candidate is `saved` with `item_id` and `revision_id`, or after honest
   captured/prepared/failed/skipped/cancelled results are reported. Never call `babata process`, a cleaning Skill,
   OCR/ASR, semantic digest, or knowledge commands as part of collection.
6. Do not split, merge, deduplicate, infer hierarchy, or rewrite source material during preparation or C0 capture.
   Preserve actual exports and record limitations. Later C1 may independently read C0.

## Workflow

### 1. Resolve source and scope

Extract:

- source platform or source kind;
- exact authorised scope;
- requested attachment coverage;
- whether this is a first collection, retry, cancellation, or recollection.

Inspect accessible context before asking the user for a path or identifier. Ask only when scope is
genuinely ambiguous, would expand to account-wide data, or the source requires an irreplaceable
login/authorisation action. Reject phrases such as "collect everything connected" until the user
names a bounded source scope.

### 2. Check runtime capability

Prefer an installed `babata` executable:

```text
babata --json capabilities list
```

From the repository, the equivalent fallback is:

```text
cargo run --quiet --manifest-path 01_app/Cargo.toml -p babata-cli --bin babata -- --json capabilities list
```

Match the source to a `source.*` ID. `enabled` means the currently declared recipe may run;
`disabled` and `unavailable` must stop before formal collection. An absent route is unavailable,
even if a browser can visibly read the site. Do not use the currently unavailable `babata routes`
command as a substitute for capability status.

### 3. Acquire through the source recipe

Follow the eight-level order in the route catalog: official free migration/export, existing
plugin/script, Agent-led export, narrow development, paid capability, heavy development,
continuous human collaboration, manual-only. Use the first route that completely and legally
covers this scope.

Keep transient downloads outside Git and outside managed C0 assets. Do not call them saved in
Babata. Verify file existence, type, size, and hash when the recipe provides those facts.

### 4. Start one unified Collector session

For an enabled adapter-backed route:

```text
babata --json collector start --route <source.id> --source <recipe-source-reference> --scope <human-readable-scope> --authorisation <local-authorisation-reference>
babata --json collector candidates --session <session_id>
babata --json collector select --session <session_id> --candidate <id> [--candidate <id> ...] --scope <selected-scope> --authorisation <same-reference> --attachments --confirm
babata --json collector status --session <session_id>
```

Select only candidates inside the authorised scope. A whole-export request may select every
candidate discovered from that one export; a visible-set or conversation request may not silently
expand beyond it.

Do not fabricate candidate envelopes or route evidence. If the recipe only obtains temporary files
but no enabled formal route can accept them, preserve and verify them without overwriting upstream
originals, report their C0-A1/C0-A2 and captured/prepared status as "拿回但未正式登记", and stop.

### 5. Handle interruption and recollection

```text
babata --json collector cancel --session <session_id> --reason <reason>
babata --json collector retry --session <session_id> --candidate <failed_candidate_id>
babata --json collector recollect --item <item_id>
babata --json collector recollect-session --session <session_id>
```

Cancellation preserves already saved items and skips remaining queued work. Retry only a reported
retryable failure. Recollection must report `changed`, `unchanged`, `inaccessible`, or `removed`;
it never overwrites an earlier C0 revision. For browser-backed batches, recollect per item so one
transient external-command failure cannot abort validation of the whole batch. Retry one failed
item once when the failure is transient; do not restart successful items.

### 6. Verify and report

Require `saved` items to contain both `item_id` and `revision_id`. Report failures and limitations
per candidate. Always report sovereignty depth C0-A1/C0-A2/C0-A3+ and readiness captured/prepared/registered,
then use the three-layer report in the collection contract and explicitly state that C1 was neither
required nor triggered. Only registered results may be called formal C0.

## Extending a Source

When a source gains images, audio, attachments, quotes, or another export format:

1. keep the same `source.*` route when it is still the same user-understood source;
2. update that source reference with acquisition and fidelity rules;
3. update capability evidence and add a real authorised test for the new shape;
4. add or widen a Rust adapter only after repeated use proves existing tools insufficient;
5. never create `babata-<platform>-collect` for the new case.

After a real run, update the same recipe with measured bottlenecks and the single next optimisation.
Speed work may reuse an acquisition session or capture lifecycle, but must keep each candidate's C0
transaction, completeness check, provenance, and failure result independent.

Create a new route only for a genuinely separate source identity or authorisation boundary, not for
a file extension, attachment type, or one acceptance sample.

## Anti-Patterns

- One Skill per platform, export format, notebook, or conversation.
- A second visible routing Skill that the user must call before collection.
- Treating PDF+MHT, MHT-only, `.notes`, or W1 DOCX as independent products.
- Running a disabled adapter because the source code exists.
- Treating browser visibility, a Recovery file, or a download as formal C0.
- Triggering C1 because the source contains multiple segments or rich media.
- Reporting database rows or tests as a substitute for user-visible collection results.

# 豆包 Recipe

Route: `source.doubao`. Current status is **enabled** for explicitly identified conversations and
batches of 1-20 conversation IDs. Runtime capability remains authoritative.

## Proven Scope

- A signed-in Chrome history can be expanded once so the Agent can discover real conversation IDs,
  titles, and URLs without asking the user to confirm every row.
- `conversation:<id>` collects one explicit conversation. `conversations:<id>,<id>,...` discovers
  1-20 explicit conversations in one Collector session. `all` is rejected.
- Two live batches containing 40 non-main conversations produced 38 formal C0 results. Two long
  conversations initially returned `HasMore=true` and were honestly rejected before any incomplete
  C0 write. A later authorised Chrome-native run completed them as 3 pages / 60 messages and
  2 pages / 25 messages, with unique message IDs and `has_more=false` on both final pages.
- All 38 saved items were recollected individually as `unchanged`, with no new revision and no C1.
- The complex conversation “战略领导力W1” also has seven verified original DOCX attachments and one
  PDF preview in the formal C0. A later temporary-root exercise proved a fresh Chrome-native handoff
  can collect the same 16-message conversation with exactly the seven originals and typed-recollect
  a refreshed equivalent handoff as `unchanged`. Other attachment and media shapes remain unproven.

## Batch Workflow

1. Open the signed-in history once and expand enough rows for the authorised count/time/visible-set
   scope. Record IDs, titles, and URLs; do not repeatedly rediscover the same sidebar range.
2. Compare `source_native_id` with existing C0 before selection. Exclude duplicates unless this is
   an explicit recollection. Exclude any user-identified oversized main conversation from a normal
   batch; collect it later only as its own explicit scope.
3. Split the remaining explicit IDs into batches of at most 20 and start each with:

   ```text
   babata --json collector start --route source.doubao --source conversations:<id>,<id>,... --scope <authorised-scope> --authorisation <reference>
   ```

4. Inspect candidates, select the batch once, and report each `saved` or `failed` result. A response
   with incomplete message pagination must fail closed; never trim it into a plausible transcript.
   For a long conversation, prefer direct control of the user's signed-in desktop Chrome. Observe
   `/im/chain/single`, then follow the official descending `anchor_index` requests until
   `has_more=false`; verify conversation identity and unique message IDs before C0. For original
   DOCX files, open Doubao Drive and locate the same filename; use the file-row `下载` action, not a
   document preview export. Build one local acquisition handoff containing the structured response
   and every declared original's path, size, MD5, and SHA-256, then start with:

   ```text
   babata --json collector start --route source.doubao --source conversation:<id> --scope <scope> --authorisation <reference> --acquisition-handoff <handoff.json>
   ```

   Select with `--attachments --confirm`. Missing, mismatched, or changed originals must fail before
   C0. If attachments were requested and the OpenCLI fallback reports nonzero attachment keys,
   reject the fallback before C0 and require a Chrome-native handoff. Do not route through the
   unauthenticated in-app browser, and do not wait on OpenCLI when desktop Chrome is already
   available.
5. Recollect every saved `item_id` with a newly captured equivalent handoff:

   ```text
   babata --json collector recollect --item <item_id> --acquisition-handoff <fresh-handoff.json>
   ```

   Expect `unchanged` and no new revision. Keep
   failure isolation at one item and retry a transient command failure once. Do not let one OpenCLI
   failure stop checks for the rest of the batch.

## Fidelity and Revision Rules

- Preserve structured message order, role, stable IDs, text, and reported attachment/media counts.
- Remove transient block IDs and expiring signed media URLs from the content fingerprint. Preserve
  relevant source payload and limitations without treating unstable delivery fields as content.
- Treat `HasMore=true` as incomplete pagination and write no C0 for that candidate.
- Do not call C1 or interpret the conversation. Collection ends after formal C0 and readback.
- Never use a second `capture file` import as recollection: it creates an import revision even when
  the JSON bytes are unchanged. Use the Collector's typed recollection path only after that path can
  consume the same Chrome-native complete read; otherwise report recollection as pending.
- Before changing fingerprint logic, test old and new payloads against existing C0. An algorithm
  upgrade must not silently claim source content changed. Use an explicit compatibility/migration
  decision and report any one-time normalisation revision separately from real source revisions.

## Speed Rule

Optimise only after stable, accurate, and real collection pass. Time is the fourth hard metric.
Reuse the already signed-in desktop Chrome tab and its native network lifecycle across a batch;
do not spend time planning parallel routes after the page contract is proven. Preserve
per-conversation completeness checks, distinct C0 identities, provenance, and failure isolation.

## Remaining Limits

Normal conversation text batches and the two authorised long conversations are in C0. Chrome-native
typed recollection with original attachments is proven only for the W1 seven-DOCX shape; ordinary
images, audio, quoted pages, and non-DOCX attachments need their own real shape evidence before the
recipe claims them. W1 already exists in formal Babata, so future exercises use a temporary data root
and stop once the loop passes.

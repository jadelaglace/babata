# 豆包 Recipe

Route: `source.doubao`. Current status is **enabled** for explicitly identified conversations and
batches of 1-20 conversation IDs. Runtime capability remains authoritative.

## Proven Scope

- A signed-in Chrome history can be expanded once so the Agent can discover real conversation IDs,
  titles, and URLs without asking the user to confirm every row.
- `conversation:<id>` collects one explicit conversation. `conversations:<id>,<id>,...` discovers
  1-20 explicit conversations in one Collector session. `all` is rejected.
- Two live batches containing 40 non-main conversations produced 38 formal C0 results. Two long
  conversations returned `HasMore=true` and were honestly rejected before any incomplete C0 write.
- All 38 saved items were recollected individually as `unchanged`, with no new revision and no C1.
- The complex conversation “战略领导力W1” also has seven verified original DOCX attachments and one
  PDF preview in C0. Other attachment and media shapes remain unproven.

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
5. Recollect every saved `item_id` with `babata --json collector recollect --item <item_id>`. Keep
   failure isolation at one item and retry a transient command failure once. Do not let one OpenCLI
   failure stop checks for the rest of the batch.

## Fidelity and Revision Rules

- Preserve structured message order, role, stable IDs, text, and reported attachment/media counts.
- Remove transient block IDs and expiring signed media URLs from the content fingerprint. Preserve
  relevant source payload and limitations without treating unstable delivery fields as content.
- Treat `HasMore=true` as incomplete pagination and write no C0 for that candidate.
- Do not call C1 or interpret the conversation. Collection ends after formal C0 and readback.
- Before changing fingerprint logic, test old and new payloads against existing C0. An algorithm
  upgrade must not silently claim source content changed. Use an explicit compatibility/migration
  decision and report any one-time normalisation revision separately from real source revisions.

## Speed Rule

Optimise only after stable, accurate, and real collection pass. The measured bottleneck is repeated
OpenCLI navigation and network-capture setup for each conversation. The one next optimisation is to
reuse a browser/capture lifecycle across a batch while preserving per-conversation completeness
checks, C0 transactions, provenance, and failure isolation. Do not add parallel speculative routes.

## Remaining Limits

Normal conversation text batches are enabled. The two conversations rejected for incomplete
pagination remain uncollected until a complete-page route succeeds. Ordinary images, audio, quoted
pages, and non-DOCX attachments need their own real shape evidence before the recipe claims them.

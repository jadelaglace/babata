# Doubao Recipe

Route: `source.doubao`. Query runtime capability before use; this file defines the proven acquisition shape,
not its current enabled status or completed user scope.

## Authorized scope

Accept either one explicit conversation or 1-20 explicit conversation IDs:

```text
conversation:<id>
conversations:<id>,<id>,...
```

Reject `all`. A signed-in Chrome history may be expanded once to discover IDs, titles and URLs inside a
user-authorized count/time/visible-set. Exclude existing C0 identities unless the request is recollection.
Handle an oversized conversation as its own explicit scope.

## Complete conversation acquisition

Use the signed-in desktop Chrome context. Read the official conversation responses from the initial anchor and
follow the reported next anchor until `has_more=false`. Before handoff, require:

- exact conversation identity;
- complete pagination with no repeated/missing message IDs;
- stable message order, role and text;
- declared attachment/media counts and honest missing-shape notes.

Any incomplete page, identity mismatch or duplicated message must fail before C0. Never trim an incomplete
response into a plausible transcript.

## Original document attachments

When nested message JSON declares `content_type=20` file objects, use the bundled deterministic script:

```powershell
./02_skills/babata-collect/scripts/acquire-doubao-originals.ps1 `
  -ConversationId <id> `
  -OutputDirectory <temporary-or-recovery-directory>
```

The script reads nested file objects, asks the signed-in page for original file URLs, downloads returned DOCX
files, validates size/MD5/SHA-256 and ZIP structure, and writes `acquisition-handoff.json`. It must not retain
signed original-download URLs in the stable handoff.

Do not click the attachment card's preview action or treat a converted PDF preview as the original. Do not use a
top-level zero attachment count without inspecting nested message JSON. If attachments were requested and a
declared original is absent, reject text-only fallback before C0. A separately authorized Drive filename search
may be a fallback, but it must not be mixed into the direct object-key path.

Other file/media shapes require their own real evidence and validator before the recipe claims them.

## Collector handoff

Start one session for the explicit IDs:

```text
babata --json collector start \
  --route source.doubao \
  --source conversations:<id>,<id>,... \
  --scope <authorized-scope> \
  --authorisation <reference> \
  [--acquisition-handoff <handoff.json>]
```

Inspect candidates, select only the authorized set once, and use `--attachments --confirm` when attachments are
required. Report every saved/failed/skipped candidate independently. One transient acquisition failure may be
retried once without restarting successful items.

## Fidelity and recollection

- Preserve stable IDs, message order, roles, text and declared attachment/media facts.
- Exclude expiring signed URLs and transient block IDs from the stable content fingerprint.
- Do not call C1 or interpret the conversation; collection ends at C0 read-back.
- Recollect through the typed Collector path with a newly acquired equivalent handoff:

```text
babata --json collector recollect --item <item_id> --acquisition-handoff <fresh-handoff.json>
```

`unchanged` creates no revision. A fingerprint algorithm change requires an explicit compatibility/migration
decision and must not masquerade as source-content change. A second generic file import is not recollection.

Historical counts, dates, accepted batches and current gaps belong to `DOC-ROUTES`, `DOC-USAGE` and runtime
receipts, not this recipe.

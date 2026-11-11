# Kimi conversations

Query runtime capability before use. Historical acquisition or an installed OpenCLI command does not
enable `source.kimi`. Check DOC-ROUTES for evidence and limitations; stop formal collection when the
runtime route is disabled. Do not bypass this through a generic route or call a historical usage run.

## Explicit scope

`recent:1..100` describes the visible history window and never proves account completeness.
For an explicitly authorized account scope, preserve read-only ChatService/ListChats pages outside Git,
including capture/tool identity, unique chat IDs, page order, counts, first/last IDs, token hashes and
termination. Keep credentials and raw pagination tokens in the browser session only.

Create `manifest:<absolute-path>` using `babata.kimi-chat-manifest/v1` with exactly these fields:

- `schema`, `readiness: prepared`;
- `account: {display_name}` (use a non-sensitive local account label);
- `enumeration: {method: ChatService/ListChats, captured_at, terminated, total_listed, pages}`;
- `pages: [{page, count, has_next}]`, contiguous from 1, only the last has no next page;
- `chats: [{chat_id, title, update_time}]`, unique lowercase UUIDs or legacy 20-character base32hex XIDs.

All fields are required and unknown fields are rejected. Counts must agree; the empty chat list is
rejected. Evidence that the listing terminated belongs to the captured listing, not the detail command.
The manifest is a declared candidate scope, not evidence that attachments are prepared or registered.
The adapter does not enumerate the account and selection remains explicit.

## Read and hand off

Use `opencli kimi detail-full <explicit-id-or-url> --window background --site-session persistent
--keep-tab false -f json`. Keep calls serial, bounded, and in the background; release the site session
between batches when necessary. Do not upgrade OpenCLI as a side effect. The page session reads GetChat
and follows ListMessages tokens. A bounded 401 retry does not prove reliable long-term authentication.

The result must contain exactly one matching ChatId/Chat.id, structured Messages, matching MessageCount,
positive MessagePages and Complete=true with no-token or empty-final-page termination. Failed HTTP,
malformed JSON/page/token, foreign chat, or unfinished pagination must not produce a ready C0.
404 is removed, authentication/permission failure is inaccessible. Previously saved content survives
either result; never delete local originals because the user deleted the source conversation.

Only the unified Collector owns save/read-back. Preserve native structured blocks and hashes.
Attachment metadata alone does not establish binary completeness; do not claim full C0-A2 for missing
required binaries. Citation ordering may vary and cause false revisions; record this limitation and
never use an old stored response to claim fresh unchanged recollection. Do not start C1/C2 or alter
old accepted notes as part of collection.

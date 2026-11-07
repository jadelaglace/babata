# Qianwen Native Conversation Recipe

Query runtime capability before use: `babata --json capabilities list` must enable `source.qianwen`.
Use the signed-in Qianwen website through OpenCLI browser eval only as acquisition; no prompt is sent.
Keep responses, scope manifests, downloaded media and browser state outside Git and managed C0.

## Discovery and scope

Use the website's official history API and retain request parameters, cursors, response IDs and page
boundaries. Repeated pages or IDs fail discovery. A successful empty oldest page without an explicit
terminal flag bounds the observed visible set; it cannot prove deleted, hidden or unexposed history.
The Rust adapter accepts only explicit `conversations:<id>,<id>,...` or one 32-hex native ID with a matching
`--acquisition-handoff <absolute-json-path>` for each candidate. It does not enumerate an account.

Read each session through the official `/api/v1/session/msg/list` endpoint, including response messages
and all events. Require HTTP success, application success and `have_next_page=false`; paginate first
if needed. Preserve every native `req_id|pos|event` row, including completion/continuation events with
the same request ID. Transport headers and credentials never enter a handoff.

When the user explicitly excludes food ordering, exclude pure order conversations and, if requested,
order/payment/delivery portions of mixed conversations at native row boundaries. Preserve independent
knowledge or other content. Record `original_keys`, `excluded_keys` and exact `excluded_fields` paths
for inherited excluded context; retain the original acquisition externally. Titles describing excluded
orders must not leak into the selected C0 title. Do not delete remote conversations.

## Native handoff and attachments

One JSON handoff contains `protocol_version: "1"`, `provider: "qianwen"`, `conversation_id`, `title`,
`source_url`, `have_next_page: false`, selected native `messages`, the scope lists above, and `assets`.
Each asset contains its native `url`, external local `path`, `sha256` and `bytes`.

Scan retained rows and nested JSON strings for native image/video/document URLs, including `url`,
`image_url`, `layout_file_url`, `preview_img_url`, `imagePath` and `previewImages` from workspace,
document and preview CDNs. Download all declared embedded media, validate byte size/SHA-256, and
require exact URL coverage. References to cited web pages remain A3 references. New unproven shapes
need investigation; do not claim arbitrary Qianwen attachments are supported.

The adapter validates identity, row partition, complete messages and media before capture. The payload
retains the native URL-to-hash mappings; repeated URLs for the same binary import one managed asset
per hash. Signed URLs are transient locators, never stable provenance. Candidate metadata holds only
small asset bookkeeping; long native messages stay in the text payload.

## Formal collection and recollection

Start the unified Collector with explicit scope and handoffs; select candidates with attachments.
Only Rust Collector/Capture assigns item/revision/asset identities. Require every selected candidate
to reach a terminal result; read back text, native selected rows, excluded positions and managed hashes.
Retry only failed candidates; keep successful items.

Run `collector recollect --item <item_id> --acquisition-handoff <fresh-json-path>` per item after a new
official API read. Without a supplied fresh acquisition, return inaccessible; persisted prefetched
snapshots cannot prove live state. Media URL signatures are replaced by binary hashes for the content
fingerprint. Native regenerated HTML/JS and rendering metadata remain preserved and may produce
`changed` even when conversational text is unchanged; report this rather than claim all unchanged.

Finish at registered C0-C / A2 with per-item evidence and visible-history limitations. C1 is not started.

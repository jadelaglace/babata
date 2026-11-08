# Yuanbao Native Conversation Recipe

Require `source.yuanbao: enabled` from `babata --json capabilities list`. Browser access is
read-only acquisition; never send prompts or delete conversations. Keep source responses,
handoffs, downloaded media and browser state outside Git under the configured data root.

## Explicit scope and native acquisition

The adapter accepts `conversation:<id>`, `conversations:<id>,<id>,...`, a single native ID,
or an exact `https://yuanbao.tencent.com/chat/<agent-id>/<conversation-id>` URL. Supply one
`--acquisition-handoff <path>` per selected conversation. It does not enumerate the account.

Use the signed-in official recent conversation/session APIs only to locate the authorised
IDs. Read current conversations with `POST /api/user/agent/conversation/v1/detail`
(`agentId`, `conversationId`, `offset`, `limit`); legacy UUIDs require the separately proven
`GET /api/conv/<id>` reader. Check HTTP/application success and native identity. Finish
pagination before creating a handoff; never treat `21000` from the wrong reader as deletion.

## Handoff

One JSON object contains `protocol_version: "1"`, `provider: "yuanbao"`, `conversation_id`,
`agent_id`, `title`, `has_more: false`, `messages` (the original `convs` rows), `media_info`
(the original `multiMediaInfo`, or null), and `assets`. Keep native row IDs, `conversationId`,
`speaker`, timestamps and full `speechesV2` unchanged, in native response order. Omit transport
headers and credentials. Empty, duplicate, foreign or unsupported message chains fail closed.

Each asset has its native `url`, external local `path`, `sha256` and `bytes`. Download every
typed image/file/audio/video dependency from `speechesV2` and `media_info`, including nested
JSON media fields. Citation URLs remain references. Missing/extra URLs, missing files and
size/hash mismatches fail before C0. Multiple URLs for identical bytes retain all mappings
but import one managed asset per hash. New media shapes need their own real-path proof.

The native upload size may differ from the downloaded platform copy. Preserve both native
metadata and saved size/hash; this adapter does not claim original upload-byte fidelity.

## Collector and recollection

Start `collector start --route source.yuanbao --source conversations:<ids> --scope <scope>
--authorisation <evidence> --acquisition-handoff <path> ...`, then explicitly select candidates
with `--attachments --confirm`. Only the Rust writer assigns formal item/revision/asset IDs.
Verify native messages, source URL, timestamps, managed bytes and SHA-256 after saving.

After another official API read, use `collector recollect --item <item-id>
--acquisition-handoff <fresh-path>`. No supplied acquisition returns `inaccessible`; stored
prefetched data is never a live-state check. Same native payload and asset hashes are
`unchanged`; content, native metadata or URL changes can create a new revision. Local asset
paths are excluded from payload/fingerprint. Stop at C0; do not start C1/C2.

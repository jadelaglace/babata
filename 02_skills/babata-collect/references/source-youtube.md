# YouTube Prepared Cache Recipe

Use this route only for playlists or videos explicitly named by the user and already represented by a
validated local cache manifest. It is not account-wide discovery, channel crawling, or a generic download
capability.

## Admission contract

The manifest must provide one row per selected video with:

- stable YouTube `video_id`, original title, canonical watch URL, playlist ID/title and observed position;
- one readable local media path, byte size and authoritative SHA-256;
- an exact or high-confidence one-to-one mapping with no unresolved source or local files;
- an explicit `prepared` result and honest limitations.

Reject duplicate video IDs, duplicate local paths, missing files, hash/size mismatches, non-HTTPS YouTube
watch URLs, unresolved mappings, or records outside the declared playlist scope. Never retain temporary
signed media, subtitle or thumbnail URLs in the manifest.

## Collection behavior

1. Run `babata --json capabilities list` and require `source.youtube=enabled`.
2. Start one Collector session with the validated manifest path as `--source` and the user's bounded URL
   scope as the authorization context.
3. Discover one candidate per manifest item. Keep `video_id` as the native identity; retain the exact title,
   playlist identity and observed position as source metadata.
4. Select only the explicitly authorized candidates. The Rust asset store rereads and SHA-256 verifies the
   local MP4 before assigning C0 IDs and managed paths.
5. Read every saved item/revision/asset back. Recollection compares the same stable `video_id` and source
   bytes; changed bytes append a revision, unchanged bytes do not.

The adapter consumes a prepared cache; yt-dlp remains an acquisition dependency and never writes SQLite or
managed assets. Playlist position is mutable metadata and must not become item identity. Embedded subtitles
may be inventoried as a limitation but are not promoted to C1 or treated as authoritative transcript text.

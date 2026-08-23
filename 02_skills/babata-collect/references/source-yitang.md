# Yitang Replay And Document Recipe

Use this route only for one replay/document pair explicitly supplied by the user. A signed-in browser is
an acquisition dependency, not account-wide authorization. Links inside the document are references and
must not be followed unless the user separately authorizes them.

## Acquisition contract

1. Open the exact replay and document URLs in the available signed-in browser. Retain only whitelisted replay
   fields: sid, title/subtitle, begin time, status, HLS/audio locators and source duration. Exclude user identity,
   chat, reactions, promotion, credential, cookie and session fields.
2. Traverse the document's full virtual scroll to its bottom. Export readable ordered blocks, headings and image
   locators; an encrypted API body or page shell is not readable document evidence.
3. Download the replay with MPMux or a direct HLS/ffmpeg stream copy, and download every declared body image to
   Git-external staging. The downloader is interchangeable: either path must yield a local MP4 whose duration,
   byte size and SHA-256 are verified. Preserve a readable Markdown/JSON representation of the rendered document
   without browser UI or session identity.
4. Generate `babata.yitang-source-manifest/v1` with one composite item. Bind the stable identity to replay sid plus
   document token/ACL pair, list every required asset with role, relative path, byte size and SHA-256, and set
   readiness to `prepared` only after complete read-back.

Reject a non-HTTPS or unexpected Yitang host, replay/document URL identity mismatch, duplicate/escaping asset path,
missing or empty media/body, incomplete traversal, missing declared image, size/hash mismatch, encrypted-body-only
capture, or any manifest containing credential/session/user/discussion fields.

## Collection behavior

1. Run `babata --json capabilities list` and require `source.yitang=enabled`.
2. Start Collector with the prepared manifest path as `--source` and the exact pair as scope/authorization context.
3. Discover one `video` candidate whose native identity is `replay:<sid>|document:<token>` and whose payload is the
   readable structured document plus safe source metadata.
4. Select only that candidate with attachments. Rust rereads every declared asset and verifies size/SHA-256 before
   assigning C0 identity, revision and managed asset paths.
5. Read back the item, revision and full asset denominator. Re-selecting the same manifest in a new session or
   running explicit recollection reports `unchanged` without a new revision only after the complete managed asset
   set passes SHA-256 read-back; missing, changed, tampered or re-paired dependencies fail closed.

The preparation tool and browser never write SQLite or managed C0 assets. Collection stops at registered C0-C and
does not trigger transcription, OCR, summarization, C1 processing, knowledge registration or publication.

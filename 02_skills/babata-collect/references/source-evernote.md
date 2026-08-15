# Evernote / 印象笔记 Recipe

Route: `source.evernote`. This recipe covers one explicit official `.notes` export. Query runtime
capability before use.

## Authorised Scope

Require one user-selected official export. Inspect likely export locations before asking for a path.
The adapter requires an absolute path and does not require the user's application password, Cookie,
or third-party credential.

Collector source reference:

```text
notes:<absolute-path-to-export.notes>
```

For a whole-export request, discover first and show the candidate count. Selecting all candidates is
allowed only because the one `.notes` file is the explicit scope. The verified real shape contains
one batch/archive candidate plus individual note candidates; other exports may differ.

## Fidelity and Verification

The Rust adapter authenticates and decrypts supported ENC0 records, validates HMAC/AES structure,
produces traceable ENEX/ENML representations, maps resources, and submits everything through the
same Collector/Capture path. Do not pre-decrypt into a second persistence path.

Reject wrong extensions, relative paths, malformed/truncated exports, invalid resources, entity
abuse, and HMAC failures. Preserve the original `.notes` export and returned resource coverage.

## Limitations

The current format does not expose a stable note GUID, update timestamp, or notebook hierarchy.
Identity is limited to immutable export hash plus note ordinal. Do not infer cross-export identity or
claim incremental matching. A repeated collection of the same bytes should return `unchanged`, not
create new revisions.

# OneNote Recipe

Route: `source.onenote`. This recipe covers one same-export PDF/MHT pair or an explicit list of
standalone MHT exports. Query runtime capability before use.

## Authorised Scope

Require the user to identify one notebook, sub-notebook, or explicit set of exported files. The
OneNote desktop client may be used to create official exports. Inspect likely export locations
before asking the user to type paths.

## Pair Shape

PDF and MHT from the same export are complementary representations of one C0 item:

- PDF preserves OneNote's rendered page division;
- MHT preserves text, images, and formatting more faithfully;
- both must be absolute, same-directory, same-stem paths;
- preserve both original byte streams; do not derive one from the other.

Collector source reference:

```text
pair:<absolute-path-to.mht>|<absolute-path-to.pdf>
```

## MHT-Only Shape

Each explicitly exported MHT is a complete C0 item, even when it represents a sub-notebook or
overlaps another export. Do not split it into pages/segments, merge it with a parent export, or infer
OneNote hierarchy during collection.

Collector source reference:

```text
mht-list:<absolute-path-a.mht>|<absolute-path-b.mht>|...
```

The adapter accepts both multipart MHT and OneNote single-part HTML exports after structural
validation. Deterministic text overlap may remain a machine, unreviewed, non-fact hint in the
manifest; it is not a C0 relation or a reason to skip either export.

## Limitations

- Official exports do not provide reliable native page/section IDs for current use.
- Cross-export notebook/sub-notebook identity is not inferred.
- PDF page numbers are not OneNote page IDs.
- Segment extraction, semantic deduplication, and hierarchy interpretation are optional later C1,
  not a OneNote collection step or completion condition.

Run the standard Collector workflow and require every selected candidate to return `saved`,
`item_id`, and `revision_id`. Recollection of unchanged exports must create no fake revision.

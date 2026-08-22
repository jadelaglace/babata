# semantic-course-obsidian builder profile

<!-- DOC-AUTHORITY-BOUNDARY: reusable-output-profile -->

<!-- Profile status: candidate-real-use; first authorized scope is Cherno. -->
<!-- C2B-DOCS-FIRST-GATE / C1B-FORMAL-HANDOFF-GATE / C2B-KNOWLEDGE-UNIVERSE-GATE / C2B-PACKAGE-OWNED-COURSE-MAP / C2B-MECE-COURSE-MAP-GATE / C2B-CRASH-COURSE-MAP-GATE / C2B-MODERN-VISUAL-MAP-GATE / C2B-RIGHT-GROWING-MINDMAP-GATE / C2B-RESPONSIVE-MAP-GATE -->

## Contract

- Profile: `semantic-course-obsidian/v1`.
- Input: a provider-neutral C2B package, `babata.course-presentation-plan/v1`, a complete formal C1B
  registration ledger, and a formal knowledge-universe course ledger. Never read external originals.
- Output: one rebuildable Obsidian live export per course, containing knowledge-first Markdown and only
  the C1B/C2B media required by those notes.
- Authority: C0/C1/core knowledge records and the package manifest remain authoritative. The Vault is a
  deletable compatibility export and never writes back.
- Activation: this profile can be proven by a named real-use scope without changing the system capability
  registry. A Cherno receipt does not claim generic `outputs.obsidian` is enabled.

## Presentation plan

`babata.course-presentation-plan/v1` declares:

- stable `course_key`, `course_identity`, `course_version`, `collection`, and user-facing `short_name`;
- `outline.mode=flat|sectioned`, with every source lesson bound to exactly one ordered unit;
- a separate ordered `learning_support` set for course overview/toolbox, exercises, review/self-test and
  visual-evidence index;
- one course-map classification axis whose display domains are mutually exclusive and collectively cover
  every unit;
- the exact named Vault and live file `Babata/<collection>/<short_name>/index.md`.

The plan, not source directories, playlist positions or filename prefixes, owns navigation and order.
`collection` and `short_name` are validated display path segments, not ontology identities.

## Materialization rules

1. Each unit is content-first: concise learning notes precede an expandable, complete hash-checked C1
   transcript. Do not replace the complete C1 with a summary.
2. Embed a retained C1B key frame, audio excerpt, video excerpt or attachment only when it materially changes
   understanding. Copy the exact managed derivative bytes and verify their hashes.
3. Provider, prompt, pricing, storage, limitations and rebuild details belong in manifests/receipts, never in
   the knowledge body.
4. Require one unique active complete-C1 transcript and structured result per source lesson, one formal essence
   decision, and all required retained media before materialization.
5. Require formal course identity/version, typed `covers` relations and complete reviewed assignments before
   publication. Course-local display domains never become ontology branches.
6. Generate editable Mermaid, rendered SVG and PNG inside the package. The publisher only copies the verified
   package and never renders or supplements it.
7. Use Obsidian-native `internal-link` classes with exact unique note filenames. Do not emit external
   `obsidian://` click targets inside Mermaid.
8. The course map is an emergency review sheet, not a graphical table of contents: each domain includes short
   grounded rules, relationships, implementation decisions and failure boundaries.
9. Use a single left-side course root and right-growing mind map, stable restrained domain colors, transparent
   text nodes, small junction dots and at least four semantic levels. No cards, filled boxes or bilateral tree.
10. At a `760px` fallback embed, essential text is at least `11px` and PNG height/width is at most `1.40`.
11. Native Mermaid is the only default-expanded map and uses `useMaxWidth: true`. The SVG must have
    `width="100%"`, a non-empty `viewBox`, controlled `max-width`, and the expected internal-link labels.
    Put PNG in a default-collapsed `[!info]-` callout as offline/print fallback.
12. Fresh materialization allows at most one visual-evidence section per unit and one occurrence of a media
    path in that unit.
13. Knowledge notes contain no control-plane language, model editorial residue or claims that the export is an
    authority. Every internal note/media link must resolve.

## Publish and acceptance

- Publish only a package whose manifest hashes, formal ledgers, source denominator, links, map outputs and
  clean rebuild all pass.
- New course output status is `pending_user_acceptance`. Profile reuse never transfers another course's
  acceptance.
- Keep one user-facing live export per course. Archive predecessor live bytes outside the Vault before an
  atomic same-volume replacement.
- `OBSIDIAN-HUMAN-VIEW-BOUNDARY`: when review is requested, launch only the exact live URI from the current
  manifest/usage status and let the user perform visual acceptance.

## First real-use scope

Cherno is the first authorized validation scope. Its course counts, identities, model usage, live paths and
acceptance state belong only to runtime receipts and `DOC-USAGE`; they must not be copied into this profile.

# semantic-obsidian builder profile

<!-- DOC-AUTHORITY-BOUNDARY: reusable-output-profile -->

<!-- Profile status: accepted; v2 current for new MBA presentation, v1 retained as historical evidence. -->
<!-- C2B-DOCS-FIRST-GATE / C1B-FORMAL-HANDOFF-GATE / C2B-KNOWLEDGE-UNIVERSE-GATE / C2B-PACKAGE-OWNED-COURSE-MAP / C2B-MECE-COURSE-MAP-GATE / C2B-CRASH-COURSE-MAP-GATE / C2B-MODERN-VISUAL-MAP-GATE / C2B-RIGHT-GROWING-MINDMAP-GATE / C2B-RESPONSIVE-MAP-GATE -->

## Contract

- Profile: `semantic-obsidian/v2`
- Input: provider-neutral C2B package, its manifest, a formal C1B registration ledger, and a formal
  knowledge-universe registration ledger; never read external originals directly.
- Output: a rebuildable Obsidian Vault containing Markdown notes and only the C1B/C2B media assets
  required by those notes.
- Authority: the package/manifest and C0/C1 records remain authoritative; the Vault is a deletable
  human-readable export and must never write back.

## Note shape

Each knowledge note uses `semantic-obsidian-note.md`. Frontmatter carries stable identity, variant,
template version, package and manifest references. The Markdown body contains only the knowledge:
conclusions, concepts, formulas, diagrams, cases, questions and relations.

## Materialization rules

0. Require `babata.mba-course-presentation-plan/v2`. Render `outline.mode=flat` as ordered units and
   `outline.mode=sectioned` as ordered sections containing ordered units. Each source module appears in exactly
   one unit. The plan, not the directory or filename prefix, owns navigation and order.

0.1. The user-facing live directory and index path use the short `short_name` exactly:
     `Babata/MBA/<short_name>/index.md`. Semester, provider/program prefixes and course numbers belong to
     internal `course` metadata, while `course_key`, schema, and version/profile fields stay in manifests/plans;
     none may appear in the user-facing directory name.

1. Organize notes by semantic nodes, course themes and relations; do not recreate an external website
   directory as the knowledge hierarchy.
2. Embed a retained C1B image, audio excerpt, video excerpt or attachment only when it changes the
   reader's understanding. A text-sufficient node may contain no new media.
3. A formula or chart may include both renderable notation and the source excerpt image; uncertain
   redraws must not replace the image evidence.
4. Every embedded asset must be retained in C1B/C2B or be covered by a recorded rebuild recipe.
5. Put provider, prompt, pilot status, storage boundary, cost, limitations and rebuild details in the
   sidecar manifest/README/REPORT, never in the knowledge body.
6. Require a Rust-core registration ledger for the complete C1 references, essence decisions and retained
   media before C2B materialization. Require the formal knowledge-universe ledger defined by the currently
   applicable versioned core/output contract before publishing. This display profile does not define course,
   branch, `covers`, lens or assignment semantics; the Obsidian export may display formal identities but never
   creates or changes them.
7. Generate editable Mermaid source and its PNG rendering inside the C2B package. The Obsidian export
   consumes both files unchanged; it must not render or supplement the package independently.
8. A course index is course-local navigation. It must not be presented as, or silently mutate, the
   universe-level large index.
9. Use one explicit classification axis for the course-local map. Its display domains must be mutually
   exclusive and collectively cover all chapter notes; learning aids remain a separate layer and the
   main tree contains no cross-domain dependency edges. These display domains are not ontology `Branch`
   identities and do not constrain universe-level multi-parent or multi-assignment relations.
10. Use Obsidian-native Mermaid `internal-link` classes with exact unique note labels. Do not emit
    external `obsidian://` click URIs. Embed the PNG with an explicit width no greater than `760px`.
11. Fresh materialization must not append duplicate visual-evidence blocks: at most one section per
    chapter and one occurrence of each media path in that chapter.
12. Render the same Mermaid source to SVG before publishing. Obsidian's actual Mermaid postprocessor
    selector must match every expected `internal-link` leaf exactly once, and each resulting link text
    must name one unique Markdown target in the package.
13. The course map must be usable as an emergency review sheet, not only as navigation. Expand each decision
    domain into concise, non-link knowledge details grounded in the validated, materialized package C2B text and collectively covering
    course objective, decision rules, formulas/relations and risk boundaries. Keep linked chapters as exact filenames.
14. Apply the profile's modern visual tokens: clear typography, soft curves, controlled line weights and
    readable whitespace. Inspect the package PNG on a
    white background; reject clipping, collisions, unreadably small type or excessive accent colors.
15. Use one left-side course root and grow all manifest-declared knowledge domains and learning aids to the
    right. Keep a stable, distinguishable color mapping and use the same color for curves in one domain.
    Use transparent, borderless text nodes and small branch dots; prohibit cards, filled boxes and bilateral
    layout. Expand root/domain/chapter/knowledge detail to at least four levels and use no empty intermediary.
    At a
    `760px` embed, essential type is at least `11px` and PNG
    height/width is at most `1.40`; enforce both from rendered dimensions before publication.
16. `OBSIDIAN-HUMAN-VIEW-BOUNDARY`: to hand the live export to the user, launch only
    the exact live URI registered by the course manifest/usage status.
    The Agent stops after launch and does not inspect the page through Computer Use, screenshots,
    accessibility, clicking or scrolling unless the user separately requests Agent-side inspection.
17. `C2B-RESPONSIVE-MAP-GATE`: make native Mermaid the only default-expanded course map and set
    `useMaxWidth: true`. Verify the rendered SVG root has `width="100%"`, a non-empty `viewBox`, a
    controlled `max-width`, and the exact expected `internal-link` labels. Keep the package-owned PNG
    at a `760px` embed inside a default-collapsed `[!info]-` callout for print, offline and renderer
    fallback. Fit-to-pane does not imply built-in Mermaid zoom or pan.
18. Keep learning support outside the outline number space. Use the profile filenames
    `学习支持-<课程工具箱>.md`, `学习支持-<课程案例练习>.md`, `学习支持-复习与自测.md`, and
    `视觉证据索引.md`, with the course-specific labels supplied by the plan. Reject new `09-/10-/11-` names.
19. A v1-to-v2 presentation migration may rename only the declared learning-support files and update their
    references, index, Mermaid source/rendering and manifest. It must preserve every other Markdown/media byte,
    record old/new plan and manifest hashes plus an exact rename map, and leave acceptance/closure unchanged.

## Publish gate

- Reject body text containing control-plane meta-language such as `本试点`, `C2 应当`,
  `外部主权库负责`, `不是正式 C2` or `我们要求`.
- Verify every internal link and media path, and check that no media reference is orphaned.
- Verify every Mermaid click target, Markdown fence, PNG dimensions, package manifest hash, and
  package/Vault hash. Mermaid links must open the current live course path.
- Delete and rebuild the Vault from the same package and template version; C0/C1 and the external
  sovereign library must remain byte- and identity-stable.

## Reference evidence

Finance is the accepted reference implementation that established v1. The v2 profile supersedes only its
navigation/learning-support naming contract. Its current formal package,
C1B/media/semantic counts, live URI, accepted/rejected history and closure receipt belong exclusively to
`00_docs/04_process/04_b_USAGE_STATUS.md` and Git-external runtime evidence. Reusing this profile does not
transfer finance completion status to another course; each course independently passes its declared input,
content, media, link, rebuild, publish and user-acceptance gates.

The profile's course-local structure and visual behavior remain accepted. Adoption of a successor
knowledge-universe ontology does not retroactively claim that historical packages satisfied that ontology;
new formal registrations must use the applicable successor core/output contract before reusing this profile.

# Outputs Skill Specification

Governance markers: `C2B-DOCS-FIRST-GATE`, `C2B-KNOWLEDGE-UNIVERSE-GATE`,
`C2B-PACKAGE-OWNED-COURSE-MAP`, `C2B-MECE-COURSE-MAP-GATE`,
`C2B-CRASH-COURSE-MAP-GATE`, `C2B-MODERN-VISUAL-MAP-GATE`,
`C2B-RIGHT-GROWING-MINDMAP-GATE`.

<!-- DOC-AUTHORITY-BOUNDARY: output-spec -->

Before changing a C2B capability, follow the single authority order in `00_docs/README.md`: review direct
wording/current requirements, PRD, AC, architecture, this output contract/active profile, process and TC.
Update only roles whose semantics or contracts changed; complete every required update before implementation,
formal registration or publication.

- Target CLI: `babata outputs ...`
- Input: explicit item/sublibrary scope and output kind
- Output: build status, output reference, and provenance manifest
- Permissions: read C0/C1 through Rust services; no reverse write from generated output
- Errors: unsupported kind, invalid scope, build failure, `capability_unavailable`
- Underlying CLI: output kinds activate only after their own AC/TC. Markdown/JSON and
  `semantic-obsidian` use different profiles and must report unavailable/candidate/accepted honestly.
  Current activation and course coverage belong to usage status, not this reusable contract.
- Note template: `02_skills/00_specs/templates/semantic-obsidian-note.md`
- Accepted semantic Obsidian profile: `02_skills/00_specs/templates/semantic-obsidian-profile.md`
- Rebuildable full-course runner: `05_scripts/build-template-preserving-c2b.ps1`; every run starts
  from a source C1 map and a fresh staging root, then materializes package and Vault from the approved
  semantic template. Every mapped module leaf is content-first and carries the complete hash-checked C1
  body; provenance is collapsed after the body. Existing C2B output is never an input to the runner.
- Content-first finance reference runner: `05_scripts/build-finance-usable-c2b.ps1`; when user validation shows
  that a semantic-template export has become form-heavy and thin, it rebuilds from the C1 map and saved
  model responses into complete chapter-level learning documents plus an expandable full-source layer.
  It rejects thin learning documents, invalid Markdown emphasis/math delimiters, editorial/model residue,
  control-plane text and dangling links before publication. It is an implementation reference, not the
  reusable profile or a current completion claim.
- Finance reference materializer: `05_scripts/materialize-finance-c2b-from-c1b.ps1` consumes accepted
  course documents, C1B decisions, a formal C1B registration ledger, source map, and the matching formal
  knowledge-universe ledger. `05_scripts/render-finance-course-map.ps1` writes Mermaid
  source and PNG into the package; `05_scripts/publish-finance-c2b-live.ps1` publishes only an
  `accepted_benchmark / registered` package with complete manifest hashes and an accepted profile.
- Successor MBA universe-registration contract (adopted, not yet implemented or enabled for new formal
  registration): a Git-external course plan supplies the distinct course identity/version,
  complete denominator, mutually exclusive course-local chapter mapping, typed `covers` relations to stable
  `Branch` identities, complete map-node non-parent typed relations and the
  intended multi-map assignment set, MBA lens membership, course-map
  domains, and one live export path. It requires a successor plan schema, core course/typed-relation contract,
  registrar, checker and compatible closure verifier to pass their own AC/TC before activation. Concrete course
  names, counts, module IDs, model usage, paths, and acceptance remain in the run plan/ledgers and usage status,
  not this reusable contract.
- Historical MBA rollout path: `babata.mba-course-c2b-plan/v1` and the current generic scripts remain evidence
  for accepted C1B/content/profile/package/live instances, but their singular foundation/discipline/branch
  registration is not conformant for new formal registrations. Compatibility work must preserve those accepted
  outcomes while migrating ontology relations through the core writer; it must not rewrite old package state.
- A newly materialized course uses `pending_user_acceptance` even when C1B, semantic registration, package,
  link, map, rebuild, and publisher engineering gates pass. Only direct user acceptance may promote that
  course instance to `accepted`; the accepted finance benchmark/profile does not transfer its status.
- `C1B-FORMAL-HANDOFF-GATE`: `05_scripts/register-finance-c1b-handoff.ps1` reuses complete active C1,
  registers each essence decision and retained media derivative through Rust `process register`, reads
  back managed paths/hashes, and emits a `registered` ledger. Formal C2B rejects a missing, partial,
  duplicated or hash-inconsistent ledger.
- Formal universe writer: `05_scripts/register-finance-c2b-knowledge.ps1` creates one candidate package
  per real C0 revision, cites its active C1 derivative/hash, calls Rust `process register` and
  `knowledge ingest`, accepts the suggestion, and assigns it to the approved branch. It is resumable
  and reuses matching entries/derivatives rather than duplicating them.
- Formal closure verifier: `05_scripts/verify-finance-c2b-formal-closure.ps1` independently rechecks the
  C1B managed ledger, active C2B/knowledge-universe uniqueness, package/live hashes, formal frontmatter,
  Wiki/media links, SQLite `quick_check` and foreign keys, then writes a runtime-only closure receipt.
- Generic MBA closure verifier: `05_scripts/verify-mba-course-c2b-closure.ps1` requires explicit user-acceptance
  evidence, reruns the generic package checker, requires exact package/live bytes, verifies raw/derived SQLite
  integrity, and writes the accepted/closed result outside Git. It does not mutate the package, live export,
  C1B ledger, knowledge ledger, or database.
- `C2B-KNOWLEDGE-UNIVERSE-GATE`: a course package requires a formal, readable registration ledger containing
  a distinct course identity/version, typed `covers` relations and the complete reviewed multi-map assignment
  set. The registrar must not require course identity to equal branch identity, model MBA as one discipline,
  or remove unrelated evidence-backed assignments.
- `C2B-PACKAGE-OWNED-COURSE-MAP`: editable Mermaid and PNG are package outputs. A renderer accepts only
  a package root; an Obsidian publisher copies the verified package and never renders or supplements it.
- `C2B-MECE-COURSE-MAP-GATE`: declare one classification axis, partition course chapters into mutually
  exclusive course-local display domains that collectively cover the course, and keep learning aids in a
  separate layer. Display domains are not universe ontology branches and do not constrain multi-assignment.
  Obsidian Mermaid leaves use native `internal-link` classes and exact unique note labels, never external
  `obsidian://` click URIs. Embed the package-owned PNG at an explicit width of at most `760px`. A fresh
  materialization keeps at most one visual-evidence section per chapter and one occurrence per media path.
  Before publish, render the same Mermaid source to SVG and require Obsidian's current Mermaid
  postprocessor selector to match exactly the expected leaf labels; source-text checks alone are insufficient.
- `C2B-CRASH-COURSE-MAP-GATE`: the map is a compressed learning artifact, not a graphical table of
  contents. Each decision domain expands into short, non-link knowledge details grounded in the validated,
  materialized package
  C2B body; together they cover the course objective, decisive rules, formulas/quantitative relations, and
  major risks or boundaries. Linked chapter nodes keep exact Markdown filenames.
- `C2B-MODERN-VISUAL-MAP-GATE`: use a restrained token set with modern typography, rounded geometry,
  soft curves, controlled line weights and readable spacing. Inspect the rendered PNG for clipping, collision and contrast;
  Mermaid and PNG remain outputs of the same package-only renderer.
- `C2B-RIGHT-GROWING-MINDMAP-GATE`: use one left-side course root and grow every knowledge domain and
  learning-aid branch to the right. Keep a stable five-color mapping for the knowledge domains and inherit
  the domain color across its curves. Use transparent borderless text nodes, small junction dots, and at
  least four levels (root/domain/chapter/detail); do not use cards, filled boxes or bilateral layout.
  At the `760px` Index width, essential text must calculate to at least `11px`, and PNG height/width must be
  at most `1.40`; the renderer rejects violations before publish.
- `C2B-RESPONSIVE-MAP-GATE`: the course Index has exactly one default-expanded map, the native Mermaid
  view. Set Mermaid `useMaxWidth` to `true` and reject `false`; render the same source to SVG and require
  root `width="100%"`, a non-empty `viewBox`, a controlled `max-width`, and the unchanged exact set of
  Obsidian `internal-link` labels. Keep the package-owned PNG and its `760px` embed in the manifest, but
  place it inside a default-collapsed `[!info]-` callout as a print, offline, and renderer fallback. This
  responsive contract means fit-to-pane; it does not claim built-in Mermaid zoom or pan.
- Output path contract: the runtime task directory is evidence-only; users open the one registered live
  export for that course. The current course path belongs to usage status.
- `OBSIDIAN-HUMAN-VIEW-BOUNDARY`: when the user asks to open Obsidian for their review, launch only
  the exact URI registered for the current live output in usage status/manifest and stop. Do not use
  Computer Use, screenshots, accessibility inspection, clicks or scrolling to substitute for the user's view
  unless the user separately asks the Agent to inspect the page.
- Reference implementation: finance proves the profile can be implemented and closed; current batch,
  counts, accepted/rejected instances and live URI are maintained only in usage status and runtime receipts.
- Never present the outer staging directory, `package/`, or `vault/` as the user-facing Obsidian result.
  The current/latest selector and the historical comparison batch must be explicit in the manifest and
  report so duplicate exports do not become competing outputs.
- Keep one user-facing live export per course. Historical C2B exports belong in the external staging
  archive and must not remain as sibling folders in `Documents/Obsidian Vault/Babata/MBA`.
- C2B canonical storage is an internal, deletable/rebuildable knowledge database or package. The Obsidian directory is a compatibility export only; it must not contain full C0/C1 source layers or become a second authoritative writer. External sovereign storage remains authoritative for originals, C0, C1, and raw context.
- The course index and course map are local navigation. They do not replace or mutate the universe-level
  large index.
- After a live directory replacement, an already open Obsidian workspace can retain archived tabs.
  Acceptance must reopen the registered current live index and verify the active file.
- Formal Skill activation: P7; this specification does not itself expose an active Skill

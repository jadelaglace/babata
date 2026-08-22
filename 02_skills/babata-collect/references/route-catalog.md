# Route Catalog

## Runtime rule

Run `babata --json capabilities list` before collection. Runtime is the execution truth. Stop when the
exact `source.*` route is `disabled`, `unavailable`, absent, or unknown, even if historical evidence or a
browser can read the source.

In this repository, `DOC-ROUTES` at
`00_docs/03_architecture/03_d_SOURCE_ROUTE_REGISTRY.md` owns route evidence, authorization and gaps.
`DOC-USAGE` owns concrete source completion. Do not copy their current status or counts into this index.

## Recipe routing

| Source identity | Load |
| --- | --- |
| `source.onenote` | `source-onenote.md` |
| `source.evernote` | `source-evernote.md` |
| `source.doubao` | `source-doubao.md` |
| `source.youtube` | `source-youtube.md` |
| Website, browser, desktop app, unknown or any other named platform | `source-browser-and-ui.md` |

Only load one source reference after reading the shared collection contract. A source-specific recipe is
an internal acquisition procedure, not a separate user Skill or data layer.

## Tool selection order

Use the first complete, lawful option:

1. official free migration, export or API;
2. maintained existing plugin, CLI, SDK or script;
3. Agent-led low-touch export through an authorized browser/app;
4. narrow development for a repeatable proven gap;
5. paid capability after explicit user decision;
6. heavy development or complex tool flow;
7. continuous human/Agent collaboration;
8. manual-only fallback.

Evaluate the chosen route in this order: stable, accurate, real, fast. Once the first three pass, measure
the run and optimize only the proven bottleneck. Browser and desktop control are tools inside a route;
they never turn `source.browser_pages` into generic support for a named platform.

## Adding or widening a route

1. Update `DOC-ROUTES` with evidence, authorization and gap.
2. Add or update one source recipe for acquisition/fidelity behavior.
3. Add real authorized capability tests and activate the runtime descriptor only when they pass.
4. Keep historical run counts in `DOC-USAGE`/receipt, not in this catalog or recipe.
5. Create a new route only for a distinct source identity or authorization boundary, never for a file type.

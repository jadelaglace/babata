# Browser and Desktop UI Recipe

Use this reference for a named platform without a dedicated recipe, browser pages/bookmarks, desktop apps,
local files, or an unknown source. It is a guarded investigation/acquisition fallback, not a generic route.

## Resolve the source first

Map the request to the exact `source.*` identity, then query runtime capability. If it is absent, disabled or
unavailable, report the route gap and stop before formal collection. Do not fall back from a named platform
to `source.browser_pages`; visible content does not prove candidate discovery, completeness, attachments or
recollection semantics for that platform.

Current evidence and user deferrals live only in `DOC-ROUTES`/`DOC-USAGE`, not in this recipe.

## Acquisition rules

- Prefer official export/API/CLI and maintained tools before browser automation.
- Reuse a signed-in browser or official desktop app only inside the explicit source scope.
- Do not ask the user to transcribe passwords, cookies, tokens or metadata that a safe official flow exposes.
- Never bypass access controls, install interception certificates or silently expand to account-wide data.
- Keep downloads outside Git and outside managed C0 until an enabled Collector route saves them.
- Record body, hierarchy, attachments, history, access state, recollection support and limitations separately.

Browser, Chrome, Lark and desktop-control Skills are acquisition dependencies only. They do not assign Babata
IDs, write SQLite, copy managed assets or change runtime capability.

## Unknown sites and new shapes

For an unknown site, perform read-only investigation of official export/API/SDK, maintained CLI/plugin and
Agent browser feasibility. Update `DOC-ROUTES` and a normal Babata Issue with evidence; do not claim collection.

For a known source's new image, audio, document, reference or attachment shape, extend its existing recipe and
tests. Do not create a new route or user Skill solely for a format.

## Stable special boundaries

- WeChat begins with official phone-to-PC migration; local recovery remains Recovery until an enabled Rust
  route saves the explicit scope.
- Browser bookmarks require their hierarchy plus selected page bodies/attachments when requested; one URL or
  manual clipping does not prove the route.
- First-party creation uses Workspace create/revise/annotate semantics, never a collection adapter.

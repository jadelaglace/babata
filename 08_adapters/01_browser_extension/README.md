# Babata Browser Collector

> Status: frozen experimental entry point. It is not a P4 gate or the current collection route.
> It requires manual clicks and bookmark captures are locator-only; automatic Agent traversal of
> bookmark URLs and page-body collection is a separate future requirement.

This unpacked Chrome extension submits explicitly prepared page, selection, and bookmark
candidates to the Babata loopback API. It never opens the data root, SQLite, or final asset
paths.

## Build

```powershell
npm install
npm run check
npm test
npm run build
```

Load this directory as an unpacked extension only after `dist/popup.js` exists.

## Local API

From `01_app`, the agent or local operator starts the Rust API with an ephemeral or
installation-scoped token:

```powershell
$env:BABATA_BROWSER_TOKEN = '<protected token with at least 32 characters>'
$env:BABATA_BROWSER_BIND = '127.0.0.1:43873'
cargo run -p babata-local-api
```

The token belongs in protected process/Chrome extension storage, not Git, logs, candidate
metadata, or C0. The server rejects non-loopback binding, non-Chrome browser origins, unknown
routes, oversized requests, and browser batches above 200 candidates.

## Workflow

```text
pair
-> prepare current page, visible selection, or a selected bookmark folder
-> inspect candidate titles and hierarchy
-> explicitly select candidates
-> collect selected
-> inspect per-item saved/skipped/failed status
```

Preparing candidates creates no C0. Only the final confirmed selection invokes the Rust
`CollectorSessionService -> CaptureService` path. Bookmarks remain locator-only unless their
webpage content is collected separately.

# Local Compass Workspace Notes

- This is the post-2.0 reboot workspace. The frozen predecessor is at
  `C:\Users\Aiano\Compass-2.0-frozen`.
- This root is intentionally not a Git repository.
- `00_docs/` is the current product and delivery authority for the reboot.
- Do not create independent module repositories, cross-module APIs, or formal
  handoff packages before a running local raw-to-view loop proves the need.
- Runtime data, source exports, media, SQLite databases, model outputs, logs,
  secrets, browser profiles, and generated views stay outside Git under the
  configured `COMPASS_DATA_HOME` data root.
- This file is local operational context, not product authority; it may be
  deleted or replaced without affecting the design record.

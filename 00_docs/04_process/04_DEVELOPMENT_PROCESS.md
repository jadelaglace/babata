# Compass Reboot Development Process

## P0 — Freeze predecessor (complete)

Compass 2.0 remains intact at `C:\Users\Aiano\Compass-2.0-frozen`. Do not
extend, merge, or revive its module repositories.

## P1 — Establish the single-repo foundation (current)

1. Create the numbered application repository and external numbered data root.
2. Configure `COMPASS_DATA_HOME`; add Git/data exclusion checks.
3. Write raw/derived/workspace migrations and a minimal `compass` CLI skeleton.
4. Define R1 PRD, criteria, architecture, and fixture tests before code.
5. Validate a SQLite-consistent fixture backup and isolated restore.

Exit: AC-01 and AC-08 pass with fixtures; no real content enters Git.

## P2 — Unified raw capture

Implement `capture text`, `capture file`, `capture export`, `create`, `revise`,
and `annotate`. Store immutable raw revisions, hashes, source/authoring context,
assets, duplicate signals, and links.

Exit: AC-02 and AC-05 pass with text, file/export, and first-party fixtures.

## P3 — First real collection routes

Enable only one route at a time: Feishu Doc/Wiki official export, browser
bookmarks/clipper or paste/import, then inbox folder watching. Record route
coverage and limitations before calling it enabled.

Exit: AC-07 passes for two distinct permitted routes.

## P4 — Bailian-derived processing

Implement mechanical extraction, then a Bailian CLI document/text run, then one
image/audio/video derivative. Add API/batch queue execution, retry, privacy
policy, and cost tracking only after the CLI loop works.

Exit: AC-03 and AC-04 pass.

## P5 — Search and views

Run Datasette/equivalent over the local indexes. Add generated Obsidian only as
a one-way view. Verify search and rebuildability.

Exit: AC-06 passes.

## P6 — Expand by real value

Add Yuque, OneNote, Evernote, WeChat, Zhihu, Bilibili, Xiaohongshu, Douyin, and
conversation sources one route at a time. Add Skills only when matching CLI
commands are proven. Reassess an abstraction whenever it has no real caller.

## P7 — Operate and harden

Schedule processing, enforce budgets/privacy, back up to local/NAS/cloud
snapshots, run restore drills, and consider a split only when the working system
has a true independent deployment or external consumer.

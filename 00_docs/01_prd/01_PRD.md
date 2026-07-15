# Babata Reboot PRD

## 1. Product definition

Babata is a local-first personal material system. It accepts external and
first-party content into an immutable raw library, creates replayable multimodal
derivatives, and exposes them through query and reading views. Human writing,
revision, annotation, and judgement are first-party records, not manual edits
to generated data.

## 2. Product surfaces

| Surface | User outcome | Authority |
| --- | --- | --- |
| Raw capture | Get text, files, exports, browser clips, and personal writing in quickly | `01_raw` |
| Derived processing | Obtain faithful text, OCR, transcripts, visual/media derivatives, and optional model aids | `02_derived` |
| Workspace | Create/revise/annotate material and connect it to other records | First-party raw revisions and relations |
| Views | Search, read, filter, export, and optionally browse generated notes | Rebuildable only |
| Skills | Let people/agents invoke real local commands safely | No data authority |

## 3. Main behaviours

### PRD-01: Unified capture

`babata capture` accepts text, local files, authorised exports, and later web
clips. It stores a raw revision and references copied or imported original assets
under the configured data root. It records provider, source locator/native ID,
collection context, source/capture times, hash, content type, and arbitrary raw
metadata. Duplicate detection signals a relationship; it never silently deletes
an event.

### PRD-02: First-party authoring

`babata create`, `revise`, and `annotate` create `first_party` raw revisions.
A new note has no external source. A revision points to its parent. An annotation
is its own authored record with an `annotates` relation. Original wording is
always preserved; model-generated format or interpretation is separate.

### PRD-03: Derivative production

`babata process` selects a configured pipeline for a raw revision. Mechanical
extraction happens before model work. Bailian CLI handles interactive processing;
Bailian/Qwen API or batch processing handles queued scale. Each run is
inspectable and retryable without mutating raw data.

### PRD-04: Media fidelity

For image/audio/video content, Babata retains the original asset and stores
OCR, transcript, subtitle, keyframe, visual-description, or summary outputs as
separate linked derivatives. It does not flatten visual or temporal meaning into
a single authoritative text field.

### PRD-05: Search and generated views

Datasette or an equivalent local SQL browser exposes raw and derived search by
text, source, collection, date, type, status, and manual metadata. Obsidian is
an optional one-way generated reading view; rebuilding it must not lose facts or
human work.

### PRD-06: Skill interaction

Skills invoke available local `babata` CLI commands. Planned skills are
Capture, Process, Workspace, Explore, and Ops; each is created only after its
underlying command passes its own tests. A Skill neither stores material nor
creates hidden bypass paths.

### PRD-07: Local Rust core and peripheral adapters

The `babata` Rust application is the only owner of raw/derived SQLite writes,
asset finalisation, revision rules, processing tasks, and backup snapshots. Its
CLI is the default interface for people, Skills, scripts, and scheduled tasks.
When a browser clipper or local UI needs direct interaction, the same Rust
application exposes a loopback-only, token-protected local API. Rust is the
default implementation for importers, provider adapters, views, and operations.
JavaScript is limited to browser-facing capture. Python is an exception-only
child-process adapter for mature Python-only tooling. Both submit validated
candidates to the Rust core rather than owning storage or business decisions.

## 4. Initial source route matrix

| Source | Preferred route | Candidate tools / light work | Fallback |
| --- | --- | --- | --- |
| Feishu Doc/Wiki/knowledge base | Official export/OpenAPI | Wiki node-to-document resolution, export/import adapter | PDF/manual export |
| Yuque | Export/OpenAPI | `yuque-exporter`-class tool or importer | Markdown/PDF/copy |
| OneNote | Office export | Microsoft Graph Pages adapter | PDF/Word/copy |
| Evernote | ENEX export | ENEX importer | HTML/PDF |
| WeChat favorites/chats | Official/local backup first | Web clipper for saved articles; local parser candidates after privacy review | Original backup/screenshot/PDF |
| Zhihu/Bilibili/Xiaohongshu/Douyin | Export/list route where available | Browser exporter, user script, source-specific adapter; `yt-dlp` for permitted media | Link/PDF/screenshot/recording |
| Browser bookmarks/pages | Browser HTML export / clipper | Bookmark importer; extension/userscript plus local command | Copy/PDF |
| Doubao/Kimi/GPT | Official export | Export parser or share-page clipper | Copy/screenshot |
| First-party content | Inbox/CLI | Folder watch, shortcut, editor command, local form | Paste |

All candidate tools are validated against permitted material before enablement.

## 5. First release slice

1. Establish the complete Babata module/file/interface/tool skeleton without
   implementing the individual business algorithms.
2. Set an external data root and create the raw schema.
3. Capture text, a local file/export, and one first-party note.
4. Import one Feishu export and one browser/bookmark/web input.
5. Produce faithful text for one document and one media derivative through
   Bailian CLI.
6. Search both raw and derived data locally.
7. Demonstrate original/revision/annotation history and rebuild an optional view.

## 6. Non-goals

No full-platform rollout, custom frontend, automatic knowledge decisions,
manual maintenance of derived/Obsidian records, or pre-built empty Skill set.

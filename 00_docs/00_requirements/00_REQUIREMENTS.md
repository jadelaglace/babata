# Compass Reboot Requirements

## 1. Reset and purpose

Compass 2.0 is frozen at `C:\Users\Aiano\Compass-2.0-frozen`. Its multi-repo,
contract-first design is reference material only. The reboot builds a useful
local personal knowledge system before introducing service boundaries.

The system must make real material easy to capture, preserve the original,
derive useful multimodal representations, and support human thought without
confusing derived model output with source truth.

## 2. Required flow

```text
external source or first-party creation
  -> 01 raw collection and append-only storage
  -> 02 derived multimodal processing
  -> 03 human workspace and rebuildable views
```

The initial implementation is one local application repository, not independent
Collector/Ingest/Core/Output repositories. Functions may later split only after
a real independent deployment or consumer proves that need.

## 3. Source and originality requirements

- Every item enters the raw layer as either `external` or `first_party`.
- `first_party` covers personal notes, drafts, revisions, annotations,
  reflections, and manual metadata decisions. It is a source kind alongside
  Feishu, Bilibili, Zhihu, and other providers, not a special bypass.
- A revision creates a new immutable revision with an optional parent revision.
  An annotation creates a first-party item related to the annotated item.
- Raw records preserve source context: favorites/list membership, conversation
  identity/order, workspace/notebook/document hierarchy, or authoring context.
- Original bytes, text, images, audio, video, exports, and attachments remain
  available unchanged whenever lawfully obtainable.

## 4. Derived-processing requirements

- The raw layer is read-only to processors. A process produces a separately
  recorded derivative with input hash, run/tool/model/prompt version, status,
  cost, error/retry data, and output hash.
- Faithful extraction (`faithful_text`, OCR, subtitle extraction, transcript)
  is distinct from model interpretation (summary, tags, classification,
  quality/relationship suggestions).
- Images, audio, video, animations, and their original assets are not discarded
  merely because text or a summary was derived. Time anchors, keyframes, and
  visual descriptions are additional derivatives.
- Alibaba Bailian CLI is the first interactive processing tool. Bailian/Qwen
  APIs and batch processing are the intended paid path for queues and scale.
  Free-only operation is not a product goal.
- Processing is queued by value, rights, privacy, modality, and cost; not every
  item must be processed immediately.

## 5. Storage, repository, and recovery requirements

- `C:\Users\Aiano\Compass` is the sole Git application repository for code,
  skills, migrations, docs, tests, and configuration templates.
- Real data lives under a configured external `COMPASS_DATA_HOME`, initially
  `C:\Users\Aiano\CompassData`; database records use relative asset keys, not
  hard-coded machine paths.
- Raw evidence and first-party records are P0 backup data. Derived data is P1.
  Views are P2 and runtime/logs are P3 because they can be recreated.
- SQLite and assets require consistent encrypted incremental backups. NAS/cloud
  copies receive SQLite-consistent snapshots, never blind concurrent sync of a
  live database file.
- Datasette is the first inspection/search view. Obsidian is optional generated
  output only; it is never the sole source of truth and can be deleted/rebuilt.

## 5.1 Technical implementation requirements

- Rust is the default and preferred implementation language for every Compass
  capability: domain types, SQLite access, migrations, asset placement,
  hashing, source importers, version/relationship rules, task queue, processing
  provenance, provider adapters, CLI, local API, worker, views, and backup
  orchestration.
- JavaScript/TypeScript is allowed only for code that must execute in a browser,
  such as an extension/userscript capture UI and DOM extraction. It never writes
  SQLite, finalises assets, or owns processing/business rules.
- Python is an exception-only escape hatch for a mature Python-only parser/tool
  whose value outweighs a Rust implementation. It runs as a versioned child
  process, emits a candidate envelope, and never writes SQLite, finalises
  assets, owns queue state, or becomes a general application layer.
- The Rust core owns every mutation. Peripheral tools submit candidate data to
  the local CLI/API or emit a versioned candidate envelope for the Rust core to
  validate and persist.
- The default interaction surface is the `compass` CLI. A loopback-only local
  API exists only for real local callers such as a browser extension or local
  UI; it is not a public service or future distributed contract by default.
- Secrets and provider tokens reside in protected local configuration outside
  Git. The API binds to loopback, requires an installation-local token, and has
  no remote listener in the initial architecture.
- Before Rust implementation begins, the architecture must name the initial
  physical Rust file inventory, each file's responsibility, its public
  types/functions/traits, dependency limits, and the later extension location.
  It must define a small first-slice inventory rather than create speculative
  empty files for every future capability.

## 6. Source-route policy

Use this route order: official export/API, maintained CLI/SDK/open-source tool,
browser extension/userscript, narrow local adapter, then PDF/screenshot/copy or
screen-recording fallback. A listed source is not "supported" until it imports
permitted material successfully and preserves its declared coverage/limits.

Initial route pool: Feishu Docs/Wiki/knowledge bases, Yuque, OneNote, Evernote,
WeChat favorites and chats, Zhihu, Bilibili, Xiaohongshu, Douyin, browser
bookmarks and pages, Doubao/Kimi/GPT conversations, local files, and first-party
authoring.

## 7. Explicit non-goals

- No return to a five-repository protocol architecture.
- No universal crawler, access-control bypass, or unverified third-party tool
  treated as product support.
- No model output overwriting original content or becoming automatic knowledge
  truth.
- No custom UI before import, processing, and local search work on real data.

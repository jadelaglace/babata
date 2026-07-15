# Babata Reboot Acceptance Criteria

## AC-01: External data root and Git boundary

The application resolves `BABATA_DATA_HOME`; all real SQLite files, source
assets, exports, derived outputs, logs, runtime state, and credentials stay
outside the repository. A Git scan finds only code, docs, tests, migrations, and
templates.

## AC-02: Raw capture is append-only and contextual

Text, a file/export, and a first-party note each create resolvable raw revisions
with hashes, provider/source kind, capture/creation time, asset references, and
collection/authoring context. Re-import or revision produces a linked record,
not an overwrite.

## AC-03: Original media and wording survive processing

After text extraction, OCR, transcript, or a model run, the original raw text
and every original asset hash/path remain unchanged and resolvable. Derivatives
identify their input revision and processing run.

## AC-04: Processing is traceable and retryable

A Bailian CLI/API run records tool/model/prompt or pipeline version, input hash,
status, output hash, cost where supplied, and error/retry information. A failed
run retries without changing raw data; a derivative can be deleted/rebuilt.

## AC-05: First-party creation, revision, and annotation are unified

An authored note, a revision, and an annotation of an external item are stored
as first-party records with explicit version or relation links. The authored
original remains readable without requiring any generated view.

## AC-06: Search and views have no hidden authority

Local search locates raw and derived content by text and metadata and exposes
source/assets/processing lineage. Removing a generated Obsidian or export view
does not remove raw, derived, or first-party records and the view can rebuild.

## AC-07: Route enablement is evidence based

A source route is marked enabled only after an authorised test/import records
its coverage, limitations, metadata/attachment result, and re-import behaviour.
Unauthorized, incomplete, or failed routes remain explicit and do not claim
support.

## AC-08: Backup is SQLite consistent

An isolated restore from an encrypted incremental backup yields readable SQLite
indexes and assets whose sampled hashes match the snapshot manifest. NAS/cloud
replication consumes the created snapshot rather than a live SQLite file.

## AC-09: Rust core is the only persistent writer

Every repository mutation path is implemented or specified through the Rust
core's CLI/API/use-case layer. Rust is the default implementation for all
application capabilities. JavaScript browser code and exception-only Python
adapters can only submit capture/process candidates or invoke the CLI; they have
no SQLite driver configuration, direct database write path, asset-finalisation,
queue-state, or business-rule path.
The local API listens only on loopback, rejects requests without an
installation-local token, and shares the same use-case implementation as the
CLI rather than duplicating business rules.

## AC-10: Rust dependency direction is explicit and acyclic

The Rust workspace separates pure domain types, application use cases and port
traits, infrastructure implementations, and delivery composition roots. Domain
has no IO dependencies; application has no SQLite/filesystem/HTTP/provider
dependencies; infrastructure implements application ports; CLI/API/worker wire
dependencies and contain no business decisions. Dependency checks and tests
reject reverse imports or a second persistence writer.

## AC-11: Whole-system skeleton is complete and coherent

Before any single module is accepted functionally, the architecture names and
the P2 workspace establishes the full module skeleton: six Rust crates, all
planned Rust source files, application services and ports, CLI command groups,
loopback API routes, worker lifecycle, source/process/view/backup adapter
locations, peripheral browser/Python boundaries, Skill specifications,
migrations, tests, engineering scripts, configuration templates, and external
tool ownership.

The complete workspace compiles and every file/interface has one owner, allowed
dependencies, forbidden dependencies, activation phase, and test home. Inactive
capabilities return an explicit unavailable state; they do not perform real
provider calls, claim support, introduce a second writer, or require one module's
functional acceptance to close P2. Early raw-capture code may remain but is not
accepted until P3.

## Traceability

| Requirement / PRD | Acceptance | Test |
| --- | --- | --- |
| PRD-01 | AC-01, AC-02, AC-07 | TC-01, TC-02, TC-07 |
| PRD-02 | AC-02, AC-05 | TC-02, TC-05 |
| PRD-03, PRD-04 | AC-03, AC-04 | TC-03, TC-04 |
| PRD-05 | AC-06 | TC-06 |
| PRD-06 | AC-01, AC-07 | TC-07 |
| PRD-07 | AC-09 | TC-09 |
| PRD-07 | AC-10 | TC-10 |
| PRD-07 | AC-11 | TC-11 |
| Storage/recovery | AC-08 | TC-08 |

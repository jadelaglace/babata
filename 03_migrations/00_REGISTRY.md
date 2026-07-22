# Migration Registry

| Database | Owner | Directory | Naming | Activation |
| --- | --- | --- | --- | --- |
| raw SQLite | `SqliteRawRepository` | `01_raw/` | `NNNN_raw_description.sql` | P3; attachment-only recovery is separated by 0005; common item metadata and append-only source observations start at 0006 |
| collection extension | `SqliteRawRepository` | `02_collection/` | `NNNN_collection_description.sql` | P4; candidate common metadata starts at 0005 |
| derived SQLite | `SqliteDerivedRepository` | `02_derived/` | `NNNN_derived_description.sql` | P5 |
| runtime SQLite | `SqliteJobRepository` | `03_runtime/` | `NNNN_runtime_description.sql` | P5 |
| raw integrity extension | `SqliteRawRepository` | `04_integrity/` | `NNNN_raw_description.sql` | P6 preflight |
| knowledge extension | `SqliteRawRepository` | `05_knowledge/` | `NNNN_knowledge_description.sql` | P6.1; 0001 superseded and data-preserved by 0002; semantic core starts at 0003; map evolution history starts at 0004; baseline foundation transition guard is closed by 0005 |
| search projection SQLite | `SqliteReadProjection` | `06_projection/` | `NNNN_projection_description.sql` | P6.2; disposable C2 read model, rebuilt only from raw/knowledge and derived authority |

Only Rust infrastructure applies these migrations. Empty phase directories are
intentional until their activation phase.

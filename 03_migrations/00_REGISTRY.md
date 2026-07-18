# Migration Registry

| Database | Owner | Directory | Naming | Activation |
| --- | --- | --- | --- | --- |
| raw SQLite | `SqliteRawRepository` | `01_raw/` | `NNNN_raw_description.sql` | P3 |
| collection extension | `SqliteRawRepository` | `02_collection/` | `NNNN_collection_description.sql` | P4 |
| derived SQLite | `SqliteDerivedRepository` | `02_derived/` | `NNNN_derived_description.sql` | P5 |
| runtime SQLite | `SqliteJobRepository` | `03_runtime/` | `NNNN_runtime_description.sql` | P5 |

Only Rust infrastructure applies these migrations. Empty phase directories are
intentional until their activation phase.

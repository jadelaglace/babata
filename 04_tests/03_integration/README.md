# Integration tests

Phase integration tests live beside the Rust composition root so `cargo test --workspace` runs
them with the built CLI:

- `01_app/04_babata_cli/tests/p3_raw.rs` covers the fresh-root C0 and first-party loop;
- `01_app/04_babata_cli/tests/p4_routes.rs` proves later provider commands and capabilities remain
  unavailable/disabled until P4.

Inputs come from `04_tests/03_fixtures`; every run uses a temporary `BABATA_DATA_HOME`. Runtime
SQLite, assets, journals and generated output never belong in this directory or Git.

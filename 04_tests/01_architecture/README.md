# Architecture tests

The active architecture and delivery checks are executable through the scripts in
`05_scripts/`. They cover TC-09, TC-10, TC-11, phase inventories, and document
traceability. `check-r1-inventory.ps1` is an obsolete guard and intentionally fails;
CI runs the maintained checks explicitly instead of globbing every script.

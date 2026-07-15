# Babata Documentation Chain

The numbered documents govern the reboot in order:

1. `00_requirements/00_REQUIREMENTS.md` — intent and non-negotiable constraints
2. `01_prd/01_PRD.md` — user-facing product behaviour and route matrix
3. `02_acceptance/02_ACCEPTANCE_CRITERIA.md` — definition of done
4. `03_architecture/03_ARCHITECTURE.md` — ownership, storage, processing, backup
5. `04_process/04_DEVELOPMENT_PROCESS.md` — implementation order, phase gates, and verification commands
6. `05_tests/05_TEST_CASES.md` — verification cases

Architecture supplements:

- `03_architecture/04_SYSTEM_SKELETON_BLUEPRINT.md` — P2 complete module, file, interface, command, route and tool skeleton.
- `03_architecture/05_RAW_FOUNDATION_BLUEPRINT.md` — delayed P3 raw capture and immutable-storage implementation detail.
- `03_architecture/06_RAW_FOUNDATION_EXECUTION_PLAN.md` — preserved P3 SQL, transaction, test and command-verification sequence.

Do not introduce a new abstraction before it has a concrete working caller.

Current execution status is maintained only in section 1 of
`04_process/04_DEVELOPMENT_PROCESS.md`. It uses the single project-phase
sequence P0–P8; data backup criticality uses C0–C3 instead.

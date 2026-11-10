# Operations Skill Specification

- Target CLI: `babata ops ...`
- Input: status/doctor command or snapshot reference
- Output: health and restore-verification evidence
- Permissions: invoke Rust CLI only; secrets remain external
- Errors: backup, restore, integrity, `capability_unavailable`
- Activation: P8 after TC-10; full-system recovery evidence also contributes to TC-11

File protocol (DOC-ARCH §7/§13, TC-10E):

- Environment-owned data/evidence/recovery/assistant roots are disjoint and outside Git.
- Backup includes unknown unique runtime/log content and durable auxiliary files. It is
  not authorization to process deferred sources. Credentials/login profiles are listed exclusions.
- A manifest records each root, file hash and start/end time; cross-database freeze still
  requires quiesced writers and relationship verification by the execution round.
- State binds repository path/id and full snapshot id; legacy location-unknown states
  are not healthy. Existing restored v1 manifests remain verifiable without rewriting them.
- Produce/check/restore on the working disk; delivery/independent verification and
  authorized retirement are separate operations. No automatic forget/prune.
- BABATA_BACKUP_EXCLUSIONS_FILE optionally supplies exact reviewed root_alias,
  relative_path and reason entries. Browser profiles require an explicit rule after
  protecting unique work; do not infer an entire directory disposable from its name.

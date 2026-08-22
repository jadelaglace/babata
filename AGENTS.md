# Local Babata Workspace Notes

## Mandatory Recovery Hook

<!-- BABATA-RECOVERY-HOOK: v1 -->

At every full recovery boundary--a new session, context compaction, Agent/task
handoff or interruption that may lose control/context, a long pause with uncertain
execution state, or an explicit `continue`, `resume`, or equivalent instruction--do
this before any other state change:

1. Call the available Goal/task-state API. A missing result is `unknown`; it is
   not evidence that work is unfinished and cannot change an active or terminal state.
2. Immediately read `00_docs/04_process/04_c_ACTIVE_PLAN.md` and execute only the
   item named by its single `CURRENT-ACTIVE` marker.
3. Do not select work from summaries, recent messages, old plan IDs, or the queue.
   A queued item marked `requires-explicit-resume` stays queued until the user
   explicitly resumes it; it is never auto-promoted.

An ordinary synchronous tool, command, or API failure that returns a clear result
while the current turn and task identity remain intact is not a full recovery
boundary; handle or retry it locally. If a stateful operation has an ambiguous
outcome, reconcile that external state and run this hook only when control/context
or the governing task may also have been lost.

The lifecycle contract is owned by
`00_docs/04_process/04_a_DEVELOPMENT_PROCESS.md`. This shallow hook only
routes recovery to that authority; it does not become a second product or plan authority.

<!-- /BABATA-RECOVERY-HOOK: v1 -->

- This is the post-2.0 reboot workspace. The frozen predecessor is at
  `C:\Users\Aiano\Babata-2.0-frozen`.
- This root is the Babata Git repository; use `main` and the configured `origin`
  unless the user gives a different Git instruction.
- Routine use of an existing Babata release to process authorized material does
  not require a GitHub Issue, development branch, or Pull Request. Record the
  Babata version/tag/commit/dirty state in the external execution receipt.
- Product features, behavior or contract changes, schema/migration work, and
  promoted bug fixes start from a GitHub Issue, use a short-lived `codex/`
  branch, and merge through a Pull Request after applicable checks. A low-risk
  usage-status or evidence-pointer writeback may be committed as housekeeping
  without an Issue after its scoped document checks.
- Collect observed bugs with their build identity and evidence before deciding
  to fix them. Do not mutate implementation inside a frozen usage round.
- `00_docs/` is the current product and delivery authority for the reboot.
- Do not create independent module repositories, cross-module APIs, or formal
  handoff packages before a running local raw-to-view loop proves the need.
- Runtime data, source exports, media, SQLite databases, model outputs, logs,
  secrets, browser profiles, and generated views stay outside Git under the
  configured `BABATA_DATA_HOME` data root.
- This file, including the mandatory recovery hook, is local operational context
  rather than product authority. Replacing it must preserve an equivalent link
  and order constraint to the same Docs authority.

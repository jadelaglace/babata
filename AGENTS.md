# Local Babata Workspace Notes

## Mandatory Recovery Hook

<!-- BABATA-RECOVERY-HOOK: v1 -->

At every recovery boundary--a new session, context compaction, Agent/task handoff,
Agent or tool interruption, long pause, or an explicit `continue`, `resume`, or
equivalent instruction--do this before any other state change:

1. Call the available Goal/task-state API. A missing result is `unknown`; it is
   not evidence that work is unfinished and cannot change an active or terminal state.
2. Immediately read `00_docs/04_process/04_f_ACTIVE_PLAN.md` and execute only the
   item named by its single `CURRENT-ACTIVE` marker.
3. Do not select work from summaries, recent messages, old plan IDs, or the queue.
   A queued item marked `requires-explicit-resume` stays queued until the user
   explicitly resumes it; it is never auto-promoted.

The lifecycle contract is owned by
`00_docs/04_process/04_g_INTENT_AND_PLAN_GOVERNANCE.md`. This shallow hook only
routes recovery to that authority; it does not become a second product or plan authority.

- This is the post-2.0 reboot workspace. The frozen predecessor is at
  `C:\Users\Aiano\Babata-2.0-frozen`.
- This root is the Babata Git repository; use `main` and the configured `origin`
  unless the user gives a different Git instruction.
- Regular work starts from a GitHub Issue, uses a short-lived branch, and is
  merged through a Pull Request after applicable checks. Do not use `main` as
  the daily development branch or push directly to it unless the user gives an
  explicit emergency instruction.
- `00_docs/` is the current product and delivery authority for the reboot.
- Do not create independent module repositories, cross-module APIs, or formal
  handoff packages before a running local raw-to-view loop proves the need.
- Runtime data, source exports, media, SQLite databases, model outputs, logs,
  secrets, browser profiles, and generated views stay outside Git under the
  configured `BABATA_DATA_HOME` data root.
- This file, including the mandatory recovery hook, is local operational context
  rather than product authority. Replacing it must preserve an equivalent link
  and order constraint to the same Docs authority.

# Evidence manifest: P21's copies in gone Codex threads (fix/stale-compaction-rows)

Not a task's evidence. From #134's review (the coordinator's decision, 2026-10-22): honest usage is a 0.1 gate, and an older build's P21 row in a thread whose rollout is gone kept double-counting, because only re-read sessions are replaced.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Only a gone thread's copy of a stored call goes | `UsageRepo::forget_duplicated_compactions`: a `…/compacted/…` row goes only when its session isn't among the listed rollouts' threads and its five token classes equal a stored record's (`thread/turn/response`). `juggler-core :: compaction_leftovers :: only_a_gone_sessions_copy_of_a_stored_call_goes` (the copy goes, and nothing else: every other row stays, checked by key. That covers a gone file's real compaction, the same tokens in a thread still here, a near-copy with one class different, a gone thread's own record with the same tokens, and a live thread's real compaction; a second run deletes nothing) | — | passed | PASS | da999c73 | 2026-10-22 | claude-opus |
| Once per account, no file re-read | Its own marker in the ingest state (`compactions_checked`), not `REREAD_VERSION`: nothing is read again, so there's nothing to coordinate with release/0.1. A listing with a rollout name that gives no thread skips the check. `usage_pipeline :: a_gone_threads_copy_of_a_stored_call_goes_once` (the copy goes on the first pass; one stored later stays: the check ran once) | fails on main | main: the copy stays; now passes | PASS | da999c73 | 2026-10-22 | claude-opus |
| #143 review: only from a whole listing with rollouts | The walk reports whether its listing is whole (a folder or entry it couldn't read, or one past the depth limit, makes it partial). The check runs only from a whole listing with rollouts in it; otherwise it is left for a later pass, so a missing, empty or unreadable sessions folder never makes every thread look gone. `usage_pipeline :: no_rollouts_listed_means_no_check_until_they_are` (missing, then empty: nothing deleted, marker unset; the rollouts listed again: the copy goes) | fails before the guard | before: the missing folder deleted the copy; now passes | PASS | 4e1bde7f | 2026-10-22 | claude-opus |

## Gates

| Gate | Head | Result |
|---|---|---|
| `just verify` (`CARGO_INCREMENTAL=0`, includes the three clippy variants) | 5c3c4446 (the review, main merged in) | 151 test binaries, 1,232 passed, 0 failed |
| `cargo deny check` | 5c3c4446 | ok |
| vitest, visual, a11y | — | nothing under `ui/` changed |

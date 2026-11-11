# Evidence manifest — AJ-E2912

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2912, Debug
mode: a detailed local log of switches and launches. Checked again on
2026-11-06 (release/0.4.1); the vitest file below passes; the Rust tests
run in the release preflight.
Status: **DONE-EVIDENCED** (2026-11-06, release/0.4.1; R1 to R4 closed
the same day).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `report-bug.test.tsx` "says when it turns itself off, and clears in one click": starts off, says "Never what's in a conversation", turns on ("then off by itself"), Show shows the scrubbed text, Clear leaves the row "Empty."; "words for off and on". Gallery `99-debug-mode` shows the section in Settings › Advanced, under Diagnostics. **Closed (2026-11-06):** `report-bug.test.tsx` "Clear the log empties it and says “Cleared.” where it was": the note "Cleared." on Debug mode's row, the log row "Empty.", Clear off. |
| R2 | PASS | `crates/juggler-handover/tests/debug_steps.rs` `the_switchs_debug_steps_never_carry_the_conversation` holds the switch's steps (start, bundled, installed, launched, done); the replica test holds a launch's `spawned` step. [debug-log.txt](debug-log.txt) shows durations, files and bytes, the mechanism, the cut, the fallback, the fidelity, the stage and the tool's own words, scrubbed. **Closed (code and test, 2026-11-06):** a switch's `bundled` step logs the conversation's turns (`turns=4`, and `left_out` with a cut; "-" from a tool whose conversations travel whole), from the adapter's own count (`juggler-handover` `pipeline.rs`); a launch logs its exit (`launch exited code=…`, or `signal=…`) from the supervisor's one exit path (`juggler-supervisor` `ExitKind::log_exit`). `bug_report_replica.rs` holds both: a `launch exited` line with its code and a `switch bundled … turns=4` line. [debug-log.txt](debug-log.txt) was written again from that run (2026-11-06). |
| R3 | PASS | Two files of at most 8 MB: `debug_log.rs` `the_file_rolls_over_and_keys_never_reach_it` (19 MB written, at most two files kept). In Report a bug: the replica test asserts a `debug` part. **Closed (code and tests, 2026-11-06):** debug mode now stops itself with a notice: `debug_log::stop_if_due` at start and on the backend's 5 s tick (`debug_log::tick`) turns it off once its seven days are up and records "Debug mode turned itself off" ("It was on for 7 days. Its log stays until you clear it in Settings › Advanced.", under Earlier, opening Diagnostics; kind `debug`, mutable); past its days nothing more is written even before the tick. `debug_log.rs` `it_stops_itself_after_seven_days_and_says_so`: on within its days, nothing written past them, stopped once with the notice, off in Settings. `diagnostics::tests::export_carries_the_debug_log_while_it_has_lines`: the preview and the saved zip carry `logs/debug.log` and `logs/debug.1.log`, scrubbed (home as `~`), and neither once cleared. |
| R4 | PASS | In code: `keep` writes the file only while on, and `set(false)` leaves the file. **Closed (2026-11-06):** `debug_log.rs` `off_writes_nothing_and_turning_it_off_keeps_the_log`: off, a step writes no file; on, each step is written; turned off, nothing more is written and the two lines stay (Settings shows them) until Clear removes the file. |
| R5 | PASS | `tests/bug_report_replica.rs` `a_failed_switch_reports_what_it_did_and_never_the_conversation`: a four-turn fixture conversation and its folder carry a marked word; after a launch, a switch and a failed switch the log has no line with it. `debug_steps.rs` holds the same for the switch pipeline alone. |

## The first part (2026-11-05)

### AJ-E2912 evidence: debug mode

- `crates/juggler-app/tests/bug_report_replica.rs`, with debug mode on: a
  launch, a switch that lands and a switch that fails, through the app's
  own commands on a replica data folder.
- [debug-log.txt](debug-log.txt): the log as written. Each step is a flow,
  a step, and numbers and closed words (sizes, counts, durations, the
  mechanism, the fidelity, the stage); the failure carries the tool's own
  words with paths scrubbed (`[path]`).
- The test holds that no line carries the conversation's marked text or
  its folder; `crates/juggler-handover/tests/debug_steps.rs` holds the same
  for the switch pipeline on its own (Req. 5).
- Bounds, the seven days and off-keeps-the-log: unit tests in
  `features/app/diagnostics/debug_log.rs`.
- Every line is now scrubbed before it's kept (home folder, user name,
  paths outside home, links, keys), not only when shown.
- The section's captures: gallery `99-debug-mode` (both themes, 1280 and
  1024, axe clean, critique stamped).
- Verdict: PASS.

Gaps closed on 2026-11-06: the whole vitest suite passed (`pnpm exec vitest run --maxWorkers=4`: 281 files, 2135 tests) with the tests named above.

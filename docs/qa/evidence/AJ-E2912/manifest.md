# Evidence manifest — AJ-E2912

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2912, Debug
mode: a detailed local log of switches and launches. Checked again on
2026-11-06 (release/0.4.1); the vitest file below passes; the Rust tests
run in the release preflight.
Status: **DONE, 4 gaps open** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | GAP | `report-bug.test.tsx` "says when it turns itself off, and clears in one click": starts off, says "Never what's in a conversation", turns on ("then off by itself"), Show shows the scrubbed text, Clear leaves the row "Empty."; "words for off and on". Gallery `99-debug-mode` shows the section in Settings › Advanced, under Diagnostics. **Missing:** the "Cleared." confirmation in place is not asserted. |
| R2 | GAP | `crates/juggler-handover/tests/debug_steps.rs` `the_switchs_debug_steps_never_carry_the_conversation` holds the switch's steps (start, bundled, installed, launched, done); the replica test holds a launch's `spawned` step. [debug-log.txt](debug-log.txt) shows durations, files and bytes, the mechanism, the cut, the fallback, the fidelity, the stage and the tool's own words, scrubbed. **Missing:** no step counts turns, and a launch logs no exit code (only its error words on failure). |
| R3 | GAP | Two files of at most 8 MB: `debug_log.rs` `the_file_rolls_over_and_keys_never_reach_it` (19 MB written, at most two files kept). In Report a bug: the replica test asserts a `debug` part. **Missing:** no notice when debug mode stops itself after 7 days (known gap, `docs/qa/0.4-completion.md` §2), and Settings says so only in advance ("then off by itself"). No test covers the 7-day stop (`DAYS_ON`, `install`, `keep`) or that Export diagnostics carries `logs/debug.log` (`diagnostics/mod.rs` lists it). |
| R4 | GAP | In code: `keep` writes the file only while on, and `set(false)` leaves the file. **Missing:** no test holds that nothing is written while off, or that turning it off keeps the log until it's cleared. |
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

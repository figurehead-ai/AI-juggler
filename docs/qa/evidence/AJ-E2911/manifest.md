# Evidence manifest — AJ-E2911

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2911, Report a
bug: what goes, shown first, from Help, Settings and every failure.
Checked again on 2026-11-06 (release/0.4.1); the vitest files below pass;
the Rust tests run in the release preflight.
Status: **DONE, 3 gaps open** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | GAP | Reached from: the failed switch's step (`switch-flow.test.tsx`, its `switch-report-bug` opens the sheet with the screen's words and detail); an error toast with no action of its own (`report-bug.test.tsx` "an error toast with no action of its own offers Report a bug with its words"); Settings (`diagnostics.test.tsx` "opens Report a bug"). **Missing:** a launch that fails in New session shows only an inline `role="alert"` line (`NewSession.tsx`), with no Report a bug (known gap, `docs/qa/0.4-completion.md` §2). Help › Report a bug… exists (`quit.rs`, `lib.rs` emits `app://report-bug`) but no test covers it. The Settings door sits in Advanced › Diagnostics beside Export diagnostics, not in About & updates as written. |
| R2 | GAP | Order: `report-bug.test.tsx` "shows every part, and the form gets only what's kept" (the words field, then what AI Juggler showed, the app and this computer, the last switch, recent errors) and `tests/bug_report_replica.rs` `a_failed_switch_reports_what_it_did_and_never_the_conversation` (`failure`, `app`, `switch` first). Version and channel: `report.rs` `the_form_is_filled_from_the_kept_parts_and_the_logs_cut_to_fit` ("0.4.0", "AI Juggler, Beta"). [report.txt](report.txt) shows the OS and the switch's stages with sizes and timings. **Missing:** no test holds the error log's 50-line cap (`ERROR_LINES`), and none holds the last switch's one-hour window (`SWITCH_WITHIN_MS`; `debug_log.rs` `the_last_flow_starts_at_its_start_and_ages_out` tests the ageing with a 60 s window only). |
| R3 | PASS | The report uses Export diagnostics' own scrubber (`Scrub::here()`), whose canary test is `diagnostics/tests.rs` `the_zip_holds_what_was_shown_and_nothing_it_shouldnt` (home, user, links). For the report itself: the replica test asserts no part carries the conversation's marked text or a path outside home; `names_are_scrubbed_longest_first_and_common_folders_kept`; `a_key_typed_into_the_words_is_removed`; `crash.rs` `a_personal_path_outside_home_goes_and_a_system_one_stays`. The whole text is shown read-only part by part (`report-bug.test.tsx`). |
| R4 | GAP | Open the bug form: `the_address_is_the_bug_form_and_encoded` (the fixed `issues/new?template=bug_report.yml` address); the window passes only the report's id and choices (`app_bug_report_open`). Fields filled from what's kept, logs cut to fit with the newest lines kept and `cut` set: `the_form_is_filled_from_the_kept_parts_and_the_logs_cut_to_fit`; the replica's link ≤ 8 000. **Missing:** no test clicks **Copy** (`report-copy`) to check the whole text reaches the clipboard, and none checks the sheet says the logs were cut (`report-cut`). |
| R5 | PASS | `report-bug.test.tsx` "shows every part, and the form gets only what's kept" (all parts go by default; unticking Recent errors drops it from what's sent) and "one closed line names what goes with it; unticking a part drops it from the line" (the preview updates). The replica test: a part left out is gone from the composed body. |

## The first part (2026-11-05)

### AJ-E2911 evidence: Report a bug

- `crates/juggler-app/tests/bug_report_replica.rs`: on a replica data
  folder built by the test (three Claude Code accounts, a four-turn
  conversation whose every turn and whose project folder carry a marked
  word), a switch through the app's own command fails because the target
  account's folder refuses the write. Report a bug is prepared from that
  failure as the failed step's button prepares it (`from: switch`, the
  screen's words, its detail and code).
- It holds: the parts come in order (what AI Juggler showed, the app and
  this computer, the last switch, then the logs); no part carries the
  marked word or a path outside home; leaving parts out removes them from
  what goes; the bug form's link fits.
- [report.txt](report.txt): the whole text Copy copies, as composed.
- [link.txt](link.txt): the link's length (1 970 characters, nothing cut)
  and each field's size.
- Found and fixed on the way: a tool's own error named the project's path
  outside home (a temporary folder); the shared scrubber now replaces a
  personal path outside home with `[path]` (`crash::scrub`, unit test
  `a_personal_path_outside_home_goes_and_a_system_one_stays`).
- The sheet's captures: gallery `99-report-bug` (both themes, 1280 and
  1024, axe clean, critique stamped).
- Verdict: PASS.

# Evidence manifest — AJ-E2403 The user's stop of another app's leftover (D-41b)

Task: `docs/tasks/E24-processes.md` AJ-E2403 (no amendments). Re-audited
against the code and tests on `release/0.4.1`, 2026-11-07, replacing the
2026-10-25/26 manifest from branch `e24-processes`. The path is
`crates/juggler-app/src/features/processes/stop.rs` (`check`, `facts_now`,
`verify`, `execute`), reached only through the IPC commands
`processes_observed_verify` / `processes_observed_stop`
(`features/processes/commands.rs`, registered in `lib.rs`; called from
`ui/src/lib/ipc/processes.ts`). Shipped in Release 0.2.0 (f6fb096cb).

Cargo and Playwright tests below are cited as written and **run in CI**; they
were not run for this audit (shared Mac). The vitest files were run here:
`cd ui && pnpm -s exec vitest run src/screens/processes-page.test.tsx src/components/kill-sheet.test.tsx`
→ 2 files, 25 tests passed (2026-11-07).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 only by the user's call; no automatic caller at any tier | `cargo test -p juggler-app --test observed_stop no_automatic_rule_ever_stops_another_apps_leftover` (every account's tier 1 and 2 on, `auto_cleanup_sweep` run 24 h later in three steps, then `processes::tick`: the leftover lives, nothing audited); `grep -rn "stop::execute\|stop::verify" crates` → only `processes/commands.rs` (`processes_observed_verify`, `processes_observed_stop`) | no caller but the user's command | as asserted; no other caller in shipped code | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R2 only after the session's own process ended; never the session's own process, the app, a process inside an app's bundle, AI Juggler or its hooks, or a ledger process | `cargo test -p juggler-app --lib features::processes::stop::tests::the_identity_check_fails_closed_on_every_leg` (session runs, session listed → `SESSION_RUNS`; session itself; ledger; app; "inside an app" `/Applications/An.app/…` → `APP`; own pid and pid 1 → `OWN`), `a_framework_bundled_interpreter_is_no_app` (Homebrew's `Python.app` inside `Python.framework` may be stopped — the lab's bug 3 fix); `cargo test -p juggler-app --test observed_stop every_refusal_sends_zero_signals` (session runs, session itself, AI Juggler itself, live fixtures) | every "never" refused | all legs but one evidenced. **Missing:** AI Juggler's hooks — `check` refuses `program == juggler_agentbridge::wire::PROGRAM` ("juggler-agent") with `refusal::OWN` (`stop.rs`), but no case covers it; add one to `the_identity_check_fails_closed_on_every_leg`: sighting `exe_name "juggler-agent"`, `now.exe ".../juggler-agent"`, `name "juggler-agent"` → `Err(refusal::OWN)` **Closed 2026-11-07:** the case "AI Juggler's hook" added to `--lib features::processes::stop::tests::the_identity_check_fails_closed_on_every_leg` (exe `/usr/local/bin/juggler-agent`, sighting `juggler-agent` → `Err(refusal::OWN)`) | PASS | 9ec9c16b5 | 2026-11-07 | Claude (self) |
| R3 the identity check against the sighting at click time and again before the first signal (pid, start time, program, boot); a mismatch sends zero signals with the reason | `cargo test -p juggler-app --lib features::processes::stop::tests::the_identity_check_fails_closed_on_every_leg` (start time, program, boot → `OTHER`; gone; unseen; zombie); `cargo test -p juggler-app --test observed_stop every_refusal_sends_zero_signals` (case 3: a sighting with another start time → `execute` refuses with `OTHER`, the process alive, nothing audited), `a_verified_leftover_of_an_ended_session_stops_and_is_audited` (`verify` → `verified`); code: `verify` and `execute` each gather `facts_now` and run `check`; the ladder's `members` closure re-reads the pid and drops it unless the start time still matches and it isn't a zombie | zero signals on any mismatch, with the reason | as asserted. The last re-read before the signal checks pid + start time (+ not a zombie); program and boot are checked by `check` in `execute` a moment earlier | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R4 the same verify sheet and TERM → grace → KILL ladder (this process only); the audit intent before any signal with `actor = user:observed` and the program's name in place of a command line | `cargo test -p juggler-app --test observed_stop a_verified_leftover_of_an_ended_session_stops_and_is_audited` (`actor == "user:observed"`, `cmdline_trunc == "sleep"` "the program's name, never the line", outcome "exited on TERM"); the KILL rung is the shared `terminate_group`: `cargo test -p juggler-supervisor --test supervisor ladder_escalates_to_kill_for_term_ignorer`; vitest `src/screens/processes-page.test.tsx` "another app's leftover is stopped alone: no scope to widen (D-41b)", "the stop sheet: one proof line, the process's own unsaved-state warning, and the report links to Cleanup history"; vitest `src/components/kill-sheet.test.tsx` "failed verification becomes the copy-exact explanation with the kill button REMOVED from the DOM"; Playwright `processes-gallery.spec.ts` "processes — another app's leftover: the D-41b sheet" → `_gallery/90-processes/stop-theirs-1680x1000-{dark,light}.png` (dark opened: "Stop astro? · Landing page · In a session ChatGPT runs", the proof line "same start time, program and boot · its session has ended", "Only this process", TERM → 5 s → force) | each clause evidenced | sheet, ladder, actor and name evidenced; vitest passes. **Missing:** "intent before any signal" on this path — `execute` writes `audit.intent(..)` before `terminate_group` and returns `AppError::AuditUnavailable` when it can't, but only the ledger path has a test for that (`kill_sheet.rs guardrail_no_audit_means_no_signal`); `observed_stop.rs` needs a test passing a failing `AuditWriter` to `stop::execute` on a verified leftover and asserting `Err(AuditUnavailable)` and the fixture still alive **Closed 2026-11-07:** `cargo test -p juggler-app --test observed_stop no_audit_record_means_no_signal_to_another_apps_leftover` (a failing `AuditWriter` → `Err(AuditUnavailable)`; the verified leftover still alive) | PASS | 9ec9c16b5 | 2026-11-07 | Claude (self) |
| R5 the collateral suite extended: with every rule on and the sweep run past every grace, another app's leftover lives; every refusal sends zero signals | `cargo test -p juggler-app --test observed_stop no_automatic_rule_ever_stops_another_apps_leftover`, `every_refusal_sends_zero_signals`; `crates/juggler-supervisor/tests/collateral.rs` unchanged (runs in CI) | leftover alive; zero signals; nothing audited | as asserted | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| Gate: `cargo test -p juggler-app --test observed_stop`, `--lib features::processes::stop`; the guardrail and collateral suites | vitest as above (run here); cargo (CI) | pass | vitest 25/25 pass; cargo not run here (CI) | PASS (vitest); cargo in CI | f6fb096cb | 2026-11-07 | Claude (self) |

## Notes

- Performance budget (the check ≤ 50 ms, targeted reads only): `facts_now`
  reads the sighting (memory, else one store row), the session's roots and
  `stop::read_now` (one `sysinfo` refresh of the given pids, exe only). Not
  timed.
- Visual testing asks for "its refusal" too: the shared sheet's refusal is
  covered by the kill-sheet vitest above, but `_gallery/90-processes/` has no
  capture of a refused D-41b sheet (only `stop-theirs-*`). A capture of the
  observed sheet refusing (e.g. session still running) is missing.

## Live evidence (2026-10-26, `docs/qa/processes-lab.md` step 7, [lab-1-2026-10-26.md](../AJ-E2406/lab-1-2026-10-26.md), bug 3)

- ~~FAIL: `verify` and `execute` refused Homebrew's `python3` :8766 with "This
  is part of an app. Quit the app instead." (it runs from inside `Python.app`)~~
  — **no longer true in code**: an `.app` inside a `.framework` that no app
  holds is not an app (`places::outer_bundle`), covered by
  `stop::tests::a_framework_bundled_interpreter_is_no_app`. Zero signals were
  sent then; the lab stopped its own leftover by hand. Not re-run live since.
- "This PID now belongs to a different program" said for a process that had
  simply exited: `check` now returns `refusal::GONE` ("It has already ended.")
  when the pid is gone (`the_identity_check_fails_closed_on_every_leg`, leg
  "gone").

## Binary artifacts

None added; cited: `_gallery/90-processes/stop-theirs-1680x1000-{dark,light}.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)

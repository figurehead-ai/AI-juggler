# Evidence manifest: AJ-E2322

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2322: Move says "opened" only when the conversation is open, and keeps a timeline of every phase.

**Branch:** `release/0.5.1`. Fixtures only (the move lab's temp folders, scripted processes: the target app's Claude Code resuming the conversation after the link, or not). No real profile or real `moves/` folder was read or written. The real Claude app's after-open evidence (its Code tab's session resuming the conversation, or the app adding to its history) is walked in AJ-E2323's PL1 lab walk.

## Requirements

| Requirement | Verdict | Proof | Commit | Date |
|---|---|---|---|---|
| R1 Verified only when the adapter's after-open check sees the conversation; otherwise pending, said plainly | PASS | After the link the move asks the adapter's writers hook for a session of this conversation in the target profile's tree (Claude Code's pid file or `--resume`), or sees the target app add to the conversation's history (lines sorted by the adapter); only then "Opened in <app>". A link delivered but not seen open is pending: "Sent to <app>, but AI Juggler couldn't see it open there. Open it from <app>", with Open in <app> (also for a link that couldn't be sent). Rust `a_link_delivered_but_not_seen_open_is_pending_with_open_offered` (failing before: it read "verified" on delivery), `a_session_of_it_running_in_the_target_is_opened`, `a_move_quits_both_profiles_…_opens_in_the_target`; vitest `move-opened.test.tsx` (Opened only when verified; the plain words and Open, which opens the moved conversation in that App; Open for a failed send) | 75c9c1f06, 71183c0c3 | 2026-11-23 |
| R2 Wait for the records' rewrite by event, capped at 15 s, instead of a fixed 6 s | PASS | The fixed `SETTLE` (6 s) is gone. `MoveSystem::wait_open`: the real system wakes on file events in the moved files' folders (`FolderWake`, a look every second in case one is lost), ends as soon as the conversation shows open, then waits for 300 ms without a change (the records' rewrite) before the checks after, all within 15 s. Rust `the_open_wait_wakes_on_a_file_event_not_a_fixed_settle` (woken in under a second), `the_open_wait_ends_at_its_cap_with_nothing_seen`, `once_open_the_wait_lets_the_records_rewrite_finish`, `the_checks_after_wait_for_the_open_not_a_fixed_settle` (no settle of 6 s; 15 looks at most) | 75c9c1f06 | 2026-11-23 |
| R3 A timeline per phase in the journal (ms), each wait's set (program, reason, evidence) and what remained; Copy details is the redacted summary | PASS | Every `phase` line carries `at_ms`; `Transaction::timeline` writes a `timeline` line per phase of the move (closing, still, backing_up, moving, checking_before, starting, opening, checking_after) with `took_ms`, the closing wait's set (each app's main process and each writer: program, pid, reason, evidence) and what of the trees remained, the files still changing (names only), and what showed it open. The Moves door's Copy details says each line in words, times from the move's start, the home folder as `~`. Rust `the_journal_keeps_a_timeline_of_every_phase_in_ms` (engine), `the_journal_keeps_each_phase_its_wait_set_and_what_remained`, `copy_details_says_the_timeline_in_words_without_the_home_folder` | 75c9c1f06 | 2026-11-23 |

## Usability answers (PAT-USE), re-checked on the built screen

1. Who: someone who just moved a conversation; "Opened" means they can see it in the target app now. Match: verified only by the target's own evidence (R1 tests).
2. "Opened in <app>" only when it is; while the move waits, the running line says "Opening the conversation in <app>…"; otherwise "Sent to <app>, but AI Juggler couldn't see it open there. Open it from <app>" with Open in <app>. Match.
3. The honest last step of the Move sheet (the Opened row of "What the move did"); the timeline sits in Copy details (Moves door), not on screen. Match.
4. Not confirmed open: said plainly, Open offered (the same for a link that couldn't be sent). Match.
5. OS prompts: none (Open uses the existing open path of the App). Match.
6. Watch-fors: "opened" while the app is still starting (the start is waited for, then the conversation's own evidence, up to 15 s) or showing another conversation (the evidence names this conversation: its id's session or its own history). Match.

## Checks

2026-11-23: `cargo test -p juggler-app --lib handover::` 137 passed; `cargo test -p juggler-handover --lib moving` 26 passed; clippy `-p juggler-handover -p juggler-app --all-targets -D warnings` clean; rustfmt clean on the changed files; vitest `move-opened`, `switch-move`, `moves-door`, `moves-backups` passed; `tsc -b`, `ui_lint.py`, `private_data_lint.py`, `terminology_lint.sh`, `plain_words_lint.py`, `platform_lint.py`, `size_budget.py`, `ipc_audit.py`, `debt_baseline.py` OK.

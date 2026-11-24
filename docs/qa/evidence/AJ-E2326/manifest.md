# Evidence manifest: AJ-E2326

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2326: A refused move leaves the conversation where it was, never in a terminal here.

**Branch:** `release/0.5.1`. Fixtures only (the move lab's temp folders and scripted processes, a temp Claude Code account in the integration test, the UI's preview seams). No real profile was read or written.

## Requirements

| Requirement | Verdict | Proof | Commit | Date |
|---|---|---|---|---|
| R1 Every start path refuses a held conversation (and in a settle window after a refused or undone move), by the shared hold | PASS | A move (and its Undo) marks its conversation in the shared move area (`places::ConversationHold`, beside AJ-E2320's profile markers, with the owner's pid and start): while it runs, and for 30 s (`SETTLE_MS`) once it ends refused or undone; a move that succeeded lets it go; one left part way keeps it until undone. The sessions' one start (`sessions::spawn::start_session`, which Continue in AI Juggler, Resume, a pickup, `continue_as` and an epoch's Resume all go through) refuses a held conversation in words. Rust `continue_is_refused_while_a_move_holds_the_conversation`, `continue_is_refused_in_the_settle_window_after_a_refused_move` (failing before: Continue launched), `the_conversation_is_held_while_its_move_runs`, `the_settle_window_ends_by_itself`, `a_done_move_holds_the_conversation_no_longer` | c0da5b980, 798d2c9dc | 2026-11-23 |
| R2 A refused or undone move restores the conversation where it was, the reopen checked | PASS | Refused after the apps were quit (while closing, or undone after the backup): the source profile starts again and the conversation is opened in it by the move's own resume link to its own process, then its own evidence that it is open there is waited for (AJ-E2322's `seen_open`, file-event woken, 15 s cap); the journal gets a `reopened` line with what showed it. Undo does the same (checked), and says plainly when it wasn't seen open. Refused before anything quit: nothing is sent (it is still open). Rust `a_refused_move_reopens_the_conversation_where_it_was_and_checks_it`, `a_move_refused_while_closing_reopens_it_in_the_source`, `a_move_refused_before_any_quit_sends_nothing`, `undo_reopens_it_where_it_was_and_checks_it` | c0da5b980 | 2026-11-23 |
| R3 The move panel never offers the terminal | PASS | Move has its own icon (`IconMove`, never Continue in AI Juggler's carry-in arrow) in the move screen's header, its choice card and its list. A conversation a move holds carries `resume_refusal.moving`, so Continue in AI Juggler isn't offered in the switch panel (and every resume door says why). On a conversation's page last open in a desktop app, "Open in <app>" is the primary and Continue in AI Juggler is secondary, saying "Starts it in a terminal here, on this account". vitest `move-not-terminal.test.tsx` (4) | 6251901d7 | 2026-11-23 |
| R4 Two live places on one conversation: "Open in two places" on both rows, one click to stop ours | NOT BUILT | Left for the next pass: needs R5's attribution first | — | — |
| R5 A desktop app's Code child's conversation shows under that App; never "nothing open" while a child holds one | NOT BUILT | Left: the live model's attribution through the ownership module (AJ-E2315) | — | — |
| R6 A continued session says where it came from; after a refused move the sheet's next steps point to Retry or the move | PARTIAL | The move sheet's next steps (AJ-E2319's classes: Retry, the holder's door) never name Continue in AI Juggler; the "Continued here from Claude app · <profile>" label is not built | — | — |

## Usability answers (PAT-USE), for R1–R3

1. Someone retrying a refused move: the conversation is back open in its app (R2), and no terminal session is offered or started meanwhile (R1, R3). Match.
2. After a refused or undone move the conversation is reopened in its app; Continue in AI Juggler is absent from the switch panel while the move holds it, and greyed with the reason elsewhere. Match for R1–R3; "Continued here from…" and "Open in two places" are not built (R4, R6).
3. No accidental terminal sessions on the wrong account: the backend refuses, not only the UI. Match.
4. n/a.
5. Reopening uses the move's own link to the profile's process (the same Automation entry as the move's). Match.
6. Watch-fors: the Move icon now differs from Continue's (test); Continue isn't in the move panel after a refusal (held); "nothing open" while a Code child holds the conversation is R5, not built.

## Checks

2026-11-23: `cargo test -p juggler-app --lib features::handover features::places` 264 passed (the fault suite's `both_profiles_quit_in_parallel` timing test read over its bound once under the full suite's load; alone it passes); `cargo test -p juggler-app --test conversation_move_hold` 2 passed; `export_bindings_resumerefusalinfo` regenerated; clippy `-p juggler-platform -p juggler-handover -p juggler-app --all-targets -D warnings` clean; vitest `move-not-terminal`, `outside-conversations`, `own-conversation-page`, `continue-on`, `switch-move`, `switch-move-stops`, `resume-refusal` passed; `tsc -b`, `ui_lint.py`, `private_data_lint.py`, `terminology_lint.sh`, `plain_words_lint.py`, `platform_lint.py`, `spawn_lint.py`, `size_budget.py` OK.

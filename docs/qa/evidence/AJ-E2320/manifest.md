# Evidence manifest: AJ-E2320

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2320: One shared, persistent hold marker every AI Juggler honours, and a move never quits AI Juggler's own parents.

**Branch:** `release/0.5.1`. Fixtures only: temp data folders, a made-up Mac, recorders in place of every start and quit. No real profile was read or written and nothing was quit.

## What changed

- Before, `places::MoveHold` lived in one process's memory (a `HashSet` in `Places::moving`), so the other AI Juggler (Dev beside the installed one) never saw it, and its keep alive started a profile again mid-move.
- Now `places/hold.rs` holds the **shared move area**: `~/Library/Application Support/AI Juggler Shared/moves/` for an edition's own data folder (`area_at`; a test's or lab's data folder gets `<data>/shared-moves` and never touches the Mac's). It has `locks/` (the canonical-root lock files, `Lock::take_in`, before in the temp folder), `journals/` (new moves' journals; the old `<data>/moves` is still read, so earlier moves can still be undone) and `held/<place key>`, one marker per profile: `{move_id, place, owner_pid, owner_start_micros}`.
- `MoveHold::take(state, ids, move_id)` writes the marker (`create_new`) before the move quits anything. The marker is removed on drop **only** when the journal reads Committed or rolled back (Undone, or never past Planned: nothing changed), or the move never began. A move left part way keeps the profile held. Only the same move id's Undo can take the marker over, once its owner is gone.
- `why_held` / `held_for_move` / `any_move` read the marker as well as the in-memory set. That covers keep alive (`tick_with`), Open, Restart, `start_place` (every start path), and the automation watch (`any_move`). A marker whose owner (pid and start) is gone reads `Held::Crashed`: held, with words that say the move didn't finish.

## Requirements

| Requirement | Verdict | Proof | Commit | Date |
|---|---|---|---|---|
| R1 shared move area (locks, journals, `held/<place-key>` naming move id and owner pid+start), written before the first quit, removed only on Committed/RolledBack | PASS | `places::hold::tests::a_hold_is_a_shared_marker_naming_the_move_and_its_owner`, `the_marker_goes_only_when_the_move_is_committed_or_undone`, `the_folder_locks_are_in_the_shared_area`, `a_tests_data_folder_never_shares_the_macs_move_area`; `handover::moving::tests::a_move_holds_both_profiles_for_every_ai_juggler_before_the_first_quit` (marker present at every scripted quit, gone after commit, journal in the shared area) | aff72d98c | 2026-11-23 |
| R2 keep alive, Open, Restart, starts and automation honour another AI Juggler's marker; a dead owner's hold is a crashed move | PASS | `places::hold::tests::another_ai_jugglers_hold_is_honoured_by_every_start_path` (keep alive starts nothing, Restart and Open refused, `start_place` refused, `any_move` true; free again once the marker goes), `a_crashed_moves_hold_keeps_the_profile_closed` | aff72d98c | 2026-11-23 |
| R3 refuse up front when the processes to quit include AI Juggler's own ancestry or the other AI Juggler's | PASS | Built under AJ-E2315 (00179c60a, `ownership::self_rule`, asked in `moving::quiet` for Move and Undo before anything quits). Re-run green on this tree: `ownership::tests::a_move_never_quits_the_ai_juggler_started_from_the_profile`, `a_move_never_quits_another_ai_juggler_started_from_the_profile`, `handover::moving::tests::a_move_that_would_quit_ai_juggler_refuses_before_anything_quits` | 00179c60a | 2026-11-23 |
| R4 keep alive's Restart kill path goes through `ownership::signal::quit_checked` (pid and start checked right before) | PASS | `places::restart` identifies the process it looked at (`Starter::identify`, its start checked against the look), then `quit_checked`. Test: `places::tests::a_restart_never_quits_a_pid_reused_since_it_looked` | 6025c0760 | 2026-11-23 |

## Failing on the old code

- R1/R2: with the marker ignored (`why_held_on_disk` → `None`, `any_move` in-memory only, as before), `a_crashed_moves_hold_keeps_the_profile_closed`, `another_ai_jugglers_hold_is_honoured_by_every_start_path` and `the_marker_goes_only_when_the_move_is_committed_or_undone` FAIL. The marker tests can't build against the old `MoveHold::take(state, ids) -> Option` either.
- R4: with the recheck removed (`quit_checked`'s listing pinned to the first look), `a_restart_never_quits_a_pid_reused_since_it_looked` FAILS (the reused pid was signalled).

## Checks

- `cargo test -p juggler-app --lib -- places:: handover::moving:: ownership::`: 202 passed.
- `cargo clippy --no-deps -p juggler-app -p juggler-handover --all-targets -D warnings`: clean for these files. The only errors are in another agent's uncommitted `moving_repair.rs` and `sign_ins/watch.rs`.
- `cargo fmt --check`: clean for these files.

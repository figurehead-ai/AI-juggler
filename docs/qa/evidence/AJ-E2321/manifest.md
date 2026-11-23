# Evidence manifest: AJ-E2321

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2321: a move survives a crash: startup recovery before keep alive, and a Moves door to undo or finish each move by id.

**Branch:** `release/0.5.1`. Fixtures only: the move lab's temp folders, crashes simulated by stopping a real move's journal mid-phase (backed up; mid-apply with the last step's done line never written; after the cold checks), hold markers naming a gone owner, scripted processes. No real profile was read or written and nothing was quit.

## What changed

- Engine (`juggler-handover`): `Transaction::abandon` (a move that changed nothing is undone without touching a path; its backups go), `retire` (a committed move's backups go), `backups_kept`, `journal`, and `room_needed` (preflight counts Undo's room: records written again, each path's entries twice, the margin).
- `handover/moving_recover.rs`: at start (`sweep_at_startup`, run before the Tauri builder and so before keep alive's first tick) every move left part way is claimed (skipped when another AI Juggler holds its lock or another move holds its profiles) and its hold markers written for every AI Juggler; then, in the background: Planned/BackedUp let go; Applying/Applied/Undoing undone by the journal when its crashed hold kept every app away since (no reopening); Verified checked once more and committed; anything else (an unclear step, a move an older AI Juggler left with its apps free since) stays held with a bell ("A move didn't finish", kind restarted, opens the conversation). `finish` by id; `retire` after a move commits and at start (next move of the files committed, or 30 days).
- `handover/moving_door.rs`: `MoveRecordInfo` gains title, from, to, can_undo, undo_why, can_finish, timeline. `switch_move_finish` command.
- UI: `components/switch/MovesDoor.tsx` (rows, Undo… sheet, Finish, Copy details), on the conversation's page (`SessionOverview`, its own moves) and in Settings › Desktop apps (`screens/settings/MovesSection.tsx`). No moves: nothing shown.

## Requirements

| Requirement | Verdict | Proof | Commit | Date |
|---|---|---|---|---|
| R1 Startup recovery before keep alive: Planned/BackedUp roll back; Applying rolls back by the journal; Verified finishes forward; unclear keeps the hold and says Finish or Undo (bell and the conversation's page) | PASS | `handover::moving::tests::recovering::a_move_a_crash_left_mid_apply_is_undone_at_start_before_keep_alive`, `a_move_a_crash_left_after_its_checks_is_finished_at_start`, `a_backed_up_move_a_crash_left_is_let_go_without_quitting_anything`, `a_move_an_older_ai_juggler_left_part_way_is_held_for_the_persons_finish_or_undo`, `a_move_another_ai_juggler_is_running_is_left_alone`; engine `a_backed_up_move_left_by_a_crash_is_let_go_without_touching_a_file`. Failing before: none of `recover`, `abandon` existed. Finish is offered only where it can work (D-127): a move stopped mid-apply says Undo (R4) | ef258253f | 2026-11-23 |
| R2 A Moves door on the conversation's page and in Settings › Desktop apps: Undo (committed), Finish/Undo (interrupted), by id; the timeline in Copy details | PASS | `recovering::the_door_finishes_an_interrupted_move_by_its_id`; `moves-door.test.tsx` (8 tests: nothing when none; title, from → to, date, state; Finish+Undo vs Undo only; Undo's sheet, confirm only; Finish; can't-undo reason; only the conversation's own; Copy details with id and timeline) | 45d82b55e | 2026-11-23 |
| R3 Preflight counts the room Undo needs; committed moves' backups dropped after the next move of the family commits, or after N days | PASS | engine `the_room_a_move_needs_counts_its_undo`, `a_committed_moves_backups_retire_and_its_undo_goes_with_them`; app `recovering::a_committed_moves_backups_retire_after_thirty_days_and_its_undo_goes` (N = 30, `KEEP_DAYS`); `execute` retires after each committed move | ef258253f | 2026-11-23 |
| R4 Rolling forward an interrupted Applying move comes later; the release notes say so | PASS | `docs/releases/0.5.1.md` Known issues | e6d616db0 | 2026-11-23 |

## Usability answers (PAT-USE), checked against the built door

1. Who and promise: a person whose move was interrupted or who wants to undo one: "Moves" lists each move with Undo…, and Finish where it can finish. Match.
2. Each row: "“title” · from → to", then "date · state" ("Moved", "Undone", "Interrupted: finish or undo" / "Interrupted: undo puts it back"); Undo's sheet says where it goes back to, what is kept aside and which Apps quit. Match.
3. One place: the conversation's page (its own moves) and Settings › Desktop apps (all); the move window's own Undo unchanged, nothing else added. Match.
4. No moves: no card, no section. An interrupted move is always listed (the door lists every move; recovery never settles one silently). Match.
5. OS prompts: none new; Undo quits and starts Apps through the existing path. Match.
6. Watch-fors: keep alive vs a half-moved folder (markers before keep alive; test), Undo of the wrong move (every action by `move_id`; test), a crash leaving no trace (bell + door; test). Match.

## Checks

- `cargo test -p juggler-handover --lib moving::` 28 passed; `cargo test -p juggler-app --lib handover::moving` 76 passed (7 new); `--lib command` 43 passed.
- vitest: `moves-door.test.tsx`, `switch-move-stops.test.tsx`, `moving-section.test.tsx`, `desktop-apps.test.tsx`, `session-chips.test.tsx`: 33 passed. `tsc -b --noEmit` clean; `ui_lint.py` OK.
- clippy `-p juggler-handover -p juggler-app --all-targets -D warnings`: clean for these files (the only findings are in another agent's uncommitted `moving_shape_tests.rs`). `private_data_lint.py`, `terminology_lint.sh` OK. `size_budget.py`: `moving.rs` down from 1586 to 1515 lines (it was over before this item).

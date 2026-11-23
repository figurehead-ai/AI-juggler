# Evidence manifest: AJ-E2325

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2325: move backups clean themselves up.

**Branch:** `release/0.5.1`. Fixtures only (the move lab's temp folders, scripted processes, crashes simulated mid-phase). No real profile or real `moves/` folder was read or written.

**Policy (the coordinator's ruling, 2026-11-23): retention is decided here only.** AJ-E2321 lets nothing go by itself (its `retire` primitive is called only by this policy and by Delete backup).

## Requirements

| Requirement | Verdict | Proof | Commit | Date |
|---|---|---|---|---|
| R1 A refused or nothing-changed attempt deletes its move folder at once | PASS | `handover::moving::tests::recovering::a_refused_attempts_folder_goes_at_once_and_an_undone_moves_record_stays` (undone before it changed a file, nothing kept aside: the folder goes; one undone after it changed files keeps its record); `execute` drops refused attempts right after a refusal, the sweep at start too | 92af8d8ea | 2026-11-23 |
| R2 Keep for 1 / 7 (default) / 30 days / until deleted; sweep at start after recovery and once a day; never while a move runs (hold, lock); never an interrupted move | PASS | `the_setting_reads_its_four_choices_seven_days_by_default`, `a_committed_moves_backups_go_after_the_chosen_days_never_before`, `the_sweep_never_runs_during_a_move_nor_touches_an_interrupted_one`, `a_second_move_of_the_same_family_leaves_the_first_moves_backups`; vitest `screens/settings/moves-section.test.tsx` (the four choices, 7 days default, a choice kept as `moves.keep_backups`). Sweep: `recover::startup_settle` after recovery, then every 24 h in the recovery thread | 92af8d8ea | 2026-11-23 |
| R3 Moves door: date, from → to, title, backup size, "Kept until <date>", Undo, Delete backup ("You can no longer undo this move"), Keep; the total; rich-mode sticky header and footer | PARTIAL | Done: date, from → to, title, "Backup kept until <date>" / "until you delete it", Undo…, Delete backup… whose sheet says "You can no longer undo this move." with Keep (cancel) — `delete_backup_by_its_id_ends_undo_and_never_takes_an_interrupted_ones`, `moves-door.test.tsx` (10 tests). Left: the backup's size and the total (they need R6's measured size), the rich-mode sticky header/footer | 92af8d8ea | 2026-11-23 |
| R4 Disk pressure: offer to clear backups older than a day, naming what it frees; never silent | NOT DONE | Nothing is cleared silently (no low-space path deletes anything); the offer itself is not built | — | — |
| R5 Backups count in the header's "N to clean up" with their size | NOT DONE | — | — | — |
| R6 Sizes are what deleting frees (APFS clones measured, behind `juggler_platform`) | NOT DONE | — | — | — |
| R7 The first 0.5.1 sweep keeps 0.4.9/0.5.0 backups until 7 days after they were made; refused-attempt folders go at once | PASS | `backups_made_before_the_first_sweep_keep_seven_days_and_until_deleted_keeps_them` (a backup older than the first sweep keeps 7 days even under "1 day"); `kept_until` in `the_setting_reads_…`; refused attempts as R1 | 92af8d8ea | 2026-11-23 |

## Usability answers (PAT-USE), built so far

1. Who: anyone who moved conversations: backups kept for Undo a while, then cleaned up. Match (R2).
2. Each backup says "Backup kept until <date>"; Delete backup says "You can no longer undo this move"; the setting names its choices in days. Partial: no size yet (R6).
3. In the Moves door (conversation's page, Settings › Desktop apps). Partial: not yet in the header's "N to clean up" (R5).
4. No backups: no "Backup kept" line and no Delete backup; interrupted moves never deleted automatically (test). Match.
5. OS prompts: none. Match.
6. Watch-fors: a backup deleted while its Undo shows (Undo goes with it, test); a clean-up during a move (never, test). The size watch-for waits for R6.

## Checks

`cargo test -p juggler-app --lib -- handover::moving places:: command_types` 226 passed; clippy `-p juggler-handover -p juggler-app --all-targets -D warnings` clean; rustfmt check clean; `size_budget.py` no longer lists `moving.rs` or `places/mod.rs`; `ipc_audit.py` clean for the move commands; vitest 23 passed (door, settings, stops, checking); `tsc -b`, `ui_lint.py` OK.

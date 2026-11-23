# Evidence manifest: AJ-E2319

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2319: Move's refusals are typed (transient, needs you, setup, bug), name what holds the files and carry the move id; Retry only for transient.

**Branch:** `release/0.5.1`. Fixtures only: the move lab's temp folders, scripted processes, recorders in place of every start and quit. No real profile was read or written and nothing was quit.

## What changed

- `AppError::Move { words, detail, unchanged }` is now `AppError::Move(Box<MoveStop>)`. `MoveStop { class, holders, move_id, unchanged, words, detail }` lives in `crates/juggler-app/src/error.rs`, with `StopClass` (`transient`, `needs_you`, `setup`, `bug`) and `MoveHolder { name, place_id }`. The error wire carries `stop: MoveStopInfo` (generated TS: `MoveStopInfo.ts`, `StopClass.ts`, `MoveHolder.ts`). `retryable` is true only for a transient stop that changed nothing.
- Each failure has its class. **Transient:** a writer still finishing, files still changing, the family changed while the apps closed, another move of these profiles running, or a process that was starting or quitting just then. **Needs you:** an app that won't quit (it may be asking something), a program outside the profiles writing the conversation, a session working or a terminal on it (refused by the offer), a profile started by hand mid-move, or a crashed move's hold. **Setup:** a tool or conversation the move can't plan, or anything the offer says the person can't clear. **Bug:** a backup, apply or check that failed, an Undo that didn't finish, or a quit the system refused. An untyped error at the edge is typed: a refusal gets the class given, anything else is a bug.
- A transient stop that changed nothing is tried once more by itself (`run_once_more`: "Trying once more…", a 3 s pause). A needs-you stop is never tried again by itself.
- Move window: **Retry** (runs the move again) only for transient. **Check again** (reads the offer, quits nothing) for needs you, plus "Holding the files: …" and a **Go to <app>** door for each app holding them. **Copy instead** for setup (Move is hidden and Copy becomes the switch). A bug shows no Retry; Copy details and Report a bug… stay. A move left part way offers **Undo the move** of the error's own `move_id`. `switchMoves().find(interrupted)` is gone.
- Undo returns `MoveUndoneInfo.notes`, the same "still running" notes the move gives, and the window shows them.

## Requirements

| Requirement | Verdict | Proof | Commit | Date |
|---|---|---|---|---|
| R1 `MoveStop { class, holders, move_id, unchanged, words, detail }` replaces `AppError::Move` | PASS | `handover::moving::tests::stops::retry_shows_only_for_a_transient_stop_that_changed_nothing`, `untyped_errors_are_typed_at_the_edge`; the move suite (`handover::moving`, 76 tests) green on the new type | uncommitted (see note) | 2026-11-23 |
| R2 Transient: Retry, auto-retried once. Needs you: no Retry, Check again re-reads the offer without quitting, holder named with a door. Setup: Move hidden, Copy offered. Bug: Copy details and Report | PASS | Rust: `stops::files_still_changing_are_transient_named_and_tried_once_more` (two moves, both undone, "Trying once more…"), `an_app_asking_something_needs_you_and_is_never_retried` (quit once, not twice), `what_the_offer_refuses_needs_you`, `a_profile_started_mid_move_needs_you_and_is_named`, `another_move_running_is_transient`. UI: `switch-move-stops.test.tsx` "transient/needs_you/setup/bug: its words, what holds the files, and only its buttons", "needs you: Check again reads the offer again and quits nothing; the door goes to the app", "setup: Copy instead hides Move and the switch is the copy"; `switch-move.test.tsx` "Retry runs the move again…" | uncommitted | 2026-11-23 |
| R3 Undo and Finish target the error's move id; never `find(interrupted)` | PASS | `switch-move-stops.test.tsx` "offers Undo of the very move that stopped part way, by its id": another conversation's interrupted `1-aa` is listed, and Undo goes to `2-bb` from the error. Rust: `stop.move_id` equals the journal's move (`a_profile_started_mid_move_needs_you_and_is_named`, `files_still_changing…`). Finish has no button in the Move window yet; AJ-E2321's Moves door adds it, by id | uncommitted | 2026-11-23 |
| R4 a table test from each failure to its class and Retry's visibility, Rust and UI | PASS | Rust: `retry_shows_only_for_a_transient_stop_that_changed_nothing` (class × unchanged → retryable, code, move id) and the scripted failures above. UI: the `it.each` class table in `switch-move-stops.test.tsx` | uncommitted | 2026-11-23 |
| R5 Undo keeps the move's "still running" notes (`MoveUndoneInfo` carries them) | PASS | `stops::undo_keeps_what_kept_running`; UI: `switch-undone-notes` shown after Undo (the R3 test) | uncommitted | 2026-11-23 |

## Failing on the old code

- Rust: the stop tests can't build against the old `AppError::Move { .. }` (no class, holders or move id on the wire). On HEAD `wire.retryable` was `unchanged`, so the needs-you rows of the table (`an app asking something`) would read `retryable = true`.
- UI: on HEAD, `nothingMoved` showed Retry for every `move_refused`, so the needs-you, setup and bug rows of the class table fail (Retry shows; no Check again, door or Copy instead). The Undo-by-id test fails too: HEAD's `find(interrupted)` picks `1-aa`.

## Usability answers on the built window (PAT-USE)

1. Someone whose move didn't go through: the window says why in the stop's own words, and the heading says whether anything changed ("Nothing was moved" or "The switch didn't complete").
2. Every refusal names what holds the files ("Holding the files: Claude app · Work") and offers one next step: Retry only for transient; Check again plus "Go to …" for needs you; Copy instead for setup; Copy details and Report for a bug.
3. The reason is in the Move sheet itself (the switch dialog's failed frame), and "Try again in a moment" now appears only for transient stops.
4. n/a: this is the unavailable state.
5. None: no new surface; it reuses `FailedStep`, `MoveUndo` and `Button`, with the same footer.
6. Watched for: Retry never shows for a stop that won't change (`an_app_asking_something…`: quit once), and Undo acts on the error's move id, not another conversation's.

## Checks

- `cargo test -p juggler-app --lib -- handover::moving error usage_stats switch::moves`: 102 passed.
- `cargo clippy -p juggler-app --all-targets -- -D warnings`: clean. `rustfmt` on the changed files.
- `vitest run switch-move.test.tsx switch-move-stops.test.tsx switch-flow.test.tsx`: 69 passed. `tsc -b --noEmit`: clean. `ui_lint.py`: nothing in these files.
- `private_data_lint.py`, `terminology_lint.sh`, `plain_words_lint.py`, `design_tokens_lint.py`, `native_controls_lint.py`: OK.

**Note:** the session's permission classifier refused the commit (`git commit -- <paths>`), so the change is in the tree, uncommitted. It touches only these paths: `error.rs`, `error_words.rs`, `usage_stats.rs`, `handover/moving.rs`, `moving_quit.rs`, `moving_repair.rs` (only its three `AppError::Move` constructions), `moving_tests.rs`, `moving_stop_tests.rs`, `ui/src/components/switch/{MoveChoice,MoveDone,MoveRunning}.tsx`, `ui/src/lib/ipc/switch-move.ts`, `switch-move.test.tsx`, `switch-move-stops.test.tsx`, `ui/src/types/generated/{MoveUndoneInfo,MoveStopInfo,StopClass,MoveHolder}.ts`.

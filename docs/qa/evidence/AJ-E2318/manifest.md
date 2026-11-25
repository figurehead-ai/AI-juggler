# Evidence manifest: AJ-E2318

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2318: Move waits only on real writers and the profiles' main processes, decides before it disrupts, and quits in parallel.

**Branch:** `release/0.5.1`. Fixtures only: the move lab's temp folders, scripted and `procs::fixture` process tables, recorders in place of every start and quit. No real profile was read or written and nothing was quit.

**Commits:** `dde303e4d` (the adapter writers hook: `InPlaceMove::writers`, Claude's by its pid files and `--resume`), `7e2bdb424` (the wait set, decide before quitting, the parallel quit by `ProcId`), and `3f7bb2a30` (AJ-E2323: the D1 shapes over the real ownership rule, and the offer's 500 ms timing test).

## Requirements

| Requirement | Verdict | Proof | Commit | Date |
|---|---|---|---|---|
| R1 an adapter `writers(tool_dir, conversations, table)` hook in `InPlaceMove`: Claude's `sessions/<pid>.json` (pid and procStart) and `--resume=<id>`; run for both accounts' folders | PASS | `claude_move::writers::tests::a_session_pid_file_names_a_writer`, `a_stale_pid_file_is_no_writer`, `a_resume_names_a_writer_of_its_conversation_only` (3 passed); asked of both accounts' folders by `moving_quit::writers_now` (`handover::moving::tests::wait::a_session_pid_file_blocks`, `a_stale_pid_file_never_blocks`, `a_resume_writer_blocks`; `shapes::d1_11_a_stale_pid_file_is_no_writer`, `d1_12_a_headless_resume_outside_the_apps_needs_you`) | dde303e4d, 7e2bdb424 | 2026-11-23 |
| R2 the wait set is each instance's main process plus the writers; the rest of the old tree is reported, never waited on | PASS | `wait::a_dev_server_under_the_session_is_reported_not_waited_on`, `crashpad_and_helpers_never_block`, `another_profiles_wrapper_never_blocks`; over the real ownership rule: `shapes::d1_01_the_live_trace_waits_on_main_and_the_session_only` (only the mains are asked; node and 9 helpers reported; a session that doesn't end is the one named), `d1_03_a_nohup_dev_server_is_reported_not_waited_on`, `d1_04_crashpad_is_reported_never_waited_on` | 7e2bdb424, 3f7bb2a30 | 2026-11-23 |
| R3 the offer runs ownership and writers before anything quits (≤ 500 ms) and refuses up front what it can know | PASS | `wait::the_offer_refuses_an_outside_writer_before_any_quit` (nothing quit, started or journaled); `shapes::d1_05…`/`d1_06…` (the self rule before any quit), `d1_12…`, `d1_13…`; timing: `shapes::the_offers_checks_stay_within_500_ms_on_a_large_table`, 3,114 processes, 6 running profiles, 400 pid files, one process table read per offer: 107 ms (debug build) | 7e2bdb424, 3f7bb2a30 | 2026-11-23 |
| R4 both profiles quit in parallel, with per-class deadlines | PASS | `wait::both_profiles_quit_in_parallel` (the second asked while the first still ran); `quit_wait` 30 s for the apps, `writer_wait` 10 s for their sessions (`moving.rs` `QUIT_WAIT`/`WRITER_WAIT`); every signal by `ProcId`: `ownership::signal::tests::the_same_process_is_asked_to_quit`, `a_reused_or_gone_pid_is_never_signalled`, `wait::a_pid_reused_between_read_and_signal_is_never_signalled`, `shapes::d1_10_a_reused_pid_is_never_waited_on_or_signalled`; the start/move race: `wait::a_start_racing_a_move_sees_its_hold` | 7e2bdb424 | 2026-11-23 |

## Usability answers (PAT-USE)

1. Someone moving a conversation to another Account: Move asks only the profiles' main processes to quit (`d1_01`: quits are exactly the two mains).
2. The offer names a blocker before anything quits, in plain words: "claude (process 730) has this conversation open outside the apps the move closes (in a terminal, say); Move becomes available once you end it" (`the_offer_refuses_an_outside_writer_before_any_quit`).
3. Fewer stuck moves: a dev server, helpers, crashpad or another profile's wrapper never refuse a move (R2's tests). The words stay in the Move sheet's offer blocks and the move's notes; no new surface.
4. When it can't move, nothing quits and the block says why and what to do (R3's tests: no quit, no start, no journal).
5. No new OS prompt: quitting an app is the Automation event already in the registry.
6. Watched for: both apps quit and then refused (now refused before: `the_offer_refuses…`, `d1_12`, `d1_13`); a background process the user cares about killed (never signalled: only mains, each by its `ProcId`).

## Checks

- `cargo test -p juggler-app --lib -- handover::moving`: 90 passed (includes the 14 `shapes` tests). `-- ownership::signal`: 2 passed. `cargo test -p juggler-adapters --lib -- claude_move::writers`: 3 passed.
- `cargo clippy -p juggler-app --all-targets -- -D warnings`: clean.

# Evidence manifest — AJ-E3505 The feed: one snapshot from the backend

`FloorSnapshot` and its parts are in `crates/juggler-floor/src/snapshot.rs`
(TS types generated into `ui/src/types/generated/`). The backend side is
`crates/juggler-app/src/features/floor/`: `feed::gather` reads the sources,
`feed::assemble` builds the snapshot purely (tests in `floor/tests.rs`),
`mod.rs` keeps it current from the app's events, `commands.rs` serves
`floor_snapshot`. The state mapping is `juggler-floor/src/state.rs`, held
to the rail by `ui/src/lib/fixtures/session-states.json`.

Not here yet: the task's evidence proof (a trace of one session going
working → waiting → working, Floor and rail side by side) has not been
recorded. The "change on screen ≤ 100 ms" budget is measured in
`docs/qa/floor/budgets-2026-11-06.md` ("A data change … worst 14.2 ms of
10", release build, replica company of 9 Apps and 340 sessions).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 `features/floor` builds a `FloorSnapshot` from `AppState`: Apps with colour slots, tools and Accounts; sessions with status, needs, subagents, tasks, project; other apps' conversations; pools; each Account's usage left and reset; the process table; This Mac's health and alerts | `cargo test -p juggler-app --lib features::floor::tests::sessions_sit_in_their_apps_live_first_then_by_start features::floor::tests::an_account_shows_its_tightest_folder_and_its_last_use`; code: `feed::gather` with `read_entities`, `read_usage`, `read_work` (tasks, pools' placements), `read_machine` (process graph CPU, `state.notifier.this_mac()` load and alerts) in `crates/juggler-app/src/features/floor/feed.rs` | every source read; the snapshot carries each part | the first test asserts an App's name, family, colour slot 3 and Account initials "SP", sessions ordered live-first with `Asks you` from `waiting_user`, 2 tasks, an ended one last, and another app's conversation as "Done, review it" with 2 subagents; the second asserts the Account's tightest usage (14 % left, its reset, out) and last use. Pools, the machine and the process table are read by `gather` (code only: `gather` needs a running app and has no test); the process table enters as per-session and per-App `cpu_pct`, not as a table | PASS | 0b2231ce7 | 2026-11-06 | Claude (self) |
| R2 rebuilds on the existing events, coalesced to at most 4 a second; sent to the render thread and as `floor://changed`; a `floor_snapshot` command | code: `SOURCES` (`session://updated`, `session://quota`, `tasks://changed`, `notifications://changed`, `conversations://changed`, `runtime://changed`, `places-changed` for the Apps) and `SLOW_SOURCES` (`machine://this-mac`, `processes://changed`, at most every 30 s) with `start()`'s 250 ms `ticks::every_while` pass draining one stale flag, then `Floor::publish` (render thread's subscribers) and `app.emit(EVENT_CHANGED)` in `crates/juggler-app/src/features/floor/mod.rs`; `floor_snapshot` in `commands.rs`; `floorSnapshot`/`onFloorChanged` in `ui/src/lib/ipc/floor.ts`; `cargo test -p juggler-app --lib features::floor::tests::a_reading_that_wiggles_between_drawn_steps_is_not_news` | ≤ 4 rebuilds a second; both consumers fed | a burst of events sets one flag that a 250 ms pass clears, so at most 4 rebuilds a second; a rebuild is published and emitted only when `differs` finds news (the test: a wiggling reading is not news, a state change is). The coalescing itself is shown by the code, not by a test | PASS | 11027899a | 2026-11-06 | Claude (self) |
| R3 new inputs, each with a test: an App's last active time, an Account's last use, a per-App CPU share, how long the overload alert has been on | `cargo test -p juggler-app --lib features::floor::tests` (`an_app_is_hot_only_after_two_minutes_over_half_the_mac`, `an_account_shows_its_tightest_folder_and_its_last_use`, `an_app_is_last_active_at_its_newest_session_or_when_last_seen`, `the_mac_shows_its_oldest_overload_alert_and_since_when`); code: `last_active_ms` in `feed::assemble`, `feed::machine_of` (This Mac's reading, out of `read_machine`) | a test for each of the four | 10 passed. CPU share: 450 % on 8 cores is hot only after `HOT_AFTER_MS`, cleared under half. Account last use: `NOW - 20`. App last active: the newest session's last move (`NOW - 40`) over last seen; last seen (`NOW - 1000`) with no sessions; nothing (not 1970) when never seen. Alert: the one on longest wins with its `since_ms` (`NOW - 90` over `NOW - 5`); none gives none; load clamped to the whole Mac | PASS | eec4b9f88 | 2026-11-06 | Claude (self) |
| R4 the session → state mapping matches the rail; one fixture read by Rust and the rail's tests | `cargo test -p juggler-floor state::tests::the_floor_and_the_rail_read_one_session_the_same_way`; `pnpm -C ui exec vitest run src/lib/session-states.test.ts`; fixture `ui/src/lib/fixtures/session-states.json` | both languages agree on every case | the fixture holds 19 own and 8 other-app cases; Rust asserts (rail state, word, Floor state) per case and at least 15/6 cases; the vitest runs `sessionItem`/`conversationItem` from `rail-items.ts` on the same file and asserts (rail state, word) | PASS | b1cb398ab | 2026-11-06 | Claude (self) |
| Gate: the shared fixture passes in both languages | the two commands in R4 | pass | Rust: 1 passed; vitest `src/lib/session-states.test.ts`: 27 passed | PASS | eec4b9f88 | 2026-11-06 | Claude (self) |
| Gate: `just structure` (IPC audit) green for the new command and event | `python3 scripts/ipc_audit.py`; `scripts/ipc_audit.allow` line `unbounded floor_snapshot  # bounded: one snapshot of the working set…` | audit green | `ipc-audit: OK (13 uncalled, 0 sync, 0 main-thread, 211 unbounded, 0 long, 17 name allowlisted)`; `check_layering.py`: OK; `size_budget.py`: OK | PASS | eec4b9f88 | 2026-11-06 | Claude (self) |
| Gate: no polling added | `python3 scripts/polling_audit.py` | no unexplained wait in the Floor's code | the Floor's waits all carry a `poll:` note: the feed's 250 ms timer (`mod.rs::start`) and the close's three 20 ms frames (`juggler-floor-render/src/lib.rs`, noted in b2ba4a8b2). The audit's 2 remaining unexplained waits are in `features/switch/execute.rs` (624, 641), not the Floor's; reported to the release session | PASS | b2ba4a8b2 | 2026-11-06 | Claude (self) |
| Budget: a snapshot of 20 Apps and 150 sessions ≤ 2 ms | `cargo test --release -p juggler-app --lib features::floor::tests::building_twenty_apps_and_a_hundred_and_fifty_sessions_is_quick` | ≤ 2 ms release (20 ms debug) | the test asserts 150 seated sessions and `took ≤ 2 000 µs × scale`; not re-run for this manifest | UNPROVEN | 0b2231ce7 | 2026-11-06 | Claude (self) |
| Budget: a change on screen ≤ 100 ms | `docs/qa/floor/budgets-2026-11-06.md` (release build, `scripts/floor_harness.py`) | ≤ 100 ms | "worst 14.2 ms of 10" | PASS | fd2637851 | 2026-11-06 | Claude (self) |
| Evidence: the working → waiting → working trace, Floor and rail side by side | — | recorded | not recorded | UNPROVEN | — | 2026-11-06 | Claude (self) |

## Binary artifacts

None yet. Binary evidence is content-addressed: `<claim>-<sha8>.png|log|json`.
Re-running a check ADDS a new file; cited artifacts are never
overwritten.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)

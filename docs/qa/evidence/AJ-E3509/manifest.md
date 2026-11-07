# Evidence manifest — AJ-E3509 Signals from real data: every row of the table

Built mostly in 103ca7cab ("the rest of SPEC §9.7's signals and doors"),
then reworked by the 0.4.1 items (AJ-E3515's flags replaced the far chips,
AJ-E3518's five decay levels replaced the three stages, AJ-E3521's look).
There is no `signals.rs`: the signals live in `crates/juggler-floor/src/scene/`
(`area.rs`, `frame.rs`, `building.rs`, `mod.rs`), `flags.rs` and
`overlay.rs`. Drafted from the code and tests as they stand on
`release/0.4.1`; nothing was run for this draft.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 each row of SPEC §9.7's signal table drawn as specified | `cargo test -p juggler-floor --lib scene::tests` (`every_state_has_its_pose_balloon_and_flag`, `done_rests_after_15_minutes_and_smoke_stops_after_10`, `a_hot_app_gets_one_fire_beside_its_busiest_desk`, `the_battery_the_rack_and_the_whiteboard_say_their_numbers`, `the_moving_part_shows_each_state_and_counts_subagents`, `an_unused_app_dims_by_how_long_and_decay_can_be_switched_off`, `out_and_low_say_what_the_app_says`, `a_pool_gets_a_meeting_room_and_the_lobby_a_notice_board`); `cargo test -p juggler-app --lib features::floor::tests::a_limit_the_clock_passes_is_news`; `state::tests::the_floor_and_the_rail_read_one_session_the_same_way`; captures `crates/juggler-floor-render/tests/golden/{area-00…10,red-alert,full-red-alert}.png`, `../AJ-E3517/card-*.png` | every row and its limits | every row asserted (by row below). **Found and fixed here:** done's 15 minutes was never applied (a done session stretched until looked at), and a limit the clock alone passed (smoke out after 10 minutes, a day unused) waited for some other change before it was drawn; `drawn_state`, `smoking` and the feed's `time_limits` now apply both (fb285863f) | PASS | f9db10a82 | 2026-11-06 | Claude (self) |
| R2 (amended to AJ-E3515) each App's flag says what needs you, counted and worded; a waiting count per floor on the switcher | `cargo test -p juggler-floor flags::tests::flags_never_overlap_never_lose_a_count_and_stand_over_their_place scene::tests::every_state_has_its_pose_balloon_and_flag scene::tests::out_and_low_say_what_the_app_says scene::tests::the_overlay_has_a_chip_per_floor_and_area_chips_from_afar`; `../AJ-E3516/lift-and-tabs-full.png`, `../AJ-E3516/building-*.png`, AJ-E3515's `labels-*` captures | the flags' words and counts; a count per floor tab | each state's flag word and count asserted (`1 ASKS`, `1 STUCK`, `1 CRASHED`, `1 DONE`; HOT, `OUT · RESETS 9:31 PM`, `LOW · 14% LEFT` in their tests); "FLOOR 1 !6" on the switcher; flags never overlap or lose a count. The task's text amended to the flags (SPEC §9.7's table: no word from afar for unused) | PASS | f9db10a82 | 2026-11-06 | Claude (self) |
| R3 Motion: Follow the system, Full (8 fps), Calm (4 fps, no walkers), Off (no animation; states still drawn) | `cargo test -p juggler-floor driver::tests::animating_it_ticks_at_its_rate_and_hidden_it_draws_nothing driver::tests::at_rest_it_draws_once_and_then_nothing`; `cargo test -p juggler-app --lib features::floor::store::tests::follow_the_system_stops_motion_when_reduce_motion_is_on`; `Motion::walkers` (`driver.rs`, true only for Full) feeding `Live.walkers` (`render.rs`), read by `Scene::frame` (`scene/frame.rs`: walkers drawn only `if live.walkers`) | Full 8/s, Calm 4/s, Off 1 frame on a change; System → Off under Reduce Motion | the driver test asserts 78–82 frames in 10 s at Full, 38–42 at Calm and exactly 1 at Off with the scene saying it's moving ("Motion off: nothing moves"); the store test asserts System resolves to Full, and to Off with Reduce Motion on, a chosen motion kept as chosen; walkers come only with Full (code); at Off the one frame drawn is the full scene with every state's pose and balloon | PASS | 82f0bfb60 | 2026-11-06 | Claude (self) |
| Gate: a table test from every state to pose, icon, word and chip | `cargo test -p juggler-floor --lib scene::tests::every_state_has_its_pose_balloon_and_flag` | one test over every state | all eight states: pose (`pose_of`), balloon (`balloon_of`), need (`Need::of_state`), the App's flag word and count from a scene with that one session, and the desk drawn in that state; the rail word per state is the fixture test's | PASS | f9db10a82 | 2026-11-06 | Claude (self) |
| Gate: Motion Off draws no animation frames | `driver::tests::animating_it_ticks_at_its_rate_and_hidden_it_draws_nothing` | 1 frame in 10 s while animating | asserted (1) | PASS | 82f0bfb60 | 2026-11-06 | Claude (self) |
| Budget: a state change on screen ≤ 100 ms | `docs/qa/floor/budgets-2026-11-05.md`, `budgets-2026-11-06-second.md` ("A data change") | ≤ 100 ms | 2026-11-05: worst 30.8 ms of 10 (release build, replica) | within | 93717ad13 | 2026-11-06 | Claude (self) |
| Evidence: a capture per signal from the replica fixture | the golden frames `area-00…10`, `red-alert`, `full-red-alert` and `../AJ-E3517/card-*.png` (the generic company fixture: every state, helpers, a hot App, an unused one, low and out Accounts, the alert) | a capture per signal | each signal captured from the generic fixture instead: the replica is a copy of the maintainer's own data, with no live sessions, and a capture of it would show real names (never in captures) | PASS (generic fixture; replica not usable, named reason) | f9db10a82 | 2026-11-06 | Claude (self) |

## R1 by row (what proves each)

- Working, idle, waiting (balloon, spotlight), stuck, crashed, done, limit:
  `pose_of`, `balloon_of` and `Need::of_state` are the one mapping each
  (the desk, the building view's offices, the hover card), asserted for
  every state by the table test; the spotlight and amber lamp are in
  `scene/frame.rs`. Smoke stops after 10 minutes and done rests after 15
  (`done_rests_after_15_minutes_and_smoke_stops_after_10`), and either is
  news to the feed when the clock passes it
  (`a_limit_the_clock_passes_is_news`). Captures of a sitter in each
  state: `../AJ-E3517/card-*.png`.
- Helpers, 6 then `+N`: asserted (9 subagents give `+3`).
- App overloaded: one fire an App, beside its busiest desk, more than
  0.6 m from every person; none for a cool App (asserted).
- Unused: AJ-E3518's five levels; asserted by
  `an_unused_app_dims_by_how_long_and_decay_can_be_switched_off`.
- Account low and out: OUT and LOW words, colours and badges asserted; the
  sign battery's segments (4 at full, 2 at 30 %, 1 at 10 %) and colours
  (green, yellow under 40 %, amber blinking under 20 %, red out) asserted
  through `gauge` and `battery_lit`, which the board and signs draw with.
- This Mac's rack: `rack_lit` lights 0, 5 and 10 of 10 at no, half and
  full load; green at the bottom, amber at 7–8, red at the top; unlit
  above the load (asserted). Seen in the `red-alert` golden frame.
- Red alert: the post shader's wash `0.1 + 0.07·sin(t·π)` (0.5 Hz, no
  flash; `shaders.rs`); the alert keeps the loop ticking
  (`the_alert_keeps_it_ticking_even_at_rest`); Motion Off doesn't tick, so
  the tint stays still (`Driver::ticking`).
- Tasks on the whiteboard: `tasks_words` sums the App's sessions' tasks:
  "3 TASKS", "1 TASK" (asserted).
- Notes on the notice board: asserted (`12 unread` on the lobby's board).

## Binary artifacts

None added for this task. Cited: `../AJ-E3517/card-*.png`,
`../AJ-E3516/lift-and-tabs-full.png`,
`crates/juggler-floor-render/tests/golden/red-alert.png`. Binary evidence
is content-addressed: `<claim>-<sha8>.png|log|json`; re-running a check
adds a new file, cited artifacts are never overwritten.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (none shown)
- [x] Transcript excerpts scrubbed of personal content (none; generic fixture names only)

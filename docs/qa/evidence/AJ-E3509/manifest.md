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
| R1 each row of SPEC §9.7's signal table drawn as specified | `cargo test -p juggler-floor scene::tests::the_moving_part_shows_each_state_and_counts_subagents scene::tests::an_unused_app_dims_by_how_long_and_decay_can_be_switched_off scene::tests::out_and_low_say_what_the_app_says scene::tests::a_pool_gets_a_meeting_room_and_the_lobby_a_notice_board state::tests::the_floor_and_the_rail_read_one_session_the_same_way`; code in `scene/frame.rs` (pose per state), `scene/area.rs` (smoke, fire, whiteboard), `scene/building.rs` (rack, notice board); card captures `../AJ-E3517/card-*.png` | every row, its limits, and a capture per signal from the replica | proof for part of the rows only (see "R1 by row" below). Missing: the quality gate's single table test from every state to pose, icon, word and chip (none exists); Done's 15-minute limit (not found in the Floor's code or any test); the sign battery's yellow-under-40% / blinking-under-20% has no test; the 10-minute smoke limit and one-fire-per-App are code only; no capture per signal from the replica fixture (the replica has no live sessions; AJ-E3516's walk says so) | UNPROVEN | 103ca7cab | 2026-11-06 | Claude (self) |
| R2 far chips `!N ?N XN`, `12%`, `OUT`, `HOT`, `UNUSED`; a waiting count per floor on the switcher | `cargo test -p juggler-floor flags::tests::flags_never_overlap_never_lose_a_count_and_stand_over_their_place scene::tests::out_and_low_say_what_the_app_says scene::tests::the_overlay_has_a_chip_per_floor_and_area_chips_from_afar`; `Need::word` in `flags.rs`; `switcher` in `overlay.rs`; `../AJ-E3516/lift-and-tabs-full.png` | the chips with those words; a count per floor tab | the switcher's per-floor count is drawn (`switcher`, "FLOOR 1 !6" in `lift-and-tabs-full.png`, AJ-E3516 R2). The far chips were replaced by AJ-E3515's flags with different words ("2 ASKING", "STUCK", "CRASHED", "HOT", "OUT · RESETS …", "LOW · 12% LEFT"), as SPEC §9.7's table now says; there is no `UNUSED` word from afar (the SPEC row says "—"). This task's R2 text was not amended to match and no D-number covers the change | UNPROVEN | c0ecc4f99 | 2026-11-06 | Claude (self) |
| R3 Motion: Follow the system, Full (8 fps), Calm (4 fps, no walkers), Off (no animation; states still drawn) | `cargo test -p juggler-floor driver::tests::animating_it_ticks_at_its_rate_and_hidden_it_draws_nothing driver::tests::at_rest_it_draws_once_and_then_nothing`; `cargo test -p juggler-app --lib features::floor::store::tests::follow_the_system_stops_motion_when_reduce_motion_is_on`; `Motion::walkers` (`driver.rs`, true only for Full) feeding `Live.walkers` (`render.rs`), read by `Scene::frame` (`scene/frame.rs`: walkers drawn only `if live.walkers`) | Full 8/s, Calm 4/s, Off 1 frame on a change; System → Off under Reduce Motion | the driver test asserts 78–82 frames in 10 s at Full, 38–42 at Calm and exactly 1 at Off with the scene saying it's moving ("Motion off: nothing moves"); the store test asserts System resolves to Full, and to Off with Reduce Motion on, a chosen motion kept as chosen; walkers come only with Full (code); at Off the one frame drawn is the full scene with every state's pose and balloon | PASS | 82f0bfb60 | 2026-11-06 | Claude (self) |
| Gate: a table test from every state to pose, icon, word and chip | — | one test over every state | no such test; the nearest are the state fixture test (rail word and Floor state) and `the_moving_part_shows_each_state_and_counts_subagents` (balloons for asks, stuck, crashed) | UNPROVEN | — | 2026-11-06 | Claude (self) |
| Gate: Motion Off draws no animation frames | `driver::tests::animating_it_ticks_at_its_rate_and_hidden_it_draws_nothing` | 1 frame in 10 s while animating | asserted (1) | PASS | 82f0bfb60 | 2026-11-06 | Claude (self) |
| Budget: a state change on screen ≤ 100 ms | `docs/qa/floor/budgets-2026-11-05.md`, `budgets-2026-11-06-second.md` ("A data change") | ≤ 100 ms | 2026-11-05: worst 30.8 ms of 10 (release build, replica) | within | 93717ad13 | 2026-11-06 | Claude (self) |

## R1 by row (what proves each, and what's missing)

- Working, idle, waiting (balloon, spotlight), stuck, crashed, done, limit:
  the pose, screen and balloon per state are one `match` in
  `scene/frame.rs` (Wave, Scratch, HandsOnHead, Stretch, Asleep…), the
  spotlight and amber lamp in the same file; the moving-part test asserts a
  balloon each for asks, stuck and crashed; the rail fixture test holds the
  state mapping. Captures of a sitter in each of the seven states:
  `../AJ-E3517/card-{asks,working,stuck,crashed,done,limit,idle}-*.png`
  (made-up company, not the replica). Smoke stops after 10 minutes:
  `scene/area.rs` (`smoke: now - last < 10 min`), no test. Done after 15
  minutes: not found.
- Helpers, 6 then `+N`: asserted (9 subagents give `+3`).
- App overloaded, one fire, never on a person: `scene/area.rs` puts one fire
  on the App's busiest desk; `fire()` in `scene/frame.rs`; no test.
- Unused: AJ-E3518 replaced three stages with five levels; asserted by
  `an_unused_app_dims_by_how_long_and_decay_can_be_switched_off`.
- Account low and out: OUT and LOW words, colours and badges asserted by
  `out_and_low_say_what_the_app_says`; the sign battery and the lobby's
  usage board are code only (`scene/frame.rs`).
- This Mac's rack: `machine()` in `scene/frame.rs` (lights rise with load);
  no test; seen in the `red-alert` golden frame.
- Red alert: the post shader's wash `0.1 + 0.07·sin(t·π)` (a 2 s period,
  0.5 Hz, no flash; `shaders.rs`); the alert keeps the loop ticking
  (`the_alert_keeps_it_ticking_even_at_rest`); with Motion Off the loop
  doesn't tick, so the tint stays still (code: `Driver::ticking`).
- Tasks on the whiteboard: `whiteboard()` in `scene/area.rs` ("N TASKS");
  no test.
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

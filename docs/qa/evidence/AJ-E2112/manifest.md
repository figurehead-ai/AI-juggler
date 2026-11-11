# Evidence manifest — AJ-E2112

Task: `docs/tasks/E21-desktop-first.md` AJ-E2112, Keep alive per app and
profile. Built in 0.3.3. Checked again on 2026-11-06 (release/0.4.1).
Status: **DONE-EVIDENCED** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | **Off until turned on**, per app or profile: `places::tests::nothing_starts_unless_keep_alive_was_turned_on`. **Refused for a place that can't start:** `keep_alive_is_refused_for_a_place_that_cant_be_started`. **Refused for an app the user never added** (D-84): `places::d84_tests::a_profile_seen_running_but_never_added_is_only_offered`. **The doors:** Settings › Desktop apps (`keep-alive.test.tsx` "Keep alive turns on and off") and the rail's menus (`rail-idle-app.test.tsx` "Keep alive from its menu"). |
| R2 | PASS | **Gone** means missing twice in a row: `places::keep::tests::one_miss_isnt_gone_two_are`. **Started after the wait, once, behind the app in front:** `a_stopped_app_starts_after_the_wait_and_once`, `places::tests::keep_alive_starts_a_stopped_place_behind_then_pauses_after_three`. **Live on aij-test** (`tests/aij_test_live.rs`, 2026-11-06): quit as ⌘Q, running again 7.3 s later. |
| R3 | PASS | **Never while locked, settling after a start or a sleep, or quitting:** `places::keep::tests::never_when_locked_while_settling_after_a_sleep_or_quitting`, `a_fresh_start_waits_a_minute_and_an_interactive_only_account_pauses`, `places::tests::nothing_starts_while_the_screen_is_locked`. |
| R4 | PASS | **Pauses after 3 starts in 10 minutes, and says why:** `places::keep::tests::three_stops_in_ten_minutes_pause_it`. **A stop in AI Juggler pauses it:** `places::tests::a_stop_from_ai_juggler_pauses_keep_alive`; also live in `tests/places_real.rs` step 4. **Open ends a pause:** `places::open` clears `paused` before it starts or brings forward. |
| R5 | PASS | **Every start and every pause is a notice** under Earlier whose action opens the switch (Settings › Desktop apps): `notifications::sources::tests::each_keep_alive_start_and_pause_is_a_notice_that_opens_the_switch` (added 2026-11-06). |

Performance (one process check per kept place every 2 s, the whole list at
most every 10 s while one is down): `places::tick_with` checks the watched
pids each tick and searches the whole process list only after
`SEARCH_EVERY_MS` (10 s).

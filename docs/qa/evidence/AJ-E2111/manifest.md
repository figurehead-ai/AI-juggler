# Evidence manifest — AJ-E2111

Task: `docs/tasks/E21-desktop-first.md` AJ-E2111, Open starts an app or
profile that isn't running. Built in 0.3.3. Reworked on 2026-11-06 for
the maintainer's 0.4.0 finding #1 and D-84 (release/0.4.1, f65f560b4,
8d59ccf0b). Status: **DONE-EVIDENCED** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | **Remembered** (bundle, folder, sessions): `places::book::tests::a_profile_is_remembered_with_its_folder_and_its_sessions`. **Bounded** (at most 32, forgotten after 90 days unless kept alive): `unseen_places_are_forgotten_unless_kept_alive_and_the_book_stays_small`. **The user's apps only** (D-84): `places::d84_tests::a_profile_seen_running_but_never_added_is_only_offered`, `not_now_hides_the_offer_and_adds_nothing`, `an_offer_whose_folder_is_gone_is_forgotten`, `a_profile_in_the_temporary_folder_is_never_remembered`. **The upgrade:** `juggler-core/tests/apps_v47.rs`. **Live:** a fresh AI Juggler that saw aij-test running offered it and didn't add it (`tests/aij_test_live.rs`). |
| R2 | PASS | **Starts a closed one** exactly as it last ran, in front: `places::tests::open_starts_a_profile_that_isnt_running_with_only_its_folder`. **Running** means the same pid, program and start time: `places::d84_tests::a_reused_pid_is_not_the_app`. **The doors:** the rail's App header (`rail-group-app.test.ts`: found by key while closed, Open starts it, says Start), Inactive apps (`rail-idle-app.test.tsx`), Home's closed tile (`keep-alive.test.tsx` "has a tile with why and Start, which starts it"), Settings › Desktop apps (`keep-alive.test.tsx` "lists each with Start or Open"). The App page and ⌘K call the same `placesOpen` (`AppPage.tsx`, `GoToPalette.tsx`). **Live on aij-test** (`tests/aij_test_live.rs`, 2026-11-06): Open on it closed answered in 40 ms and started it; Open on it running brought it forward in 1.0 s with no second copy. |
| R3 | PASS | **An ended session offers Open in its app:** `keep-alive.test.tsx` "is the place it was last open in, by its tool and id". **A session started in a closed profile** that was never seen open is that profile's: `places::d84_tests::a_session_started_in_a_closed_profile_is_that_profiles` (`Book::users_place_on`, from the session's own started-in mark). **Codex** opens the thread by its link: `places::tests::open_in_codex_opens_the_thread_by_its_link_and_starts_nothing_else`. |
| R4 | PASS | **Start checks:** `places::start::tests::a_place_that_isnt_the_apps_any_more_is_never_started`, `an_app_other_users_can_change_is_never_started`. **The window's setting writer can't change the record:** `places::tests::the_windows_setting_writer_cant_name_a_program`. |

Performance (Open answers within 1.5 s): 40 ms to start a closed profile
and 1.0 s to bring a running one forward, live.

Still to walk by hand: clicking each door in AI Juggler Dev's window. It
waits on the maintainer's keychain Allow for the fresh Dev build, as on
the 0.4 completion page.

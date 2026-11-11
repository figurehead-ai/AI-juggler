# Evidence manifest — AJ-E2506

Task: `docs/tasks/E25-notifications-and-user-menu.md` AJ-E2506, the
menu-bar item (§4.17). Built 2026-10-27; R3 amended 2026-11-06 (the open
count comes from the backend, D-49). Checked requirement by requirement on
2026-11-06 (release/0.4.1). The gaps in all three requirements closed the
same day: DONE-EVIDENCED.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | **Tested:** the builder's title and tooltip: `juggler-app` `features::notifications::tray::tests::the_menu_lists_what_needs_you_then_what_is_open` (title "3", tooltip "AI Juggler · 3 things need you") and `nothing_needs_you_and_nothing_open_say_so_with_no_count` (no title, "AI Juggler · nothing needs you"); the title equals the bell's count (`tests/notification_model.rs` `the_count_is_the_same_on_the_bell_the_badge_and_the_menu_bar_item`). **Closed (2026-11-06):** `notification_model.rs` `the_badge_and_the_menu_bar_item_follow_every_change`: connecting the views draws the menu-bar item at once (made at start: no title, "AI Juggler · nothing needs you"), and every change redraws it through the same seam the app's `tray::apply` sits behind (`views::Surface`): title "2" and "AI Juggler · 2 things need you" after the tick, "1" after a snooze. The real tray isn't made in a test (macOS makes menus on the main thread only). |
| R2 | PASS | **Tested:** the menu's lines, in order: "Needs you · 3", the notices, "Open now · 10 · 4 working" and the apps, Open AI Juggler, New session… (`the_menu_lists_what_needs_you_then_what_is_open`); five lines then "and 3 more…" (`a_long_list_ends_with_the_rest_in_the_centre`); every line's id maps to a known click (`every_id_the_menu_makes_is_a_click_it_knows`); the gallery fixture is the builder's output (`the_gallerys_menu_is_the_builders`). **Closed (2026-11-06):** `notification_model.rs` `a_menu_line_brings_ai_juggler_forward_on_its_notice_and_nothing_else` (`tauri::test::mock_app()`, a main window, a listener on every `AppEvent`): a notice's line asks the window to open that notice (`notifications://open` with its id) and nothing else is emitted; Open AI Juggler, a line that only says something and an unknown id emit nothing; "and N more…" opens the centre. |
| R3 | PASS | **Tested:** the backend's tick works out what is open with no window: `notification_model.rs` `needs_you_is_worked_out_in_the_backend_with_no_window` asserts `open_now()` total 3, 1 working, and builds the menu from `state.notifier.open_now()`. **Closed (2026-11-06):** `notification_model.rs` `open_now_is_counted_by_app_in_the_backend`: `needs::work_out` counts what is open by app, most first (AI Juggler 4: the waiting, stuck, limited and running sessions, never a failed, parked, archived or forgotten one; Codex 3; Claude app 2: a background session and an app turned off left out), 9 in all, 2 working; a snoozed ask waits in its place (8); the menu lists it as worked out ("Open now · 8 · 2 working", "AI Juggler · 3 · Codex · 3 · Claude app · 2"). |

Run on 2026-11-06: `cargo test -p juggler-app --lib features::notifications`
(48 passed, the six tray tests among them); `cargo test -p juggler-app
--test notification_model` (7 passed). The gallery's tray-menu capture was
not run here.

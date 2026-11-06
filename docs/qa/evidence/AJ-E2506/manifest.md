# Evidence manifest — AJ-E2506

Task: `docs/tasks/E25-notifications-and-user-menu.md` AJ-E2506, the
menu-bar item (§4.17). Built 2026-10-27; R3 amended 2026-11-06 (the open
count comes from the backend, D-49). Checked requirement by requirement on
2026-11-06 (release/0.4.1). All three requirements have gaps, so the
status stays DONE-PENDING-EVIDENCE.

| Req | Result | Evidence |
|---|---|---|
| R1 | GAP | **Tested:** the builder's title and tooltip: `juggler-app` `features::notifications::tray::tests::the_menu_lists_what_needs_you_then_what_is_open` (title "3", tooltip "AI Juggler · 3 things need you") and `nothing_needs_you_and_nothing_open_say_so_with_no_count` (no title, "AI Juggler · nothing needs you"); the title equals the bell's count (`tests/notification_model.rs` `the_count_is_the_same_on_the_bell_the_badge_and_the_menu_bar_item`). **Missing:** no test that the tray is made at start (`tray.rs` `TrayIconBuilder`) or redrawn on every change (`views::refresh` → `tray::apply`). A `tauri::test::mock_app()` test could cover both. |
| R2 | GAP | **Tested:** the menu's lines, in order: "Needs you · 3", the notices, "Open now · 10 · 4 working" and the apps, Open AI Juggler, New session… (`the_menu_lists_what_needs_you_then_what_is_open`); five lines then "and 3 more…" (`a_long_list_ends_with_the_rest_in_the_centre`); every line's id maps to a known click (`every_id_the_menu_makes_is_a_click_it_knows`); the gallery fixture is the builder's output (`the_gallerys_menu_is_the_builders`). **Missing:** no test of what a click does (`views::tray_clicked`): that a notice line brings AI Juggler forward on that notice and nothing else happens. |
| R3 | GAP | **Tested:** the backend's tick works out what is open with no window: `notification_model.rs` `needs_you_is_worked_out_in_the_backend_with_no_window` asserts `open_now()` total 3, 1 working, and builds the menu from `state.notifier.open_now()`. **Missing:** the by-app part: no test that `needs::work_out` counts open sessions per app (`OpenNowInput.apps`); the builder's test hands the apps in. |

Run on 2026-11-06: `cargo test -p juggler-app --lib features::notifications`
(48 passed, the six tray tests among them); `cargo test -p juggler-app
--test notification_model` (7 passed). The gallery's tray-menu capture was
not run here.

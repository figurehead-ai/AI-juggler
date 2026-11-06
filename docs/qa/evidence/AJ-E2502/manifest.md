# Evidence manifest — AJ-E2502

Task: `docs/tasks/E25-notifications-and-user-menu.md` AJ-E2502, one count:
the bell, the app badge, Needs you. Built 2026-10-27; D-49 made Needs you
the backend's one list. Checked requirement by requirement on 2026-11-06
(release/0.4.1). Three requirements have gaps, so the status stays
DONE-PENDING-EVIDENCE.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | **The backend's one list and count, with no window:** `juggler-app/tests/notification_model.rs` `needs_you_is_worked_out_in_the_backend_with_no_window` asserts the bell's summary, the centre's Needs list, the menu-bar item's title and the badge value are equal (2, then 1 while snoozed, 2 again when the tick ends the snooze); `the_count_is_the_same_on_the_bell_the_badge_and_the_menu_bar_item` does the same with a model need (an account almost out of room) in the list. System notifications read the model's snoozes and mutes (`a_snoozed_need_or_a_muted_kind_doesnt_interrupt`). **The window reads it:** `ui/src/count-agreement.test.tsx` (the bell's count = Home's heading count = the rail's Needs you, and the rail lists the account need, a need that isn't a session's); `components/rail-folded.test.tsx` "counts what the bell counts — a sign-in too" (the collapsed rail); `components/rail-folded-needs.test.tsx` "a folded project shows how many need you, with the Needs you mark" (the Project lens's folded headers). The preview's stand-in runs the same rules (both UI tests run on it). |
| R2 | GAP | **Tested:** Home lists the model's own need: `count-agreement.test.tsx` waits for "Work Max is almost out of room" in `home-needs`. The button's words for a limit: `components/home-chips.test.tsx` asserts `primaryWords` of a `limit` need is "Continue in another app…". **Missing:** no test asserts the button on Home's account-almost-out-of-room row (kind `quota`). The code (`lib/home-needs.ts` `primaryWords`) says "Continue in another app…" only when the account has an open session, else "Open its app" (F16), which the 2026-11-06 amendment ("everywhere") doesn't mention. Needs a test of both cases, and the amendment updated or the code changed. |
| R3 | GAP | **Tested:** `components/notifications.test.tsx` "shows the model's one count, and opens the centre": the count reads 2, the button's name starts "Notifications: 2 things need you", `aria-keyshortcuts` is `Meta+Alt+N`, and a click opens the centre. `app-keys.test.tsx` "D-50: ⌥⌘N opens the notifications (by the key's place)". **Missing:** no test that the count is hidden at zero, and none of the unread part of the name ("…, 2 unread"; `lib/notice-words.ts` `bellLabel`). |
| R4 | GAP | **Tested:** the platform seam's value: `juggler-platform` `system::tests::a_badge_shows_a_count_and_goes_at_zero`; a model change calls the views' hook (`notification_model.rs`: "a change reaches the views", the change counter moves on each write). **Missing:** no test that the hook reaches `views::show_badge` (`views.rs` `refresh` → `window.set_badge_count`). A `tauri::test::mock_app()` test (as `tests/system_notifications.rs` already uses) could assert it. |

Run on 2026-11-06: `cargo test -p juggler-app --test notification_model` (7
passed); `cargo test -p juggler-platform badge` (1 passed); `pnpm exec
vitest run src/count-agreement.test.tsx src/components/notifications.test.tsx
src/app-keys.test.tsx src/components/rail-folded.test.tsx
src/components/home-chips.test.tsx` (passed).

# Evidence manifest — AJ-E2507

Task: `docs/tasks/E25-notifications-and-user-menu.md` AJ-E2507, your menu
(the avatar). Built 2026-10-27; R3 amended 2026-11-06 (D-49: My Macs only
with a Figurehead account). Checked requirement by requirement on
2026-11-06 (release/0.4.1); the two gaps were closed the same day.
Status: DONE-EVIDENCED (2026-11-06, release/0.4.1).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | The header keeps Home, Accounts, Usage and History: `ui/src/screens/nav-screens.test.tsx` "are exactly Home · Projects · Pools · Tasks · Accounts · Usage · History" (`PRIMARY_NAV`; Projects, Pools and Tasks were added by the 0.3 redesign). ⌘, opens Settings: `tests/visual/a11y.spec.ts` presses `ControlOrMeta+Comma` and expects `settings-screen`; `app-keys.test.tsx` matches `settings` to ⌘, and Ctrl+,. Settings from the menu: `notifications.test.tsx` "Appearance changes the theme in place; Settings opens Settings". |
| R2 | PASS | **Tested:** without an account, "On this Mac · no Figurehead account" and the one "Connect a Figurehead account (optional)", no Disconnect (`components/notifications.test.tsx` "without a Figurehead account: one quiet Connect…"); with one, the address shortened ("s…@example.com · Figurehead", `shortEmail`), Cloud backup and My Macs first, Disconnect Figurehead… last, no Connect ("with one: the address shortened…"). The backend's name and initial: `juggler-app` `features::app::user` `the_menu_shows_your_first_name_and_its_initial`. **Closed (2026-11-06):** `ui/src/components/notification-views.test.tsx` "R2: your name and initial; with an account its plan, Cloud backup's when-last and My Macs' count" (the avatar "F", named "Your menu: Faruk"; the head "Faruk · f…@example.com · Figurehead · Pro"; Cloud backup's hint `agoWords` of the last backup; "My Macs" with 2) and "R2: without an account, the system's name and initial and no plan". |
| R3 | PASS | **Tested:** the lines in order, "Settings", "This Mac", "Processes", "Privacy", "Keyboard shortcuts", "Help and guides", "Report a bug…", "About and updates", with Appearance as three inline radios Light / Dark / Auto, and My Macs absent without an account (D-49) ("without a Figurehead account…"); Light applies in place ("Appearance changes the theme in place…"). **Closed (2026-11-06), as amended ("Report a bug…", as AJ-E2911 names it):** `ui/src/components/notification-views.test.tsx` "R3: the hints ⌘, · ? · version · channel, and “Report a bug…”": Settings' hint ⌘, (the registry's key), Keyboard shortcuts' ?, About and updates' "0.4.1 · Beta", and "Report a bug…" with no "Report a problem…". |
| R4 | PASS | Icons on every line, three groups (two separators), `role=menu` named "Your menu": `notifications.test.tsx` "without a Figurehead account…". The arrows and Esc: the menu is the shared `RowMenu`; `components/item-menu.test.tsx` "is a role=menu that takes focus and gives it back on Escape" (↓, Home, End move focus; Esc closes and returns focus) tests that component. |

Run on 2026-11-06: `pnpm exec vitest run src/components/notifications.test.tsx
src/screens/nav-screens.test.tsx src/app-keys.test.tsx
src/components/item-menu.test.tsx` (passed). The gallery (user-menu,
user-menu-account) and `a11y.spec.ts` were not run here.

Gaps closed on 2026-11-06: `pnpm exec vitest run src/components/notification-views.test.tsx src/components/user-menu-facts.test.tsx src/screens/system-notifications.test.tsx` (26 passed); the whole suite passed too (281 files, 2135 tests).

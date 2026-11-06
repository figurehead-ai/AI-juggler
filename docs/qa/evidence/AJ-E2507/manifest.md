# Evidence manifest — AJ-E2507

Task: `docs/tasks/E25-notifications-and-user-menu.md` AJ-E2507, your menu
(the avatar). Built 2026-10-27; R3 amended 2026-11-06 (D-49: My Macs only
with a Figurehead account). Checked requirement by requirement on
2026-11-06 (release/0.4.1). Two requirements have gaps, so the status
stays DONE-PENDING-EVIDENCE.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | The header keeps Home, Accounts, Usage and History: `ui/src/screens/nav-screens.test.tsx` "are exactly Home · Projects · Pools · Tasks · Accounts · Usage · History" (`PRIMARY_NAV`; Projects, Pools and Tasks were added by the 0.3 redesign). ⌘, opens Settings: `tests/visual/a11y.spec.ts` presses `ControlOrMeta+Comma` and expects `settings-screen`; `app-keys.test.tsx` matches `settings` to ⌘, and Ctrl+,. Settings from the menu: `notifications.test.tsx` "Appearance changes the theme in place; Settings opens Settings". |
| R2 | GAP | **Tested:** without an account, "On this Mac · no Figurehead account" and the one "Connect a Figurehead account (optional)", no Disconnect (`components/notifications.test.tsx` "without a Figurehead account: one quiet Connect…"); with one, the address shortened ("s…@example.com · Figurehead", `shortEmail`), Cloud backup and My Macs first, Disconnect Figurehead… last, no Connect ("with one: the address shortened…"). The backend's name and initial: `juggler-app` `features::app::user` `the_menu_shows_your_first_name_and_its_initial`. **Missing:** no UI test that the menu shows the name and initial, the plan, Cloud backup's "when last" (`agoWords`), or My Macs' count. |
| R3 | GAP | **Tested:** the lines in order, "Settings", "This Mac", "Processes", "Privacy", "Keyboard shortcuts", "Help and guides", "Report a bug…", "About and updates", with Appearance as three inline radios Light / Dark / Auto, and My Macs absent without an account (D-49) ("without a Figurehead account…"); Light applies in place ("Appearance changes the theme in place…"). **Missing:** the menu says "Report a bug…" where R3 says "Report a problem…": the wording needs the code or the requirement changed. No test of the hints: ⌘, on Settings, ? on Keyboard shortcuts, "version · channel" on About and updates (`UserMenu.tsx` line ~192). |
| R4 | PASS | Icons on every line, three groups (two separators), `role=menu` named "Your menu": `notifications.test.tsx` "without a Figurehead account…". The arrows and Esc: the menu is the shared `RowMenu`; `components/item-menu.test.tsx` "is a role=menu that takes focus and gives it back on Escape" (↓, Home, End move focus; Esc closes and returns focus) tests that component. |

Run on 2026-11-06: `pnpm exec vitest run src/components/notifications.test.tsx
src/screens/nav-screens.test.tsx src/app-keys.test.tsx
src/components/item-menu.test.tsx` (passed). The gallery (user-menu,
user-menu-account) and `a11y.spec.ts` were not run here.

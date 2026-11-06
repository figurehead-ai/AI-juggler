# Evidence manifest — AJ-E2905

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2905, settings
you can search and shortcuts you can rebind. Built in 0.4. Checked again,
requirement by requirement, on 2026-11-06 (release/0.4.1). Status:
**DONE, 3 gaps open** (agent self-check, 2026-11-06). The vitest files
below pass (2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `ui/src/screens/settings/settings-search.test.tsx` "`/` focuses it; ↓ and ↩ open the match at its section; others dim; Esc clears": `/` on the body focuses the search field; the first Esc empties it and keeps focus; the second Esc leaves it. |
| R2 | PASS | **Static index beside `PANES`:** `ui/src/screens/settings/settings-index.ts`. `settings-search.test.tsx` "finds by a setting's words, title matches first": "mute" finds Notifications' mutes, "keep alive" finds Keep alive, "inspector" finds a Keyboard row, nonsense and blanks find nothing. |
| R3 | PASS | Same test: the hit reads "Advanced" + "Local cache"; panes with no match carry `opacity-45`; ↩ opens Advanced, marks it current and focuses the section's title; no match shows "No setting matches". "“quiet hours” lands on Notifications' Quiet hours" proves a hit lands on its row. Playwright `ui/tests/visual/settings-keyboard-gallery.spec.ts` "Settings search" captures results, none and opened (`99-settings-search`). |
| R4 | PASS | `settings-search.test.tsx` "indexes every section every pane draws, and no section that's gone" renders every pane in `PANES` and fails on a drawn section missing from the index or an index entry with no section. |
| R5 | GAP | The pane lists the registry by `SHORTCUT_AREAS` with keys, Reset once changed and Reset all (`settings-search.test.tsx` "changes a key, refuses a taken one…"; `shortcut-bindings.test.ts` "Reset all puts every key back, and Undo restores them" proves `restoreBindings`). Missing: no test clicks the Undo on the Reset all toast (`KeyboardPane.tsx` `resetEverything`); the UI test only checks Reset all clears the change. Reset all sits at the top, a recorded change from the task text (Delivered note). |
| R6 | PASS | `ui/src/lib/shortcut-bindings.test.ts` "every shortcut is either rebindable or says why it's fixed" (each registry entry has a default binding or a `FIXED_WHY` reason). `settings-search.test.tsx` checks the Esc row shows "Esc closes what's open everywhere." |
| R7 | GAP | Refusals are proven: `shortcut-bindings.test.ts` "refuses another shortcut's key, a fixed one, the system's, and plain keys outside Home" (exact "⌘N is New session. Change that one first.", "⌘Q quits the app on this computer. Pick another key.", a plain key refused for Anywhere, a plain W allowed for a Home row key); Esc cancels a capture in `settings-search.test.tsx`. Missing: the system's keys are one Mac list (`RESERVED` in `ui/src/lib/shortcut-bindings.ts`), not a per-OS list behind the platform seam that the text asks for (and D-52's "OS seams always"). |
| R8 | GAP | Proven for `matches`, the ? sheet and ⌘K: `shortcut-bindings.test.ts` "a change applies at once…" (`keysOf`, `ariaKeysOf`, `matches` follow the change) and Playwright `release-04-screens-gallery.spec.ts` "AJ-E2905: a rebound key shows on the ? sheet and in ⌘K". Missing: key text still hard-coded in JSX in `components/SwitchGraph.tsx` ("(⌘R)"), `components/RowTools.tsx` ("⌥⌘↑ and ⌥⌘↓"), `components/PlaceHeader.tsx` (`⌘${shortcut}`, `aria-keyshortcuts` built by hand), `components/ItemMenu.tsx` ("(⌘↩)", `aria-keyshortcuts="Meta+Enter"`), `pages/messages/Compose.tsx` ("⌘↵") and `pages/subscription/TermChoice.tsx` ("⌘↩"). |
| R9 | PASS | `shortcut-bindings.test.ts`: "a change applies at once, is kept, and the old key stops" (written to the app setting `ui.shortcuts`, cleared on reset); "a stored change that now clashes falls back once, and says so" (one notice naming the shortcut; the store rewritten without it); "a broken store reads as no changes". |
| R10 | PASS | `settings-search.test.tsx` (Keyboard test): the go-to row refuses ⌥Space with "ChatGPT uses ⌥Space", then takes ⌃⌥Space. `ui/src/screens/settings/desktop-apps.test.tsx` "has no key to open AI Juggler until one is chosen, refuses ⌥Space, and can turn it off" proves the same refusal in Desktop apps. Both panes use `useGoToKey` and the controls in `ui/src/screens/settings/KeyCapture.tsx`. |

Run 2026-11-06: `pnpm exec vitest run` on `settings-search.test.tsx`,
`shortcut-bindings.test.ts`, `app-keys.test.tsx`, `desktop-apps.test.tsx`
— all pass.

## The first part (2026-11-05)

- vitest: `src/lib/shortcut-bindings.test.ts`, `src/screens/settings/settings-search.test.tsx`
  (the drift test: every Settings row findable), `src/app-keys.test.tsx`
  — pass (2026-11-05).
- Playwright, `tests/visual/release-04-screens-gallery.spec.ts`
  "AJ-E2905: a rebound key shows on the ? sheet and in ⌘K": New project
  rebound to ⌥⌘J in Settings › Keyboard; the ? sheet and the palette's
  New project row then show ⌥⌘J — pass.
- Galleries `99-settings-keyboard` (changed, capturing, refused, the go-to
  key) and `99-settings-search` (results, nothing found); both themes,
  1280 and 1024; axe 0 critical/serious; critiques stamped.
- Verdict: PASS.

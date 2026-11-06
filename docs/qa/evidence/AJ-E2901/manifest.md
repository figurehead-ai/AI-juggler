# Evidence manifest — AJ-E2901

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2901, the
command palette. Built in 0.4. Checked requirement by requirement on
2026-11-06 (release/0.4.1). The palette's vitest files pass (8 files, 48
tests, 2026-11-06). Status: **DONE, 9 gaps open** (agent self-check,
2026-11-06). The task header still says DONE-EVIDENCED from the first
check below; that check was by test file, not by requirement.

| Req | Result | Evidence |
|---|---|---|
| R1 | GAP | `palette-dialog.test.tsx` "is a named modal dialog; focus goes in, Tab stays inside, and closing returns focus to the invoker" asserts `role="dialog"` named "Search, act or go", `aria-modal="true"`, Tab from the last button wraps to the field, and focus back on the invoker; "Esc closes it" asserts Esc. `goto-palette.test.tsx` "combobox a11y" asserts `aria-expanded` and `aria-activedescendant`. **Missing:** no test presses ⇧Tab in the palette, and the Playwright `keyboard-flows.spec.ts` flow (focus return, Tab) named in the quality gates was never added. |
| R2 | GAP | `palette-dialog.test.tsx` "shows Recent (ids only, still here) and the top actions, then what's open" asserts Recent, then Actions with New session… and Settings; "a first run (no recents)…" asserts Actions only. `palette-actions.test.tsx` "lists its actions first…" asserts the selected item's actions come first. **Missing:** no test for "Processes — n left running" and "Notifications — n unread", or that they show only when the count is above 0 (`palette/rows.ts` `topActions`). |
| R3 | GAP | `palette-dialog.test.tsx` "caps a group at 5 and says how many more" asserts 5 rows and "· 1 more"; "groups in order…" asserts Sessions first and Search last for "wrk bil". **Missing:** the fixed order Selected · Actions · Sessions · Apps · Projects · Accounts · Search is not what ships. Groups now sort by their best match (the 0.4 design review, 16; `rows.test.ts` "puts the exact match above a loose action"). The task text needs an amendment for that order, or the code needs the fixed order. |
| R4 | GAP | No test renders an Accounts row. **Missing:** a test that an account shows its label, its tool's name and its room in words (never 0 % when unknown), that ↩ opens its page (`openAccountById`) and that ⌘↩ starts New session on it (`startSessionOn`). |
| R5 | GAP | No test renders an Apps row. **Missing:** a test that an open app shows its own icon and launch badge, that ↩ opens its place, and that ⌘↩ calls `placesOpen` (`GoToPalette.tsx` `choose`). |
| R6 | PASS | `palette-match.test.ts`: "matches a word at the start, a word's start, inside, or its letters within one word", "every typed word must match the title or another field", "ranks the title starting with what's typed first, then word starts, then the rest, then recency", "the work-billing story" and "returns the title's ranges for bold, merged"; "the rail's words still parse out before matching" checks `is:` and `app:`. `palette-dialog.test.tsx` "groups in order, loosely matched, the matched letters bold" asserts the matched letters are in `<b>`. |
| R7 | GAP | `palette-dialog.test.tsx` "→ previews the active session beside the list; ← goes back" asserts the preview opens named "Preview of Fix billing export", shows the account, is in `aria-controls`, and ← closes it. `RowView.test.tsx` "names the Account by its real name, and Continue as Home does" checks the Continue action. **Missing:** no test previews an app, a project or an account, and none checks the state word, place and last activity lines. |
| R8 | GAP | `palette-dialog.test.tsx` "See all results searches History for the words" asserts History is searched for "billing". **Missing:** no test asserts the footer's words ("↑↓ move · ↩ open · ⌘↩ go to its app · → preview · Esc close", `goto-keys`). |
| R9 | PASS | `palette-dialog.test.tsx` "no matches: says so, with Search History for it active" asserts "No matches for “qqqzz”." and the active row `command__search-history`; "offline: says it shows what was open last" asserts the offline line word for word. |
| R10 | GAP | `palette-match.test.ts` "keeps ids only, newest first, at most 8, drops gone ones and clears" asserts kind and id only, a cap of 8, pruning and `clearRecents`; `palette-dialog.test.tsx` "choosing remembers it (ids only)" checks the stored value. **Missing:** no test that Settings › Advanced › Local cache's Clear empties Recent (`LocalCacheSection.tsx` calls `clearRecents`, untested). |
| R11 | GAP | The ⌘K label is "Search, act or go" in `ui/src/lib/shortcuts.ts:100`. **Missing:** no test reads the shortcut registry's label (the dialog test checks the dialog's own name, which is a separate string), and no test checks that each palette action also has a visible control. |

Performance: [timing.json](timing.json), `palette/palette-timing.test.ts`
"opens and re-ranks within budget on the big fixture" (500 sessions, 200
projects, 20 accounts, 30 apps).

Gallery: `ui/tests/visual/palette-gallery.spec.ts` (family `99-palette`).

## The first part (2026-11-05)

- vitest: `src/components/goto-palette.test.tsx`, `src/lib/palette-match.test.ts`,
  `palette-commands` and `palette-actions` tests — pass (2026-11-05).
- [timing.json](timing.json): 500 sessions, 200 projects, 20 accounts,
  30 apps: opens in 0.54 ms, a keystroke 2.7 ms p95 (budgets 50 and 16).
- Gallery `99-palette`: empty, typed, the preview, no matches, offline;
  1024×700, 1280×800 and 1440×900, dark and light; axe 0
  critical/serious; critique stamped.
- Found on the way: words typed straight after ⌘K were lost while the
  palette's code loaded; it's now fetched first after paint (App.tsx).
- Verdict: PASS.

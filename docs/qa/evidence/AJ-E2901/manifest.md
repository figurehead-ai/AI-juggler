# Evidence manifest — AJ-E2901

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2901, the
command palette. Built in 0.4. Checked requirement by requirement on
2026-11-06 (release/0.4.1). The palette's vitest files pass (8 files, 48
tests, 2026-11-06). Status: DONE-EVIDENCED (2026-11-06, release/0.4.1): the nine
gaps were closed the same day (`ui/src/components/palette-evidence.test.tsx`).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `palette-dialog.test.tsx` "is a named modal dialog; focus goes in, Tab stays inside, and closing returns focus to the invoker" asserts `role="dialog"` named "Search, act or go", `aria-modal="true"`, Tab from the last button wraps to the field, and focus back on the invoker; "Esc closes it" asserts Esc. `goto-palette.test.tsx` "combobox a11y" asserts `aria-expanded` and `aria-activedescendant`. **Closed (2026-11-06; a vitest dialog test in place of the Playwright flow, as the maintainer's brief allows):** `ui/src/components/palette-evidence.test.tsx` "⇧Tab from the field wraps to the last control; Esc closes, and focus goes back to the invoker": ⇧Tab from the field to the last button, Tab from it back to the field, `aria-expanded` and `aria-activedescendant` on the combobox, Esc calls `onClose` once, and once closed the invoker has focus. |
| R2 | PASS | `palette-dialog.test.tsx` "shows Recent (ids only, still here) and the top actions, then what's open" asserts Recent, then Actions with New session… and Settings; "a first run (no recents)…" asserts Actions only. `palette-actions.test.tsx` "lists its actions first…" asserts the selected item's actions come first. **Closed (2026-11-06):** `palette-evidence.test.tsx` "adds Processes with what's left running and Notifications with the unread count, only when above 0" (2 left running, 3 unread: Actions are New session…, Settings, "Processes — 2 left running", "Notifications — 3 unread", in that order) and "leaves both out at zero". |
| R3 | PASS | `palette-dialog.test.tsx` "caps a group at 5 and says how many more" asserts 5 rows and "· 1 more"; "groups in order…" asserts Sessions first and Search last for "wrk bil". **Closed (2026-11-06), as amended (groups sorted by their best match, design review 16):** `components/palette/rows.test.ts` "puts the exact match above a loose action" and "keeps an action group first when it matches at least as well"; in the dialog, `palette-evidence.test.tsx` "an exact project leads a loose action; each group has its section label; Search last" ("payments" puts Projects first, every group has its label, Search ends the list; "new" puts Actions first). |
| R4 | PASS | No test renders an Accounts row. **Closed (2026-11-06):** `palette-evidence.test.tsx` "each by its name, its tool and its room in words (never 0 % unknown); ↩ opens its page, ⌘↩ New session on it" (Claude Max 20x · "Claude Code · 31% left in the 5-hour window"; ChatGPT Pro · "Codex · Not measured yet", no 0 %; ↩ calls `openAccountById` with its account) and "⌘↩ starts New session on its signed-in folder" (`startSessionOn` with the folder, not `openAccountById`). |
| R5 | PASS | No test renders an Apps row. **Closed (2026-11-06):** `palette-evidence.test.tsx` "an open app shows its own icon and launch badge, open ones first; ↩ opens its place, ⌘↩ brings it forward" (open now before not running; the row's `app-icon` is the app's own, `claude-app`, with the profile badge "Work Max"; ↩ asks to open its page, never `placesOpen`) and "⌘↩ calls the platform's open for its place" (`placesOpen` with the place's id). |
| R6 | PASS | `palette-match.test.ts`: "matches a word at the start, a word's start, inside, or its letters within one word", "every typed word must match the title or another field", "ranks the title starting with what's typed first, then word starts, then the rest, then recency", "the work-billing story" and "returns the title's ranges for bold, merged"; "the rail's words still parse out before matching" checks `is:` and `app:`. `palette-dialog.test.tsx` "groups in order, loosely matched, the matched letters bold" asserts the matched letters are in `<b>`. |
| R7 | PASS | `palette-dialog.test.tsx` "→ previews the active session beside the list; ← goes back" asserts the preview opens named "Preview of Fix billing export", shows the account, is in `aria-controls`, and ← closes it. `RowView.test.tsx` "names the Account by its real name, and Continue as Home does" checks the Continue action. **Closed (2026-11-06):** `palette-evidence.test.tsx` → on a session (Now "Working", Where, Account, Last active, Open and Continue in another app…), an app (Now "Open now", App "Claude app · Work Max", Open its page, Bring it forward), a project (Folder, Last active) and an account (Tool, Room, Plan, New session on it…). |
| R8 | PASS | `palette-dialog.test.tsx` "See all results searches History for the words" asserts History is searched for "billing". **Closed (2026-11-06):** `palette-evidence.test.tsx` "says the keys, and carries See all results →": `goto-keys` reads exactly "↑↓ move · ↩ open · ⌘↩ go to its app · → preview · Esc close". |
| R9 | PASS | `palette-dialog.test.tsx` "no matches: says so, with Search History for it active" asserts "No matches for “qqqzz”." and the active row `command__search-history`; "offline: says it shows what was open last" asserts the offline line word for word. |
| R10 | PASS | `palette-match.test.ts` "keeps ids only, newest first, at most 8, drops gone ones and clears" asserts kind and id only, a cap of 8, pruning and `clearRecents`; `palette-dialog.test.tsx` "choosing remembers it (ids only)" checks the stored value. **Closed (2026-11-06):** `palette-evidence.test.tsx` "Settings › Advanced › Local cache › Clear cache empties Recent": two recents stay through the confirm's opening and are gone (storage key removed) once Clear is confirmed. |
| R11 | PASS | The ⌘K label is "Search, act or go" in `ui/src/lib/shortcuts.ts:100`. **Closed (2026-11-06):** `palette-evidence.test.tsx` "the registry names ⌘K “Search, act or go”" (the `palette` entry's label and ⌘K) and "every action ⌘K lists has a visible control" (each of `appCommands`, Fleet included, maps to a control found in the header, its tabs, your menu, the Floor's menu, the rail's head or Projects' New project; none missing). |

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

Gaps closed on 2026-11-06: the whole vitest suite passed (`pnpm exec vitest run --maxWorkers=4`: 281 files, 2135 tests) with the tests named above.

# Evidence manifest — AJ-E3105

Task: `docs/tasks/E31-everything-links.md` AJ-E3105, every picker:
suggested for where you are, then All, with search. Built 2026-11-03.
Checked on 2026-11-06 (release/0.4.1). Vitest files named below were run
on 2026-11-06 and passed. Status: DONE-EVIDENCED (2026-11-06, release/0.4.1): its gap was
closed the same day.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | The shared Picker's props force each call site to name its groups or say it is a `choice` (`ui/src/components/controls/Picker.tsx`, a type union), and every picker of things passes groups (New session, Expert setup, a task's Start, the project's default App, Compose, the pool sheets, the related lists' filters). The helper is tested: `ui/src/lib/suggested.test.ts` "lists the open ones first, the most used of the rest next…". **Closed (2026-11-06):** `ui/src/components/picker-in-app.test.tsx` "Start with the same settings as: Open now, then Most used, then All": the real `StartWithSettings` asks the relation counts for its seven Apps and its picker opens on "Open now" (Lab), "Most used" (Work, Side), then "All" with the rest and none twice. The K4 walk and the gallery's open pickers are not vitest and weren't run here. |
| R2 | PASS | `ui/src/components/controls/picker-groups.test.tsx` "lists the group first, then All with the empty ones last, leaving out what the group lists". |
| R3 | PASS | Every picker is the shared Picker (no raw `<select>` in `ui/src`). `ui/src/components/controls/controls.test.tsx` "opens from the keyboard, moves with the arrows, and Escape closes only the menu"; `picker-groups.test.tsx` "past 8 options the menu opens on a search field; typing filters, Enter picks the first match". |

Performance (opens ≤ 100 ms with 1 000 items): `picker-groups.test.tsx`
"draws at most 60 lines and says how many more typing finds" (windowed).

Gaps closed on 2026-11-06: the whole vitest suite passed (`pnpm exec vitest run --maxWorkers=4`: 281 files, 2135 tests) with the tests named above.

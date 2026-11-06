# Evidence manifest — AJ-E3105

Task: `docs/tasks/E31-everything-links.md` AJ-E3105, every picker:
suggested for where you are, then All, with search. Built 2026-11-03.
Checked on 2026-11-06 (release/0.4.1). Vitest files named below were run
on 2026-11-06 and passed. Status: **DONE, 1 gap open** (agent
self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | GAP | The shared Picker's props force each call site to name its groups or say it is a `choice` (`ui/src/components/controls/Picker.tsx`, a type union), and every picker of things passes groups (New session, Expert setup, a task's Start, the project's default App, Compose, the pool sheets, the related lists' filters). The helper is tested: `ui/src/lib/suggested.test.ts` "lists the open ones first, the most used of the rest next…". Missing: no test opens any call site's picker and checks its top group (Open now, Most used, Used in this project, Most here), and the K4 walk and the gallery's open pickers (both themes) aren't done. |
| R2 | PASS | `ui/src/components/controls/picker-groups.test.tsx` "lists the group first, then All with the empty ones last, leaving out what the group lists". |
| R3 | PASS | Every picker is the shared Picker (no raw `<select>` in `ui/src`). `ui/src/components/controls/controls.test.tsx` "opens from the keyboard, moves with the arrows, and Escape closes only the menu"; `picker-groups.test.tsx` "past 8 options the menu opens on a search field; typing filters, Enter picks the first match". |

Performance (opens ≤ 100 ms with 1 000 items): `picker-groups.test.tsx`
"draws at most 60 lines and says how many more typing finds" (windowed).

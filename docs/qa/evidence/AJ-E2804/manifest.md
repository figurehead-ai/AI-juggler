# Evidence manifest — AJ-E2804 The Projects screen

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2804, judged against the
requirements as amended 2026-11-06 (R2, R3 and R6: paged in place over the
project list the app keeps, no separate backend summary, so the planned
`projects_summary` cargo test and the load-gate's backend page time no
longer apply). The screen is `ui/src/screens/projects/ProjectsScreen.tsx`
and `ProjectRow.tsx`, its facts from `ui/src/lib/project-summary.ts`; built in
0.3 (e0645f9dc, shipped in ca2247e04) and finished on `release/0.4.1`
(f752aba1d; tests moved to their own file in 872f44f0a); re-checked against
the code, its tests and the committed gallery, 2026-11-06.

Run for this manifest (one command):
`cd ui && pnpm -s exec vitest run src/screens/projects/projects-screen.test.tsx src/screens/nav-screens.test.tsx src/lib/working-count.test.ts`
→ 3 files, 27 tests passed. Playwright was not run here: its tests run in the
gallery gate, and the captures cited are the committed ones.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 each project one line: mark and name, folder (home as `~`), what needs you ("1 asks you"), sessions working, open tasks, this week's tokens | `ProjectRow.tsx` (one grid: Project · Needs you · Sessions · Tasks · This week); vitest `screens/nav-screens.test.tsx`: "says its needs, sessions and tasks in plain words" ("2 ask you", "2 working · 3 open", "4 ready · 1 in review"), "lists every project, the most recently worked first, with its facts; a row opens its page"; `lib/working-count.test.ts`: "a project counts a starting session as working"; Playwright `nav-gallery.spec.ts` "Projects"; `_gallery/97-nav/projects-1280x800-dark.png` | every fact on the row | tests pass; the capture shows the mark, name, `~/…` folder, need chips ("1 asks you"), "1 working · 3 open", tasks and This week's tokens (31M…) on each row | PASS | e0645f9dc | 2026-11-06 | Claude (self) |
| R2 sorted by recent activity; search by name or folder; paged 40 at a time over the app's project list, sorted and searched in place (amended) | `ProjectsScreen.tsx` (`projectsShown`: filter on name or root, sort by last worked; `PROJECTS_PAGE = 40`); vitest `screens/nav-screens.test.tsx`: "lists every project, the most recently worked first, with its facts; a row opens its page", "searches by name or folder, says when nothing matches, and offers New project", "shows a page at a time, and skeleton rows while the projects are read" (40, then Show more) | 40 a page; sorted; name and folder match | all pass | PASS | e0645f9dc | 2026-11-06 | Claude (self) |
| R3 instant: the list shows at once from the projects the app keeps in memory and refreshes on the backend's events; no first-open skeleton (amended) | `shell/screens.tsx:20` (the screen gets `shell.shownProjects`, held in memory; `loading` only until the shell's own first load); `ProjectsScreen.tsx` `useProjectTasks` re-reads on `onTasksChanged`; vitest `screens/nav-screens.test.tsx`: "lists every project, the most recently worked first, with its facts; a row opens its page" (rows on the first render, no wait) | rows on first render | passes; no read stands between opening the screen and its rows | PASS | e0645f9dc | 2026-11-06 | Claude (self) |
| R4 Enter or a click opens the project page; the row's ⋯ / right-click / ⇧F10 menu is the shared project menu (amended), icons on every line | vitest `screens/projects/projects-screen.test.tsx`: "gives each row the project's own menu: ⋯, right-click and ⇧F10 (AJ-E2804 R4)", rendered under the app's own action host (Open its page, Open in Finder, New session here, Copy path, Rename…, Pin to top, Forget…; Open its page and Rename… run; an icon on every line); `components/project-manage.test.tsx` (the rail's menu, the same list); `screens/nav-screens.test.tsx`: "lists every project, the most recently worked first, with its facts; a row opens its page", "moves between rows with the arrow keys". Walked by hand: `docs/qa/evidence/E28-walk-2026-11-07.md`. | all three openers; the same menu as the rail; icons | pass; the walk found the screen's menu had two lines (its host was the screen's own), fixed in 5485a076c and walked again: ten lines, as the rail | PASS | 5485a076c | 2026-11-07 | Claude (self) |
| R5 empty: "No projects yet. A project appears when a session runs in its folder." with New project… (⌘⇧N) | vitest `screens/projects/projects-screen.test.tsx`: "says how a project appears when there are none, and offers New project… (AJ-E2804 R5)" (the words, the button, its ⌘⇧N tooltip) | exact words; button with its key | passes | PASS | f752aba1d | 2026-11-06 | Claude (self) |
| R6 no separate summary call: a line's facts come from the sessions, needs and tasks the app already holds (one count of what needs you, D-49) (amended) | `ProjectsScreen.tsx` (needs from `useBackendNeeds` via `needsByProject`, sessions via `openByProject`, tasks one `tasksProjectCounts` read per page, tokens from the usage history); vitest `screens/nav-screens.test.tsx`: "says its needs, sessions and tasks in plain words", "lists every project, the most recently worked first, with its facts; a row opens its page" (tasks read for the page in one call); `lib/working-count.test.ts`: "a project counts a starting session as working"; `_gallery/97-nav/projects-1280x800-dark.png` | no second source of needs | tests pass; in the capture the rows' needs (5) match the rail's Needs you 5 | PASS | e0645f9dc | 2026-11-06 | Claude (self) |
| Gate: the screen's tests | the vitest command above | pass | 27 / 27 pass | PASS | f752aba1d | 2026-11-06 | Claude (self) |

## Binary artifacts

None added; cited: `_gallery/97-nav/projects-*.png`,
`_gallery/97-nav/projects-search-empty-*.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)

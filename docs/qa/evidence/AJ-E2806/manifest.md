# Evidence manifest — AJ-E2806 The Tasks screen: a board and the plan across every project

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2806, judged against the
requirements as amended 2026-11-06 (the holder as the session's App, "No one
yet"; Approve and Send back… on the card and in its panel; the plan as a native
outline with the treegrid's keys). Built in 0.3 (e0645f9dc, the screen;
f9120a667, the backend's read across pools) and finished on `release/0.4.1`
(a2eb5489d, 555b243a9, 9bf68f753, 49bf3e49c, a28240e39); re-checked against the
code, its tests and the gallery, 2026-11-06.

The screen: `ui/src/screens/tasks/TasksScreen.tsx`, `TaskBoard.tsx`,
`TaskCard.tsx`; the plan `ui/src/pages/tasks/TaskTree.tsx`; the backend
`crates/juggler-app/src/features/tasks/across.rs` and `needs.rs`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 Board columns Ready, In progress, In review, Blocked with counts; To do, Done and Cancelled one click away | `pnpm -C ui exec vitest run src/screens/nav-screens.test.tsx`: "shows the board across pools, each column with its count and its epic › story", "shows To do, Done and Cancelled one click away, each its column, kept per Mac (AJ-E2806 R1)"; `_gallery/97-nav/tasks-board-1280x800-dark.png` | four columns with counts; the other three one click each | pass: "Ready2"; Done shows on one click ("Nothing done yet."), To do and Done follow the open columns, kept in `aij.tasks.more` | PASS | a2eb5489d | 2026-11-06 | Claude (self) |
| R2 a card: epic › story, title, project with its holder as the App ("No one yet" when none); Blocked says what it waits on; In review offers Approve and Send back… on the card and in its panel | `pnpm -C ui exec vitest run src/screens/tasks/tasks-board.test.tsx`: "says who has a task and the App they work in", "offers Approve and Send back… on a card in review"; `src/screens/nav-screens.test.tsx`: "shows the board across pools, each column with its count and its epic › story" ("Checkout v2 › Totals", "Waits on:Refresh-token retries"), "opens a card beside the board with the pool's own actions; Esc closes it"; `TaskCard.tsx:21` ("No one yet"); `_gallery/97-nav/tasks-board-1280x800-dark.png` | every part on the card | pass: "Builder · <App>"; Approve from the card calls `approve:aj-0003`, Send back… opens its note; the panel has `task-approve` and `task-send-back-open`; the capture shows "No one yet" on the unheld Ready cards | PASS | 555b243a9 | 2026-11-06 | Claude (self) |
| R3 Plan: epics → stories → tasks as a native outline (nested lists with aria-level), folded below the first level, arrows move and fold, ↩ opens | `pnpm -C ui exec vitest run src/pages/tasks/task-tree.test.tsx`: "names its rows' levels and folds, and moves with the arrows"; `src/screens/nav-screens.test.tsx`: "shows the plan by pool: each pool's epics and stories"; `cargo test -p juggler-app --test tasks the_lead_plans_an_epic_with_stories_and_the_user_sees_the_tree`; `_gallery/97-nav/tasks-plan-1280x800-dark.png` | levels named; story folded; ↓, →, ← as specced | vitest pass: levels `["1","2"]`, the story folded, ↓ moves, → unfolds, ← folds then goes up; the cargo test runs in CI / the gallery gate | PASS | 9bf68f753 | 2026-11-06 | Claude (self) |
| R4 filters as pickers (Project, Pool, Holder) and a search over titles and briefs | `pnpm -C ui exec vitest run src/screens/nav-screens.test.tsx`: "filters by holder from a picker (no system select) and links each card's project"; `cargo test -p juggler-app --test tasks the_tasks_screen_reads_every_pool_by_filter` (the brief's word "carousel", one pool, a project, the holder "nobody"); `_gallery/97-nav/tasks-holder-menu-1280x800-dark.png`, `tasks-board-1280x800-dark.png` (Project, Pool, Holder pickers and Search tasks) | no system select; brief searched | vitest pass (no `select` on the screen; Holder "nobody" empties In progress); the cargo test runs in CI / the gallery gate | PASS | 49bf3e49c | 2026-11-06 | Claude (self) |
| R5 only D-44's four actions (Approve / Send back…, Free, Cancel…, Start a session for this task); no create or edit | `pnpm -C ui exec vitest run src/pages/tasks/tasks.test.tsx`: "writes no task: no New task, no field to change, links read only (D-44)", "a task in review: you approve it, and your approval is in its history", "a task in review: you send it back with a note for its holder", "frees only a task whose holder's session ended; a running holder's has no Free", "cancels an open task only after you say so; a closed one offers nothing", "starts a session for a task only on your pick: your project's default, never by usage", "the IPC module offers no way to create, edit, assign or link a task"; `src/screens/nav-screens.test.tsx`: "shows the board across pools, each column with its count and its epic › story" (no "New task", "Add task", "Edit task") | four actions, no authoring | pass | PASS | e0645f9dc | 2026-11-06 | Claude (self) |
| R6 a backend list across pools, paged by keyset cursor per column; counts in one call | `cargo test -p juggler-core --lib across`: `a_column_across_pools_pages_by_its_cursor`, `across_pools_lists_and_counts_what_each_filter_names`; `TaskBoard.tsx` one `usePagedList` per status over `tasksAcross` | a second page after the first's last, no repeat, no gap; counts in one call | 2/2 pass: 7 ready tasks in two pools page 4 + 3, every one once, nothing past the last; counts in one call | PASS | 5572e4f7b | 2026-11-07 | Claude (self) |
| R7 review items stay in Needs you too, one count (D-49) | `pnpm -C ui exec vitest run src/components/task-needs.test.tsx`: "says so by its title and opens it beside its pool's tasks"; `cargo test -p juggler-app --test tasks the_lead_plans_and_gives_tasks_out_and_members_work_what_they_hold` (the AJ-E2806 R7 block: one `review` need, by title, gone when done) | once, by title | vitest pass ("«Checkout copy pass» waits on your review", opens its pool's Tasks on it); the cargo test runs in CI / the gallery gate | PASS | a28240e39 | 2026-11-06 | Claude (self) |
| Gate: the touched screens' tests | `cd ui && pnpm -s exec vitest run src/screens/nav-screens.test.tsx src/screens/tasks/tasks-board.test.tsx src/pages/tasks/task-tree.test.tsx src/components/task-needs.test.tsx src/pages/tasks/tasks.test.tsx`; `cargo test -p juggler-app --test tasks` and `nav-gallery.spec.ts` run in CI / the gallery gate | 0 failures | 5 files, 48 tests passed (2026-11-06 run); cargo and Playwright not run here | PASS | 9bf68f753 | 2026-11-06 | Claude (self) |

## Binary artifacts

None added; cited: `_gallery/97-nav/tasks-board-*.png`, `tasks-plan-*.png`,
`tasks-holder-menu-*.png`, `tasks-open-review-*.png`. They predate 555b243a9
(Approve and Send back… on the card itself); that part is held by
`tasks-board.test.tsx`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)

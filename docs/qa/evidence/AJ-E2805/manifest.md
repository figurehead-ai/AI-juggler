# Evidence manifest — AJ-E2805 The project page: Overview · Sessions · Tasks · Pool · Subagents · Usage · History

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2805, judged against the
requirements as amended 2026-11-06 (tabs Overview · Sessions n · Apps n ·
Tasks n · Pools n · Subagents · Usage · History; Apps used as the fourth
figure; the Pools tab lists the project's pools with Open, the canvas a
click away). Built in 0.3 (e0645f9dc, the project page; 9e4d19fa4, AJ-E3104's
Apps and Pools tabs) and finished on `release/0.4.1` (7f472ab9a, 029427998);
re-checked against the code, its tests and the gallery, 2026-11-06.

The page lives in `ui/src/pages/ProjectPage.tsx` (head, Overview, Sessions),
`ui/src/pages/project/ProjectTabs.tsx` (the tabs, each heavy one mounted the
first time it shows), `ui/src/pages/project/ProjectChips.tsx` (the pills) and
`ui/src/pages/registry.ts` (the tab list).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the head: "Projects › <name>", the folder, branch and upstream as its facts, the name, and pills for what needs you and what's working | `pnpm -C ui exec vitest run src/components/inspector-shell.test.tsx`: "a project's header reads its root's git state: branch, ahead/behind, not committed"; `src/components/inspector-aggregate.test.tsx`: "the header says where it is and its branch; Overview, its open sessions, tasks and accounts"; `src/screens/nav-screens.test.tsx`: "says its needs, sessions and tasks in plain words" (the pills' words, `needChips`); `pages-gallery.spec.ts` checks `crumb-projects`; `_gallery/79-pages/project-overview-1280x800-dark.png` | crumb, folder, branch, upstream, need and working pills | tests pass ("· trunk", "· 2 ahead", "· 3 files not committed"; "2 ask you", "1 to review"); the capture shows "Projects › payments-api", the folder and "feat/read-model", and the pills "1 asks you" and "1 working" | PASS | e0645f9dc | 2026-11-06 | Claude (self) |
| R2 the tabs Overview · Sessions n · Apps n · Tasks n · Pools n · Subagents · Usage · History, Overview first; every tab stays when empty and says so; Subagents carries no count | `pnpm -C ui exec vitest run src/screens/nav-screens.test.tsx`: "a project's page has its eight tabs, Overview first"; `src/components/inspector-shell.test.tsx`: "a project's page: its type, its name, and only the tabs with something to show" (project row: all eight with nothing in Apps, Tasks, Pools), "a project's Usage tab with nothing measured says so in one line (AJ-E2805 R2)"; `src/components/inspector-aggregate.test.tsx`: "a project with no sessions renders the Start-first-session CTA" | eight tabs in order, counts where amended, none dropped when empty | pass: labels and counts `[Overview, Sessions 3, Apps 2, Tasks 12, Pools 2, Subagents, Usage, History]`; with nothing measured the tabs stay and Usage says "No usage in this project yet. It shows here once a session in its folder has run." | PASS | 7f472ab9a | 2026-11-06 | Claude (self) |
| R3 Overview's four figures: open sessions and apps, tasks with ready and in review, this week's tokens, Apps used with the account named on each; the existing callouts | `pnpm -C ui exec vitest run src/components/inspector-aggregate.test.tsx`: "the header says where it is and its branch; Overview, its open sessions, tasks and accounts" (`project-summary`, `tile-accounts` names "Work Max", `tile-tasks-note`); `src/screens/nav-screens.test.tsx`: "says its needs, sessions and tasks in plain words" (`taskWords` "4 ready · 1 in review"); `_gallery/79-pages/project-overview-1280x800-dark.png` | four figures in one row | pass; the capture shows Open sessions "4 · 1 app · 1 needs you · 1 working", Tasks, Tokens this week with its trend, Apps used "7" with the apps named | PASS | a6756c0e1 | 2026-11-06 | Claude (self) |
| R4 Tasks = the project's pool's tasks; Pools lists the project's pool and yours with Open (the canvas on its own page); Subagents = its sessions' subagents; History = its ended sessions, paged | `ProjectTabs.tsx` (Tasks: `PoolTasks` on `defaultPoolIdOf(project.id)`; History: `History projectId=`); `pnpm -C ui exec vitest run src/screens/nav-screens.test.tsx`: "its Pools tab sums its own pool up and opens the pool's page", "its Subagents tab is the project-wide Subagents view over its sessions"; `src/screens/history-paged.test.tsx`: "shows a skeleton at once, then the first page, never every session"; `nav-gallery.spec.ts` "a project's {Tasks,Pool,Subagents,History}"; `_gallery/97-nav/project-{tasks,pool,subagents,history}-1280x800-{dark,light}.png` | each tab reads only its project; History paged | pass: Open calls `pool:pool-default:<project>`; the subagents read is `subagentsProject(<project id>)`; History reads a page, never all; the Tasks capture shows the project's pool's 3 open tasks with their filters | PASS | 9e4d19fa4 | 2026-11-06 | Claude (self) |
| R5 the remembered tab per page type (AJ-E1534) keeps working with the new tabs | `pnpm -C ui exec vitest run src/components/inspector-shell.test.tsx`: "a project's page opens on the tab you last chose on one, the new tabs too (AJ-E1534, AJ-E2805 R5)", "restores a type's last tab, never carrying it to another type" | Subagents, Usage, Pools, Apps each reopen selected | pass for all four new tabs | PASS | 029427998 | 2026-11-06 | Claude (self) |
| Gate: the touched screens' tests | `cd ui && pnpm -s exec vitest run src/components/inspector-shell.test.tsx src/screens/nav-screens.test.tsx src/components/inspector-aggregate.test.tsx src/screens/history-paged.test.tsx`; the gallery family (`nav-gallery.spec.ts`, `pages-gallery.spec.ts`) runs in CI / the gallery gate | 0 failures | 4 files, 60 tests passed (2026-11-06 run); Playwright not run here | PASS | 029427998 | 2026-11-06 | Claude (self) |

## Binary artifacts

None added; cited: `_gallery/79-pages/project-overview-*.png`,
`_gallery/97-nav/project-{overview,tasks,pool,subagents,history}-1280x800-*.png`.
These captures predate AJ-E3104's Apps tab (the tab row in them shows Pool, not
Apps and Pools); the tab list itself is held by the vitest rows above.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)

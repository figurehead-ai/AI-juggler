# Evidence manifest — AJ-E3104 Header chips and counted tabs

Task: `docs/tasks/E31-everything-links.md` AJ-E3104, every page leads to
what's related: header chips and counted tabs on project, session, App,
Account, pool, Mac and Tool. Judged against the tab table in
`docs/research/relations-map.md` §4 (the requirement's own reference) and
today's code on `release/0.4.1`, 2026-11-07 (the header's tab row and the
rail's App headers as changed since by Back/Forward and the App headers).

Vitest run 2026-11-07: `cd ui && pnpm -s exec vitest run
src/lib/notice-actions.test.tsx src/lib/home-needs-unloaded.test.ts
src/count-agreement.test.tsx src/pages/machine/this-mac-tabs.test.tsx
src/app-tool-chip.test.tsx src/pages/session/session-chips.test.tsx
src/pages/session/session-pools.test.tsx src/screens/nav-screens.test.tsx
src/pages/app/app-page.test.tsx src/screens/settings/accounts-pane-tools.test.tsx
src/lib/relation-counts.test.tsx`: 11 files, 53 tests, all passed. Cargo
tests are cited as written and run in CI.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a relation to one thing is a header chip; to many, a tab with its count, per the tab table (the project's Pool tab becomes Pools) | `ui/src/pages/registry.ts` `tabsOf`; vitest `src/screens/nav-screens.test.tsx` "a project's page has its eight tabs, Overview first"; `src/pages/machine/this-mac-tabs.test.tsx` "opens on its Overview, then Sessions, Apps, Projects and Pools", "counts its Apps as the Apps tab lists them"; `src/pages/app/app-page.test.tsx` "its header names the Mac it runs on, a door to This Mac, and says when its room is shared (AJ-E3204 R1)"; `src/app-tool-chip.test.tsx` "Enter on About Claude Code opens the tool's page, with focus on its tabs"; `src/pages/session/session-chips.test.tsx` "opens to the App, its account, the working folder and a door to each of their pages (A3)"; cargo `juggler-app`, `tests/relations.rs` `every_count_equals_the_list_it_heads_and_a_recount` (project, App, Account, Tool pages), `every_pool_counts_its_members_and_a_move_shows_in_the_next_count`; `juggler-core`, `tests/relations_storage.rs` `the_pools_stamp_moves_when_a_session_changes_pool` (run in CI); K1 walk `docs/qa/0.4-walks/K1.md`, `walks-0.4/K1/4-account-pools-1280x800-dark.png` (opened); gallery `_gallery/72-tool-view/overview-1280x800-dark.png` (opened) | every page's chips and counted tabs as the table names them | most of the table is built and counted (see "R1 page by page" below): Project Pools replaces Pool, Apps added; App, Account, Tool and This Mac carry counted Sessions/Apps/Projects/Pools; the backend counts equal their lists. Not built as the table says: the session's Pools tab has no count and its header has no Mac or Work item chip; the project header has no default-App or Macs chip; the pool header has no scope or sealed chip; the App's Launch settings tab has no count; This Mac's page has no Processes or Actions tab. No vitest asserts the App, Account or Tool tab counts in the UI **Closed 2026-11-07:** built as the table names them ("R1 page by page" below, updated): a session's Pools tab counted from the read its tab lists (3b395e2f6; vitest `src/pages/session/session-pools.test.tsx` "its Pools tab counts the pools the tab lists, from the same read", `src/components/inspector-instance.test.tsx` "its Pools tab carries its own count, never its project's" — Pools 2 where its project would count 3); a CLI App's Launch settings counted (3b395e2f6; `src/pages/app/app-page.test.tsx` "…counted; a desktop App doesn't": Launch settings 1, its own set only); a session here has a This Mac door after where it runs (9b6b9c96b, bd87da027; inspector-instance "a session on this Mac has a door to This Mac"); a project's header doors to its default App ("starts in …") and, worked on more than one Mac, to Fleet ("on 2 Macs") (9b6b9c96b, e14df502a; `src/pages/project-default-app.test.tsx`, 3 tests); a pool you made shows its scope as a chip that opens its scope sheet, and sealed (9b6b9c96b; `src/pages/pool/canvas-answers.test.tsx` "the pool's header chips", 2 tests; `src/lib/pools-scope.test.ts`); This Mac's Processes tab (9b6b9c96b; `src/pages/machine/this-mac-tabs.test.tsx`). Looked at in the preview at 1280 and 1024 (dark): the pool header's chips beside its name; "Running in AI Juggler · This Mac · started …" on one line at 1280, the pool door wrapping at 1024; "… · starts in Claude Code · Work Max"; This Mac's six tabs. Two cells are not chips, each for a reason recorded below: the session's Work item (no page: `docs/research/relations-map.md` row "Work item ↔ project / profile / Mac" names its surface as the session's History) and This Mac's Actions (a paired Mac's Actions tab only points to Fleet; this Mac has no actions of its own, and an empty tab is ruled out, AJ-E1533) | PASS | e14df502a | 2026-11-07 | Claude (self) |
| R2 every notice opens the right page and tab; Needs you counts the same notices as the bell | vitest `src/lib/notice-actions.test.tsx` "a pool's message opens the pool on Messages, at its thread, not its sender's page", "a task opens the pool on Tasks, with or without the task's id", "a session working hard opens on Processes; a restart on History; one asking on its body", "another app's session not loaded yet opens by its ids, on Overview, not its App", "a sign-in opens its App on Overview, where its sign-in card is"; `src/count-agreement.test.tsx` "the bell, Home and the rail show one Needs you count, the model's own needs included"; `src/lib/home-needs-unloaded.test.ts` "one of AI Juggler's shows from the notice and opens its session", "another app's opens by its account and conversation"; K3 walk `docs/qa/0.4-walks/K3.md` (quota, failed, limit, sign-in, stuck opened live; Needs you 5 = bell 5 at 1280 and 1024), `walks-0.4/K3/K3-2-bell-light-1280.png` (opened) | each kind on its page and tab; one count | every tested kind lands on its page and tab; bell = Home = rail in the test and the walk. No test or walk for: an Account's renewal opening its Settings and its limits its Overview (`notice-actions.ts`, the `target.account_id` branch, `openAccount(account, notice.kind === "renewal" ? "settings" : "overview")`); "Nothing to open for this one" (same file, the final `showToast`); Home's heading leaving out snoozed caps, messages and tasks (`useSnoozedSubjects`, `lib/needs-you.ts`, used in `screens/Home.tsx`); the rail's Project lens counting the bell's other needs (`RailSections.tsx`; `rail-sections.test.tsx` "keeps Needs you and its count whatever the grouping (AJ-E2802 R7)" seeds session needs only) **Closed 2026-11-07:** vitest `src/lib/notice-actions.test.tsx` "an Account's renewal opens its page on Settings; its limits on Overview" (the Account, never the App) and "a notice whose subject is gone says so, and opens nothing"; `src/lib/home-needs-snoozed.test.ts` (Home's own rows drop a snoozed cap, message and task through `notSnoozed`, which Home's heading counts; a snooze counts until it ends, one for good always); `src/components/rail-sections.test.tsx` "counts and shows the bell's other needs in every grouping, the Project lens too" (a sign-in and an agent's message: Needs you 4 and 2 notice rows in the account, App and Project lenses) | PASS | 9bc81eee7 | 2026-11-07 | Claude (self) |
| R3 a session's Pools tab says how it got into each pool and which it's skipped from (AJ-E3004 is DONE, so the full form applies) | vitest `src/pages/session/session-pools.test.tsx` "says how it got in, for every kind of member", "lists each pool with its role, then what left it out and why"; cargo `juggler-core`, `tests/pools_resolver.rs` `a_project_pool_skips_sessions_named_elsewhere_but_not_ones_a_broad_member_takes`, `an_unknown_sign_in_stays_in_through_any_other_member_and_says_so` (run in CI) | each pool with how and role; each skip with reason and step | tests pass: two pools with "Through its App" and its level, "Left out of" with "Add the project to its scope", the unknown-sign-in line; the resolver decides via and skip | PASS | 3eae09e4d | 2026-11-07 | Claude (self) |
| Gate: the touched screens' tests | the vitest command above | pass | 53 / 53 passed (2026-11-07) | PASS | 9e4d19fa4 | 2026-11-07 | Claude (self) |

## R1 page by page (today's code against relations-map §4)

| Page | Chips the table asks | Built | Tabs the table asks (counted) | Built |
|---|---|---|---|---|
| Project | default profile; Macs (Fleet) | both as doors on its facts line: "starts in" its default App, and "on n Macs" (Fleet) once more than one (`ProjectPage.tsx` `ProjectFacts`) | Overview · Sessions (n) · Apps (n) · Pools (n) · Tasks · Subagents · Usage · History | all; Sessions, Apps, Tasks, Pools counted (`useProjectTabCounts`, `ProjectTabs.tsx`); nav-screens test |
| Session | project · profile · account · Mac · Work item | project (the crumb), App and Account (the App menu's doors, `SessionChips`), its pool and tool on the facts line; This Mac's door after where it runs (another Mac's chip for a remote one); Work item: its History (see Closed) | … · Pools (n) · … | Pools counted from the tab's own read (`useSessionPools`, `session-tabs.tsx`); the counts follow the page kind (`RightInspector.tsx`) |
| App | app · account · Mac | tool door, Account door, Mac door (`AppPage.tsx`: `SessionToolChip`, `AccountDoor`, `MacDoor`); app-page and app-tool-chip tests | Sessions (n) · Projects (n) · Pools (n) · Launch settings (n) · Usage | Sessions, Projects, Pools and Launch settings counted (`AppPage.tsx`; launch settings from `useLaunchSettings`, the tab's own read) |
| Account | plan | named by its plan, room/renewal/plan-choice chips (`SubscriptionPage.tsx`) | Apps (n) · Sessions (n) · Projects (n) · Pools (n) · Usage | all counted (`SubscriptionPage.tsx` `pageTabs("subscription", … counts)`); K1 capture shows Pools 2. `_gallery/99-account-page/account-overview-*.png` predates the Pools tab |
| Pool | scope · sealed | both: a Scope chip that opens its scope sheet, and sealed (`pool/PoolHeader.tsx`) | Canvas · Tasks · Messages (unchanged, D-55) | as before |
| Mac | — | — | Overview · Sessions (n) · Profiles (n) · Projects (n) · Pools (n) · Processes · Actions | This Mac: Overview, Sessions, Apps, Projects, Pools, counted (`ThisMacPage.tsx`, this-mac-tabs test); Processes added (`ThisMacPage.tsx`); no Actions tab (see Closed). `_gallery/81-this-mac/` predates the tabs |
| Tool | — | — | Overview · Profiles (n) · Sessions (n) · Projects (n) · Pools (n) | all, counted (`ToolTabs.tsx` `useToolTabCounts`); relations.rs Tool pages; `_gallery/72-tool-view/overview-1280x800-dark.png` shows Apps 2 · Sessions 9 · Projects 2 · Pools 2 |

## Closed (2026-11-07)

Every GAP row above is closed by the commits it names. Two cells of the
table are deliberately not chips:

- **The session's Work item.** A line of work is the switch graph's
  (`juggler-core` `switch_graph`); it has no page and no read in the UI.
  The relations map's own row for it names its surface: the session's
  History, which shows the line of work. A chip would open nothing new.
- **This Mac's Actions tab.** A paired Mac's Actions tab says only that its
  pairing lives in Fleet; this Mac has no actions of its own (its
  processes' actions are on its Processes tab, built here), and a tab with
  nothing on it is ruled out (AJ-E1533).

Still open, outside R1–R3: the gallery for the Account page's Pools tab,
This Mac's tabs and the new header chips, both themes, comes with the 0.4.1
session's single re-baseline (what changed is in the notes file).

## Walks

K1 (the chain, both ways: `docs/qa/0.4-walks/K1.md`, 2026-11-06,
re-measured 2026-11-07) and K3 (notices land: `docs/qa/0.4-walks/K3.md`,
2026-11-06) are on file with captures under `docs/qa/evidence/walks-0.4/`.

## Binary artifacts

None added; cited: `walks-0.4/K1/4-account-pools-1280x800-dark.png`,
`walks-0.4/K3/K3-2-bell-light-1280.png`,
`_gallery/72-tool-view/overview-1280x800-dark.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none; lab and fixture names only)

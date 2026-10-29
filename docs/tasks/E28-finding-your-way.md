# E28 — Finding your way: projects, pools, tasks and subagents

> Source: the maintainer, 2026-10-29, on the 0.2.0 walk (verbatim):
> - Subagents: "Subagents are really poorly implemented … it is listing
>   437 subagents … it makes no sense to show all of them as one. We need
>   to have filters for currently active, history of, search them … some
>   of the subagents are indented ??? what does that mean ??? … if so we
>   should have tree like lines. Also we should have better breadcrumbs for
>   subagent, also we should have section for subagent usage and processes
>   too … Remember these subagents need to be useful, for users, think why
>   would users want to view subagents in our app ?? Possibly to see token
>   usage, resource usage, what processes each subagent started. Also we
>   should have a summary of subagent is doing. maybe original prompt
>   given."
> - Projects: "Also where is the projects ??? … We should have some way to
>   group sessions also by projects. maybe that is what we are missing a
>   top filter or a tab or flag, that switching grouping between : Account,
>   App (current) and project … Also Where is tasks/ epics ???"
> - The View menu: "this screen is to bland, I mean look at it, ugly what
>   happened with UI/UX quality reviews ??"
> - Pools: "we need to have dedicated section for pools. BEcause pool page
>   should have dual purpose, a view of all of the apps in pools, showing
>   them, their statuses, and configuration of pools. Pools requires a
>   really very interactive view. I am not saying you shouldn't have group
>   by pools, keep that … if someone creates a pool, they want to see
>   interactions, how they configured in the pool, alerts within the pool"
>   and "In same place where you view you should create, edit manage pool
>   items and same view, there is no reason to have different screens.
>   Think how diagram dashboards are."
> - The rulings on the design: "Put the whole redesign in 0.3. Header: OK
>   as proposed. Pool canvas: I arrange nodes and it remembers, with an
>   automatic first layout."
>
> The approved design (the approval record, mock-ups included):
> [`docs/design/finding-your-way-2026-10-29.html`](../design/finding-your-way-2026-10-29.html)
> — §0 what's wrong, §1 navigation, §2 the View menu and the shared
> controls, §3 Projects, §4 Tasks, §5 Pools, §6 Subagents, §7 honest data
> per tool, §8 how it is built and checked.
> Decisions: [D-54](../review/decision-log.md) (finding your way), D-55
> (the pool canvas), D-56 (subagents), D-57 (the design-quality review).
> Read before any task here: the design above; `docs/wireframes/index.html`
> (screens 01, 03, 05); `docs/review/deep/11-ux-redesign-screen-spec.md`
> §1, §2, §3, §4.2, §4.3, §8, §9, §10; `docs/SPEC.md` §9; D-25 (plain
> words), D-28..D-33 (pools, messages, access), D-36 (places), D-44 (agents
> write tasks), D-47..D-49, D-52 and D-53.
>
> **Order.** The shared controls (AJ-E2801) come first, so every screen
> after them is consistent from the start. Then in parallel: navigation
> (AJ-E2802–E2805), Tasks (AJ-E2806), Pools (AJ-E2807–E2808) and Subagents
> (AJ-E2809–E2812). The sweep's fixes (AJ-E2813) and the review process
> (AJ-E2814) run alongside and close the epic. All of it ships in 0.3.
>
> **Law for every task here:**
> - **Design quality is part of done (D-57).** One alignment grid; a clear
>   hierarchy (section titles, options, hints); consistent spacing; icons
>   on menu and action lines (the app's own set, `ui/src/components/icons*.tsx`,
>   new ones in the same stroke style); the shared controls, never a
>   system select, radio or checkbox; disabled states that say why; plain
>   words, no internal ids (D-25); body 14 px and nothing under 12 px; both
>   themes. Each screen's critique sits beside its captures (AJ-E2814).
> - **Desktop first, instant and paged.** Cached or skeleton data at once;
>   every growing list paged by keyset cursor or windowed, never loaded
>   whole; keyboard and screen readers (roles, focus, Esc, arrows in lists
>   and trees).
> - **Agents write tasks; only the user changes pools** (D-44, D-28, D-31).
>   No screen here authors a task or lets an agent change a pool; adding a
>   session to a pool never starts one.
> - **Tool specifics only in adapters** (ADR-0002); macOS specifics behind
>   `juggler-platform`; nothing here is macOS-only (D-52).
> - **Honest data.** A number the tool doesn't record isn't shown as if it
>   were; "probably" and "estimated" are said where they apply (D-56).
> - **Real accounts, read-only.** Live checks read the tools' records;
>   nothing signs in, answers an Allow, or reads a credential or a
>   process environment.

## Step 1 — shared controls

### AJ-E2801 Shared controls: choice line, checkbox, radio and picker | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort M | Depends: —
**Goal**: One set of controls every screen uses, so a choice looks and behaves the same everywhere: a choice line, a checkbox, a radio and a picker, replacing the system controls.
**User story**: As someone moving between the View menu, the stop sheet and Settings, I see the same choice drawn the same way, with an icon, a hint under it and a ✓, and never a grey system popup in the middle of the app's own look.
**Requirements**:
1. `ChoiceLine` SHALL draw an icon, a label, a hint under it and ✓ on the right when chosen, in one grid (icon column, text column, mark column), promoted from the rail's order menu (`RailOrderMenu.tsx`); it SHALL work as a `menuitemradio` / `menuitemcheckbox` in a menu and as a `radio` / `checkbox` in a form.
2. `Checkbox` and `Radio` SHALL use the accent colour, show focus with the app's ring, and have a disabled state that is visibly off (not "selected and grey") and carries its reason as text or a tooltip read by screen readers.
3. `Picker` SHALL show the value and a chevron and open a menu of `ChoiceLine`s with ✓ on the current one (the `RowMenu` / `MenuItem` / `MenuSubmenu` patterns: `aria-haspopup="menu"`, arrows, type-ahead, Home/End, Esc returns focus); it SHALL replace `form.tsx`'s `Select`.
4. The system selects SHALL be replaced (about 21 today: How it runs, History's filters, Open projects in, Add account, …) and the system checkboxes and radios (about 38 files), including the stop sheet's "What to stop" (`KillSheet.tsx:444`) and Settings' choices.
5. A lint SHALL fail a new `<select>`, `type="checkbox"` or `type="radio"` in shipped UI code outside the shared controls, with an allowlist that only shrinks (the `plain_words.allow` pattern).
6. Both themes; 12 px minimum; a hit area of at least 24×24 px (WCAG 2.2, 2.5.8).
**UI/UX reasoning**: Nielsen #4 (consistency and standards) and Apple HIG "Pop-up buttons" and "Menus" (a menu shows the current value with a checkmark); WCAG 2.2 1.4.11 (non-text contrast ≥ 3:1 for the box and the mark), 2.4.7 (focus visible), 4.1.2 (name, role, value). States: unchecked, checked, mixed (Checkbox only), disabled-unchecked, disabled-checked, focus, hover. Copy: a disabled line's reason reads as a hint ("Once a pool has members besides a project's own"). Keys: Space toggles, arrows move in a radio group, Enter or Space opens a picker.
**Performance budget**: a picker's menu opens in ≤ 16 ms (one frame) with 50 items; no layout shift when a choice changes (CLS 0 in the gallery state).
**Quality gates**: vitest for each control (roles, keys, disabled reason, focus return); the new lint in `just verify` with its `--selftest`; `just ui`; the a11y gate over every touched screen (axe 0 critical/serious); `scripts/size_budget.py`.
**Evidence/validation proof**: `cd ui && npx vitest run src/components/controls.test.tsx` all pass; `python3 scripts/native_controls_lint.py` exits 0 and its `--selftest` passes; the gallery family `controls` (every state, both themes) and the a11y gate's summary; manifest `docs/qa/evidence/AJ-E2801/manifest.md`; pass = 0 failures, 0 axe critical/serious, the allowlist at or below its first count.
**Visual testing**: a new `ui/tests/visual/controls-gallery.spec.ts` capturing each control in each state at 1280×800 and 1024×700, dark and light, plus each replaced screen's popover open (the picker's menu, the View menu, the stop sheet); compared with the approved baselines after the central render; axe on each.
**Files to touch**: new `ui/src/components/ChoiceLine.tsx`, `Checkbox.tsx`, `Radio.tsx`, `Picker.tsx`, `controls.test.tsx` (new); `ui/src/components/RowMenu.tsx`, `RailOrderMenu.tsx`, `RailOrderChoice.tsx`, `form.tsx`, `KillSheet.tsx`, `SettingSwitch.tsx`, `icons-menu.tsx`; the screens under `ui/src/screens/settings/` and `ui/src/screens/History.tsx`; new `scripts/native_controls_lint.py` and `scripts/native_controls.allow`; `justfile`.
**Research basis**: the design §2 ("The same fixes, everywhere"); `ui/src/components/ViewPopover.tsx:104-145` (system radios, checkboxes and selects in today's View menu); `ui/src/components/RailOrderChoice.tsx:20-29` (the order choice drawn with a system radio); `ui/src/components/KillSheet.tsx:444-445`; `ui/src/components/form.tsx:38` (`Select`); `ui/src/components/RowMenu.tsx:141,192` (`MenuItem`, `MenuSubmenu`); https://developer.apple.com/design/human-interface-guidelines/pop-up-buttons; https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/; https://www.w3.org/TR/WCAG22/#target-size-minimum.
**Handover note**: Being built on branch `controls-0.3`; the other 0.3 builders build on `MenuItem` / `MenuSubmenu` and `SettingSwitch` until it lands, and swap. Don't restyle `SettingSwitch` (an on/off switch is not a checkbox). A radio group needs one tab stop and arrows, not a tab stop per option; the browser gave that for free and the shared `Radio` must keep it.

## Step 2 — navigation

### AJ-E2802 The rail's grouping switch and a real View menu | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort M | Depends: AJ-E2801
**Goal**: Grouping by app, project, account or pool is a visible switch on the rail, and the View menu is a real menu: one grid, icons, the shared controls, Reset only when something changed.
**User story**: As someone with sessions in several projects, I switch the rail to Project in one click without hunting for an unlabelled icon, and the View menu reads like the rest of the app.
**Requirements**:
1. A segmented switch **App · Project · Account · Pool** SHALL sit above Needs you (`role="radiogroup"`, label "Group by"), App first and the default (D-36); it SHALL replace Group by in the View menu.
2. The choice SHALL be remembered per Mac (`lib/rail-view.ts`, the stored view) and restored at start.
3. **Pool** SHALL show once any pool has members besides a project's own (not greyed out "once you make a pool"); a saved Pool with no such pool shows App, as today.
4. The View menu SHALL have **Show** (Quiet ones — "Idle sessions a place folds away"; Ended today; Subagents — "How many each session runs"; Other Macs) and **Only** (App, Account, Project, each a `MenuSubmenu` of choice lines, never a system select); every line icon · label · hint · ✓.
5. **Reset** SHALL show in the menu's head only when the view differs from the defaults, and undo with a toast (as Reset my order does).
6. ⇧V SHALL still open the View menu; the switch is reachable by Tab and moves with the arrows.
7. The rail's sections, counts and Needs you SHALL be unchanged by the move (one count, D-49).
**UI/UX reasoning**: Nielsen #6 (recognition rather than recall: the grouping is visible, not behind an icon) and #4 (consistency: the menu matches the row menus); Apple HIG "Segmented controls" (2–5 short labels, mutually exclusive). States: App, Project, Account, Pool (shown or not), View menu default (no Reset) and changed (Reset). Copy: "Group by" (the switch's accessible name), "Reset", "All ›".
**Performance budget**: switching the grouping repaints the rail with 50 sessions in ≤ 100 ms (the rail budget is 300 ms for a cold render); the View menu opens in one frame.
**Quality gates**: vitest `src/components/rail-view-words.test.tsx`, `rail-sections.test.tsx`, `rail-keyboard.test.tsx` (extended), a new `rail-group-switch.test.tsx`; `just ui`; the a11y gate on the rail.
**Evidence/validation proof**: `cd ui && npx vitest run src/components/rail-` all pass; the gallery family `rail-switch-view` (each grouping, the View menu default and changed, an Only sub-menu open); manifest `docs/qa/evidence/AJ-E2802/manifest.md`; pass = 0 failures, 0 axe critical/serious, the switch's choice survives a reload in the Playwright run.
**Visual testing**: `ui/tests/visual/places-gallery.spec.ts` and `menus-gallery.spec.ts` extended (the View menu was never captured): each grouping and each menu state at 1280×800 and 1024×700, dark and light; the rail narrow (`rail-narrow.spec.ts`) with the switch not wrapping.
**Files to touch**: `ui/src/components/ViewPopover.tsx`, `RailToolbar.tsx`, `RailHeader.tsx`, `RailSections.tsx`, `rail-groups.tsx`, `WorkflowRail.tsx`, `icons-menu.tsx`; `ui/src/lib/rail-view.ts`, `rail-places.ts`, `pools.ts` (`hasUserPools`), `shortcuts.ts`; new `ui/src/components/RailGroupSwitch.tsx`.
**Research basis**: the design §1 (the switch, "Pool appears once…") and §2 (Today vs Proposed); `ui/src/components/ViewPopover.tsx:18` ("once you make a pool"), `:104-170`; `ui/src/lib/rail-view.ts:22-72` (`RailView`, `GROUPS`); `ui/src/lib/shortcuts.ts:136` (⇧V); D-36, D-47; https://developer.apple.com/design/human-interface-guidelines/segmented-controls.
**Handover note**: "App" is today's `place` grouping (an app and its profile); keep the stored value `place` so saved views don't reset. Don't drop the rail search's `project:` and `account:` words: the switch groups, the search filters.

### AJ-E2803 The header: Home · Projects · Pools · Tasks · Accounts · Usage · History | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort S | Depends: —
**Goal**: Projects, Pools and Tasks get a door in the header, and the page title folds into the highlighted tab to make room.
**User story**: As someone who wants their projects, pools or tasks, I find each one in the header, one click away, and I always know where I am from the highlighted tab.
**Requirements**:
1. `PRIMARY_NAV` SHALL be Home · Projects · Pools · Tasks · Accounts · Usage · History, in that order (D-54).
2. The header's separate `<h1>` title SHALL go; the highlighted tab carries `aria-current="page"`, and each screen keeps a visually hidden `<h1>` with its name.
3. A page below a screen (a project, a pool, a session, a subagent) SHALL show its breadcrumb in the page ("Projects › storefront"), each step a link, the last one plain.
4. The window's title SHALL be "AI Juggler · <screen>" ("AI Juggler · Projects").
5. Keys SHALL be added to the shortcut registry: ⌘⇧P Projects, ⌘⇧O Pools, ⌘⇧T Tasks (the agents' choice, free today; the maintainer to confirm, D-54); the ? sheet and ⌘K list them.
6. Processes, This Mac and Settings SHALL stay in your menu (AJ-E2507); New session, the bell and the avatar stay on the right.
7. The header SHALL NOT wrap at 1024×700, and no tab label SHALL truncate.
**UI/UX reasoning**: Nielsen #1 (visibility of system status: where am I) and #6 (recognition: the doors are visible); Apple HIG "Tab bars" (the selected tab is the location); WCAG 2.4.8 (location) and 2.4.6 (headings). States: each screen's tab highlighted; a page below a screen with its parent tab highlighted. Copy as the requirement; keys as listed.
**Performance budget**: switching screens paints the new screen's skeleton in ≤ 100 ms; the new screens' code loads lazily (`lazyParts.ts`), prefetched after first paint, adding ≤ 5 KB gzip to the first bundle.
**Quality gates**: vitest `src/lib/nav-focus.test.tsx`, `src/app-keys.test.tsx`, `src/components/palette-commands.test.tsx` (extended); Playwright `keyboard-flows.spec.ts` (the three keys); `just ui`; the bundle size check.
**Evidence/validation proof**: `cd ui && npx vitest run src/app-keys.test.tsx src/lib/nav-focus.test.tsx` pass; the gallery family `header` at 1024×700 and 1280×800 with each tab highlighted; manifest `docs/qa/evidence/AJ-E2803/manifest.md`; pass = 0 failures, no wrap at 1024 px (the spec asserts the nav's height equals one row).
**Visual testing**: `ui/tests/visual/pages-gallery.spec.ts` extended: the header on Home, Projects, Pools, Tasks and a project page, at 1024×700, 1280×800 and 1440×900, dark and light; the core visual check on Home's baseline.
**Files to touch**: `ui/src/lib/nav.ts`, `ui/src/components/MainNav.tsx`, `AppHeader.tsx`, `ui/src/App.tsx`, `ui/src/lazyParts.ts`, `ui/src/lib/shortcuts.ts`, `ui/src/lib/app-keys.ts`, `ui/src/lib/palette-commands.ts`, `ui/src/pages/ItemHeader.tsx`, `ui/src/pages/PageShell.tsx`.
**Research basis**: the design §1 ("Every important thing gets a door in the header"); `ui/src/lib/nav.ts:78-84` (today's four); `ui/src/components/AppHeader.tsx:60` (the `<h1>` title); `ui/src/lib/shortcuts.ts:138-143` (screen keys); D-25, D-50, AJ-E2507; https://developer.apple.com/design/human-interface-guidelines/tab-bars.
**Handover note**: The header already wrapped at 1280 px with the title; the fold is what makes room, so don't put the title back as a pill. ⌘⇧P is a common "command palette" key in editors, but AI Juggler's palette is ⌘K; say so in the ? sheet only if users ask.

### AJ-E2804 The Projects screen | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort M | Depends: AJ-E2803
**Goal**: A screen that lists every project the same way, with what needs you and what moves in each.
**User story**: As someone with a dozen projects, I see which ones need me, which are busy and how much each used this week, and open one in a click.
**Requirements**:
1. Each project SHALL be one line: its mark and name, its folder (home shortened to `~`), what needs you ("1 asks you"), sessions working, open tasks, this week's tokens.
2. Sorted by recent activity; a search by name or folder; paged by keyset cursor (50 a page), never all at once.
3. Instant: the last list from the cache at once, then refreshed; a skeleton on the first ever open.
4. Enter or a click opens the project page (AJ-E2805); the row's ⋯ / right-click / ⇧F10 menu is the shared row menu (Open, New session in this project ⌘N, Show in Finder, Rename…, Archive…), icons on every line.
5. Empty: "No projects yet. A project appears when a session runs in its folder." with New project… (⌘⇧N).
6. A backend summary SHALL compute the line's facts in one paged call, read-only, from the existing stores (sessions, needs, tasks, usage).
**UI/UX reasoning**: Nielsen #1 and #8 (aesthetic and minimalist: one line per project, the same facts in the same columns); the readable-and-visual rule (body 14 px, a grid not a strip). States: loading (skeleton), cached, populated, searching with none found ("No project matches "api""), empty, offline (the last list with the shared offline banner). Keys: ⌘⇧P opens it; ↑/↓ move; ↩ opens; ⌘F focuses the search.
**Performance budget**: the cached list paints in ≤ 100 ms; a page of 50 summaries ≤ 30 ms in the backend (debug build, local SQLite, 20 projects × 50 sessions fixture).
**Quality gates**: `cargo test -p juggler-app --test projects_summary` (new); vitest `src/screens/projects.test.tsx` (new); the IPC audit (`scripts/ipc_audit.py`: paged, Read class); `just ui`.
**Evidence/validation proof**: the tests above pass; `just load-gate` row "Projects screen, 20 projects" within budget; the gallery family `projects`; manifest `docs/qa/evidence/AJ-E2804/manifest.md`; pass = 0 failures and the measured page time ≤ 30 ms.
**Visual testing**: new `ui/tests/visual/projects-gallery.spec.ts`: loading, populated, search with none, empty, offline, the row menu open; 1024×700, 1280×800, 1440×900; dark and light; axe on each.
**Files to touch**: new `ui/src/screens/Projects.tsx`, `ui/src/screens/projects.test.tsx`; `ui/src/lib/ipc/projects.ts`, `ui/src/lazyParts.ts`, `ui/src/App.tsx`; new `crates/juggler-app/src/features/projects/summary.rs`, `crates/juggler-app/tests/projects_summary.rs`; `crates/juggler-app/src/features/projects/mod.rs`, `crates/juggler-app/src/lib.rs` (registered as Read); `scripts/ipc_audit.allow` if the audit asks; the dev bridge's route.
**Research basis**: the design §3 ("The Projects screen lists each project the same way…"); `docs/review/deep/11-ux-redesign-screen-spec.md` §4.2; wireframe screen 03; `crates/juggler-app/src/lib.rs:362` (how a Read command is registered); `crates/juggler-app/src/paging.rs` (keyset pages).
**Handover note**: A "project" here is AI Juggler's project (a folder), including folders found only in other apps' session files; don't invent a second list. Tokens are this week's from the usage store, never re-read from transcripts on open.

### AJ-E2805 The project page: Overview · Sessions · Tasks · Pool · Subagents · Usage · History | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort M | Depends: AJ-E2803, AJ-E2804
**Goal**: A project page with everything about the project: its overview, sessions, tasks, pool, subagents, usage and history.
**User story**: As someone opening "storefront", I see in one place what asks me, what works, its tasks, who is in its pool, what its subagents cost and its usage, without visiting four screens.
**Requirements**:
1. The head SHALL show the breadcrumb "Projects › <name>", the folder and branch on the right ("~/Dev/storefront · branch main"), the name, and pills for what asks you and what works.
2. The tabs SHALL be Overview · Sessions n · Tasks n · Pool · Subagents n · Usage · History, Overview first; a tab with nothing to show stays and says so ("No tasks in this project yet. Agents add them as they plan.").
3. Overview SHALL show four figures (open sessions and how many apps; tasks with ready and in review; this week's tokens; accounts used) and the existing callouts.
4. Tasks SHALL be the project's pool's tasks (the same list as the Tasks screen, filtered); Pool SHALL open the project's pool canvas (AJ-E2808) in place; Subagents SHALL list the subagents of the project's sessions (AJ-E2810's list, scoped); History SHALL be the project's ended sessions, paged.
5. The remembered tab per page type (AJ-E1534) SHALL keep working with the new tabs.
**UI/UX reasoning**: §4.3 of the screen spec (Project detail) and Nielsen #6; Apple HIG "Tab views" (tabs for peer views of one thing). States: each tab populated, empty, loading; the head with and without asks. Keys: ⌘[ / ⌘] or Ctrl+Tab move between tabs as on the session page; Esc from a tab's detail returns to the list.
**Performance budget**: the page head and Overview paint in ≤ 150 ms from cache; each tab's code and first read load only when it opens.
**Quality gates**: vitest `src/pages/project-page.test.tsx` (new), `src/components/project-manage.test.tsx` (extended); `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; the gallery family `project-page` (every tab); manifest `docs/qa/evidence/AJ-E2805/manifest.md`; pass = 0 failures, 0 axe critical/serious.
**Visual testing**: `ui/tests/visual/project-manage-gallery.spec.ts` extended, or a new `project-page-gallery.spec.ts`: each tab populated and empty, 1024×700 and 1280×800, dark and light.
**Files to touch**: `ui/src/pages/ProjectPage.tsx`, `ui/src/pages/registry.ts`, `ui/src/pages/project/Overview.tsx`, `ui/src/pages/ProjectUsage.tsx`, `ui/src/pages/SessionsList.tsx`, `ui/src/pages/tasks/PoolTasks.tsx`, `ui/src/pages/ItemHeader.tsx`; new `ui/src/pages/project-page.test.tsx`.
**Research basis**: the design §3 (the mock-up's head, tabs and figures); `ui/src/pages/registry.ts:115-116` (today: Sessions · Overview · Usage); `docs/review/deep/11-ux-redesign-screen-spec.md` §4.3; D-33 (the project's own pool).
**Handover note**: Subagents and Usage across a project's sessions must read the index and the usage store, never every transcript; if AJ-E2809 isn't merged yet, show the tab with its count from the rail's reader and the list once the index lands.

## Step 3 — tasks

### AJ-E2806 The Tasks screen: a board and the plan across every project | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort M | Depends: AJ-E2801, AJ-E2803
**Goal**: One screen with every pool's tasks by state, and the plan (epics → stories → tasks), filtered by project, pool or holder.
**User story**: As someone whose agents plan and share work, I see what is ready, moving, waiting on my review or blocked across all my projects, and approve or send back from there.
**Requirements**:
1. **Board** SHALL show columns Ready, In progress, In review, Blocked, each with its count; To do, Done and Cancelled are one click away ("Show: To do · Done · Cancelled").
2. A card SHALL show its epic › story path, its title, and its project with its holder (account · app) or "unclaimed"; Blocked says what it waits on ("waits on Refresh-token retries"); In review offers Approve and Send back… (D-44).
3. **Plan** SHALL show epics → stories → tasks as a tree (`role="treegrid"`), folded below the first level.
4. Filters SHALL be pickers (AJ-E2801): Project: all, Pool, Holder: anyone; and a search over titles and briefs.
5. Only D-44's four actions (Approve / Send back…, Free, Cancel…, Start a session for this task); no create or edit.
6. A backend list across pools SHALL be paged by keyset cursor per column; counts in one call.
7. Review items SHALL stay in Needs you too (one count, D-49).
**UI/UX reasoning**: D-44 (show and track, basic management only); Nielsen #1 and #7 (flexibility: board for flow, plan for structure); Apple HIG "Outline views" for the plan. States: board populated, a column empty ("Nothing ready"), filtered with none, plan folded and unfolded, a send-back note open, loading, offline. Keys: ⌘⇧T opens it; ←/→ between columns, ↑/↓ in a column; ↩ opens a task; A approves and S sends back on a focused in-review card (listed in the ? sheet).
**Performance budget**: the board's first page (20 a column) paints in ≤ 150 ms from cache; a column's page ≤ 20 ms in the backend with 2,000 tasks across 10 pools.
**Quality gates**: `cargo test -p juggler-app --test tasks_across_pools` (new); vitest `src/screens/tasks-screen.test.tsx` (new), `src/pages/tasks/tasks.test.tsx` (unchanged behaviour); the IPC audit; `just ui`.
**Evidence/validation proof**: the tests above pass; `just load-gate` row "Tasks board, 2,000 tasks"; the gallery family `tasks-screen`; manifest `docs/qa/evidence/AJ-E2806/manifest.md`; pass = 0 failures and a page ≤ 20 ms.
**Visual testing**: `ui/tests/visual/tasks-gallery.spec.ts` extended: board, plan, filters open, send back open, empty column, 1024×700 and 1280×800, dark and light; axe on each.
**Files to touch**: new `ui/src/screens/Tasks.tsx`, `ui/src/screens/tasks-screen.test.tsx`; `ui/src/pages/tasks/TaskRow.tsx`, `TaskTree.tsx`, `TaskView.tsx`, `words.ts`; `ui/src/lib/ipc/tasks.ts`; `crates/juggler-app/src/features/tasks/views.rs`, `commands.rs`; `crates/juggler-core/src/storage/tasks.rs`; `crates/juggler-app/src/lib.rs`; new `crates/juggler-app/tests/tasks_across_pools.rs`.
**Research basis**: the design §4; `crates/juggler-app/src/features/tasks/commands.rs:22-33` (`tasks_list` is per pool today); `crates/juggler-core/src/tasks.rs:73-89` (`TaskStatus`); D-43, D-44; https://developer.apple.com/design/human-interface-guidelines/outline-views.
**Handover note**: Don't add a "New task" button, even disabled: D-44 is explicit. A task's holder is a session; show its account and app, never an internal id.

## Step 4 — pools

### AJ-E2807 The Pools screen | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort S | Depends: AJ-E2803
**Goal**: A screen of every pool as a card, with a small map of its members and what needs you in it.
**User story**: As someone with a few pools, I see each one's members, what asks me and what moves, and open or start one from the same screen.
**Requirements**:
1. A card per pool SHALL show a small map of its members (the lead marked, a member with an ask marked), its name, "3 members · 1 asks you · 2 tasks moving" (or "quiet"); a project's own pool says "(project pool)".
2. Sorted: pools that ask you first, then by recent activity; paged when there are more than 24.
3. **New pool** SHALL open the canvas of a new, empty pool with the palette open (AJ-E2808).
4. A card opens the pool's canvas; its ⋯ menu offers Open, Rename…, Delete… (with what happens to its members said), icons on every line.
5. Empty is impossible while any project exists (each has its pool, D-33); with no project: "Pools appear with your first project."
**UI/UX reasoning**: Nielsen #1 and #6; the readable-and-visual rule (cards in a grid, not a strip). States: populated, one asks, all quiet, loading, the menu open, delete confirm. Keys: ⌘⇧O opens it; arrows move between cards; ↩ opens.
**Performance budget**: the screen paints from cache in ≤ 100 ms; the mini maps draw from the saved layout (AJ-E2808) with no layout pass.
**Quality gates**: vitest `src/screens/pools-screen.test.tsx` (new), `src/components/pools.test.tsx`; `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; the gallery family `pools`; manifest `docs/qa/evidence/AJ-E2807/manifest.md`; pass = 0 failures, 0 axe critical/serious.
**Visual testing**: `ui/tests/visual/pools-gallery.spec.ts` extended: the screen populated, with an ask, the menu, delete confirm; 1024×700 and 1280×800; dark and light.
**Files to touch**: new `ui/src/screens/Pools.tsx`, `ui/src/screens/pools-screen.test.tsx`; `ui/src/components/PoolCards.tsx`, `PoolSheet.tsx`, `PoolSheetHost.tsx`; `ui/src/lib/pools.ts`, `ui/src/lib/ipc/pools.ts`; `crates/juggler-app/src/features/pools/cards.rs`.
**Research basis**: the design §5 (the second mock-up); `crates/juggler-app/src/features/pools/commands.rs:18` (`pools_list`); D-33, D-55.
**Handover note**: Today a pool's page opens from Settings › Agent access or a Needs you card; keep those links, pointing at the canvas.

### AJ-E2808 The pool canvas: nodes, edges, alerts, edit in place, an inspector and a remembered layout | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort XL | Depends: AJ-E2801, AJ-E2807
**Goal**: A pool's page is one live canvas the user watches and edits in place: members as nodes, interactions as edges, alerts on what they concern, an inspector for whatever is selected, and a layout that is automatic at first and remembered once arranged.
**User story**: As someone who made a pool, I see who is in it, what they give and ask each other and what waits on me, answer it where it shows, and add, move or set up members on the same screen, laid out the way I arranged it.
**Requirements**:
1. Nodes SHALL show the session's name, Lead when it leads, account · app, its state and what it's on ("Working · Totals epic"); a node with an alert is marked and shows the answers there (an ask: Allow once · Always · Deny, through the same path as Needs you, D-28).
2. Edges SHALL show interactions: a task given (solid), messages (dashed), an ask or a held message (amber), each with a text name for screen readers ("Planner gave 1 task to Fix the cart totals"); edges between the same two members merge with a count. A held message shows "1 message held · Deliver".
3. Edit in place: drag a session from the palette ("Add a session") onto the canvas to add it, onto empty space to start a new pool; a node's menu offers Make lead, Move to pool…, Remove from pool; every drag has a menu and keyboard equivalent.
4. The inspector SHALL show the selection: the pool (members and lead, Agents may — the access level as a picker, Delivery, Rules you granted with Take back, Notify me, a live feed of the last events); a member (its tasks, messages, usage, rules); an edge (its messages and tasks). Changes apply on the next agent call (D-28).
5. Layout: the first layout SHALL be automatic and deterministic (the lead on top, members below); a dragged node SHALL stay where it was put, saved in AI Juggler's database per pool and member and restored after a restart; a member added later SHALL be placed automatically without moving placed ones; "Arrange automatically" SHALL start over, with Undo (D-55).
6. The page's tabs SHALL be Canvas · Tasks · Messages; the header's "Work together ▾" SHALL be the level picker; the old Members and Access tabs fold in.
7. Accessibility: a list view of the same nodes and edges (toggle "Canvas · List"); Tab enters the canvas, arrows move between nodes, ↩ selects, ⌥ + arrows moves a node by 8 px (⇧ for 32 px), Delete asks to remove; every alert's answers are reachable by keyboard.
8. Only the user changes a pool (D-28); adding a session never starts one (D-31).
**UI/UX reasoning**: the maintainer's "Think how diagram dashboards are" (direct manipulation, Shneiderman); Nielsen #1 (live state), #3 (user control: Undo for arrange and remove); WCAG 2.2, 2.5.7 (dragging movements have a single-pointer and keyboard alternative), 1.4.1 (edges differ by line style and label, not colour alone). States: a new empty pool with the palette open; populated; an ask on a node; a held message; a member selected; an edge selected; the list view; dragging; after Arrange automatically with the Undo toast; offline (read-only, "Changes wait until AI Juggler's service answers again").
**Performance budget**: 50 members and 200 edges drag at 60 fps (≤ 16 ms a frame); the automatic layout of 50 members ≤ 50 ms; a saved position written ≤ 100 ms after the drag ends (debounced, one write); the canvas's code loads only when a pool opens.
**Quality gates**: `cargo test -p juggler-core --test pools_storage` (the layout table and its upgrade), `cargo test -p juggler-app --test pool_layout` (new: save, restore, a new member placed, arrange); vitest `src/pages/pool/canvas.test.tsx` (new: keyboard moves, the list view, alert answers, undo); the IPC audit; `just ui`; the a11y gate; `scripts/size_budget.py` (split files rather than raise caps).
**Evidence/validation proof**: the tests above pass; Playwright `pools-gallery.spec.ts` drags a node, reloads and finds it where it was put; a frame-time trace of a 50-member drag in `docs/qa/evidence/AJ-E2808/frames.json`; manifest `docs/qa/evidence/AJ-E2808/manifest.md`; pass = 0 failures, p95 frame ≤ 16 ms, position restored exactly.
**Visual testing**: `ui/tests/visual/pools-gallery.spec.ts` extended with every state above at 1024×700, 1280×800 and 1440×900, dark and light; axe on each; the list view captured too.
**Files to touch**: `ui/src/pages/PoolPage.tsx`, `ui/src/pages/registry.ts`; new `ui/src/pages/pool/Canvas.tsx`, `CanvasNode.tsx`, `CanvasEdges.tsx`, `Inspector.tsx`, `Palette.tsx`, `layout.ts`, `canvas.test.tsx`; `ui/src/components/PoolEditor.tsx`, `PoolAccess.tsx`, `AskCard.tsx`; `ui/src/lib/pools.ts`, `ui/src/lib/ipc/pools.ts`; `crates/juggler-core/src/storage/pools.rs` (the layout table, at the next free schema version), `crates/juggler-core/src/storage/mod.rs`; `crates/juggler-app/src/features/pools/commands.rs`, `members.rs`, `dto.rs`; new `crates/juggler-app/tests/pool_layout.rs`; `crates/juggler-app/src/lib.rs` (the layout command as Write).
**Research basis**: the design §5 (the canvas mock-up, its edges and inspector); `ui/src/pages/registry.ts:127-128` (today: Members · Tasks · Messages · Access); `crates/juggler-app/src/features/pools/commands.rs:46-199` (place, create, move, set role, access, delivery, revoke rule); `crates/juggler-core/src/storage/mod.rs:594-596` (pools own their tables); D-28, D-29, D-31, D-32, D-33, D-55; https://www.w3.org/TR/WCAG22/#dragging-movements.
**Handover note**: Being built on branch `pools-0.3`. Positions are per Mac (D-55, the agents' reading); a pool across Macs (D-32) never syncs them. Draw with SVG and HTML, not a canvas element, so nodes stay in the accessibility tree. The parked pools branch renumbers to the next free schema version; check `storage/mod.rs` before taking one.

## Step 5 — subagents

### AJ-E2809 A subagent index and the adapter contract for subagent details | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort L | Depends: —
**Goal**: A small saved index of every session's subagents, updated as the tools' files grow, and an adapter contract that reads each subagent's prompt, result, work and usage where its tool records them.
**User story**: As someone opening a session that ran 437 subagents, I see the Subagents tab at once, page through it, and see what each was asked, what it returned and what it cost.
**Requirements**:
1. The index SHALL hold one row per subagent (session, id, parent, depth, kind, the message that started it, state, start and end, model, tokens, tool-use counts by kind, lines added and removed, files, a clipped prompt and result, the commands it ran for process matching), at the next free schema version.
2. It SHALL be updated incrementally from where each record was last read (offset and size), never by re-reading a whole file that only grew; a file that shrank or changed is re-read.
3. Lists, counts and the per-message groups SHALL be read from the index, paged by keyset cursor.
4. The adapter contract (`juggler_adapter::subagents`) SHALL grow optional details per subagent (prompt, result, model, tokens and whether they are separate from the parent's or part of the session's total, tool uses, lines, files, commands, the starting message) and a capability per detail (exact, derived, where recorded, part of the session, not recorded), answered by every adapter; the conformance kit SHALL refuse an adapter that leaves one unanswered.
5. Prompts and results SHALL be redacted (D-14) before they are indexed; the index keeps only what the screen shows.
6. The index SHALL be bounded (prompt and result clipped to 4 KB each), pruned with its session's records, and rebuilt from the tools' files if it is lost.
7. Reading stays read-only (D-28, D-37); no process environment is read.
8. "Running" SHALL stay truthful: no end record after the session ends reads Unknown (D-06); a quiet running one carries its last activity time.
**UI/UX reasoning**: Instant and paged (the desktop-app rule) and honest data (D-06, D-56): the screen can only be fast and truthful if the data under it is. The user-visible states this enables: Running, Done, Failed, Stopped, Unknown, and "last activity 3 min ago".
**Performance budget**: the first index of the 437-subagent session (≈ 700 MB of records) ≤ 20 s in the background without blocking the UI; an update after 1 MB appended ≤ 20 ms; a page of 50 rows ≤ 10 ms; the index ≤ 2 KB a subagent on average.
**Quality gates**: `cargo test -p juggler-adapter` (the contract), `cargo test -p juggler-adapters --test adapter_conformance` (every adapter answers), the `*_subagents` unit tests per adapter with fixtures, `cargo test -p juggler-core --test subagents_storage` (new), `cargo test -p juggler-app --test subagents_index` (new: incremental, shrink, rebuild, prune, redaction); clippy `-D warnings`; `just platform`; the polling audit.
**Evidence/validation proof**: the tests above pass; a timed run over a generated 700 MB fixture (`just load-gate` row "Subagents index, 437 subagents") recorded in `docs/qa/evidence/AJ-E2809/manifest.md`; a read-only live check on a real Claude Code session's records in the lab (counts match Claude Code's own files); pass = 0 failures and every budget met.
**Visual testing**: None directly (a backend task); its screens are AJ-E2810 and AJ-E2811.
**Files to touch**: `crates/juggler-adapter/src/subagents.rs`, `crates/juggler-adapter/src/adapter.rs`; `crates/juggler-adapters/src/claude_subagents.rs`, `codex_subagents.rs`, `kimi_subagents.rs`, `grok_subagents.rs`, `opencode_subagents.rs`, `subtasks.rs`, `fake.rs`; `crates/juggler-adapters/tests/conformance/mod.rs`; new `crates/juggler-core/src/storage/subagents.rs` and `crates/juggler-core/tests/subagents_storage.rs`, `crates/juggler-core/src/storage/mod.rs`; `crates/juggler-app/src/features/subagents/mod.rs`, `count.rs`, `dto.rs`, `commands.rs`; new `crates/juggler-app/src/features/subagents/index.rs` and `crates/juggler-app/tests/subagents_index.rs`.
**Research basis**: the design §6 and §7 (the honest-data table, "Speed", "No double counting", "Running stays truthful"); `crates/juggler-adapter/src/subagents.rs:20-76` (`SubagentStatus`, `Subagent`), `:135-151` (`SubagentReader`); `crates/juggler-app/src/features/subagents/mod.rs:45-47` (`READERS_KEPT = 8`, readers in memory only); D-06, D-14, D-48.
**Handover note**: Being built on branch `subagents-0.3`. Keep D-48's one reader: the rail's count and the tab both read the index once it exists. Codex's per-subagent tokens are part of the session's total: never add them to it. Claude Code forgets finished subagents after 30 seconds in its own UI; its files keep them, and that's what is read.

### AJ-E2810 The Subagents tab: running, failed and cost first, grouped by the message that started them, with tree lines | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort L | Depends: AJ-E2801, AJ-E2809
**Goal**: A Subagents tab that answers "what's running, what failed, what did it cost" first, groups subagents under the message that started them and draws who started whom with tree lines.
**User story**: As someone whose session looks idle but still costs, I open Subagents, see the three still running and the two that failed, and which of my messages started them.
**Requirements**:
1. A summary line SHALL read "437 subagents · 3 running · 2 failed · 41% of this session's tokens".
2. Filters SHALL be Running n · Needs attention n (failed, stopped, unknown) · All; the tab SHALL open on Running when anything runs, else All.
3. A search SHALL match task, prompt, result and file; Type ▾ and Sort: newest ▾ are pickers (AJ-E2801).
4. Subagents SHALL be grouped under the message that started them ("You asked: "fix the totals and add tests for rounding" · 14:02", then "6 subagents · 2 running · 1.2M"); the newest group open, older ones folded.
5. Tree lines (├ └ │) SHALL show who started whom, in place of the unexplained indent; a parent's tokens include its children's, its own on hover and in its accessible name.
6. Columns: Subagent (task and type), State, Time, Tokens, Model.
7. Windowed and paged: never all rows rendered at once; a group's rows page in as it scrolls.
8. The tree SHALL be a `treegrid`: ↑/↓ move, →/← unfold and fold, ↩ opens the detail (AJ-E2811), / focuses the search.
9. A tool that keeps no subagent records says so in the tab ("Kimi Code doesn't record its subagents' prompts"), per the honest-data table.
**UI/UX reasoning**: the maintainer's "think why would users want to view subagents"; Nielsen #1 (status first), #8 (minimalist: running and failed before 437 rows); WAI-ARIA treegrid pattern; tree lines as the outline convention (Apple HIG "Outline views"). States: running (opens on Running), none running, a failed one, a search with none, a folded group, a deep tree (4 levels), unsupported tool, loading from the index, the first index still building ("Reading this session's subagents… 120 of about 437").
**Performance budget**: the tab paints its first page from the index in ≤ 150 ms for the 437-subagent session; scrolling holds 60 fps; a search answers in ≤ 100 ms.
**Quality gates**: vitest `src/pages/session/subagents-tab.test.tsx` (extended: filters, groups, tree lines, treegrid keys, opening on Running); `src/lib/tree-grid` tests; `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; the gallery family `88-subagents` extended; manifest `docs/qa/evidence/AJ-E2810/manifest.md`; pass = 0 failures, 0 axe critical/serious, the first page within budget in `just load-gate`.
**Visual testing**: `ui/tests/visual/subagents-gallery.spec.ts` extended with every state above at 1024×700, 1280×800 and 1440×900, dark and light; axe on each.
**Files to touch**: `ui/src/pages/session/SubagentsTab.tsx`, `useSubagents.ts`, `session-tabs.tsx`, `subagents-tab.test.tsx`; `ui/src/lib/ipc/subagents.ts`, `ui/src/lib/subagents.ts`, `ui/src/lib/tree-grid.ts`; `ui/src/components/PagedList.tsx`.
**Research basis**: the design §6 (the tab's mock-up and its notes); `ui/src/pages/session/SubagentsTab.tsx:185,216` (the indent: `paddingLeft` by depth, no lines); `ui/src/lib/tree-grid.ts`; D-48; https://www.w3.org/WAI/ARIA/apg/patterns/treegrid/.
**Handover note**: Being built on branch `subagents-0.3`. "Needs attention" includes Unknown on purpose: a subagent whose end was never recorded is worth a look. Don't colour the tree lines by state; the State column carries it.

### AJ-E2811 A subagent's detail: its prompt, result, work, usage and processes | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort M | Depends: AJ-E2809, AJ-E2810
**Goal**: A subagent's own view with a breadcrumb through its parents: the task it was given, its result, its work, its usage, the processes it probably started, and an honest Stop.
**User story**: As someone wondering why a subagent cost 288k tokens and failed, I open it and read what it was asked, why it failed, what it touched and which dev server it left running.
**Requirements**:
1. The breadcrumb SHALL read "<project> › <session> › Subagents › <parent> … › <name>", each step a link; Esc goes back one step.
2. The head SHALL show the name, its state and time ("● Running · 1m"), type · model, "in the background" when it ran there, and "started by <parent>".
3. **Task it was given**: the first lines of the prompt and "Show the whole prompt".
4. **Result**: the result's summary, or "It's still running. Its answer shows here when it ends.", or the failure's reason in one line.
5. **Work**: tool uses by kind ("38 · 12 reads · 9 searches · 6 commands · 4 edits"), lines (+120 −34), files (two named, "+3 more"), "Open its conversation" when the tool keeps one.
6. **Usage**: tokens, estimated cost ("~$0.04"), share of the session ("0.6%"); a tool whose tokens are part of the session's total says so instead of a share.
7. **Processes**: each process matched from the commands it ran, marked **probably**, with its memory, and "Stop it from the session's Processes tab" (a link); none: "No process matched its commands."
8. **Stop**: "AI Juggler can't stop one subagent inside a Claude Code session it doesn't drive. In Claude Code: /tasks, select it, press x." (per tool, from the adapter); in a session AI Juggler drives and the tool allows it, a Stop button instead.
9. CPU and memory of the subagent itself SHALL NOT be shown: "Every subagent runs inside its tool's one process" (D-56).
**UI/UX reasoning**: Nielsen #2 (match the real world: "the task it was given", not "input"), #9 (a failure in one line); the serious-choices rule for Stop (consequence and the way to do it). States: running, done, failed, stopped, unknown; with and without processes; a tool without prompts; prompt expanded.
**Performance budget**: the detail paints from the index in ≤ 100 ms; the whole prompt and the conversation load only on request.
**Quality gates**: vitest `src/pages/session/subagent-detail.test.tsx` (new: each state, breadcrumb and Esc, "probably", the Stop words per tool); `cargo test -p juggler-app --test subagents_index` (the detail read); `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; the gallery family `88-subagents` (detail states); manifest `docs/qa/evidence/AJ-E2811/manifest.md`; pass = 0 failures, 0 axe critical/serious.
**Visual testing**: `ui/tests/visual/subagents-gallery.spec.ts`: each state above at 1024×700 and 1280×800, dark and light.
**Files to touch**: new `ui/src/pages/session/SubagentDetail.tsx`, `ui/src/pages/session/subagent-detail.test.tsx`; `ui/src/pages/session/SubagentsTab.tsx`, `ui/src/components/ConversationPage.tsx`, `ui/src/lib/ipc/subagents.ts`; `crates/juggler-app/src/features/subagents/commands.rs`, `dto.rs`.
**Research basis**: the design §6 ("a subagent" mock-up) and §7; `crates/juggler-app/src/features/subagents/commands.rs:43-45` (`subagents_conversation`); D-14, D-41 (stopping processes of other apps' sessions), D-56.
**Handover note**: The process match is a label, never a reason to stop anything; the only Stop for a process stays on the Processes tab. Cost is an estimate from the usage engine's prices; say "~" and never a precise figure.

### AJ-E2812 Cross-links: the session's Usage and Processes, and the project's subagents | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P1 | Effort S | Depends: AJ-E2809, AJ-E2811
**Goal**: Subagents show where people already look: the session's Usage tab shows their share, its Processes tab names the subagent that probably started a process, and the project page lists its subagents.
**User story**: As someone looking at an expensive session's Usage, I see that its subagents used 41% and which one used most, and open it in a click.
**Requirements**:
1. The session's Usage tab SHALL show "Subagents: 41% of this session's tokens" and its three most expensive, each a link to its detail.
2. The session's Processes tab SHALL show "probably started by <subagent>" on a process the index matched, linking to the subagent.
3. The project page's Subagents tab SHALL list the subagents of the project's sessions (AJ-E2810's list, grouped by session, then by message).
4. The Processes screen SHALL NOT change its stop rules; the subagent line is information only.
**UI/UX reasoning**: Nielsen #6 and #7 (the answer where the question is asked); honest data ("probably"). States: a session with and without subagents; a process with and without a match.
**Performance budget**: each link reads the index (≤ 10 ms); no tab re-reads transcripts.
**Quality gates**: vitest `src/components/processes-tab.test.tsx` (extended) and a new `src/pages/session/usage-subagents.test.tsx`; `just ui`.
**Evidence/validation proof**: the tests above pass; the gallery families `88-subagents` and `processes`; manifest `docs/qa/evidence/AJ-E2812/manifest.md`; pass = 0 failures.
**Visual testing**: `ui/tests/visual/session-pages-gallery.spec.ts` and `processes-gallery.spec.ts` extended with the new lines, 1280×800, dark and light.
**Files to touch**: `ui/src/pages/session/UsageTab.tsx`, `UsagePanel.tsx`, new `usage-subagents.test.tsx`; `ui/src/components/RightInspector.tsx`; `ui/src/screens/processes/ProcessTree.tsx`, `cells.tsx`; `ui/src/pages/ProjectPage.tsx`; `crates/juggler-app/src/features/processes/` (the match, read from the index).
**Research basis**: the design §6 (why someone opens it: "which one started a process") and §7 ("Processes it started: probably"); `ui/src/pages/session/session-tabs.tsx:27` (the Processes link); D-41, D-56.
**Handover note**: Don't double count: the session's total already includes its subagents; the share is a part of it, not an addition.

## Step 6 — the sweep and the review

### AJ-E2813 The sweep's smaller fixes | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P1 | Effort S | Depends: AJ-E2801
**Goal**: The small faults the design sweep found are fixed, each captured before and after.
**User story**: As someone using AI Juggler daily, I don't trip over an orphan line, a wrapped label or a menu that hides its own button.
**Requirements**:
1. The limit dialog SHALL lose its orphan line and gain a × (Esc closes it, focus returns).
2. New session SHALL keep "Suggested" on one line, and its text triangles (▸ ▾) SHALL become the app's chevron icons.
3. The tool menu SHALL open beside its button, never over it (placed with `menuEdge`).
4. Settings › General's squeezed card SHALL get its full width on the settings grid.
5. Home's app buttons SHALL align on one baseline and one grid.
**UI/UX reasoning**: Nielsen #4 and #8; the D-57 checklist (one grid, icons not text glyphs). States: each before and after, in both themes.
**Performance budget**: no change to any screen's paint time (the load gate within 10 % of its baseline).
**Quality gates**: vitest `src/components/limit-dialog.test.tsx`, `src/components/launch-sheet.test.tsx`, `src/screens/home.test.tsx` (extended); `just ui`; the a11y gate on the touched screens.
**Evidence/validation proof**: the tests above pass; before and after captures in `docs/qa/evidence/AJ-E2813/` with the manifest; pass = 0 failures and each fault gone in its after capture.
**Visual testing**: the existing galleries for these screens (`limit-ready.spec.ts`, `launch-advisor-gallery.spec.ts`, `home-gallery.spec.ts`, `menus-gallery.spec.ts`) at 1024×700 and 1280×800, dark and light.
**Files to touch**: `ui/src/components/LimitDialog.tsx`, `ui/src/screens/NewSession.tsx`, `ui/src/components/LaunchPickWhy.tsx`, `ui/src/components/RowMenu.tsx` (`menuEdge`), `ui/src/screens/settings/GeneralPane.tsx`, `ui/src/components/HomeApps.tsx`, `ui/src/components/icons-actions.tsx`.
**Research basis**: the design §2 ("Smaller fixes from the sweep"); `ui/src/screens/NewSession.tsx:356,381` ("Customise ▸", "Customise ▾"); `ui/src/components/RowMenu.tsx:130` (`menuEdge`).
**Handover note**: The tool menu's opener is named in the sweep's capture; place it with the shared `menuEdge` rather than a one-off offset.

### AJ-E2814 The design-quality review: a critique beside every new or changed screen, and every popover captured | Status: IN-PROGRESS (claude, 2026-10-29) | Priority P0 | Effort S | Depends: —
**Goal**: Every new or changed screen is critiqued for design quality, not only for regressions and accessibility, and every popover is in the gallery.
**User story**: As the maintainer, I open the gallery and see each new screen beside a short critique against the app's best screens, and I don't find a bland or broken menu that nobody captured.
**Requirements**:
1. A critique file per gallery family (`docs/qa/evidence/_gallery/<family>/critique.md`) SHALL answer the D-57 checklist (grid, hierarchy, spacing, icons, shared controls, disabled reasons, plain words, type sizes, both themes, 1280×800 and 1024×700) against the reference screens (the row menus, the rail's order menu, Usage, Processes), and say what was fixed.
2. `scripts/gallery_index.sh` SHALL show each family's critique beside its captures on the review page.
3. A family whose screens changed without a critique SHALL be flagged by a check in `just verify` (a family's captures newer than its critique).
4. Every popover, menu and sheet SHALL be captured; one that can't be SHALL be listed in `docs/qa/evidence/_gallery/not-captured.md` with its reason.
5. `docs/plan/QUALITY.md` §2 SHALL carry the line (D-57), and the PR template's checklist SHALL mirror it.
**UI/UX reasoning**: Nielsen #8 (aesthetic and minimalist design) as a gate, not a hope; heuristic evaluation (NN/g) by a named checklist and named references, so a critique is repeatable.
**Performance budget**: the check runs in ≤ 2 s; a critique is at most one screen of text per family.
**Quality gates**: the new check's `--selftest`; `./scripts/evidence_check.sh`; `python3 scripts/docs_links_check.py`.
**Evidence/validation proof**: the check passes on `release/0.3` with E28's families critiqued; the review page generated by `scripts/gallery_index.sh` shows them; manifest `docs/qa/evidence/AJ-E2814/manifest.md`; pass = every E28 family has a critique and every E28 popover is captured or listed.
**Visual testing**: the review page itself, opened and checked by eye in both themes (`docs/qa/evidence/_gallery/index.html`).
**Files to touch**: `docs/plan/QUALITY.md`, `scripts/gallery_index.sh`, new `scripts/critique_check.py`, `justfile`, `.github/pull_request_template.md`, `docs/qa/evidence/_gallery/not-captured.md`, and new `docs/qa/evidence/_gallery/<family>/critique.md` files.
**Research basis**: the design §0 ("Our gates check "unchanged" and "accessible", never "good". The View menu wasn't even captured.") and §8; `scripts/gallery_index.sh:1-30`; `docs/plan/QUALITY.md` §2 and "Visual checks"; https://www.nngroup.com/articles/how-to-conduct-a-heuristic-evaluation/.
**Handover note**: A critique isn't a baseline note (AJ-E803's PR section): the note says what changed on purpose, the critique says whether it is good. Never commit after a local Playwright run with `-a`: the gallery rewrites itself; stage by name.

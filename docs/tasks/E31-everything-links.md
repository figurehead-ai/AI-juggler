# E31 — Everything links to everything: each page leads to what's related

> Source: the maintainer, 2026-10-30, on 0.3.0's project Pool tab and an
> account's Sessions (verbatim): "whether I am viewing a project, session
> or app. I should be in tabs have linked or relevant each other so I can
> easily go to them. For example If I am viewing a project, I should be
> able to see all of the Apps, pools and session using that project. Same
> goes for app, I should be able to see all of the projects, ...etc and
> vice versa. We want to make it super easy for users to navigate and get
> around. And always if there is a lot of info, you also want to have
> filters, searches, and most used, most relevant first, like active ones,
> recently active ...etc. That is the advantage of our tool, we want to
> make it easier to find than even native apps have capabilities provide
> for navigating through things." Placed in 0.4 with E30 (D-61): pools are
> one of the things every page links to.
>
> **The rule this epic builds.** Every entity page (project, session,
> app on an account (a profile), account, pool, Mac, tool) shows every
> related entity as a tab or section with its count, one click to each,
> both ways. Every growing list there opens on the most relevant (active,
> then recently active), has search and filters, and is paged
> (`docs/plan/QUALITY.md`, the pickers rule of 2026-10-30).
>
> **Order.** AJ-E3101 maps the relations and each page's tabs, with
> wireframes, and goes to the maintainer with E30's pool proposal (pools
> are among the linked entities). The build tasks are written from what's
> approved.
>
> **Approved** on 2026-10-30 (D-62, decisions 19–21). The build tasks
> follow E32's stored Account and App and come before E30's screens,
> which use the one list and the pickers. Each names its scenarios in
> [`docs/qa/0.4-validation-plan.md`](../qa/0.4-validation-plan.md)
> (K1–K4) and isn't done until they pass by hand, scored (D-62).

## Step 1 — the map and the pages

### AJ-E3101 What relates to what: the map, each page's tabs, and one list for all of them | Status: DONE (2026-10-30: `docs/research/relations-map.md`, the review page, approved as D-62) | Priority P0 | Effort M | Depends: —
**Goal**: A reviewed plan for how every page leads to everything related, before any page changes.
**User story**: As someone on a project's page, I see its apps, accounts, pools and sessions, each a click away, active ones first, with search when there are many; on an app's page I see its projects and sessions the same way, and so on from every page.
**Requirements**:
1. A relation map SHALL list each pair of entities (project, session, profile, account, pool, Mac, tool), how the relation is known (stored, read from a tool's files, inferred and said "probably" per D-36), its cost to compute, and whether it shows on both pages.
2. Each entity page SHALL get a proposed tab set (today's tabs kept unless replaced) with counts, its default order (active, recently active, then the rest), its filters and search, and its empty states that teach (e.g. a project pool with no members says why sessions other apps run can't join, E30).
3. One list component SHALL serve every related-items tab: the active-first order, search, filters, paging by keyset cursor, a row anatomy shared with the rail and Home (app icon, profile badge, state, when).
4. The plan SHALL include wireframes for the project, profile, account and session pages at 1280 and 1024, both themes, following the readability rule (headings, labels, values and areas distinct by colour and surface). (Amended 2026-11-06: as AJ-E3001 R4, approved as D-62; the pages as built are in the gallery, both themes at 1280 and 1024: `79-pages` (the project, account and session pages), `99-app-page` (the App, once the profile page), `99-account-page`, `92-session-pages`, `99-related-list`.)
5. It goes to the maintainer with E30's proposal; the build tasks are written from what's approved.
**UI/UX reasoning**: the 5-second test (what's open, where, on which profile, one click there); progressive disclosure (counts and the active few at once, the full list a click away); one row anatomy everywhere (D-47).
**Performance budget**: a related-items tab opens in ≤ 150 ms from cache; counts never re-read transcripts (summary tables, D-58's cache).
**Quality gates**: the map's "how known" column carries a file:line or a tool doc per relation.
**Evidence/validation proof**: `docs/research/relations-map.md`; the proposal page; the maintainer's recorded decision.
**Visual testing**: wireframes in the proposal.
**Files to touch**: `docs/research/relations-map.md` (new), this epic (the build tasks, once approved).
**Research basis**: E28 (projects, project page, subagents), E21 (places, rows), E22 (usage), E30 (pools); `ui/src/pages/registry.ts` (today's tabs).
**Handover note**: The pickers rule (suggested first, then All, search when long) applies to every picker these pages add.

## Step 2 — build

### AJ-E3102 Relations made cheap: instance and conversation indexes, and a relation-count cache | Status: DONE-EVIDENCED (2026-11-06, release/0.4.1; built agent, 2026-11-03: schema v43 indexes `instances` by project, launch settings, App and Mac and adds `relation_stamps`, moved by triggers when a value a relation is decided by changes; the conversations other apps started are indexed by project, App, tool and folder (`features/conversations/by_relation.rs`); `features/relations` is D-58's "Counts on pages" cache, in memory: two parts (AI Juggler's sessions, the outside conversations), each kept with the stamps of its sources and rebuilt when one moves (the app's events, enumerated in `relations::Change`, and the database's stamps); `relations_counts` answers a batch of entities; `sessions_counts` without a search reads it; the project, App and Account pages' Sessions counts come from it. Measured (debug build): 10 000 sessions rebuilt cold in ~125 ms, a cached count ≤ 0.1 ms. `tests/relations.rs` holds K2's counts = list = recount on every page, X1's cache cleared mid-walk, and the budgets; the K1/K2/X1 walks by hand remain) | Priority P0 | Effort M | Depends: AJ-E3202
**Goal**: Every count and related list on every page comes from an index or the cache, never from a scan or a transcript.
**User story**: As someone with thousands of sessions, a project's tabs show their counts at once.
**Requirements**:
1. Indexes SHALL be added on instances (project, launch settings, App, Mac) and a conversation index by project, App and Tool.
2. A relation-count entry in D-58's cache SHALL hold each entity's counts per relation, invalidated by stamp on change events, cleared by the user's cache clear (the page shows at once, counts a moment later).
3. Counts SHALL agree with the lists they head.
**UI/UX reasoning**: instant and paged.
**Performance budget**: any count ≤ 5 ms from cache; a cold rebuild for 10 000 sessions ≤ 1 s, off the main thread.
**Quality gates**: L2: counts equal the reference on the replica; a property test that every event kind invalidates what it should.
**Evidence/validation proof**: scenarios K1, K2, X1 (cache cleared mid-walk).
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-core/src/storage/`, `crates/juggler-app/src/cache/`.
**Research basis**: `docs/research/relations-map.md` (cost column).
**Handover note**: —

### AJ-E3103 One related-items list and one row, everywhere | Status: DONE-EVIDENCED (2026-11-06, release/0.4.1; built 2026-11-03: `components/RelatedList.tsx` is every related tab's list (the project, App and Account pages' Sessions): it opens on needs you, working, open, the last 7 days, then earlier, headed by section once there's more than one (the backend orders by `sessions::rows::Rank`, the rail's own state rule, so pages agree); search and filter chips appear past 8 rows (State: Open now, Recent, Ended; App, Account, project with "No project", Mac only when its sessions ran on two Macs), each chip offered only when it has two values, with each value's row count (`sessions_facets`, read only once the filters show); `sessions_page` takes `machine_id`, `since_ms` and `project_id: ""`; keyset paging as before; a shown list follows sessions and conversations changing and holds still: new rows wait behind "N new", a row that changed section stays put behind "Show in order" (`related/hold.ts`); empty states teach (what would show, the one step) and a search or filter that empties it says so with Clear. One row, `components/row/SessionRow.tsx` (densities cozy and compact for the rail, list for pages and Home; shell `row/RowShell.tsx`), replaces SessionRowBody, the rail's PlaceRow and Home's "Pick up where you left off" rows: the app icon and state dot, the title, then project · App · when, the state word at line 2's end, in the same places everywhere. Tests: `tests/relations.rs` K2 order across pages of 7 and every filter value = list = heading on every page; vitest all-pairs page × size (0, 1, 8, 9, 50, 1 000) × filter × search, 42 cases, against a reference written from the fixture. Budgets (Playwright, preview build): the Sessions tab from memory 7–9 ms; 1 000 rows scrolled at a 16.8 ms median frame (p95 17.9 ms), 21 rows drawn; backend (debug) first page of 50 of 10 000 sessions ~112 ms, filter values ~105 ms. Gallery 99-related-list (critique written, not stamped: stamping needs the captures staged). Not done: Home's Needs you card lines and the rail's Needs you rows keep their need-line anatomy; the "N new" pill is not captured (needs a live backend); K2 by hand remains) | Priority P0 | Effort L | Depends: AJ-E3102
**Goal**: Every related tab, the rail and Home use one list and one row, so a session looks and behaves the same wherever it's seen.
**User story**: As someone moving between pages, a session's row always shows its app, state, App badge, project and when, in the same places.
**Requirements**:
1. `RelatedList` SHALL open on needs you, working, open, active in the last 7 days, then the rest; search past 8 items; filter chips (state, App, Account, project, Mac when paired); keyset paging; new rows wait behind "N new".
2. One row SHALL replace SessionRowBody, the rail's PlaceRow and Home's session tiles (Pick up where you left off). Needs you lines, on Home and in the rail, are action lines (what waits and its button) and keep their own layout. (Amended 2026-11-06: a Needs you line says what waits on you and offers its action; it isn't a session tile, and squeezing it into the session row lost the action.)
3. Empty states SHALL teach (what would appear here, and the step that brings it).
**UI/UX reasoning**: one row anatomy (D-47); the 14 principles.
**Performance budget**: a tab opens ≤ 150 ms from cache; scrolling 1 000 rows stays at 60 fps.
**Quality gates**: L3 all pairs over page × tab × size × filter; gallery unchanged where only the row moved, critique where it changed.
**Evidence/validation proof**: scenario K2.
**Visual testing**: every page's lists, both themes, 1024 and 1280.
**Files to touch**: `ui/src/components/RelatedList.tsx` (new), `ui/src/components/SessionRow*.tsx`, `WorkflowRail.tsx`, `Home*.tsx`.
**Research basis**: `docs/research/relations-map.md` "One list".
**Handover note**: —

### AJ-E3104 Every page leads to what's related: header chips and counted tabs on project, session, App, Account, pool, Mac and Tool | Status: DONE-EVIDENCED (2026-11-07, release/0.4.1; built: agent, 2026-11-03, first part: schema v44's pools stamp, so the relation counts count each entity's pools (a session's pool: its member row, else its project's own) and `relations_related` lists a page's related ids of one kind from the same cache; `sessions_page` narrows by `tool_id`. Pools tabs, counted, on project (replaces Pool: its own pool's brief, then its other pools and yours for it), session (its one pool, how it got there and its role, until AJ-E3004), App, Account and Tool, one list (`components/RelatedPools.tsx`); the Tool page is reached again (the session's tool door opens it; Tool settings in its header leads to Settings) with Overview · Apps · Sessions (the one list) · Projects · Pools. Tests: `tests/relations.rs` every pool counts its member list, a move shows in the next count, a Tool page's count = its list; `relations_storage.rs` the pools stamp; vitest all; gallery 72-tool-view re-shot and critiqued. The App page's facts line opens its tool's page and its Account's (found as its Plan card finds it; no Mac door: an App is this Mac's). A project's Apps tab (after Sessions, counted) lists every App its sessions ran in, from the relation cache. Req. 2, routing: each notice opens its page on its tab (a pool's message on Messages at its thread, `NoticeTarget.thread_id`; a task on Tasks; a session working hard on Processes, a restart on History, the rest on its body; a conversation on Overview; a sign-in on its App's Overview; an Account's renewal on Settings, its limits on Overview), the page open now too (`openOnTab`), another app's session not loaded yet opens by its ids, and a notice with nothing left to open says so (`lib/notice-actions.test.tsx`). Req. 2, counts: Home's heading leaves out snoozed caps, messages and tasks as the bell does (`useSnoozedSubjects`), and the rail's Project lens counts and draws the bell's other needs (a sign-in, a cap, an agent's message) as the App lens does. This Mac's page (2026-11-04): Overview (AJ-E1528's state, figures and what's using it) · Sessions · Apps · Projects · Pools, counted from the relation cache by this Mac's id (`fleet_self`'s `machine_id`, the one `instances.machine` carries), the one list anchored on `machine_id` (`pages/machine/ThisMacPage.tsx`, `pages/machine/this-mac-tabs.test.tsx`); a paired Mac keeps its peer's tabs; the page label reads Mac. Home shows a session's need whose session isn't loaded (past the working set) in the notice's own words, opened by its ids, so its Needs you counts what the bell counts (`lib/needs-you.ts`, `lib/home-needs-unloaded.test.ts`, 2026-11-04). Left: K1 and K3 by hand) | Priority P0 | Effort L | Depends: AJ-E3103, AJ-E3204, AJ-E3205
**Goal**: From any page, every related thing is one click away, and it links back.
**User story**: On a project I open Apps (5), then an App's Sessions, then a session's Account chip, then that Account's Pools, each in one click, and back again.
**Requirements**:
1. A relation to one thing SHALL be a header chip; a relation to many SHALL be a tab with its count, per the tab table in `docs/research/relations-map.md` (the project's Pool tab becomes Pools).
2. Every notice SHALL open the right page and tab; Needs you counts the same notices as the bell.
3. A session's Pools tab SHALL say how it got into each pool and which it's skipped from (needs AJ-E3004; until then, its one pool).
**UI/UX reasoning**: desktop-first navigation: what is open, where, one click to it.
**Performance budget**: every hop ≤ 150 ms.
**Quality gates**: L2 counts; L3 pairs; the chain walked both ways.
**Evidence/validation proof**: scenarios K1, K2, K3.
**Visual testing**: each page's header and tabs, both themes.
**Files to touch**: `ui/src/pages/registry.ts`, `ui/src/pages/*`, `ui/src/pages/ItemHeader.tsx`.
**Research basis**: `docs/research/relations-map.md`; the review's page wireframes.
**Handover note**: —

### AJ-E3105 Every picker: suggested for where you are, then All, with search | Status: DONE-EVIDENCED (2026-11-06, release/0.4.1; built: agent, 2026-11-03: the Picker's props are now one of `groups` (a picker of things names what goes first) or `choice` (fixed words: a setting, a sort), so the compiler lists any picker of things without a top group (the lint, by type). Pickers of things: Apps and launch settings (copy settings from, Expert setup, a task's Start, New session's launch settings) open on Open now, then Most used (`lib/suggested.ts`, from the relation counts in one ask); a project's default App on Used in this project (the relation cache's folders); Compose's To on Open now; every related list's filter on Most here (by rows), All pinned on top; the Processes filters list only what has processes now (`groups={[]}`, said why). Type-to-filter, arrows and Return were already the Picker's (0.3.2). Tests: `lib/suggested.test.ts`; vitest all. Left: K4 by hand; each picker open in the gallery, both themes) | Priority P0 | Effort M | Depends: AJ-E3102
**Goal**: Each picker's top group is what you'd most likely choose here.
**User story**: Choosing a project from a session page, the session's project and the open ones come first, then All, and I can type to filter.
**Requirements**:
1. Every entity picker SHALL use the shared Picker's suggested-then-All form (built in 0.3.2) with its call site supplying Open now, Recent and In this project as fit.
2. Empty or irrelevant items SHALL go last or behind "Show empty".
3. Type-to-filter, arrows and Return SHALL work in every one.
**UI/UX reasoning**: `docs/plan/QUALITY.md` "Finding things".
**Performance budget**: opens ≤ 100 ms with 1 000 items (windowed).
**Quality gates**: a lint listing pickers without a top group; L3 pairs.
**Evidence/validation proof**: scenario K4.
**Visual testing**: each picker open, both themes.
**Files to touch**: `ui/src/components/controls/Picker.tsx` call sites.
**Research basis**: D-62 decision 21.
**Handover note**: —

### AJ-E3106 Beside a session's conversation, what the tools' own apps show: subagents running, the sessions it started; and each tool's own marks | Status: DONE-EVIDENCED (2026-11-07, release/0.4.1; built: Req.1–2 built in 0.3.4: `pages/session/ConversationSwitcher.tsx`, Running, Started from here, Done. Req.3–4 built 2026-11-04: Codex's `phase: "final_answer"` marks a text block (`conversation_lines`, the feed never merges it with commentary, `TurnBlockInfo.final_answer`) and `foldOf` takes it as the reply, commentary folded even after it; a step's own description shows with its command beside it (`Steps`). Tests: `conversation_feed_tests::codexs_final_answer_is_its_own_block_and_marked`, `turn-marks.test.tsx`. Left: the K-walk on a live session with running and ended subagents) | Priority P0 | Effort M | Depends: AJ-E3103
**Goal**: A session's Overview lets you switch, from beside its conversation, to what runs inside it and to what it started, the way the Claude app's tasks pane and ChatGPT's Codex side panel do. The Subagents tab stays the full, deeper list.
**User story**: As someone watching a long session, I see the three subagents running now and the two sessions it started beside its conversation, and one click shows each one's conversation or page, without leaving the Overview.
**Requirements**:
1. Beside the Conversation card, a list SHALL show the session's subagents running now, then the last few that ended ("Done · 12 ›" opens the Subagents tab). Each opens its own conversation in the same card, with a way back. This matches Claude's tasks pane (running work) and Codex's Active / Done panel.
2. The same list SHALL show the sessions this one started, from the Claude app's own record (`spawnedFrom.sessionId`; archived ones left out). Each opens its page.
3. Codex's own marks SHALL decide a turn's reply: the message Codex marked `final_answer` (its `phase`), commentary folded with the work even when it comes after.
4. A step's line SHALL be its own description when the tool records one (Claude Code's Bash `description`: "Run the full preflight"), with the command beside it.
5. Read-only and paged, like every list (D-37, instant and paged).
**UI/UX reasoning**: the maintainer, testing 0.3.3: "on overview we want to match that capability as close as possible how claude and codex and other apps default to what subagents are showing … while on subagents tab is a full like we have right now". Research: `docs/research/native-conversation-display.md`.
**Performance budget**: the list paints from the subagent index already read for the tab; ≤ 100 ms.
**Quality gates**: vitest for the list and the switch in place; the conformance kit's subagent check for each adapter's "running now"; L3 pairs.
**Evidence/validation proof**: scenario K-series walk on a session with running and ended subagents and started sessions.
**Visual testing**: the Overview with the list, both themes, 1024 and 1280.
**Files to touch**: `ui/src/pages/session/SessionOverview.tsx`, `SessionColumns.tsx`, `LastMessages.tsx`; `crates/juggler-adapters/src/codex_transcript.rs`, `last_turns.rs`, `tool_summary.rs`; a Claude app reader for `spawnedFrom`.
**Research basis**: `docs/research/native-conversation-display.md` §1–§4.
**Handover note**: 0.3.4 already folds a finished turn's work and fixed the Subagents tab; this is the side list and the tools' own marks.

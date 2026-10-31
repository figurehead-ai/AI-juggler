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
4. The plan SHALL include wireframes for the project, profile, account and session pages at 1280 and 1024, both themes, following the readability rule (headings, labels, values and areas distinct by colour and surface).
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

### AJ-E3102 Relations made cheap: instance and conversation indexes, and a relation-count cache | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3202
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

### AJ-E3103 One related-items list and one row, everywhere | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3102
**Goal**: Every related tab, the rail and Home use one list and one row, so a session looks and behaves the same wherever it's seen.
**User story**: As someone moving between pages, a session's row always shows its app, state, App badge, project and when, in the same places.
**Requirements**:
1. `RelatedList` SHALL open on needs you, working, open, active in the last 7 days, then the rest; search past 8 items; filter chips (state, App, Account, project, Mac when paired); keyset paging; new rows wait behind "N new".
2. One row SHALL replace SessionRowBody, the rail's PlaceRow and Home's tiles.
3. Empty states SHALL teach (what would appear here, and the step that brings it).
**UI/UX reasoning**: one row anatomy (D-47); the 14 principles.
**Performance budget**: a tab opens ≤ 150 ms from cache; scrolling 1 000 rows stays at 60 fps.
**Quality gates**: L3 all pairs over page × tab × size × filter; gallery unchanged where only the row moved, critique where it changed.
**Evidence/validation proof**: scenario K2.
**Visual testing**: every page's lists, both themes, 1024 and 1280.
**Files to touch**: `ui/src/components/RelatedList.tsx` (new), `ui/src/components/SessionRow*.tsx`, `WorkflowRail.tsx`, `Home*.tsx`.
**Research basis**: `docs/research/relations-map.md` "One list".
**Handover note**: —

### AJ-E3104 Every page leads to what's related: header chips and counted tabs on project, session, App, Account, pool, Mac and Tool | Status: TODO | Priority P0 | Effort L | Depends: AJ-E3103, AJ-E3204, AJ-E3205
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

### AJ-E3105 Every picker: suggested for where you are, then All, with search | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3102
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

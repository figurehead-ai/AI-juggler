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

## Step 1 — the map and the pages

### AJ-E3101 What relates to what: the map, each page's tabs, and one list for all of them | Status: TODO | Priority P0 | Effort M | Depends: —
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

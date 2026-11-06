# Evidence manifest — AJ-E3101

Task: `docs/tasks/E31-everything-links.md` AJ-E3101, what relates to what:
the map, each page's tabs, and one list for all of them. Research done
2026-10-30, approved as D-62. Checked on 2026-11-06 (release/0.4.1). R4
was amended on 2026-11-06: the wireframes must live in the repo beside
their approvals. Status: **DONE, 1 gap open** (agent self-check,
2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `docs/research/relations-map.md` §2 "The map (each pair)": every pair of project, session, profile (App), account, pool, Mac, tool, plus tasks and Work items, with how it is known (stored, read, inferred) and a file:line, the list today, its cost, where it shows and whether both ways. "What's missing" lists the absent indexes and cache. |
| R2 | PASS | `relations-map.md` §4 "Proposed tabs": a tab set per page (project, session, profile, account, pool, Mac, tool) with counts, today's tabs kept unless named as replaced (Pool → Pools); the default order (needs you, working, open, last 7 days, the rest); filter chips; search past 8; empty states that teach (a project's and a profile's Pools tab). |
| R3 | PASS | `relations-map.md` §5 "One list for every related tab: `RelatedList`": the order, search, filters, keyset paging (`paging.rs` `PageRequest` / `page_of_fetched`), one row anatomy shared with the rail and Home (replacing `SessionRowBody`, `PlaceRow` and the Home tiles), counts from a relation-count cache. |
| R4 | GAP | The amended text needs wireframes for the project, profile, account and session pages (1280 and 1024, both themes) in `docs/wireframes/`, beside their approvals. They are not there: that folder holds only the 2026-10-13 screens 01–27 and their `wireframe-approvals.json`. `relations-map.md` line 5 says the wireframes are on a proposal page handed over outside the repo. |
| R5 | PASS | D-62 (`docs/review/decision-log.md`, 2026-10-30) records the approval with E30's proposal; decisions 19–21 are this map's. The build tasks AJ-E3102 to AJ-E3106 in `docs/tasks/E31-everything-links.md` are written from them. |

Performance: n/a (research). The map's cost column and §5 state the
budget the build holds (a related tab ≤ 150 ms from the cache).

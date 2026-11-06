# Evidence manifest — AJ-E3103

Task: `docs/tasks/E31-everything-links.md` AJ-E3103, one related-items
list and one row, everywhere. Built 2026-11-03. Checked on 2026-11-06
(release/0.4.1). Vitest files named below were run on 2026-11-06 (85
tests, all passed); Rust tests run in the release preflight. Status:
**DONE, 1 gap open** (agent self-check, 2026-11-06; R2's test of Home's
Pick up rows added the same day; its Needs you lines wait on a design
call).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | **Order:** `ui/src/components/related/related-order.test.ts` "is needs you, working, open, the last 7 days, then earlier, each newest first"; `ui/src/components/related-list.test.tsx` "heads its sections once there's more than one, in order". **Search and filters past 8, against a reference:** the all-pairs cases page × size (0, 1, 8, 9, 50, 1 000) × filter (state, App, Account, project) × search in `related-list.test.tsx`. **Mac filter, keyset paging:** `juggler-app/tests/relations.rs` `every_list_opens_on_what_needs_you_and_every_filter_narrows_it` (pages of 7 hold the order with no row twice; every project, App, Account and Mac value narrows to its count). **"N new":** `related-order.test.ts` "a row that comes in above what's shown waits…" ("1 new", "2 new") and `related/use-held.test.tsx`. |
| R2 | GAP | `components/row/SessionRow.tsx` replaces `SessionRowBody` and the rail's `PlaceRow`, and Home's "Pick up where you left off" uses it (`HomeRecent.tsx`). **Closed in part (2026-11-06):** `components/home-chips.test.tsx` "AJ-E3103 R2 — Home's Pick up rows are the shared session row": a Pick up row draws through `SessionRow`'s list density with the same app icon, title and place line as a page's list row, its Mark reviewed where a page says "Ended". **Missing (a design call):** Home's Needs you lines and the rail's Needs you rows keep their own need-line anatomy (the task's own "Not done"); whether "Home's tiles" includes those lines, and moving them onto the shared row, is the maintainer's call. |
| R3 | PASS | `related-list.test.tsx` "an empty list teaches: what would appear and the one step" (the words and its New session button) and "a filter that empties it says so, and Clear brings everything back". |

Performance (a tab ≤ 150 ms from cache; 1 000 rows at 60 fps): measured
in the task's Playwright run (Sessions tab 7–9 ms; 1 000 rows at a
16.8 ms median frame). Gallery `ui/tests/visual/related-list-gallery.spec.ts`.

Not yet done: the K2 walk by hand; the "N new" pill isn't captured in the
gallery (needs a live backend).

Gaps closed on 2026-11-06: the whole vitest suite passed (`pnpm exec vitest run --maxWorkers=4`: 281 files, 2135 tests) with the tests named above.

# AJ-E3303 — Links validated

2026-11-07, release/0.4.1. The chain from Home to a pool and back, every hop and every count.

| Req | Result | Evidence |
|---|---|---|
| R1 (L2) | PASS | Every page's counts on a fresh replica (9 setups, 16 projects, 340 conversations): the backend harness (`crates/juggler-app/tests/qa_real_readonly.rs`) now records what each page heads its tabs with, from the relations cache, for everything, each of the 16 projects, each of the 9 folders (their CLI Apps), each of the 8 Accounts and both tools; `scripts/qa/real_truth.py --compare` checks each against the transcripts' own tallies: 36 of 36 equal (2026-11-07). The projects and each folder's conversations equal the oracle too (`docs/qa/evidence/AJ-E3205/usage-oracle-2026-11-06.md`) |
| R1 (L3) | PASS | `ui/tests/visual/l3-pages.spec.ts`: all 52 rows of `docs/qa/pairwise/pages.csv` (every pair of page, tab, size 0–1 005, filter, search, 3 new arriving, first state, theme and width), every page reached the way a user would; 52 of 52 pass, 84 clicks, feedback p95 12 ms, max 22 ms (`docs/qa/evidence/l3/pages.json`) |
| R1 (L4) | PASS | Walks K1–K4 in the Dev edition through the lab write mode, both themes at 1024 and 1280: 15 steps, none under 4 (K1's two speed 3s re-measured on 2026-11-07: a session's page draws in 138–163 ms and back in 51–62 ms in a debug build; `docs/qa/0.4-walks/K1.md`–`K4.md`) |
| R2 | PASS | 1 000+ sessions page without a stall: the L3 rows with 1 005 sessions on the App, Account, project, This Mac and Tool pages open, search and filter within the budget; walk K1 paged 334 and 501 live. "N new" never jumps the list: L3's arriving rows on every page with a list (rows held behind "3 new", the first row unmoved; under a lone row that needs you they join the end, as `hold.ts` says) and K2 |
| Budget | PASS | ≤ 150 ms per hop from cache: K1's hops 24–63 ms back, a session's page 138–163 ms in a debug build (the release build is timed with AJ-E3306) |

Fixed on the way: the design preview's paged list now follows changes as the live one does (d4ebc327c), and says `data-read="read"` once its first fresh read shows (70f7ba35f).

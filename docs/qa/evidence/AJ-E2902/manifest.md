# Evidence manifest — AJ-E2902

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2902, cost
you can act on. Built in 0.4. Checked requirement by requirement on
2026-11-06 (release/0.4.1). vitest `cost-review.test.tsx` and
`cost-tile.test.tsx` pass (2026-11-06); the Rust tests named below run in
the release preflight. Status: **DONE, 3 gaps open** (agent self-check,
2026-11-06). The task header still says DONE-EVIDENCED from the first
check below; that check was by test file, not by requirement.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `cost-review.test.tsx` "per finished task: the average, its tokens and the count" asserts "~$1.92" and "120k tokens a task · 44 finished"; "nothing finished, nothing wasted: words, never $0" asserts "No finished tasks in this range". `crates/juggler-app/tests/cost_cuts.rs` `outcomes_read_the_ending_and_the_last_state_change` and `cuts_count_finished_and_wasted_and_sum_accounts_with_their_top_project` count only sessions that ended as Finished (D-77). AJ-E2903 counts the same status (`juggler-app/tests/account_quality.rs`). |
| R2 | GAP | `cost-review.test.tsx` "wasted: the estimate, what happened (stuck says probably), and the sessions a click away" asserts "~$6.30" and "On 2 sessions: 1 crashed, 1 probably stuck — see them"; "plain rows under its headings…" asserts the sheet's rows say Crashed or Probably stuck. `cost_cuts.rs` asserts the crashed and stuck counts and $6.30. **Missing:** no test clicks a row in the sheet and checks it opens that session's page, and none loads a second page. |
| R3 | PASS | `cost-review.test.tsx` "each account with ~cost, its top project and how sure the price is; sorts by tokens" asserts the rows costliest first, "~$80", "list price, but one model has none", the top project link opening `p-aij`, and re-sorting by tokens. `cost_cuts.rs` asserts Work's two folders summed to $8.00 (the session estimates only, no fee) with top project AIJuggler, and Personal unpriced. |
| R4 | GAP | `cost-review.test.tsx` "the last full week starts on the locale's first day" asserts the week in en-GB (Monday) and en-US (Sunday); "opens on that week against the one before, and exports just that week" asserts the heading, a "~$" cost and an export for exactly that week. **Missing:** no test checks the review's tokens and week-before figures, its per-account, per-task and wasted parts, or the five costliest sessions (sorted and capped at 5). |
| R5 | GAP | `cost-review.test.tsx` "names the price list, and says so when some prices are the user's" asserts "List prices of 3 Nov 2026"; Cost by account shows it. **Missing:** no test for the cost tile's note (`Dashboard.tsx` `priceNote`), for the date in Prices and details (`CostMatrix.tsx` `cost-price-list`), for the stale warning still showing, or that no Update button is shown. |
| R6 | PASS | [load.txt](load.txt): `cost_cuts_load.rs` reconciles the account table with Prices and details to the cent ($685.70 both) on 2,000 sessions. `cost-tile.test.tsx` "says words, never $0, when nothing could be priced" and `cost-review.test.tsx` "nothing finished, nothing wasted" assert words, never $0; the tiles' figures assert the "~". |

Performance: [load.txt](load.txt), the cuts in 14 ms median (budget 50),
the review's reads in 1.2 ms (budget 100).

Gallery: `99-usage-cost`.

## The first part (2026-11-05)

- `cargo test -p juggler-app --test cost_cuts` (finished, wasted, top
  project, endings) and vitest `cost-tile.test.tsx`, `cost-review.test.tsx`
  — pass (2026-11-05).
- [load.txt](load.txt): the `just load-gate` row "Usage cost cuts, 2,000
  sessions" (`tests/cost_cuts_load.rs`: 10 accounts, 90 days): the cuts in
  14 ms median (budget 50), the weekly review's reads in 1.2 ms (budget
  100); the account table's dollars equal Prices and details' total to the
  cent ($685.70 both).
- Gallery `99-usage-cost`: the tiles, the account table, the wasted sheet,
  the review; both themes, 1280 and 1024; axe 0 critical/serious;
  critique stamped.
- Verdict: PASS.

# Evidence manifest — AJ-E3301

Task: `docs/tasks/E33-validated-by-use.md` AJ-E3301, the validation kit. Built
2026-11-04. Checked 2026-11-06 (release/0.4.1): every self-test below was
run and passed, and `just qa-kit` runs them in `verify-static`. Status:
**DONE-EVIDENCED** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | 32 scenario files in `docs/qa/scenarios/` (P1–P11, A1–A10, K1–K4, N1–N5, X1–X2), each with its setup and every step's expected outcome. `python3 scripts/qa/scenario.py check`: "32 file(s), 91 steps, OK". `--selftest` ok. Each walk sheet is generated from them (`scenario.py sheet <id>`, the header of `docs/qa/0.4-walks/P4.md`), so a sheet and its automation can't drift. |
| R2 | PASS | `scripts/qa/pool_oracle.py`, written from D-62 and `docs/research/pools-model.md`, not from the Rust. `--selftest`: "15/15 ok; tables (28224, 9216, 27)". It emits L1's tables, which `juggler-core/tests/pools_l1.rs` checks the resolver against in all 28,224 membership, 9,216 talk and 27 channel cases. |
| R3 | PASS | `scripts/qa/pairwise.py`: deterministic IPOG, no randomness. The wizard has 70 rows, the canvas 42, the pages 52 and New session 18, each with a coverage report naming the row for every pair. The matrices are checked in. `check` fails when one is stale or a pair is uncovered: "new_session: 18 rows, every pair covered". `--selftest` ok. |
| R4 | PASS | `ui/src/lib/walk-timing.ts`, opt-in with `localStorage["aij.walk-timing"]` or `?walk`, records each click or ↩'s time to feedback and to done (`[walk] feedback 42 ms · done 180 ms`), kept in `window.__AIJ_WALK__` for the sheet's ms column. The P4 sheet's "ms (feedback / done)" column was filled this way on 2026-11-05. |
| R5 | PASS | The walk sheet: `scenario.py sheet` makes it, one row per step with Do, Expect, Seen, ms, the four scores and both themes' captures (`docs/qa/0.4-walks/README.md`). The lab: `scripts/qa/lab_scenario.sh setup\|open\|teardown` builds a scenario's state in a throwaway HOME (`tests/qa_scenario_seed.rs`, which refuses any folder that isn't a lab's) and tears it down: sessions stopped, processes with files open under the lab ended, the lab removed. P3 was set up and torn down end to end in about 2 s. |
| R6 | PASS | The replica (`scripts/replica.py`) holds two Claude folders on one sign-in (several Apps on one Account) and three Claude app profiles under `ClaudeProfiles/`, the first run by `live` and the rest closed. It also holds sessions other apps started, which a pool covering them lists as Watched. Pools, their scope, level, seal and members are seeded per scenario (`qa_scenario_seed.rs` `seed_scenario`: `create_scoped`, seal, members). |

Performance (L1's tables in < 2 s): the oracle's tables and selftest
finish in under a second.

The task's proof run, P1 through all four layers, is the pools walks'
(AJ-E3304). Its live steps use the lab's Claude Code sign-in (done
2026-11-06) and run with those walks.

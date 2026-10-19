# Evidence manifest — AJ-E1108

Task: `docs/tasks/E11-usage-statistics.md` AJ-E1108 (the puzzle solver
and the user agent). 2026-10-18, reviewer claude-opus. Log:
[`../AJ-E1104/cargo-test-juggler-stats.txt`](../AJ-E1104/cargo-test-juggler-stats.txt).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| With `pow_bits` > 0, a lowercase-hex nonce (≤ 32 chars) such that `sha256("ajstats-pow-v1:" ‖ report_id ‖ ":" ‖ week ‖ ":" ‖ nonce)` has that many leading zero bits, on a low-priority thread after the upload delay; 60 s budget then next tick; above 28 bits postpone 24 h | Req.1 | `pow::solve` (counts up from 0), `pow::check`; solved only for a due report, inside the tick, which runs on its own thread at the lowest priority (`the_tick_thread_runs_at_the_lowest_priority`). `a_budget_stops_the_search`; `a_puzzle_above_28_bits_postpones_a_day` (no POST) | PASS | 2026-10-18 | claude-opus |
| `403 {"error":"pow","bits":N}` refetches the config and solves again next tick (counts as an attempt); with `pow_bits` 0 there is no `pow` field | Req.2 | `a_puzzle_is_solved_and_a_raised_one_refetches_the_config` (403 → retry, second config GET, nonce passes `check` at 8 bits); every other upload test sends no `pow` | PASS | 2026-10-18 | claude-opus |
| Every stats upload sends `User-Agent: AIJuggler/<version>` | Req.3 | app `a_debug_build_uploads_to_the_local_endpoint_with_the_contract_headers` asserts it on the config GET and the report POST, read off the wire | PASS | 2026-10-18 | claude-opus |
| Vectors the server's check accepts | quality gate | Eight nonces (8, 12, 16 and 20 bits, two reports and weeks) checked on 2026-10-18 with the service's own verifier, run locally from the read-only website clone: all accepted; all rejected at +8 bits and for another week. They are pinned in `pow::tests::the_solver_reproduces_the_checked_vectors` (the solver finds exactly these) | PASS | 2026-10-18 | claude-opus |
| 20 bits ≈ 0.1 s on one Apple-silicon core | perf budget | `pow: 20 bits in 189 ms (nonce 4267b9)`: that vector needed 4.35 M tries (an unlucky draw), so ≈ 43 ns per try; 20 bits takes 2²⁰ ≈ 1 M tries on average, ≈ 45 ms (debug test build; `sha2` and the solver are optimised in dev builds). The solver writes the nonce into a stack buffer (no allocation per try) | PASS | 2026-10-18 | claude-opus |

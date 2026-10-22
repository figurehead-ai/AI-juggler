# Evidence manifest — the sixth fix batch (fix/maintainer-reports-6)

Not a task's evidence. This covers the coordinator's batch 6, items 1 and 3:
- tests leave no process behind when they fail, or when the run is killed;
- Link off means off: the `fleet_sync` "race" was a product bug (the coordinator's review).

Item 2 (Codex reads ×1.015 over compressed sessions) follows separately, once QA's per-session comparison is in. Only test code changes here.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Every test's processes reaped on failure | The sweep of every `crates/*/tests` file that starts a process. `audit`, `kill_sheet`, `recovery`, `tiers`, `cleanup`, `second_launch` and `observed` register their supervised roots' process groups, and their lone processes, with the shared `fixtures::Reaper` the moment each starts (`ports` and `collateral` since #131). The Reaper sends `kill -s KILL -- -<pgid>` on drop, a panic's unwind included. Injected check: a panic right after `tiers`'s drill tree started its dev server and MCP helper; afterwards both pids were gone | none left | passed; the injection reverted | PASS | aa5a1a81 | 2026-10-22 | claude-opus |
| Every test's processes bounded in time | For a run killed outright, where no `Drop` runs: everything ends itself within `FIXTURE_LIFETIME_SECS` (300 s). This covers the python `http.server`s (`fixtures::http_server`, and `escaping_http_server` for the setsid escapee), `kill_sheet`'s TERM-ignoring loop (time-bounded), `observed`'s busy loop (bash `SECONDS`, no forks) and `identity`'s sleeps (300 s, not 600). `observed` no longer pkills every busy loop on the machine. The rest were already bounded: `sleep` ≤ 300 s, `trap_helper.py`, fake-desktop and fake-harness (they end with their parent) | ≤ 300 s | as stated | PASS | aa5a1a81 | 2026-10-22 | claude-opus |
| Link off means off (was "`fleet_sync` catch-up race") | The flake asked from 31, past the node's top sequence when the console's Link went off: the console kept applying after Link off. `stop()` aborted the reconcile loop only, and dropping its connections detached their heartbeat and telemetry tasks. `stop()` now ends the loop, then every connection's tasks, and waits for each (`link::end_tasks`). `fleet_sync :: a_console_with_its_link_off_applies_nothing` (the node's own 2 s pass sends 4 changes after the console's Link is off; its mark and replica must not move); the original test keeps `resumed.0 <= high_water` and adds a check that the mark stayed at or below it | fails on the old code | old code: failed 2 of 2 (mark 1 → 5); new: fleet_sync 5 of 5, 3 runs | PASS | (this push) | 2026-10-22 | claude-opus |
| Reaper: never a reused id | Each target's start time is noted at registration. A lone pid is killed only while it is the same process; a group is killed unless its leader's pid now names another process (while a group has members no new process can take its id). Ids 0 and 1 are never registered. `audit`'s shell leads its own group, so its `sleep` dies with it. `reaper :: only_what_the_test_started_is_signalled`, `ids_zero_and_one_are_never_signalled` (decisions tested through `Reaper::targets`, without signalling) | — | passed | PASS | (this push) | 2026-10-22 | claude-opus |

## Gates

| Gate | Head | Result |
|---|---|---|
| `just verify` (`CARGO_INCREMENTAL=0`) | dc60814d | 150 test binaries, 1,200 passed, 0 failed; every step after the tests OK; no fixture or `yes` left running afterwards |
| `cargo deny check` | dc60814d | advisories, bans, licenses, sources ok |
| `vitest`, visual, a11y | — | nothing under `ui/` changed |

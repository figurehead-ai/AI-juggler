# Evidence manifest — the sixth fix batch (fix/maintainer-reports-6)

Not a task's evidence. This covers the coordinator's batch 6, items 1 and 3:
- tests leave no process behind when they fail, or when the run is killed;
- the `fleet_sync` catch-up race.

Item 2 (Codex reads ×1.015 over compressed sessions) follows separately, once QA's per-session comparison is in. Only test code changes here.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Every test's processes reaped on failure | The sweep of every `crates/*/tests` file that starts a process. `audit`, `kill_sheet`, `recovery`, `tiers`, `cleanup`, `second_launch` and `observed` register their supervised roots' process groups, and their lone processes, with the shared `fixtures::Reaper` the moment each starts (`ports` and `collateral` since #131). The Reaper sends `kill -s KILL -- -<pgid>` on drop, a panic's unwind included. Injected check: a panic right after `tiers`'s drill tree started its dev server and MCP helper; afterwards both pids were gone | none left | passed; the injection reverted | PASS | aa5a1a81 | 2026-10-22 | claude-opus |
| Every test's processes bounded in time | For a run killed outright, where no `Drop` runs: everything ends itself within `FIXTURE_LIFETIME_SECS` (300 s). This covers the python `http.server`s (`fixtures::http_server`, and `escaping_http_server` for the setsid escapee), `kill_sheet`'s TERM-ignoring loop (time-bounded), `observed`'s busy loop (bash `SECONDS`, no forks) and `identity`'s sleeps (300 s, not 600). `observed` no longer pkills every busy loop on the machine. The rest were already bounded: `sleep` ≤ 300 s, `trap_helper.py`, fake-desktop and fake-harness (they end with their parent) | ≤ 300 s | as stated | PASS | aa5a1a81 | 2026-10-22 | claude-opus |
| `fleet_sync` catch-up race | `fleet_sync :: an_offline_console_catches_up_with_only_what_it_missed_and_keeps_last_known_rows` failed once under load, asking from 31, past the `high_water` the test had read (a batch already on its way when the link went down). The test now reads the console's own mark just before it reconnects; the catch-up must ask from exactly that mark (never 0) and send exactly the changes after it. The AJ-E806 record's first reading is corrected | stable | 3 of 3 alone; 3 of 3 beside 24 busy processes (load 17) | PASS | dc60814d | 2026-10-22 | claude-opus |

## Gates

| Gate | Head | Result |
|---|---|---|
| `just verify` (`CARGO_INCREMENTAL=0`) | dc60814d | 150 test binaries, 1,200 passed, 0 failed; every step after the tests OK; no fixture or `yes` left running afterwards |
| `cargo deny check` | dc60814d | advisories, bans, licenses, sources ok |
| `vitest`, visual, a11y | — | nothing under `ui/` changed |

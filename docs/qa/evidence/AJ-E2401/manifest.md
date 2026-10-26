# Evidence manifest — AJ-E2401

Task: `docs/tasks/E24-processes.md` AJ-E2401, The process graph: one model of apps, sessions and processes. Branch `e24-processes`. Status: **IN-PROGRESS**.

## What exists now

`crates/juggler-app/src/features/processes/` (model, observe, query, roles, mod, dto, commands) builds the graph on the 2 s tick; `crates/juggler-supervisor/src/live_stats.rs` adds a zombie flag, CPU time, the observed trees' rows, the listeners and the view's own observed roots (no new scan); `process_ledger.rs` adds the live and ended reads.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1–3 every state in one vocabulary, from fixture inputs | `cargo test -p juggler-app --test processes_graph every_state_is_in_the_one_vocabulary` | 11 states as expected; the zombie names its parent | pass | PASS | — | 2026-10-25 | agent |
| Live: a real zombie and Left behind on a real Mac | `docs/qa/processes-lab.md` steps 2 and 8, 2026-10-26 ([lab-1-2026-10-26.md](docs/qa/evidence/AJ-E2406/lab-1-2026-10-26.md)) | Left behind within 40 s; the zombie as "Waiting for its parent" | Left behind in 34–36 s (the ledger's reconciler, even with the lifecycle forwarder ended); a real `Z` child under an AI Juggler session never reached the graph (`zombies: 0` for over 2.5 min, bug 4). The 2 s tick also died once in a debug build, in sysinfo's disk listing (AJ-E1902 lab, bug 4) | FAIL | — | 2026-10-26 | agent |
| Req.4 whose, how it may be stopped, roles in words | `cargo test -p juggler-app --test processes_graph whose_and_how_each_may_be_stopped` | as asserted | pass | PASS | — | 2026-10-25 | agent |
| Req.5 totals and the top bar's count; Keep and Remind me later | `cargo test -p juggler-app --test processes_graph totals_roll_up_and_the_top_bar_counts_what_to_clean` | to_clean 5 → 3 → 4 | pass | PASS | — | 2026-10-25 | agent |
| Req.6 paging by cursor, no repeats or gaps | `cargo test -p juggler-app --test processes_graph pages_follow_a_cursor_without_repeats_or_gaps` | every key once, in order | pass | PASS | — | 2026-10-25 | agent |
| Req.1/6 end to end on a real fake-harness session: built from the tick, HelperOrphaned wired, Left behind, This Mac agrees, Keep, Ended | `cargo test -p juggler-app --test processes_live` | pass | pass (6.3 s) | PASS | — | 2026-10-25 | agent |
| Req.7 AI Juggler's own hidden and never counted | `cargo test -p juggler-app --lib processes::roles`; `--test processes_graph everything_running_folds_healthy_sessions_to_one_line` | pass | pass | PASS | — | 2026-10-25 | agent |
| Budget: 50 sessions, 2,000 processes | `cargo test --release -p juggler-app --test processes_graph the_graph_and_a_page_fit_the_budget_at_two_thousand_processes -- --nocapture` | build ≤ 5 ms, page ≤ 50 ms | build 1.92 ms; Needs attention page 0.16 ms; Everything running page 0.52 ms; search 0.02 ms (release, this Mac) | PASS | — | 2026-10-25 | agent |
| Budget: the tick's extra CPU ≤ 1 % of a core | derived: build 1.92 ms per 2 s tick = 0.1 % of a core, plus the reads it shares with the tick | ≤ 1 % | ≈ 0.1 % measured part; the gathering not timed separately | UNPROVEN | — | 2026-10-25 | agent |
| Existing suites unchanged (collector, reconciler, guardrails, collateral) | `cargo test -p juggler-supervisor --test collateral --test reconciler --test observed --test orphan --test ledger --test identity`; `cargo test -p juggler-app --test guardrails --test processes --test cleanup --test tiers --test observed_guard --test kill_sheet --test ports --test recovery --test machine_fixtures` | pass | pass | PASS | — | 2026-10-25 | agent |
| Visual: the page | AJ-E2404's gallery, `_gallery/90-processes/` | looked at | looked at (48 captures) | PASS | — | 2026-10-26 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)

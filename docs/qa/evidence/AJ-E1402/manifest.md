# Evidence manifest — AJ-E1402

Task: `docs/tasks/E14-machine-health.md` AJ-E1402 (see the AI tools AI
Juggler didn't start, read-only). Status: **DONE-EVIDENCED**.

## What exists now

- `running_setups::tools()`: one root per running Claude tool — each Claude
  desktop app instance, and each `claude` outside it. A `claude` started
  by another `claude`, or inside the desktop app, is part of that tree and
  never a second root.
- `running_setups::tools_from()`: classifies from the collector's own
  process table, with no second full refresh and no environment. It then
  reads `CLAUDE_CONFIG_DIR` for the roots alone, through a throwaway table
  that is dropped at once. Re-read every 30 s (15th tick).
- `LiveStatsCollector`: observed roots are refreshed on the hot path and
  measured over their whole tree, with a 5-minute ring. The sums exclude
  every supervised pid and AI Juggler itself, with their subtrees (a dev
  build may run under a terminal `claude`). Observed trees never reach the
  ledger, the reconciler or a stop path.
- Attribution: a tool's folder is matched against each Account's managed
  folder and the folder it was brought in from.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 one root per tree | `cargo test -p juggler-app --lib -- running_setups` (`tool_roots`: desktop app's own `claude` and a `claude -p` run by a `claude` are not roots; folder match with `~` and trailing slash) | pass | pass | PASS | 583c2f0d+ | 2026-10-20 | claude-opus |
| Req.2 measured, no double count | `cargo test -p juggler-supervisor --test observed`: pruning (AI Juggler and its session under a terminal `claude` excluded with subtrees); a real `sh → 2 × sleep` tree measured (2 helpers, RSS > 0, ring), absent from the sessions, dropped after exit | pass | 2 passed | PASS | 583c2f0d+ | 2026-10-20 | claude-opus |
| Req.3 never managed (D-08) | `cargo test -p juggler-app --test observed_guard`: an observed pid is refused by `kill_verify_inner`, `kill_execute_audited` and `orphan_stop_inner` (any claimed session), the automatic sweep runs, and the process is still alive | refused, alive | refused, alive | PASS | 583c2f0d+ | 2026-10-20 | claude-opus |
| Req.4 environment read limited | code review: `config_dirs_of` keeps only `CLAUDE_CONFIG_DIR` of root pids; nothing logged; the live probe prints counts only | — | as stated | PASS | 583c2f0d+ | 2026-10-20 | claude-opus |
| Budget: scan ≤ 10 ms p95 | `cargo test -p juggler-app --lib machine_health::live -- --ignored --nocapture` (reference Mac) | ≤ 10 ms | first design (every environment): median 107 ms, max 1.39 s. Rebuilt two-phase: median 3.0 ms, max 4.1 ms | PASS | 583c2f0d+ | 2026-10-20 | claude-opus |
| Req.5 rail rows? (D-25 amendment) | Walked the rail at 1440×900 as built, and with a quiet "Also running · not started by AI Juggler" group of six rows (this Mac's real count; a throwaway DOM mock-up in the preview): [mock-up](docs/qa/evidence/AJ-E1402/req5-rail-mockup-1f0b5cd1.jpg) | a decision with evidence | No rail rows. The six rows pushed the user's own sessions down, looked openable with nothing to open, and repeated This Mac. Recorded as the D-25 amendment of 2026-10-20 | PASS | 40293b45+ | 2026-10-20 | claude-opus |
| Live reading (reference Mac) | same probe | real values | 6 Claude desktop trees, 132–191 helpers, 16–28 GB, 2.2–12.7 cores (it varies with what those windows' sessions run; the windows' own Claude Code sessions, e.g. cargo builds, belong to the app's tree); 0 terminal `claude` | PASS | 583c2f0d+ | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout (no paths printed)
- [x] Transcript excerpts scrubbed of personal content (none)

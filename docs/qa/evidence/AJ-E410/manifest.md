# Evidence manifest — AJ-E410 (Processes tab, resource collector, orphan contract)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E410.
Branch: `stage2/e4-processes-orphans`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Rust suites run in CI (verify job); component tests
in the ui job.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 2s staggered sampling | `cargo test -p juggler-supervisor --test orphan` (`collector_targeted_tick_overhead_under_budget`) + app tick (lib.rs) | per-tree sample every 2 s; <1% CPU at 20 Instances | 2 s `processes_tick` thread: targeted refresh of known pids each tick (one sampler thread — the per-tree herd cannot exist by construction), full-table discovery every 5th tick + 3 warmup ticks; 150-sample (5 min) ring per tree asserted; targeted-tick mean at 20 live fake-harness trees < 20 ms (1% of the 2 s budget) asserted | PASS | 2026-10-13 | zcode-primary |
| Req.1 ppid-map attribution | same + `tree_rows` (supervisor.rs) | RSS/CPU by ppid-map via descendants walk | rows enumerate root + descendants breadth-first from a ppid→children map built over the (already refreshed) sysinfo table; no process groups, no name matching | PASS | 2026-10-13 | zcode-primary |
| Req.2 Processes tab renders per §7.3 | `pnpm --dir ui test` (processes-tab) | PID/name/CPU/RAM/age/⑂, 5s refresh, headers, rollups read-only | table with 6 `th[scope=col]` headers, tree-depth indent glyphs, honest `— first sample` for null CPU; 5 s interval refresh; banner + manual `Retry` on read failure (`role=alert` asserted); provider/account/project rollups render read-only with the note pointing at Instance-level stop actions | PASS | 2026-10-13 | zcode-primary |
| Req.3 orphan snapshot + 10s grace | `cargo test -p juggler-supervisor --test orphan` (`orphan_detected_after_grace_and_stopped_gone`) | snapshot at non-clean exit; survivors surface ≤10s+poll | real fake-harness spawns `sleep 60` child, exits code 7: pre-exit rows frozen in the collector → `ExitSnapshot` + journal `descendants` line; real-clock poll confirms exactly the surviving child between 10 s and 13 s (grace + 2 s poll + slack asserted) with PID/name/RSS/parent label/age-since-exit | PASS | 2026-10-13 | zcode-primary |
| Req.4 orphan actions | same + processes-tab tests | Verify owner; Stop (TERM→KILL, confirm names PID); Stop all (confirm lists PIDs) | `Verify owner` shows re-read ppid evidence (`re-parented to init/launchd`); `Stop process 39871` opens a confirm `alertdialog` naming the PID + the ladder copy, then lands `gone ✓`; `Stop all orphans…` confirm lists every PID and lands the escalated state verbatim (`still alive - escalated to SIGKILL`); live-tree rows carry no kill controls (Actions owns kills except orphan controls) | PASS | 2026-10-13 | zcode-primary |
| Req.5 auto-cleanup opt-in + guardrail | `cargo test -p juggler-app --test processes` (`tier1_auto_cleanup_only_touches_opted_in_accounts`, `orphan_stop_refuses_a_pid_the_journal_never_recorded`) | off by default; 60s grace; journal-descendant PIDs only; every kill audited | notify-only account: orphan survives past 60 s (asserted) while manual stop still works at tier 0; opted-in account: sweep kills the leftover and the journal carries `auto-tier1` (D-08 audit); an unrelated live PID is refused (`refused - not a recorded descendant`) and survives — name-based kills impossible by construction (allow-list = recorded pids only) | PASS | 2026-10-13 | zcode-primary |
| Req.6 stop-ladder + escalation | `cargo test -p juggler-supervisor --test orphan` (`stop_ladder_escalates_when_sigterm_ignored`, `ladder_stops_normal_child_with_term`) | TERM→grace→KILL; ignore-TERM escalates | plain child: `Gone{escalated:false}`; `--child-ignore-term` child (bash `trap '' TERM` wrapper, SIG_IGN survives exec): `Gone{escalated:true}` — KILL used; UI `gone` ✓ / `still alive - escalated to SIGKILL` states asserted in processes-tab tests | PASS | 2026-10-13 | zcode-primary |
| Req.7 resources unavailable | processes-tab test + `instance_processes` | adopted/desktop-kind → named reason, never zeros | `unavailable_reason: "resources unavailable"` renders its card, no table (asserted); unknown instance → error naming the id, never an empty table | PASS | 2026-10-13 | zcode-primary |
| I-07 sparkline source | `instance_processes` payload + processes-tab test | 5-min 2s ring exposed | `spark` carries the ring; the tab renders a CPU sparkline (`processes-spark` asserted); Quality/Performance tab now shows `CPU / RAM now` Measured from the ring + `2s sampling` footnote (completing E409 Req.6) | PASS | 2026-10-13 | zcode-primary |
| Ring retention budget | supervisor orphan test | ≤150 samples/tree | `ring.len() <= 150` asserted structurally (VecDeque::truncate(150)) | PASS | 2026-10-13 | zcode-primary |
| Storage v7 | `cargo test -p juggler-core --test storage_roundtrip` | additive migration, default off | v4 DB upgrades to v7 exactly once on open (asserted ×2 reopen); `cleanup_opt_in INTEGER NOT NULL DEFAULT 0`; roundtrips green | PASS | 2026-10-13 | zcode-primary |
| Visual | `pnpm --dir ui visual:capture` | §7.3 wireframe states | 78/78 incl. new `10-processes/` (live tree 3 levels + orphans, stop-confirm popover naming the PID, `gone ✓` verification, good empty); 09-quality recaptured (CPU/RAM + footnote); a11y suite 3/3 (axe gate + no-overflow); every Stop button labelled with its PID | PASS | 2026-10-13 | zcode-primary |
| Regression | full suites | prior gates intact | `just verify` green (fmt, clippy -D warnings, workspace tests incl. 17 supervisor unit + 5 orphan integration + 5 app processes + prior suites, docs, evidence-check, terminology); 111/111 component (102 prior + 9 new); `pnpm --dir ui build` green | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- Attribution remains **ppid-map + journal only** — no setpgid/killpg
  exists (pass2 05 §2). PID-identity hardening (start-time/argv verify
  against pid reuse, port re-probe) is the E5 epic's seam (SPEC §5.2,
  AJ-E503); `Stop all` ships with the recorded-descendant allow-list as
  its only guard — wire ProcessIdentity verify-before-kill there.
- The orphan snapshot freezes the collector's LAST pre-exit rows: a
  descendant spawned <2 s before death (or between the last full
  discovery scan and exit) is missed — the safe direction (no stop
  offered). New-descendant discovery runs every 5th tick (10 s) plus
  3 warmup full scans; the 2 s hot path refreshes known pids only.
- `ProcessesBridge` is the tab's injectable seam (RightInspector prop,
  default = the real bridge): vitest 5's mocker did not intercept this
  tab's dynamic-import site (QualityTab's identical import WAS
  intercepted — a genuine tool quirk, documented in the test file).
  Extend the seam rather than reintroducing module mocks.
- The inspector mounts all six panels hidden — any new panel component
  must tolerate fixture rows with missing optional fields (the seeded
  visual fixture predates `cwd`; guard optional field access).
- The supervisor stop ladder for the ROOT process (`Supervisor::stop`)
  is unchanged; the orphan ladder is separate (`stop_pid_ladder`) and
  shares only the signal helpers.

# Evidence manifest — AJ-E510 (Juggler-crash recovery: re-adoption with stop control)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E510.
Branch: `stage2/e5-recovery`. Local runs (macOS arm64, CLT-only machine)
2026-10-13, reviewer zcode-primary.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 recovery loads live rows + identity-verifies; failures Unverifiable, never killed | `crash_drill_readopts_verifies_and_reports` (recovery-drill.txt) | roots verified per E504 (boot+start-time for roots — exe legitimately moves across `exec`; full triple for helpers); alive-but-failed → `Unverifiable`, listed, untouched | the FORGED row (a real live pid seeded with `started_at: Some(1)`) ends `unverifiable` + label `Unverifiable — will not be touched` AND the process is STILL ALIVE at the end of the drill (`alive(forged_pid)` asserted) | PASS | 2026-10-13 | zcode-primary |
| Req.1 F-3 closed: adopted roots have a working stop | same drill | `stop()` on a re-adopted root kills the whole verified tree; no `UnknownSession` | `state2.supervisor.stop(&a)` succeeds; every pre-stop PGID member of A's tree is dead within 5 s (asserted per-pid); `alive_adopted_sessions()` still contains B (untouched survivor) | PASS | 2026-10-13 | zcode-primary |
| Req.1 reconciliation never marks adopted roots Stopped | same drill + `state.rs` | adopted sessions stay Running | reconciliation skips the adopted-marker set (`alive_adopted_sessions` = the `adopted` map keys — the old journal-minus-supervised heuristic retired); B stays adopted+alive through the whole drill | PASS | 2026-10-13 | zcode-primary |
| Req.2 down-period helpers: Orphaned + original attribution + distinct label | same drill + `cleanup_inventory` | helper of a root that died while Juggler was closed → `orphaned`, `down_period`, label `left behind while Juggler was closed` | C's http.server helper: state `orphaned`, `down_period: true`, label exact; attribution preserved (`root_pid`/`parent_pid` = C's root, first_seen unchanged); marker is the DURABLE v16 column (survives further restarts) — the in-memory `reaped_by_us` heuristic was tried first and failed because detached watcher threads reap, which is exactly why it is durable | PASS | 2026-10-13 | zcode-primary |
| Req.3 ONE summary, exact template, shown once, audited | same drill + `recovery_summary_get` | exact string `{a} Instances re-adopted, {b} helpers left behind, {c} processes could not be verified and were left alone.`; second read after dismiss → None; audit `actor=system:recovery` | summary text byte-exact (2/1/1); audit history contains the `system:recovery` row (asserted with dump); after `recovery_summary_consumed=1` the read returns None — shown exactly once | PASS | 2026-10-13 | zcode-primary |
| Req.3 auto-tiers suspended 24 h after recovery | `cleanup_status_inner(&state2)` | suspended ⇒ true after a finding | asserted in the drill right after restart; ANY finding (adopted/left-behind/unverifiable non-empty) stamps `cleanup_suspended_until` — per the handover trap, mandatory not a nicety | PASS | 2026-10-13 | zcode-primary |
| Req.4 UI: Running — re-adopted, detached notice, E507-sheet stop only | `recovery-summary.test.tsx` (6) + `processes-tab.test.tsx` E510 suite (2) + `cleanup-centre.test.tsx` E510 rows (2) | exact labels; notice leads the Processes tab; killable only through the verified sheet | `↻ re-adopted` chip (inspector header), `Running — re-adopted after restart` + detached-terminal copy asserted verbatim; notice is the FIRST child of the processes tab; `killableRow` admits `adopted` rows ONLY via the E507 sheet path; down-period label distinct from the live-session orphan one (both icon+text) | PASS | 2026-10-13 | zcode-primary |
| WCAG (UX reasoning) | dialog tests + axe gates | focus-on-open, Esc dismisses, focus returns to the board, no permanent dismiss shortcut | focus lands inside the dialog on open and returns to `<main>` on dismissal (component test); Esc + Got-it both dismiss through `recovery_summary_dismiss` (the ONLY way it goes away); axe 0 serious/critical dark+light — one contrast fix (h2 → `text-ink`) | PASS | 2026-10-13 | zcode-primary |
| Visual testing | `pnpm --dir ui visual:capture` + a11y spec | summary dialog, Re-adopted filter + left-behind label, adopted panel w/ detached notice; 1440×900 light+dark; axe 0 | `_gallery/22-recovery/` 6 captures; suite 201/201 (capture + a11y); dialog seam `__AIJ_RECOVERY__` pins the 2/1/1 text — same string the Rust drill asserts (one source of truth) | PASS | 2026-10-13 | zcode-primary |
| Schema v16 | `storage_roundtrip.rs` | v4→16 exactly once on open; v12 rows read `down_period=false`; standalone repo shape updated | asserts bumped to 16 (both sites) + new v12-row default assertion; the standalone `ProcessLedgerRepo` CREATE carries the column (supervisor unit tests build standalone ledgers); migration registered in `MIGRATIONS` (a first run failed with `Invalid column name: down_period` — the SQL existed but was not wired into the array; fixed and re-drilled) | PASS | 2026-10-13 | zcode-primary |
| Regression | fmt/clippy/tests/doc/evidence/terminology + UI build/test | all green, no premise weakened | clippy `-D warnings` 0 (recovery block extracted to `run_startup_recovery` for the 100-line budget); ALL workspace test binaries green chunked (core 4 suites, supervisor 7 incl. reconciler, adapter/adapters, app lib 58 + every integration binary incl. recovery, tiers 8, kill_sheet 5, desktop 9); doc `-D warnings` green; UI 190/190 + build green. Flake record (AJ-E806): `orphan_surfaces_only_after_grace_and_two_ticks` failed ONCE under full-suite load (real-process sub-second grace window racing compile churn), then passed 3/3 (alone + full suite twice) — no quarantine, recorded here | PASS | 2026-10-13 | zcode-primary |

## Implementation-mapping

- `crates/juggler-supervisor/src/supervisor.rs` — `recover()` (the
  pass), `register_adopted()` (ChildHandle rebuild: no ring readers,
  no PTY, `force_kill` = verified PGID ladder), the `adopted` marker
  map + rewritten `alive_adopted_sessions()`, `RecoveryReport`.
- `crates/juggler-supervisor/src/tree.rs` — `mark_orphaned_down` on
  the `LedgerStore` trait (+ `CoreLedgerStore` impl, epoch computed
  inside like every other trait method).
- `crates/juggler-core/src/storage/process_ledger.rs` — v16
  `down_period` column, `LedgerRow.down_period`, repo-level
  `mark_orphaned_down` (state/last_seen/down_period WHERE
  running+helper), standalone CREATE updated.
- `crates/juggler-app/src/state.rs` — `run_startup_recovery` (called
  BEFORE reconciliation): candidate sessions → full rows →
  `supervisor.recover` → summary persistence + `system:recovery`
  audit intent + 24 h tier suspension on any finding; reconciliation
  skips adopted sessions.
- `crates/juggler-app/src/commands.rs` — `RecoverySummaryInfo` +
  `recovery_summary_get/_arc/dismiss`; `InstanceInfo.adopted`;
  `CleanupRow.down_period` + the left-behind label override; the
  `re-adopted` filter + `Running — re-adopted` label.
- `ui/src/components/RecoverySummaryDialog.tsx` (new) — the one-time
  summary (exact text, modal, focus contract); `App.tsx` wiring;
  `RightInspector.tsx` adopted chip + Processes-tab detached notice;
  `ProcessRow.tsx` `down_period`; `bridge.ts` seams
  (`recoverySummaryGet/Dismiss`, `__AIJ_RECOVERY__`).
- Tests: `crates/juggler-app/tests/recovery.rs` (the crash drill),
  `ui/src/components/recovery-summary.test.tsx`, E510 suites in
  `processes-tab.test.tsx` + `cleanup-centre.test.tsx`,
  `ui/tests/visual/capture.spec.ts` §22 + `a11y.spec.ts` recovery
  gate.

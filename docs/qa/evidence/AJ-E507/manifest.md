# Evidence manifest — AJ-E507 (Verify-before-kill sheet and manual TERM→KILL ladder)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E507.
Branch: `stage2/e5-kill`. Local runs (macOS arm64, CLT-only machine)
2026-10-13, reviewer zcode-primary.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 three steps, no one-click kill, consequence-named button | `ui-tests.txt` (kill-sheet 9/9) + code | verify → scope → confirm; scope defaults narrowest; button names the outcome | `kill_verify` (re-reads NOW: exe/cmdline/cwd/ports + PGID collateral) → scope radios (`This process only` default; goal-port fixture asserts `Terminate vite (PID 8123) and free :5173`) → confirm; no `Don't ask again` anywhere; the ONLY kill entry is the verified row's `Terminate…` affordance | PASS | 2026-10-13 | zcode-primary |
| Req.2 failed verification: copy-exact, button REMOVED | `kill-sheet.test.tsx` failed-verification test + visual assertion | exact copy; no kill button in DOM; T5/not-managed never open | `kill-refused` renders `This PID now belongs to a different program — Juggler will not touch it.`; `queryByTestId(kill-confirm)` NULL (asserted); Unverifiable/exited rows render NO affordance (cleanup-centre test asserts absence); the execute gate RE-verifies same-tick and refuses with zero signals (`guardrail_stale_identity_refuses_with_zero_signals`: refusal copy, `refused` outcome, NO audit row, process untouched) | PASS | 2026-10-13 | zcode-primary |
| Req.3 visible cancellable grace; `Ignored SIGTERM`; Windows verbatim | ladder tests + KillSheet tests + captures | TERM → live countdown (aria-live 1 s) → KILL survivors only; cancellable; `TerminateProcess only` verbatim | `terminate_group`: TERM members → grace poll (cancel flag each 50 ms) → KILL only survivors → 2 s honest wait; vocabulary `exited on TERM`/`needed KILL`/`already gone`/`refused`/`TERM'd — outcome unknown` (cancel leaves the audit PENDING — E509's copy); sheet countdown at 1 s steps + per-process exit ticks (`kill_liveness` poll) + Cancel; `ladder_escalates_to_kill_for_term_ignorer` proves `needed KILL` with the trap-log WITNESS (`term\n` — TERM delivered and survived); `ladder_cancel_mid_grace_leaves_survivor_running` proves no escalation on cancel; Windows: `ladder_platform()` = `TerminateProcess only`, the sheet renders `Windows: TerminateProcess only — there is no grace window here.` (verbatim, tested + captured) | PASS | 2026-10-13 | zcode-primary |
| Req.4 `terminate_group` primitive; `stop` delegates (F-9); honest port re-probe (C2) | `stop-delegation-tests.txt` + `kill-tests.txt` | group death asserted; still-held never claimed freed; pid-only re-offer | `process_tree_descendants_are_tracked` now asserts EVERY tracked pid is gone after `stop()` (the spec's missing assertion); `Supervisor::stop` TERMs the root (exact reaped-flag path — an unreaped zombie answers `kill -0` as alive) and the group's OTHER members go through the ladder; zombie-aware probes (`member_gone`: signal-0 + sysinfo status — a killed-but-unreaped direct child reads GONE, never "refused"); fixture (3): the `setsid` escapee keeps the port after the root's pid-only kill — reported `Still holding :PORT by python (PID N)` + re-offer `Re-verify PID N`, and the re-offer (fresh verify → pid-only) frees it | PASS | 2026-10-13 | zcode-primary |
| Req.5 PGID collateral list + unsaved-state warning copy | KillSheet ready-state test + captures | every member listed; `Includes a {role} that may hold unsaved state.` exact | `kill_collateral` enumerates `pgid_members` with ledger attribution (strangers named `not Juggler-managed`); `guess_role` (pass2 §5 vocabulary, unit-tested); the McpServer member's warning asserted copy-exact | PASS | 2026-10-13 | zcode-primary |
| Req.6 per-process outcomes; `already gone` = success; audit written | `kill-tests.txt` + audit history asserts | one outcome per member; success styling for already-gone; AJ-E509 row carries them | fixture (1) vite-like helper: `exited on TERM`, port honestly re-probed FREE (`:port is free.`), audit row completed with `pid N: exited on TERM` + ports before/after; fixture (2): `needed KILL` completed likewise; report renders outcomes per process; `already gone` asserts the SUCCESS copy (never an error toast) | PASS | 2026-10-13 | zcode-primary |
| Quality gate: no signal without same-tick verify + audit intent | `guardrail_no_audit_means_no_signal` + `guardrail_stale_identity…` | zero signals on either failed gate | FailingAudit seam → `AppError::AuditUnavailable`, trap log EMPTY, process ALIVE (asserted); stale identity → refusal copy, zero signals, NO audit row (history empty, asserted); the verify rule is ONE shared fn (`verify_kill_target`) for sheet and gate — they cannot disagree (helpers: full boot/start/exe triple; live roots: the handle's settled identity start-time — E504's `/bin/sh`-vs-`/bin/bash` exec-aliasing rule, `verify_start_time`) | PASS | 2026-10-13 | zcode-primary |
| Fixtures 3/3 (SPEC §10 shape) | `kill_sheet.rs` (5/5) with `--nocapture` | vite-like freed; TERM-ignorer escalated with witness; setsid honest still-holding + re-offer | `[vite-fixture] port N freed; audit: …` (python http.server helper, port from the ephemeral allocator — the fixture is collision-proof); `[ignorer-fixture] witness log: "term\n"; members: [needed KILL]`; `[escapee-fixture] root gone, :N still held by pid M` → `re-offer done, :N free`; 0 signals to unverified PIDs (both guardrails green) | PASS | 2026-10-13 | zcode-primary |
| Visual: 8 states × 2 viewports × 2 themes | `pnpm --dir ui visual:capture` | verify (each scope), verification-failed (no kill button in DOM — asserted), grace countdown at 3 s, outcome report, freed, Windows wording; axe 0 serious/critical | `_gallery/20-kill/` 32 PNGs; suite 177/177; the failed-state capture HARD-FAILS if a kill button exists in the DOM; axe gate added for ready+refused in both themes (2 contrast fixes landed: scope hints `text-muted`, solid confirm button) | PASS | 2026-10-13 | zcode-primary |
| Regression | `just verify` + `pnpm --dir ui build` + `pnpm --dir ui test` | all green; premises updated, none weakened | verify green (fmt, clippy `-D warnings`, workspace tests); UI 174/174 + build green; two E506-era test premises honestly updated to E507 semantics (reconciler + cleanup dogfood: the root now dies by EXTERNAL SIGKILL — the crash shape — because `stop()` TERMs the whole group by design; `tests/supervisor.rs` group-death assertion ADDED) | PASS | 2026-10-13 | zcode-primary |

## Implementation-mapping

- `crates/juggler-supervisor/src/tree.rs` — `LadderScope`,
  `ProcessOutcome` (5 labels incl. `TermSent`), `LadderOutcome`,
  `ladder_platform()`, `terminate_group` (TERM → cancellable grace →
  KILL survivors → 2 s wait), zombie-aware `member_gone`,
  `pgid_members`, `live_details`, `verify_start_time`.
- `crates/juggler-supervisor/src/supervisor.rs` — `stop()` delegates:
  root keeps the exact reaped-flag path, the rest of the group runs
  the ladder; helpers excluded from the root's path (corpse rule).
- `crates/juggler-app/src/commands.rs` — `kill_verify` (+`_inner`),
  `kill_execute` (+`kill_execute_audited` seam), `kill_cancel`,
  `kill_liveness`; the shared `verify_kill_target` gate;
  `guess_role`, `kill_collateral`, `scope_members`,
  `still_holding_report`, `kill_intent_record`; DTOs
  `KillVerifyInfo`/`KillMemberInfo`/`KillExecuteInfo`/
  `StillHoldingInfo`/`KillMemberOutcome`.
- `crates/juggler-app/src/state.rs` — the live-ladder cancel-flag
  registry (`register/flip/unregister_kill_cancel`).
- `crates/juggler-app/src/lib.rs` — the three commands registered.
- `ui/src/components/KillSheet.tsx` (new) — the staged sheet;
  `ui/src/components/ProcessRow.tsx` — `killableRow` + the
  `Terminate…` affordance (verified-live rows only);
  `ui/src/screens/CleanupCentre.tsx` — sheet state, goal-port from
  the row's port, focus-return trigger, reload on report close;
  `ui/src/lib/bridge.ts` — `killVerify`/`killExecute`/`killCancel`/
  `killLiveness` + fixture seams.
- Tests: `crates/juggler-app/tests/kill_sheet.rs` (guardrails +
  3 fixtures), `tests/supervisor.rs` ladder vocabulary (4) +
  group-death, `tests/reconciler.rs`/`tests/cleanup.rs` premise
  updates, `ui/src/components/kill-sheet.test.tsx` (9),
  `cleanup-centre.test.tsx` affordance test, `capture.spec.ts`
  20-kill gallery, `a11y.spec.ts` kill-sheet axe gate.

## Notes for the next agent

- E508 tiers may ONLY kill through `kill_execute_audited` (verify →
  audit intent → `terminate_group`); never signal directly.
- The kill sheet's targets are LEDGER rows; a post-restart adopted
  root has no live handle — its verification currently fails closed
  (E510's crash-recovery replay owns widening that).
- `goal_port` is caller-stated (the port that opened the sheet): it
  scopes the honest still-holding re-probe and the audit's
  ports-before; unset ⇒ the target's own held ports.
- The `Ignored SIGTERM` pre-KILL label is the ladder's
  grace-expiry moment; the sheet renders the escalation line in the
  countdown (`Escalating to KILL — survivors ignored SIGTERM`).

# Evidence manifest — AJ-E511 (Fleet-shaped ownership: machine identity on every process record)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E511.
Branch: `stage2/e511-e601-batch` (batched with AJ-E601 per the
maintainer's batching directive). Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 machine_id on every record; ONE persisted identity | `grep '"this-mac"' crates/juggler-app/src/commands.rs` | zero product literals; identity = the persisted `local_machine_id` | the three real session-creation sites (launch terminal, launch desktop, handover continuation) migrated to the identity provider — hoisted ABOVE the storage lock (the first attempt deadlocked the launch paths: `local_machine_id` locks storage itself; the desktop suite hung and caught it). The one remaining literal is a TEST fixture. Ledger rows, audit rows, `CleanupRow`, `KillVerifyInfo`, and the inventory (`local_machine`) all carry the machine | PASS | 2026-10-13 | zcode-primary |
| Req.2 typed WrongMachine; zero signals; sweep skips foreign | `foreign_machine_row_is_refused_with_zero_signals_and_listed_display_only` (foreign-refusal.txt) | foreign candidate → typed refusal, zero signals, no audit row; sweep-invisible | `kill_verify_inner`: refusal `This process record belongs to another machine — Juggler will not touch it.`, `machine_id` echoed, NO member listing; `kill_execute_as` guardrail-0 (BEFORE identity — a foreign row must not misread as PID-reuse): refused + trap log EMPTY + alive + no audit row; the sweep (machine-scoped candidates) never signals it; the local tree still verifies | PASS | 2026-10-13 | zcode-primary |
| Req.2 typed error names both machines | `wrong_machine_is_typed_and_names_both_machines` | `SupervisorError::WrongMachine { recorded, local }` | typed variant added; Display contains both machine ids + the no-signal law | PASS | 2026-10-13 | zcode-primary |
| Req.3 composite key: same session, two machines, disjoint | `composite_key_two_machines_same_session_stay_disjoint` (fleet-identity.txt) | session-only reads would MERGE machines — the fleet-scale false-kill trap | `rows_for_session_on_machine` returns 2+2 rows, each set exclusively its machine's; `live_session_machines` lists BOTH composite keys, never merged | PASS | 2026-10-13 | zcode-primary |
| Req.3 candidates machine-scoped | `candidates_are_machine_scoped` | this node's sweep never sees foreign rows | all candidate rows carry machine-a when asked as machine-a | PASS | 2026-10-13 | zcode-primary |
| Req.3 Machine column + filter + display-only | `cleanup-centre.test.tsx` E511 suite (3) + `24-fleet` gallery + axe | `This Mac` label locally; labelled combobox default This machine; foreign row has NO kill affordance in the DOM | machine cell asserts `This Mac` vs `machine-other`; the combobox widens the bridge call to `all`; foreign row: `cleanup-kill-9001` absent while the local row's is present; 6 captures (1440×900 + 1024×700 × light+dark) + 2 axe gates green; the `__AIJ_CLEANUP__` seam mirrors the backend scope (the default filters foreign rows) | PASS | 2026-10-13 | zcode-primary |
| Req.3 inventory grouping by composite key + v17 index | `cleanup.rs` + storage_roundtrip (v17) | same-session-two-machines renders as TWO groups; indexed ≤5 ms | groups keyed `(session, machine)`; v17 `process_ledger(session_id, machine_id)` index; roundtrip asserts v4→17 once; the guardrails foreign test asserts the default scope lists ONLY this machine — which CAUGHT a real leak: `local_ledger_rows` read full session rows and a foreign row of a shared session leaked into the local scope; now composite-keyed | PASS | 2026-10-13 | zcode-primary |
| Req.4 ADR stub with the five open-question areas | `docs/adr/0013-fleet-architecture.md`… rechecked: `0013-fleet-at-launch.md` | identity, sync/transport, remote-ownership, offline, security as beta-exit criteria; NO transport built | the "Open questions (beta-exit criteria)" section added to the EXISTING ADR-0013 (the epic's file map predicted a new 0013 stub; the ADR already existed from the D-13 work — extended, not duplicated); zero `juggler-link` code touched | PASS | 2026-10-13 | zcode-primary |
| Quality gates: no NEW "this-mac" literals | grep | touched files introduce none | the migration REMOVES three; UI/bridge fixture literals are pre-existing test data, unchanged | PASS | 2026-10-13 | zcode-primary |
| Regression (affected batch) | fmt/clippy + affected suites | all green | fleet_identity 3/3; guardrails 9/9 (incl. the new foreign test); kill_sheet 5/5 (its fixture ctx migrated to the identity provider — a hardcoded test machine now rightly refuses); cleanup, recovery, desktop, golden_path, profiles_launch, onboarding, renewal green; supervisor suite 9/9 ok; storage_roundtrip 24/24; UI 196/196 + build; the desktop-suite HANG from the reentrant lock was the migration's one real defect — caught by the suite, fixed by hoisting | PASS | 2026-10-13 | zcode-primary |

## Implementation-mapping

- `crates/juggler-core`: v17 index; `rows_for_session_on_machine` +
  `live_session_machines` (the composite-key reads).
- `crates/juggler-supervisor`: `LedgerStore` trait methods;
  `SupervisorError::WrongMachine`.
- `crates/juggler-app`: identity at every session creation (hoisted
  above locks); `KILL_WRONG_MACHINE_COPY` + guardrail-0 in
  `kill_execute_as` + the typed early-refusal in `kill_verify_inner`;
  machine-gated `verify_kill_target`; composite-keyed
  `local_ledger_rows`; `all_ledger_rows` (the walk); inventory
  `(session, machine)` grouping + `local_machine` + `machine_scope`.
- UI: Machine cell, labelled machine filter, foreign rows
  display-only, kill sheet names the machine, seam mirrors the scope.
- Docs: ADR-0013 open questions.

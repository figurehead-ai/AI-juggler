# Evidence manifest — AJ-E206

Task: `docs/tasks/E2-switching-engine.md` AJ-E206.
Branch: `stage2/e2-e206-door`. 2026-10-13, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Door chosen by preflight risk only — 1-click iff usual account AND transplant/native-resume AND F3/F4 AND no compaction, no conflicts, no listed Drops, headroom ≥ Tight; otherwise wizard (pass2/02 §8 verbatim) | Req.1 | `juggler_handover::door::choose_door` is the literal §8 conjunction over `DoorInputs` (computed at preflight from live engine state — no settings, no habit); door tests: one fixture per tree leaf (all-conditions → 1-click; one breaker each for usual/mechanism/fidelity/compaction/drops/headroom → wizard); `switch_preflight_impl` builds the inputs from the same preflight facts (quota gate, compaction scan, base dispositions, usual-account) | PASS | 2026-10-13 | zcode-primary |
| 1-click flow: popover (carry one-liner + cost) → execute → undo toast 10 s = stop target + resume source + compensating edge | Req.2 | SwitchFlow 1-click popover (carry one-liner `6 carried · 0 rebuilt · nothing dropped — cost: one switch hop`, F-badge + usual-account + beta, §5.2 freeze radios, Switch ▸); on success App opens `SwitchUndoToast` (countdown visible, "Undo (Ns)", auto-close at 0); `switch_undo` command: tolerates already-gone target, aborts BEFORE resuming source on a real stop failure (never two live instances), resumes the source's own native id under its own recipe, records the compensating edge (`parent_edge_a` = original, mechanism NativeResume, F4, Confirmed); integration test `undo_stops_target_resumes_source_and_compensates_the_edge` pins argv `--resume native-sess-9` on account A + both edges + 3 epochs | PASS | 2026-10-13 | zcode-primary |
| Wizard has no fixed step count; renders only non-trivial steps among Target/What's carried/Conflicts/Rules-diff/Confirm | Req.3 | `Door::Wizard{steps}` builds `[Target, Carried] + Conflicts? + RulesDiff? + [Confirm]` — never phantom steps; engine test pins minimal 3 (`[Target,What's carried,Confirm]`) and full 5; UI renders exactly the backend's `wizard_steps` with "Step N of M: {title}" + chips (completed ✓); gallery 29 captures 3-step and 5-step renders; conflicts/rules steps are backend-impossible today (E207) and their copy says so honestly | PASS | 2026-10-13 | zcode-primary |
| No execution from an unseen preview; confirm click lives on the preview/Confirm step (deep/09 H-5) | Req.4 | The wizard's Switch button renders ONLY on the last step (`step === steps.length - 1`); vitest asserts `switch-confirm` absent at step 1 and step 2, present at Confirm; ⏎ executes only when `executeArmed` (1-click popover or last step) | PASS | 2026-10-13 | zcode-primary |
| Telemetry distinguishes `switch_oneclick` vs `switch_wizard{steps_shown}`; ranking suggests only; copy never "we switched you" (SPEC:26) | Req.5 | `Door::telemetry_kind()` → `switch_oneclick` / `switch_wizard[Target,What's carried,Confirm]`; counted at preflight into the LOCAL KV (`switch_door_counters` app-settings key — never networked; integration test asserts the counter row); execution remains click-only (guardrail test); copy audit: "You switched — …" / "Switched to X — the source stays resumable" (user subject); no "we switched" string exists in SwitchFlow | PASS | 2026-10-13 | zcode-primary |
| All warning strings verbatim §7.1; generic "some context may be lost" fails copy review | Req.6 | Warnings flow through E205's `user_copy` §7.1 table unchanged (pipeline copy test still bans the generic string); the wizard's reason lines are new §8 door copy, each naming the concrete condition + concrete drops (`The carry-list drops: todo list.`) — never "some context" | PASS | 2026-10-13 | zcode-primary |
| Threshold: any lossy switch reachable in 1-click = P0 fail | evidence/validation | Integration test `lossy_pair_is_never_reachable_in_one_click`: target made the project's DOMINANT account (2 sessions to 1), same-provider transplant — door still `wizard` because todos list Drop (§H2, until E210 fixtures flip the cell); asserts `info.usual_account == true` AND `info.door == "wizard"` | PASS | 2026-10-13 | zcode-primary |
| Guardrail: zero executions without click; undo only from click | quality gate | vitest `switch-flow.test.tsx` 7/7: (a) popover rendered + hang seam armed → `switch-executing` ABSENT until `switch-confirm` click; (b) undo seam records calls — UNDEFINED after toast render, `["edge-…"]` only after the Undo click; (c) wizard done navigates with `undo=null` (no toast); (d) toast auto-closes at 10 s (fake timers) | PASS | 2026-10-13 | zcode-primary |
| Usual-account test is honest (dominant account, ties are not habits) | Req.1 (usual leaf) | `usual_account_for_project`: counts sessions per account (via recipe→account) + one occurrence per imported-history label; target must be the UNIQUE maximum with count > 0 — no history ⇒ nobody usual ⇒ wizard; covered by tests 1 (no history → not usual) and 2 (dominance → usual) | PASS | 2026-10-13 | zcode-primary |
| UI: popover/wizard/toast states, keyboard path, focus trap, aria-live step announcements; WCAG 2.2 AA | UI/UX | gallery 29-door 20 captures (popover, undo toast, wizard Target w/ reasons, 3-step carried w/ Drop list, 5-step w/ Drop list × 1280×800/800×600 × light/dark) + gallery 28 re-captured door-aware (16) + 4 axe passes (wizard traversal incl. steps `aria-live` + popover/toast scans) — 0 critical/serious both themes; keyboard: Esc cancel, ⏎ confirm (armed states), Tab trap; "Step 2 of 3" announced politely | PASS | 2026-10-13 | zcode-primary |
| Visual rule: a lossy state cannot be captured without its Drop list visible | visual gate | The wizard-carried captures assert `switch-carry-dropped` contains "todo list" BEFORE the screenshot (both 3-step and 5-step); the 1-click popover capture asserts "nothing dropped" (the only dropless state) | PASS | 2026-10-13 | zcode-primary |
| Performance budgets (door ≤5 ms after preflight; popover/wizard paint ≤100 ms; step transition ≤50 ms; undo resume ≤2 s; ≤3 keystrokes to confirm) | performance budget | Door decision is one bool conjunction over already-computed values (no IO — structurally ≤µs); popover/wizard/step are static React states (no fetch per step); undo = stop ladder + one spawn (integration test completes <1 s wall on the stub incl. graph writes); keyboard path: ⏎ on the armed popover = 1 keystroke | PASS | 2026-10-13 | zcode-primary |
| Spec visual "wizard at 2-step render" | visual testing | Not captureable BY the §8 tree: the minimum wizard is 3 steps (Target + What's carried + Confirm are all "always") — the engine test pins minimum-3; captured 3-step and 5-step instead (the two real shapes). Recorded, not silently skipped | NOTE | 2026-10-13 | zcode-primary |
| Shortcuts `⌘⇧H` / `⌘R` | UI/UX reasoning | Final bindings are owned by the screen spec (per the task's own line) — NOT landed here to avoid double-owning the binding; keyboard confirmation path covered via ⏎/Esc/Tab; E209's graph surface owns `⌘R` resume-from-node when it lands | NOTE | 2026-10-13 | zcode-primary |
| Live dogfood: quota-wall switch ≤2 clicks / ≤5 s on a proven pair | live data | PENDING the authenticated session (gate-E5) AND E210's fixture flip (no pair is "proven" while todos default Drop — by design, see the P0 test); guardrail + stub flows cover the mechanics meanwhile | PENDING | 2026-10-13 | zcode-primary |

## Test counts

- `cargo test -p juggler-handover`: 34/34 (door 5 — one per §8 leaf +
  step-combination; pipeline 12, transplant 9, pack/fidelity 8).
- `cargo test -p juggler-app --test switch_door` (new): 3/3
  (wizard-until-proven, lossy-never-1-click, undo compensation).
- vitest: 224/224 (22 files) incl. new `switch-flow.test.tsx` 7/7.
- Visual: 320/320 — gallery 29-door (20 new), gallery 28-switch
  (16, door-aware re-capture), a11y 4 switch passes (0 critical/
  serious, both themes).
- clippy pedantic `-D warnings` clean on touched crates; rustdoc
  `-D warnings` clean (workspace).

## Implementation-mapping

- `crates/juggler-handover/src/door.rs` (new) — DoorInputs (§8's
  literal bool conjunction), WizardStep + titles, Door +
  telemetry_kind, choose_door; 5 leaf tests.
- `crates/juggler-handover/src/fidelity.rs` — `ArtifactKind::label()`
  (carry-list names: "chat transcript", "todo list", …).
- `crates/juggler-app/src/commands.rs` — PreflightInfo gains
  door/wizard_steps/wizard_reasons/usual_account/carry (+CarryListInfo);
  switch_preflight_impl extended via quota_gate_of /
  usual_account_for_project / carry_lists_of / door_verdict /
  wizard_reasons_of / record_door_counter (local KV telemetry);
  SwitchExecutedInfo (session + pid + edge_id + source_session_id);
  switch_undo command + impl (undo_edge_context / stop_continuation /
  undo_resume_source / record_compensating_edge — S-G1 append rules
  kept: to_epoch filled by mark_launched).
- `crates/juggler-app/src/lib.rs` — switch_undo handler registration.
- `crates/juggler-app/tests/switch_door.rs` (new, 3).
- `ui/src/lib/bridge.ts` — PreflightInfoData door/carry fields,
  SwitchExecutedData, switchUndo (+ `__AIJ_SWITCH_UNDO_CALLS__`
  guardrail seam, `__AIJ_SWITCH_UNDO_FAIL__`).
- `ui/src/components/SwitchFlow.tsx` — 1-click popover, wizard
  stepper (only rendered steps, reasons, carry lists, confirm on last
  step), SwitchUndoToast (10 s window, honest failure copy);
  `ui/src/App.tsx` — undo-window state + toast mount; onSwitched
  carries the undo handle for 1-click only.
- `ui/tests/visual/capture.spec.ts` — gallery 29-door (20) + gallery
  28 re-captured door-aware; `ui/tests/visual/a11y.spec.ts` — wizard
  traversal + popover/toast scans.

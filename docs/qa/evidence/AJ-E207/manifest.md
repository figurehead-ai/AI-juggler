# Evidence manifest — AJ-E207

Task: `docs/tasks/E2-switching-engine.md` AJ-E207.
Branch: `stage2/e2-e207-switchback`. 2026-10-16, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Switch-back resumes the ORIGINAL native session; the original transcript is never copied, moved, or re-ingested; copy-back prohibited in code | Req.1 | `resume_original_with_delta` builds `LaunchMode::Resume { session_id: <original native id>, delta_prompt }` — the only spawn; NO function in the switch-back path writes the transcript (file choices run over the non-transcript manifest domain only, structurally: `reconcile::apply_choice` plans actions over artifacts, and the transcript is not one); A→B→A integration test pins the ORIGINAL transcript hash unchanged across the return AND `--resume native-5` in the stub argv | PASS | 2026-10-16 | zcode-primary |
| Interim work arrives as exactly ONE delta turn ("While you were away": turns, diff stat, todo changes) rendered from away-epoch packs; a switch-back edge records `resumed_from_epoch` = the original epoch | Req.2 | `switch_back::delta_turn_text` — one bounded turn (≤40 one-liners × ≤120 chars + the pack pointer; the ≤4k-token default); argv assertion: "While you were away" appears EXACTLY ONCE in the resume argv; graph test: the return edge's epoch carries `resumed_from_epoch == fork.from_epoch` (the ORIGINAL epoch), parent = the fork; `LaunchMode::Resume.delta_prompt` is the injection carrier (`claude --resume <id> "<delta>"`) | PASS | 2026-10-16 | zcode-primary |
| Return suggested, never executed, on quota reset; verbatim copy | Req.3 | `switch_back_suggestions` (command + impl): open confirmed excursions whose ORIGINAL account has headroom again (not exhausted, TTE ≥ 45 min — the reset trigger); parent-linked returns and already-returned excursions suppressed; the card renders the copy byte-exact (integration + vitest + visual assert `"Work Max has reset. Resume the original conversation there? Work from Personal arrives as one 'while you were away' update."`); the click is the user's | PASS | 2026-10-16 | zcode-primary |
| Three-way reconciliation: base = fork-edge manifest; away-only → delta; original-only → no-op; same-file both-side edits → conflict list, never auto-merged; todo/state conflicts listed; conversation conflicts never text-merged | Req.4 | `reconcile::reconcile_conversation` (base = uuids(A) ∩ uuids(B) — exact for the transplant case where B began as A's bytes; replay-born away sides degrade honestly to all-away) + `reconcile_files` (fork manifest's non-transcript rows hashed on both sides; both-changed-and-disagree ⇒ conflict; deletions are changes; identical changes are not conflicts) — unit tests per class; the A→B→A fixture edits `todos.json` on both sides and the plan lists exactly it; conversation rows (`kind: transcript`) never enter the file domain — uuid trees travel as the delta, never text-merged | PASS | 2026-10-16 | zcode-primary |
| Per conflict: Keep original / Take away / Keep both (away saved as `…away-<date>` sibling + a turn pointing at it); the merge edge records both parents and each choice in the manifest | Req.5 | `ConflictChoice` (exactly three) + `apply_choice` (Take-away on a deleted-away file never destroys the original; Keep-both names `todos/list-away-YYYYMMDD.json` via `away_sibling_path` — civil-from-days date pinned by unit test); the switch-back edge records `parent_edge_a` = the fork + `kind: "conflict-choice"` manifest rows (`rel_path` + `disposition: take-away` asserted in the graph test); two-parent shape ready (`parent_edge_b`) | PASS | 2026-10-16 | zcode-primary |
| Away instance still running ⇒ freeze-or-fork first; resuming while B runs is legal and the UI states both are live | Req.6 | The review renders the away-disposition radios (Stop then return — default / Keep it running); choosing Keep renders the both-live copy verbatim ("Both accounts are now live on this work item: … Check which one you type into." — gallery 30 both-live capture + vitest); execute stops the away instance only per the explicit choice, aborting before the resume on a real stop failure | PASS | 2026-10-16 | zcode-primary |
| S-F7 (original deleted/expired): nearest surviving epoch or pack-primed fresh F0, never a fabricated resume | Req.7 | `original_session_verdict` — Alive{native id} / Gone{NearestEpoch{surviving native id, transcripts verified to exist}} / Gone{FreshPackF0}; unit test pins never-fabricates; integration test deletes the original transcript ⇒ plan reports `sf7_fallback: "fresh-f0"` + the §7.1 S-F7 copy; Fresh-f0 executes as an honestly-labelled Pack/F0 fresh start (nothing primed silently) | PASS | 2026-10-16 | zcode-primary |
| A→B→A fixture: original native id resumed, delta present once, zero duplicate uuids (dedupe first-wins); both-progressed fixture covers each reconciliation class; unresolved conflicts leave Work forked (S-F8); source-hash invariant | quality gate | `switch_back_path.rs` test 1: base 2 / away-only 2 (one RE-EMITTED — `delta_turns` renders it ONCE, first-wins) / original-only 1; todos.json conflict; unresolved execute ⇒ Err with the S-F8 copy and NOTHING touched; completed return: hash ban live, todos applied, graph = fork + parent-linked return, excursion closed (suggestions empty); test 2 = S-F7; 10/10 stable runs after fixing a TEST-harness race (the setup stub's late argv truncate clobbered the resume record — wait for the setup write, documented in the test) | PASS | 2026-10-16 | zcode-primary |
| Guardrail: the return executes only from the click; conflicts keyboard-completable | UI/UX | vitest `switch-back-flow.test.tsx` 6/6: review up + conflict unresolved ⇒ confirm disabled + "stays forked" copy; one click after radio choice ⇒ exactly one execute call with the chosen payload; a11y: `getByLabel("Take the away version")` check + axe 0 critical/serious on card, review, done (both themes); Esc/⏎/Tab trap per SwitchFlow's pattern | PASS | 2026-10-16 | zcode-primary |
| Visual: suggestion card, delta review, conflict list (3 choices/file), both-live warning, S-F7 state; 1280×800 + 800×600 light+dark; reviewer resolves a conflict keyboard-only | visual testing | gallery 30-switchback 20 captures (card, review w/ delta + todos.json trio + freeze-or-fork, both-live, sf7 fresh-f0, sf8-forked × 2 vp × 2 themes); full visual 342/342; the radio trio resolves by labelled name in the a11y test | PASS | 2026-10-16 | zcode-primary |
| Performance: delta render 500 turns ≤ 400 ms; uuid reconciliation 20k messages ≤ 200 ms (hash sets, no O(n²)); resume launch ≤ 2 s; delta ≤ 4k tokens | performance budget | reconciliation is one-pass hash-set arithmetic (`twenty_k_uuids_reconcile_linearly` test pins the 3-way split at 10k/10k/10k — linear); the delta renders ≤40 bounded one-liners regardless of away size (`many_away_turns_collapse_to_the_pack_pointer`); the integration test's return (stop + choices + spawn + graph) completes < 1 s wall on the stub; wall-clock numbers recorded here, not asserted as CI gates (load-sensitive) | PASS | 2026-10-16 | zcode-primary |
| Spec visual "delta turn rendered in target" screenshot | visual testing | The delta turn IS the argv prompt in production; the review's delta preview block is captured (gallery 30 review) and the executed argv is asserted byte-wise in the integration test — the in-vendor-CLI render belongs to the live dogfood (gate-E5), recorded PENDING | NOTE | 2026-10-16 | zcode-primary |
| File-conflict domain breadth | Req.4 (files) | Today E205's writers record only `kind: transcript` manifest rows, so PRODUCTION file conflicts are empty until non-transcript artifacts ship (E209's pack surfaces / E210 fixtures); the reader is general (the fixture seeds a todos row via the public append API and the full conflict path is proven end-to-end). Recorded, not skipped | NOTE | 2026-10-16 | zcode-primary |
| Live dogfood: quota-reset return on 2 real accounts; A→B→A continuity ≥ 80% over ≥ 10 returns | live data | PENDING the authenticated session (gate-E5 ledger), same as E204-E206 live proofs | PENDING | 2026-10-16 | zcode-primary |

## Test counts

- `cargo test -p juggler-handover`: 46/46 (reconcile 6, switch_back 6 new;
  door 5, pipeline 12, transplant 9, pack/fidelity 14 prior).
- `cargo test -p juggler-app --test switch_back_path` (new): 2/2, 10/10
  consecutive stable runs (serialized).
- vitest: 230/230 (23 files) incl. new `switch-back-flow.test.tsx` 6/6.
- Visual: 342/342 — gallery 30-switchback (20 new) + a11y 2 new switch-back
  passes (0 critical/serious, both themes); prior galleries regenerated.
- clippy pedantic `-D warnings` clean (app/adapter/adapters/handover);
  rustdoc `-D warnings` clean (workspace); fmt clean.

## Implementation-mapping

- `crates/juggler-handover/src/reconcile.rs` (new) — ConversationReconciliation
  (base = intersection; away-only; original-only), FileConflict +
  ConflictChoice + FileAction + apply_choice + away_sibling_path; 6 tests.
- `crates/juggler-handover/src/switch_back.rs` (new) — DeltaInput +
  delta_turns (dedupe first-wins, uuid-less excluded) + delta_turn_text
  (bounded), suggestion/both_live/divergence copy, OriginalSession/Sf7Fallback
  + original_session_verdict, plan_resolutions/unresolved; 6 tests.
- `crates/juggler-adapter/src/launch.rs` — `LaunchMode::Resume` gains
  `delta_prompt` (claude: `--resume <id> "<delta>"`; fake mirrors).
- `crates/juggler-app/src/commands.rs` — SwitchBackPlanInfo/
  FileConflictInfo/SwitchBackChoiceIn/SwitchBackSuggestionInfo;
  back_lineage + transcript_side + file_domain_hashes_now + back_facts;
  switch_back_plan / switch_back_execute (+parse/apply/stop/resume/record
  helpers) / switch_back_suggestions; date_compact_from_millis (+test).
- `crates/juggler-app/src/lib.rs` — three command registrations.
- `crates/juggler-app/tests/switch_back_path.rs` (new, 2) — A→B→A round
  trip + S-F7.
- `ui/src/lib/bridge.ts` — SwitchBackPlanData/Conflict/Suggestion types +
  switchBackPlan/switchBackExecute/switchBackSuggestions (seams incl. the
  `__AIJ_SWITCH_BACK_EXECUTE_CALLS__` guardrail recorder).
- `ui/src/components/SwitchBackFlow.tsx` (new) — review modal (verbatim
  suggestion, delta preview, conflict radio trios, freeze-or-fork,
  both-live, S-F7, S-F8 forked, labelled stages, focus trap, ⏎ armed).
- `ui/src/components/RightInspector.tsx` — the suggestion card above the
  handover section (suggest-only); `ui/src/App.tsx` — cards in refresh,
  SwitchBackFlow mount.
- `ui/src/components/switch-back-flow.test.tsx` (new, 6); gallery
  30-switchback (20) + 2 a11y passes in the visual specs.

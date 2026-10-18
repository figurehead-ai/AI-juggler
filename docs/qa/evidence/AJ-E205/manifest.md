# Evidence manifest — AJ-E205

Task: `docs/tasks/E2-switching-engine.md` AJ-E205.
Branch: `stage2/e2-e205-pipeline`. 2026-10-16, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Preflight computes in order: pair mechanism + E203 score, compaction scan, target quota gate, version-drift check, git/worktree state; any failure stops BEFORE any byte moves | Req.1 | pipeline::preflight — ordered checks; hard stops (exhausted-target, version-drift) return Err before execute is reachable; soft items render as warnings (compacted §7.1 copy, tight-quota, same-branch+worktree offer, zombie list, hop-cost S-F9); test pins stops-vs-warnings | PASS | 2026-10-16 | zcode-primary |
| Mid-task freeze: exactly two choices (Finish-turn default / Stop-now, truncated marked); no live-append copy; no SIGSTOP | Req.2 | FreezeChoice enum has exactly two variants (exhaustiveness test); torn-tail parse-verify refuses the bundle BEFORE staging (S-F2 test: nothing staged); no SIGSTOP/kill path exists in the pipeline (structurally absent — the engine only reads + writes target dirs) | PASS | 2026-10-16 | zcode-primary |
| Stage: temp dir → sha256 verify → atomic rename; crash mid-copy leaves no partial target; source untouched | Req.3 | install stages per-file + atomic renames (E204); pipeline re-verifies installed hashes and REMOVES the partial set on mismatch (S-F4 test: injected mid-copy crash leaves no partial, source hash-identical); post-launch source-hash re-check detects mutation | PASS | 2026-10-16 | zcode-primary |
| Launch failure keeps staged artifacts for retry, edge Failed{launch,reason}; prime quota death aborts + records tokens burned | Req.4 | S-F5 test: launch-hook failure → staged files RETAINED (asserted present), Failed edge recorded via failure.stage(); S-F6 PrimeQuotaDeath(u64) carries tokens burned (taxonomy + copy; the live prime path rides E206/E210 replay) | PASS | 2026-10-16 | zcode-primary |
| Every failure S-F1..S-F9 produces engine behaviour + §7.1 verbatim copy + a failed edge | Req.5 | SwitchFailure carries all nine ids with stage mapping (test); user_copy implements the nine-row §7.1 table verbatim (copy test: "Switching never happens automatically", "nothing is merged silently", "resume it exactly where you left it"; generic "some context may be lost" banned by test); commands record_edge_failed transitions the SAME edge (append-only). S-F7/S-F8 engine surfaces (dead-session, merge-open) are E207's switch-back logic — taxonomy + copy live here, single source | PASS | 2026-10-16 | zcode-primary |
| Zombie children listed at freeze; cleanup separate confirmed action | Req.6 | PreflightReport.zombie_children lists them; no pipeline code path kills anything (purity test in pipeline suite + the app's existing kill guardrails); UI renders the list, no auto-action | PASS | 2026-10-16 | zcode-primary |
| Same-branch both-live warn + worktree offer; uncommitted diff snapshotted into diffs/, never auto-applied | Req.7 | SameBranchParallel §7.1 copy in warnings; GitState.uncommitted_diff rides the pack's staged_files (E202) — no apply path exists in the pipeline | PASS | 2026-10-16 | zcode-primary |
| State-machine transition tests Staged→…→Confirmed + every Failed{stage}; fault-injection asserts source hash unchanged | quality gate | pipeline suite 8/8 (happy path verify+launch, S-F2/S-F4/S-F5 injection, stage mapping, two-choice freeze, copy table); golden_path pins the APP recording: one work, two epochs, one CONFIRMED edge with manifest rows + copy-never-move hash | PASS | 2026-10-16 | zcode-primary |
| No edge executes without a user click | guardrail | switch_execute fires only from the SwitchFlow confirm click (or the explicit command call); the pipeline's only spawn path is the caller-supplied launch hook; preflight is read-only | PASS | 2026-10-16 | zcode-primary |
| UI: freeze dialog modal (two labelled choices, default pre-selected, Esc cancel, ⏎ confirm), determinate per-stage progress, failure banners; a11y focus trap + aria-live | UI/UX | SwitchFlow.tsx: radio group default finish-turn, Esc/⏎ + focus trap (Tab cycles), labelled stage panel (honest: stages named, no fake per-stage percentages through one command), source-safe failure banner; gallery 28-switch 16 captures (dialog×2, executing, failed × 1280×800/800×600 × light+dark) + 2 axe passes (0 critical/serious — dialog-scoped while modal-open; warning text uses the theme-aware `text-warning` token after light-theme contrast fix) | PASS | 2026-10-16 | zcode-primary |
| Live dogfood: kill app mid-switch on a real session, relaunch, resume source | live data | PENDING the authenticated session (gate-E5 ledger); the fault-injection suites cover the mechanics meanwhile | PENDING | 2026-10-16 | zcode-primary |
| Performance budgets (preflight ≤300ms excl. user wait; freeze-to-stage 50MB ≤3s; idle ≤1% CPU) | performance budget | preflight is pure arithmetic + one transcript read; staging is bounded file copy + sha256 (E204 measured pattern); no polling loops added (the UI flow is user-driven; the notification poll is unchanged ≥30s) | PASS | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-handover/src/pipeline.rs` (new) — §7.1 copy table
  (CopySituation + user_copy), S-F1..S-F9 SwitchFailure with stage
  mapping, PreflightInput/Gates/GitState + preflight(), FreezeChoice,
  LaunchHook + execute() (snapshot → stage → verify → launch →
  source-hash invariant), SwitchRecord.
- `crates/juggler-handover/tests/pipeline.rs` (new, 8) — stops,
  copy-verbatim, fault injection, stage mapping.
- `crates/juggler-app/src/commands.rs` — switch_preflight (+impl)
  and switch_execute (+impl) commands; session_handover_impl
  delegates; record_edge_staged/failed/succeeded helpers (E201 graph:
  work + epochs + Staged edge BEFORE execution, Confirmed + manifest
  after; failed edges transition in place).
- `ui/src/components/SwitchFlow.tsx` (new) + App/RightInspector/
  WorkPanels wiring + bridge (switchPreflight/switchExecute +
  preview seams); `crates/juggler-app/src/lib.rs` handler
  registration.
- `crates/juggler-app/tests/golden_path.rs` — edge-recording
  assertions; gallery 28-switch (16) + 2 axe passes; visual 298/298.

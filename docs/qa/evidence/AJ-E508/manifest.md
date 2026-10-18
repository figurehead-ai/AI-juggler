# Evidence manifest — AJ-E508 (Cleanup tiers Tier 0-3: notify-only default, per-Account opt-in)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E508.
Branch: `stage2/e5-tiers`. Local runs (macOS arm64, CLT-only machine)
2026-10-13, reviewer zcode-primary.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.6 ADR-0010 + SPEC wording BEFORE tier code | docs review + `just evidence` | ADR present, linked from SPEC §5; tier wording per D-08 | ADR-0010's E508 amendment (operating tiers 0-3, hard exclusions, demotion, pause, crash suspension, storage shape incl. the prototype-flag→Tier-1 migration) + SPEC §5.2 rewritten to name Tier 2, Tier 3 (never automatic), the exclusion list, demotion, pause, suspension — same change as the code, ahead of merge | PASS | 2026-10-13 | zcode-primary |
| Req.1 Tier 0 default: never signals | `default_install_signals_nothing` (tier-tests.txt) | default install signals nothing across the fixture suite | clean-exit tree with a dev-server AND an MCP helper: both alive after the sweep; ZERO audit rows; the manual sheet remains the only path | PASS | 2026-10-13 | zcode-primary |
| Req.1 Tier 1 drill (pass2 §14.5) | `tier1_drill_kills_only_the_dev_server` | dev-server auto-TERMed after grace, MCP helper untouched, audit intent BEFORE signal | `[tier1-drill] dev N gone, mcp M alive; audit: actor auto:tier-1, outcome 'exited on TERM'`; the journal records `auto-tier1`; candidates come from the LEDGER's orphan verdict (the E410 tracker drops clean exits BY DESIGN and cannot serve Tier 1 — the sweep was re-based onto the ledger for exactly this) | PASS | 2026-10-13 | zcode-primary |
| Req.1 Tier 2: 5 min + ONE notification + ≥30 s Undo window | `tier2_crash_flow_notification_undo_and_lapse` | no signal before 5 min; exactly one notice; Undo wins; lapse signals with actor auto:tier-2 | before 5 min: nothing (no notice); at +5 min: exactly ONE notice (repeat sweep does not duplicate); Undo ⇒ never signalled + notice dismissed; a second crash orphan left to lapse ⇒ killed at +31 s with audit `auto:tier-2` (`[tier2-lapse]`) — the window constants: `TIER2_CRASH_GRACE_MS = 5 min`, `TIER2_UNDO_WINDOW_MS = 30 s` | PASS | 2026-10-13 | zcode-primary |
| Req.1 Tier 3: no automatic hog shed | code review + evaluator tests | alert + staged sheet only | no hog → signal path exists anywhere; the evaluator's eligible set is exactly `DevServer|Watcher` (`evaluator-tests.txt` 4/4: hard-exclusions matrix incl. Database/McpServer/TerminalShell at EVERY tier even with both opt-ins on) | PASS | 2026-10-13 | zcode-primary |
| Req.2 hard exclusions at every tier | `hard_exclusions_apply_at_every_tier` + role vocabulary | roots, McpServer, TerminalShell, DB roles, adopted, unverifiable, escaped-group, remote machine, unknown exit kind, non-opted-in | all eleven exclusions assert ObserveOnly EVEN with both tiers on; `guess_role` gained the `Database` role (postgres/mysqld/redis/mongod/memcached/sqlite3) and the `http.server` dev-server needle; the role gate reads the TRUE command line (helper ledger rows now record it — `ProcessRow.cmd` via the tick's existing refresh, zero extra System constructions) | PASS | 2026-10-13 | zcode-primary |
| Req.3 audit-first + same-tick re-verify + demotion | sweep code + `verification_failure_demotes_to_tier0_with_audit_note` | intent BEFORE signal; failed verify ⇒ Tier 0 with an audit note, zero signals | the sweep calls `kill_execute_as` (the E507 path: shared verify gate → `kill_intent_record` → ladder); a candidate dead by sweep time ⇒ audit row `demoted — re-verification failed, listed only (Tier 0)`, nothing else signalled | PASS | 2026-10-13 | zcode-primary |
| Req.4 pause kill-switch + exact toggle labels + separate consent | `pause_blocks_every_signal` + `tier_toggles_persist_independently` + UI tests | pause within one tick; persistent exact banner; two separate toggles, both off by default, never bundled | `cleanup_pause_set(true)` ⇒ zero signals + `Automatic cleanup is paused.` banner (`cleanup-paused-banner` asserted copy-exact); resume ⇒ the sweep proceeds; labels `Automatically tidy left-behind dev servers` + `Also tidy after a crash (with notification)` asserted verbatim; tier-2 consent NEVER implied by tier-1 (repo + UI tests); the E410 single-flag surface migrates to Tier 1 only (v15) and mirrors it for older readers | PASS | 2026-10-13 | zcode-primary |
| Req.5 24 h crash suspension, shown not silent | `crash_suspension_blocks_until_reconfirmed` + UI test | suspended ⇒ zero signals; re-confirm resumes; the banner names it | AppState::init stamps `cleanup_suspended_until` when adopted survivors are found at boot (the crash-recovery shape; E510 deepens it); the banner `suspended for 24 h after the app closed unexpectedly` + the Centre's `Re-confirm` button (asserted); while set, the sweep stands down entirely | PASS | 2026-10-13 | zcode-primary |
| Per-Account isolation (quality gate) | `account_isolation_only_cleans_the_opted_in_account` | opt-in on A never cleans B | two accounts, two equally-eligible clean-exit trees: A's dev-server cleaned, B's never touched (both alive-or-dead asserted per side) | PASS | 2026-10-13 | zcode-primary |
| UI: toggles, banners, Tier-2 notice + Undo | `tier-toggles.test.tsx` (2) + `cleanup-centre.test.tsx` status suite (4) | exact labels; honest copy; pause/resume; suspension; notice names the process, counts down (aria-live), Undo dismisses | all asserted (labels verbatim, `Automatic cleanup is paused.` verbatim, `survived a crash` + `N seconds until the automatic tidy` + Undo → dismissed); 180/180 UI incl. the updated aggregate-copy assertion | PASS | 2026-10-13 | zcode-primary |
| Visual | `pnpm --dir ui visual:capture` | toggles off/on, paused banner, suspended banner, Tier-2 at appearance + 5 s; light+dark; axe 0 serious/critical | `_gallery/21-tiers/` 12 captures; suite 193/193; two NEW axe gates (toggles + Tier-2 notice) — one contrast fix landed (the toggles' explanatory copy → `text-muted`; the spec's 4.5:1 toggle-labels requirement) | PASS | 2026-10-13 | zcode-primary |
| Regression | fmt/clippy/tests/doc/terminology/evidence + UI build/test | all green; premises updated, none weakened | clippy `-D warnings` 0; all app suites green (incl. desktop after the E507-era guard); the E410 prototype tier test retired INTO the new tiers suite (its semantics are E508's); doc gate green after link fixes; the cleanup dogfood fixture moved to an ephemeral port (a leaked :5173 listener from a failed run once cascaded every later run — never assume a fixed port) | PASS | 2026-10-13 | zcode-primary |

## Implementation-mapping

- `docs/adr/0010-…md` — the E508 amendment (tiers 0-3 operating rules,
  exclusions, demotion, pause, suspension, storage shape).
- `docs/SPEC.md` §5.2 — Tier 2/3 + exclusions + pause/suspension wording.
- `crates/juggler-core/src/storage/cleanup_settings.rs` (new) —
  `CleanupTierSettings` + repo (own WAL connection, mutex-guarded);
  v15 migration (table + `cleanup_opt_in`→Tier-1 seed).
- `crates/juggler-supervisor/src/tree.rs` — `TierCandidate` /
  `TierVerdict` / `evaluate_tier` (pure; 4 unit tests incl. the
  eleven-exclusion matrix).
- `crates/juggler-supervisor/src/supervisor.rs` — `ProcessRow.cmd`
  carried through the tick's existing refresh (role gates read the
  TRUE command line; zero new System constructions).
- `crates/juggler-core/src/storage/process_ledger.rs` — helper rows
  record the real cmdline (hash + trunc) with an exec-settle window
  (≤10 s) for image refinement; after it, identity freezes and
  mismatches flip unverifiable (PID-reuse guard intact).
- `crates/juggler-app/src/commands.rs` — `auto_cleanup_sweep`
  (ledger-candidate based; pause/suspension gates; evaluator; Tier-2
  pacing + notice registry; demote-with-note; kills ONLY via
  `kill_execute_as` with actors `auto:tier-1|2`); commands
  `cleanup_status/_pause_set/_tier2_decide/_resume_after_crash/
  _tiers_get/_tiers_set` (+ `_arc` cores).
- `crates/juggler-app/src/state.rs` — `cleanup_settings` repo,
  `tier2_pending` registry, the boot-time 24 h suspension stamp.
- UI — `TierToggles` (RightInspector account panel, exact labels);
  CleanupCentre pause button + paused/suspended banners + Tier2Notices
  (countdown, Undo/Keep); bridge `cleanupStatus/PauseSet/Tier2Decide/
  ResumeAfterCrash/TiersGet/TiersSet` + fixture seams.
- Tests — `crates/juggler-app/tests/tiers.rs` (8: guardrail, drill,
  pause, isolation, tier2 flow, demote, suspension, toggles);
  `tier-toggles.test.tsx` (2) + centre status suite (4); `21-tiers`
  gallery (12) + 2 axe gates.

## Notes for the next agent

- E510 (crash recovery) owns deepening the boot-time suspension
  trigger; today it stamps when adopted survivors exist at boot.
- The E410 tracker (`state.orphans`) still serves the Processes tab's
  manual stop allow-list; the AUTOMATIC tiers read the ledger only.
- E512 fixtures: build on `drill_tree` in tiers.rs (dev-server + MCP
  helper + clean/crash root in one tree).
- Widening the auto-eligible role set (`DevServer|Watcher`) requires a
  NEW decision-log entry — role is a guess (C4), that is why it is
  narrow.

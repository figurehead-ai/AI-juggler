# Evidence manifest — AJ-E512 (False-kill collateral fixtures, hog alerts, and gate evidence)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E512.
Branch: `stage2/e5-collateral`. Local runs (macOS arm64, CLT-only
machine, loadavg ≈ 9 — the honest loaded case) 2026-10-13, reviewer
zcode-primary.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 fixture harness: 7 scriptable kinds, each traps + logs every signal | `tests/fixtures/` (trap_helper.py + mod.rs) | vite-like port-holder, TERM-ignorer, setsid-escapee, MCP-role holder, shared-PGID shell sibling, unmanaged user server, PID-reuse stranger; per-fixture trap files; ready-file handshakes | one python helper with modes (`--port`/`--setsid`/`--ignore-term`/argv role words) + a sh sibling; `spawn_detached` wraps spawns in an exiting `sh` (the clippy zombie-process law); fixtures self-clean after 5 min (a failing test's leaks die, not linger) | PASS | 2026-10-13 | zcode-primary |
| Req.2 ladder geometry (supervisor level) | `cargo test -p juggler-supervisor --test collateral` | stop() TERMs ONLY the own group; KILL only after grace; escapee/neighbour/unmanaged ZERO signals; port freed | `stop_signals_only_the_own_group_and_kills_survivors_after_grace`: cooperative fixture dies on TERM (trap log exactly `TERM`); TERM-ignorer logs TERM, survives grace, dies to KILL; escapee + session-B tree + unmanaged server trap logs EMPTY + alive; `port_free(port)` true (collateral-supervisor.txt) | PASS | 2026-10-13 | zcode-primary |
| Req.2 kill-path matrix (app level) | `cargo test -p juggler-app --test guardrails` | Tier 1, Tier 2, manual pid/pgid, crash recovery × all fixtures; zero-signal assertions on trap logs; intended orphans TERM'd (KILL only after grace); any deviation fails CI P0 | 8/8 (guardrails-app.txt): Tier-1 sweep TERMs ONLY the verified DevServer orphan (port freed); Tier-2 crash flow same after notice+lapse; manual pid scope signals ONE process and REFUSES the stranger (exact copy, zero signals, NO audit row); manual pgid scope NAMES shell+MCP members (`Includes a {role} that may hold unsaved state.`) then TERMs only the confirmed group — escapee/neighbour/unmanaged/stranger untouched; recovery signals NOTHING, adopted stop scoped; TERM-ignorer as target: TERM → grace → KILL (`needed KILL`) | PASS | 2026-10-13 | zcode-primary |
| Req.2 the E507-law exception, documented | guardrails `manual_pgid_scope…` | manual pgid/tree scope is the ONE path that TERMs group siblings — by AJ-E507 Req.3/5 law (warned, user-confirmed); everything OUTSIDE the confirmed group zero-signalled | asserted in-suite: the sheet lists every member with the exact unsaved-state warning BEFORE the kill; collateral outside the group untouched. The suite's split: collateral.rs (supervisor) = signal geometry; guardrails.rs (app) = role/identity gates + tier/recovery paths | PASS | 2026-10-13 | zcode-primary |
| Req.3 hog alerts: exact copy, passive, staged sheet only | `cleanup_alerts` + `hog_is_never_auto_signalled_and_alerts_with_exact_copy` | `{helper} under {Instance} holds {RSS} — review in Cleanup Centre.`; hog NEVER auto-signalled at any tier; manual sheet still works | alert text `Python under acc-guard holds 11 MiB — review in Cleanup Centre.` (exact template; helper = platform process name, RSS GiB/MiB); the sweep STANDS DOWN for hog pids AND hog-flagged trees (sweep uses the REAL clock for hog facts — the fake sweep clock emptied the 60 s window, caught by the test); manual pid-scope kill still succeeds, port freed | PASS | 2026-10-13 | zcode-primary |
| Req.3 idle nudges: once per orphan, 1 h + CPU≈0 + age only | `idle_nudge_fires_exactly_once_per_orphan` | fires at most once (persisted flag); CPU≈0 + age gates only | nudge `Python has been idle for 0m — review it in the Cleanup Centre.` (threshold settings-shrunk to 1 s for the test; production default 3_600_000); second read surfaces NOTHING for that orphan; the once-flag is persisted (`idle_nudge:{session}:{pid}`) | PASS | 2026-10-13 | zcode-primary |
| Req.3 UI: passive badges + ONE staged pre-fill + WCAG | `cleanup-centre.test.tsx` (3 new) + `23-alerts` gallery + axe | `role=status` (never a dialog); exact text; stage button opens the SAME verified sheet; idle nudge has NO action; keyboard pass | alert region `aria-live=polite` `role=status` labelled; `Stage Terminate…` opens the E507 sheet for the alert's pid (row must be in the inventory — same verified path as every row); idle card has no button; recorded keyboard-only pass (Tab → stage → Enter → sheet → Esc → focus RETURNS to the stage button); KillSheet now focuses its root from mount (Esc works during the verify phase — a gap the pass found); axe 0 serious/critical both themes | PASS | 2026-10-13 | zcode-primary |
| Req.4 gate report | `docs/qa/gate-E5.md` | E501-E511 links + collateral result + CPU <1% at 50 Instances + live-dogfood record | gate written: task table (E511 TODO marked), collateral results, soak `self cpu 0.03%` at 50 Instances (avg tick 15 ms, 0 System constructions, ON A LOADED BOX), dogfood record with the two honest open items (authenticated-helper dogfood = the gate-M1 standing re-login step; E511 pending) | PASS | 2026-10-13 | zcode-primary |
| Live dogfood (real claude mechanics) | `cargo test -p juggler-app --test dogfood_e5 -- --ignored --nocapture` | real Claude Instance → real tree → verified cleanup | real claude 2.1.300 under the real supervisor/PTY with a throwaway CLAUDE_CONFIG_DIR: UI streamed (login screen — gate-M1's auth boundary), root in the durable ledger with attribution, `kill_verify` verified, ladder `exited on TERM` (code 143) `all_succeeded`, audit row present, every tree row `exited`; `~/.claude` and t3 profiles untouched (dogfood-claude.txt) | PASS | 2026-10-13 | zcode-primary |
| Performance budgets | suite runtimes + soak | collateral ≤5 min CI; hog eval on the existing tick | guardrails 8/8 in ~60 s (serialized on purpose — see flake note); collateral 4 s; hog/idle ride `hog_facts` on the existing tick (settings-backed thresholds, zero new polling); Cleanup Centre paint: the alerts region adds ONE bridge read | PASS | 2026-10-13 | zcode-primary |
| CI as a required job | `.github/workflows/ci.yml` | macOS + Linux | Linux: `verify` job's `cargo test --workspace`; macOS: NEW `Collateral suite (AJ-E512)` step in `build-macos` (both suites) | PASS | 2026-10-13 | zcode-primary |
| Regression | fmt/clippy/doc/tests + UI | all green, no premise weakened | clippy `-D warnings` 0 (fixtures use the wrapper-spawn pattern for the zombie-process lint); doc `-D warnings` green; ALL workspace binaries green chunked (supervisor 8 suites + every app binary incl. guardrails 8/8, kill_sheet, tiers, recovery); UI 193/193 + build; playwright 216/216 (13 new: 12 captures + keyboard pass) + 2 new axe gates | PASS | 2026-10-13 | zcode-primary |
| PRODUCT HARDENING the fixtures forced (first CI run) | reconciler two-tick mismatch discipline | one transient sysinfo read must not permanently flip a healthy row | `mismatch_agree` counters in the reconciler: a row flips `unverifiable` only after TWO CONSECUTIVE mismatching ticks (the same discipline as exit-by-absence and orphan confirmation, AJ-E504); a clean tick clears it. KILL-time verify (E503) is UNCHANGED — single-shot fail-closed. Root cause: CI's 2-core runners starve sysinfo refreshes mid-read; a single-read flip had also been the behind-the-scenes cause of the load flakes seen locally. All flip-dependent suites re-run green | PASS | 2026-10-13 | zcode-primary |
| Cross-platform fixture honesty (CI hardening) | guardrails on ubuntu + macOS runners | both platforms pass | the shell sibling runs `bash` explicitly (Ubuntu's `/bin/sh`→dash SYMLINK flaps the exe fact across sysinfo reads → helper full-triple verify fails); root lifetime 25 s (slow runners), tick deadlines 30/35 s, hog poll 30 s with an RSS diagnostic; macOS CI runs the app test binary with `DYLD_FALLBACK_LIBRARY_PATH` into Xcode's swift dir (the dyld `libswift_Concurrency.dylib` load — the local CLT script's CI analogue); the hog alert poll matches the session's TOP helper (the alert names the top-RSS helper, not a specific fixture); the vite/unmanaged fixtures bind OS-assigned ports published through the ready file (kills the probe-then-bind ephemeral race — the macOS runner crash); the ladder grace is settings-backed (`kill_grace_secs`, production default 5 s untouched) and the suites widen it to 12 s on loaded machines — the SIGNAL always arrived, only the KILL deadline moved | PASS | 2026-10-13 | zcode-primary |

## Implementation-mapping

- `crates/juggler-supervisor/tests/fixtures/` (new) — `trap_helper.py`
  (the scriptable signal-logging helper) + `mod.rs` (FixtureSet,
  spawn_detached, trap-log assertions) — shared by both crates via
  `#[path]`. The shell sibling is `bash` (not `sh`: Ubuntu's dash
  symlink flaps the exe fact).
- `crates/juggler-supervisor/src/supervisor.rs` — the reconciler's
  two-tick mismatch discipline (`mismatch_agree`): one transient
  sysinfo read no longer permanently flips a healthy row to
  `unverifiable`; kill-time verify unchanged.
- `crates/juggler-supervisor/tests/collateral.rs` (new) — the ladder's
  signal geometry.
- `crates/juggler-app/tests/guardrails.rs` (new) — the kill-path ×
  fixture matrix (serialized: real-process suites flake under
  parallel sysinfo contention).
- `crates/juggler-app/tests/dogfood_e5.rs` (new, `#[ignore]`
  operational) — the real-claude gate dogfood.
- `crates/juggler-app/src/commands.rs` — `cleanup_alerts` +
  `CleanupAlert(Info)`/`rss_text`; the sweep's hog stand-down (real
  clock); settings-backed hog/idle thresholds
  (`hog_helper_rss_limit_mb`, `hog_cpu_pct`, `idle_nudge_min_age_ms`).
- `crates/juggler-app/src/lib.rs` — command registration.
- `ui/src/lib/bridge.ts` — `cleanupAlerts` + the
  `__AIJ_CLEANUP_ALERTS__` seam; `ui/src/screens/CleanupCentre.tsx`
  — the alerts region (`role=status`) + staged-sheet pre-fill;
  `ui/src/components/KillSheet.tsx` — focus-on-mount (Esc during
  verify).
- `.github/workflows/ci.yml` — the macOS collateral step.
- `docs/qa/gate-E5.md` (new) — the epic gate report.

## Flake record (AJ-E806) — exec-settle race, 2026-10-16, PR #83

**Observed.** `build-macos` → guardrails: `idle_nudge_fires_exactly_once_per_orphan`
FAILED ("orphans never surfaced"); runner log: `pid 9036 failed
re-verification — flipped unverifiable`. Docs-only change; suite green
locally and in PRs #80–#82.

**Diagnosis.** A full-scan discovery caught a fixture child between
`fork` and `exec` and recorded the WRAPPER's exe (`/bin/sh`). The
upsert's exec-settle refinement (AJ-E508, 10 s window) corrects the
recorded exe — but it is throttled to 2 s while the reconciler counts
mismatch ticks EVERY pass. Two consecutive mismatches landed before the
first refinement; the flip set `state = unverifiable`, and the
refinement's `WHERE state = running` never matched again — the row was
stuck, never orphaned, `wait_orphans` timed out. Slow CI runners widen
the fork→exec window enough for discovery to land inside it.

**Fix (recorded, not retried).** One shared constant —
`juggler_core::storage::EXEC_SETTLE_MS` (10 s) — now also gates the
reconciler's exe leg: inside the window a wrong exe withholds its
verdict (the upsert owns the refinement); after it the identity is
frozen and a mismatch flips as before. The START-TIME leg (the true
PID-reuse signal — a start time never legitimately changes) stays
immediate at every age, so the stranger/foreign/forged-row defenses are
unchanged. The flip log now names the fired leg + row age. Regression
test `exe_mismatch_holds_inside_settle_window_and_flips_after_it`
pins both directions: young wrong-exe rows hold, aged ones flip, and
the window closes.

**Verification.** Affected suites re-run green (storage_roundtrip 24,
supervisor all, recovery 1, guardrails 10 — including the CI-flaking
test and the new regression); full `just verify` + `pnpm --dir ui
build` on the fix tree.

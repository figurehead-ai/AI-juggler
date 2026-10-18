# Evidence manifest — AJ-E504 (Reconciler loop, orphan detection, and children-map eviction)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E504.
Branch: `stage2/e5-reconciler`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Rust suites run in CI (verify job); the
fixtures run everywhere (no GUI, no grants).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 ONE shared `System`; `descendants()` on the cached ppid map | construction regression in `fifty_instance_soak_and_construction_regression` → `reconciler-test.txt` | ≤1 `System` construction per tick; zero per-query full refreshes | `LiveStatsCollector::tick` builds ONE `ProcessTable` (parents + identity facts) from its own persistent table and publishes it to `Supervisor`; `descendants()` walks the cached snapshot (cold fallback before the first tick is a single COUNTED scan); every `System::new()` in the crate routes through the counter (`juggler_supervisor::system_construction_count()`); soak: **0 constructions over 20 ticks + 1,000 `descendants()` calls** (gate ≤1/tick); journal adoption's one-shot scan is start-of-life, not per-tick (counted, documented) | PASS | 2026-10-13 | zcode-primary |
| Req.2 tick: ppid map once; helper upserts; RSS/CPU; re-verify Running\|Orphaned\|Adopted | `reconciler-test.txt` + code review (`Supervisor` reconcile) | verify failure → `Unverifiable`, permanently un-killable | the tick: ONE table pass → `rebuild_trees` (rows/stats/ring incl. the new kernel `start_time`/`exe` row facts) → batched helper upsert (now records the REAL start time + REAL exe — never wall-clock or name) → `reconcile`: candidates (`running`\|`orphaned`) per machine re-verified against the tick's OWN table (boot gate first, FMEA C8; start-time for all rows; exe KERNEL-vs-kernel for helper rows only — root rows record the launch program which the kernel may alias, so roots compare against the ChildHandle's kernel-recorded identity; legacy name-exe rows are healed once by the upsert then verified); mismatch → `mark_unverifiable` + log; helper exits patched by TWO-tick absence (`unknown`, not reaped); ports refresh is AJ-E505's arm on this same tick (its spec) | PASS | 2026-10-13 | zcode-primary |
| Req.3 orphans only after grace + two agreeing ticks; in-grace exits never surface | `orphan_surfaces_only_after_grace_and_two_ticks`, `helper_exiting_inside_grace_never_surfaces` → `reconciler-test.txt`, `orphan-row-dump.txt` | invisible at grace−1 s; `Orphaned` at grace+2 ticks with Provider/Account/Project; in-grace exit → normal exit row | grace drill (per-Provider grace 3 s for the test; DEFAULT stays 30 s — never lowered): helper `running` at grace−1 s, `orphaned` after grace + 2 consecutive agreeing ticks (in-memory flap counters reset on any non-candidate tick), attribution columns exact (`claude-code`/`acc-reconciler`/`proj-reconciler`/`machine-reconciler`), kernel exe `/bin/sleep` on the row, `HelperOrphaned` event drained; early-exit drill: helper dying ~0.6 s after root → `exited` (`unknown`), NEVER `orphaned`; `detached_from` = ppid chain walk to a live owned root (init/launchd = detached, 64-hop cycle guard) | PASS | 2026-10-13 | zcode-primary |
| Req.3 grace tunable | code review + `set_orphan_grace` | default 30 s; per-Provider; stored in settings | `DEFAULT_ORPHAN_GRACE_SECS = 30`; `Supervisor::set_orphan_grace(provider, secs)` (+ `""` default-override key); `AppState::init` reads `app_settings.orphan_grace_secs` and pushes it; per-Provider override UI lands with the Cleanup Centre settings (AJ-E506) | PASS | 2026-10-13 | zcode-primary |
| Req.4 evict `ChildHandle`s after the retention window | eviction arm in the soak → `reconciler-test.txt` | stopped Instances leave no phantom Running rows; rings stop growing | `emit_exit` stamps `exit_at_ms` (observed exit, not stop call); each tick `sweep_finished` drops handles where finished ∧ now−exit > `OUTPUT_RETENTION_AFTER_EXIT_MS` (120 s — the UI's last-output read window); soak: 25 stopped sessions → **50/50 handle lookups evicted**, all 25 live untouched; ledger rows keep the durable history | PASS | 2026-10-13 | zcode-primary |
| Req.5 lifecycle deltas as events; UI never polls | `orphan_surfaces…` (event arm) + app wiring | deltas emitted; no `ps`/`lsof` in UI | new `LifecycleEvent::HelperOrphaned { session, pid }` (asserted drained); the app forwarder maps it to no session-record change (the ledger row is the surface; Cleanup Centre consumes in AJ-E506); role-guess vocabulary (`Harness\|…\|Unknown`) lands with E505/E506 where its evidence (ports/cmdline) exists — recorded here as the mapping | PASS (roles deferred to E505/E506 by design) | 2026-10-13 | zcode-primary |
| Req.6 fork storms do not flood | two-tick + grace construction | single-tick sightings hidden | a helper must survive grace AND two agreeing ticks before any orphan surface exists; helper rows for single-tick sightings are upserted but never orphan-flagged (E506 renders trees collapsed — its spec); exit-by-absence also needs two ticks (rustc flicker never patches) | PASS | 2026-10-13 | zcode-primary |
| Performance budget | soak log | CPU <1% of one core; tick ≤100 ms p95 at 50 Instances; zero per-query `System::new()` | `reconciler-test.txt`: 50 Instances × (root+helper), 20 ticks — **avg 12 ms, max 25 ms** (budget 100 ms), self CPU 0.34% (<1%), 0 constructions; absence/orphan counters are in-memory HashMaps (≤8 MB far beyond 50×2 rows) | PASS | 2026-10-13 | zcode-primary |
| Regression | full suites | prior gates intact | `just verify` green (fmt, clippy `-D warnings`, docs, evidence, terminology); supervisor suites: lib 23, identity 2, ledger 2 (E501/E502 intact), orphan/app 5, reconciler 3, supervisor 9; core roundtrip 24 incl. the new orphan arm (orphaned stays candidate; dead orphan resolves to exited; unverifiable never returns) | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- E505 (ports) next: JOIN THIS TICK — `lifecycle` says "AJ-E505 ports
  join this same tick, not a new loop". The reconcile pass is the
  hook; `listens_on` is the column. Role guesses belong with it.
- The orphan row's kill path (E507) must re-verify in the same tick
  as the signal and honor `escaped_group` (E502) + `unverifiable`
  (E503) — the three permanent no-go states.
- Helper exe/start facts are first-sight facts: the upsert UPDATE
  heals legacy name-exe rows once (non-absolute → absolute), then
  verification compares first-sight vs live. An exec AFTER first
  sighting fails closed (documented trade-off, §6.2's letter).
- The in-memory two-tick counters reset on collector recreation
  (restart) — grace re-confirmation after a crash is honest, not a
  bug.
- `snapshot_descendants` (the E410 journal allow-list) still runs at
  exit-time via the cached table; the SNAPSHOT tracker (orphans.rs)
  feeds today's Cleanup stub until AJ-E506 reads the ledger instead.

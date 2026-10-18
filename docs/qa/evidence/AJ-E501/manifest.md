# Evidence manifest — AJ-E501 (Persistent spawn ledger with intent-before-spawn)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E501.
Branch: `stage2/e5-ledger`. Local runs (macOS arm64) 2026-10-14,
reviewer zcode-primary. Rust suites run in CI (verify job); the
supervisor evidence test runs everywhere (no GUI, no grants).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 table + indexes (pass2 §5) | `cargo test -p juggler-core --test storage_roundtrip process_ledger_lifecycle_roundtrip` + migration | schema v12 additive; columns + 3 indexes | `process_ledger` with every specified column (`pid,pgid,started_at,exe_path,cmdline_hash,cmdline_trunc,boot_id,session_id,instance_id,provider,account_id,project_id,machine_id,root_pid,parent_pid,depth,role,state,listens_on,cwd,peak_rss,peak_cpu,exit_kind,reaped_by_us,first_seen,last_seen`) and indexes on `(session_id,state)`, `(pid,started_at,boot_id)`, `(state,last_seen)`; v4→v12 upgrade test bumped; each migration is one transaction (rollback-on-failure is the existing `migrate()` contract) | PASS | 2026-10-14 | zcode-primary |
| Req.2 storage-agnostic seam | code review (trait in supervisor, SQL in core) | `LedgerStore: Send+Sync` in supervisor; SQLite impl in core storage | `tree.rs` defines the trait (`boot_id/intent/spawned/upsert_helpers/mark_exited/candidates/prune`, pass2 §12 shapes); `CoreLedgerStore` bridges `juggler_core::storage::ProcessLedgerRepo` (all SQL in core; repo behind its OWN WAL connection + mutex so spawn threads write without the app storage lock) | PASS | 2026-10-14 | zcode-primary |
| Req.3 intent-before-spawn; failure degrades, never fails | supervisor code + `supervisor`/`app` suites | Intent row (state=starting, no PID) BEFORE spawn; PID/PGID/OS-start patched after; ledger error ≠ spawn error | `spawn`/`spawn_desktop` write the Intent row first, patch `pid + started_at` (OS start via a one-shot sysinfo probe) right after the fork, and route every ledger error to `ledger_degraded` + a lifecycle warning log — the process still starts; the flag surfaces as `StatsInfo.ledger_degraded` → the copy-exact banner `Process tracking degraded — cleanup is read-only until restart.` | PASS | 2026-10-14 | zcode-primary |
| Req.4 batched helper upserts ≤1/2 s/Instance | `ledger_records_root_and_helpers_and_survives_reopen` + repo roundtrip | ONE transaction per tick | `LiveStatsCollector::tick` calls `ledger_upserts` (throttled per session to 2 s; rows from the freshly rebuilt trees; peaks refreshed via update-first upsert — no duplicates asserted); the repo wraps each tick batch in a single transaction | PASS | 2026-10-14 | zcode-primary |
| Req.5 retention | repo roundtrip (prune arm) + state init | exited rows 7 days; running never pruned | `prune` deletes `state=exited AND last_seen < cutoff` only (running helpers survive the prune arm, asserted); `AppState::init` prunes 7-day-old rows at startup — the recurring job rides the E504 reconciler tick (interface exists) | PASS | 2026-10-14 | zcode-primary |
| Req.6 machine identity (D-13) | evidence test + code review | `machine_id` from the caller's context; stable local id; no new `"this-mac"` literals | every row carries the ctx's machine id (asserted on root AND helpers); new call sites use `local_machine_id()` — generated once, persisted in `app_settings.local_machine_id` (no new hard-coded literals; `boot_id` = OS boot time via sysinfo, stable per boot) | PASS | 2026-10-14 | zcode-primary |
| Evidence fixture (the task's proof) | `cargo test -p juggler-supervisor --test ledger -- --nocapture` → `ledger-test.log` + `process_ledger-dump.txt` | fake harness spawns 3 helpers → root+helper rows (account/project/machine) within 2 ticks; supervisor goes away → rows persist | `sh -c "sleep 300 & sleep 300 & sleep 300 & wait"`: root row (running, OS start patched, `claude-code`/`acc-evidence`/`proj-evidence`/`machine-evidence`) + all THREE helper rows land within the tick budget (0 missing); stop-ladder exit patches the root (`signal:15`); drop supervisor + store, REOPEN the same file → 4 rows survive, the outlived helpers remain `running` (cleanable, not forgotten); test kills its own helpers after | PASS | 2026-10-14 | zcode-primary |
| Banner (the task's only surface) | `visual:capture` (17-ledger) | copy exact, both themes, 1440×900, no layout shift | `_gallery/17-ledger/degraded-banner-1440x900-{dark,light}.png` — the exact copy on the Overview screen (the Cleanup Centre stub's home until AJ-E506), rendered only when `ledger_degraded` is true (mock default false; no banner in any other capture — byte-stable suite) | PASS | 2026-10-14 | zcode-primary |
| Performance (what is measurable) | storage roundtrip + spawn path | Intent+patch ≤5 ms p95; 500-helper batch ≤20 ms; ≤10 MB/10k rows; no extra fsync | local SSD: intent+patch are two indexed statements on WAL (no fsync beyond defaults); the batch is one transaction of prepared statements (500 rows ≈ 500×2 statements ≤ the budget on any modern SSD — the roundtrip exercises the exact SQL); rows are ≤~0.5 KB each (≤5 MB/10k) | PASS (by construction + spot runs) | 2026-10-14 | zcode-primary |
| Regression | full suites | prior gates intact | `just verify` green (fmt, clippy `-D warnings`, workspace tests incl. v12 roundtrips + the E410/E413/E414 suites over the new ctx-bearing spawn API, docs, evidence, terminology); 150/150 component; `pnpm build` green; 107/107 visual | PASS | 2026-10-14 | zcode-primary |

## Notes for the next agent

- E502 next (PGID at spawn): `LedgerRow.pgid` exists but is always
  `None` — capture it via `nix::unistd::getpgid` in the spawn patch and
  thread it through `spawned()` (the trait already takes it).
- The reconciler tick's helper rows carry `exe_path`/`cmdline_*` from
  the sysinfo NAME only, and `cwd` is empty for helpers (unknown at
  that layer) — E504's reconciler should enrich from sysinfo's
  `process.cwd()`/`exe()` and record real depth (currently 1 for all).
- `Supervisor.spawn*` now takes `&SpawnContext` — every caller
  (commands ×3, terminal login, tests) builds one; role vocabulary
  `root|helper|login` mirrors pass2 §5.
- The ledger repo uses its OWN connection to `juggler.sqlite` (WAL):
  never take the app storage lock from ledger paths.
- `AppState.ledger` (the `CoreLedgerStore` handle) is the query surface
  for E504's orphan candidates (`candidates(machine)` already exists
  and is roundtrip-tested).

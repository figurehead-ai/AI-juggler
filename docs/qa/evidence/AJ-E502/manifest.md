# Evidence manifest — AJ-E502 (Process-group ownership at spawn)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E502.
Branch: `stage2/e5-pgid`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Rust suites run in CI (verify job); the
evidence fixtures run everywhere (no GUI, no grants).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 piped path spawns into its own group; PTY path RECORDS the child's PGID | code review + grep gate + `groups_own_descendants_and_flag_setsid_escapees` | piped root is group leader (PGID == PID); ≤1 `getpgid` per root; `grep -R "process_group\|setsid" crates/juggler-supervisor/src` shows exactly the spawn sites; zero `unsafe` | `group.rs` module: `make_group_leader` uses the STABLE-STANDARD `std::os::unix::process::CommandExt::process_group(0)` (== `setpgid(0,0)`, the spec's `pre_exec setsid` intent with ZERO `unsafe` — the workspace's absolute `unsafe_code = "forbid"` is the stronger law, recorded as the module's mechanism pin); applied at the one piped spawn site (`supervisor.rs spawn_piped`); PTY + desktop paths capture `pgid_of(pid)` (one `getpgid`, never assuming portable-pty's session convention); grep gate: `process_group` appears once (group.rs:59) + docs, `setsid` only in comments; workspace forbids unsafe (compile-enforced) | PASS | 2026-10-13 | zcode-primary |
| Req.2 PGID in `ChildHandle.identity` + ledger `spawned()` patch before `spawn` returns | `groups_own_descendants_and_flag_setsid_escapees` + code review | `ChildHandle.pgid`, `SpawnedProcess.pgid`, ledger root row PGID set before return | `ChildExtras`/`ChildHandle` gained `pgid: Option<u32>` (new `Supervisor::root_pgid(session)` reads it — the reconcile-time containment key); all three `Ok(SpawnedProcess{..})` sites emit it; the shared `spawn()` wrapper patches `ledger.spawned(session, pid, spawned.pgid, started_at)` before returning (evidence dump: root row `pgid == pid == 12905`); `Supervisor::stop` signature UNCHANGED (Req.5 — callers in `commands.rs`/`terminal.rs` untouched) | PASS | 2026-10-13 | zcode-primary |
| Req.3 `GroupOwnership` seam; Windows no-op that never silently falls back | code review (`group.rs`, cfg-gated) | documented `UnsupportedPlatform` for group ops on non-Unix; no silent PID-only fallback claim | `crate::group` is the seam: `GroupError::{UnsupportedPlatform, ProcessGone}`; `make_group_leader`/`pgid_of` return `Err(UnsupportedPlatform)` under `cfg(not(unix))`; the piped spawn site LOGS the degradation (`process groups unsupported; tracking pid-only`) — never claims containment; reconcile detection computes `escaped=false` on probe failure (absence stays absence); Job-Object seam left per pass2 §4.2/EPICS M7-5 with the module doc pointing there | PASS | 2026-10-13 | zcode-primary |
| Req.4 `setsid` helpers detected at reconcile, flagged `escaped_group=true`, containment never claimed | `groups_own_descendants_and_flag_setsid_escapees` → `supervisor-ledger-groups-test.txt`, `evidence-db-dump.txt` | escapee row `escaped_group=1`, PGID ≠ root PGID; in-group helpers PGID == root PGID; flag sticky | root `sh` spawns an inner `sh -c 'sleep 300 & wait'` (double-forked grandchild) AND a `python3 -c "os.setsid(); os.execvp('sleep',…)"` escapee: dump shows root 12905 pgid=12905; grandchild chain (bash 12908, sleep 12912) pgid=12905 IN-GROUP; escapee sleep 12910 pgid=12910 `escaped_group=1`; detection rides the existing 2 s tick (`ledger_upserts` — one `getpgid` per helper, zero new polling); store upsert makes the flag STICKY (`MAX(escaped_group, ?)` — a later clean re-tick never revokes it; roundtrip-asserted in `process_ledger_lifecycle_roundtrip` step 3b) | PASS | 2026-10-13 | zcode-primary |
| Schema v13 | `cargo test -p juggler-core --test storage_roundtrip` → `core-storage-v13-test.txt` | additive column, exactly-once migration, v12 rows readable | `V13 = ALTER TABLE process_ledger ADD COLUMN escaped_group INTEGER NOT NULL DEFAULT 0`; `v12_ledger_upgrades_in_place_to_v13` builds a GENUINE v12 file (migrations[..12] + row), Storage::open lands user_version 13 exactly once, then the repo opens the same file (CREATE no-ops) and reads the v12 row as `escaped_group=false`; repo reopen idempotent; single ALTER owner = the migration runner (repo-first ordering would race "duplicate column" — production always opens Storage first, asserted by the test's ordering); v4→current assert bumped to 13 | PASS | 2026-10-13 | zcode-primary |
| Evidence fixture (the task's proof) | `cargo test -p juggler-supervisor --test ledger -- --nocapture` (2/2 green) + `AIJ_E502_DB` persistent dump | both spec assertions green on macOS (Linux = CI verify job) | full row dump in `supervisor-ledger-groups-test.txt` (root leader, in-group grandchildren, flagged escapee) + `evidence-db-dump.txt` (sqlite rows + schema incl. the v13 column); the E501 ledger test stays green beside it; `#[cfg(unix)]`-gated | PASS | 2026-10-13 | zcode-primary |
| Performance budget | code construction + fixture timing | `setsid` <0.1 ms/spawn; ≤1 `getpgid` per root; zero added polling | `process_group(0)` is one flag on the existing fork path (no measurable cost); spawn captures exactly one `getpgid` per root (piped+desktop+pty each once; `root_pgid` reads the CACHED handle — no syscall at reconcile); helper probes ride the existing 2 s tick (no new loop); full fixture incl. 15 s wait_for budget finishes in 0.17 s | PASS | 2026-10-13 | zcode-primary |
| Regression | full suites | prior gates intact | `just verify` green (fmt, clippy `-D warnings` incl. scoped `similar_names` allows with the kernel-names comment, workspace tests, docs, evidence, terminology); scoped `#[allow(clippy::similar_names)]` on the three spawn fns only; no UI change (visual suite untouched) | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- E503 next (ProcessIdentity + fail-closed verify-before-kill): the
  spawn patch already records `pid + pgid + started_at (OS start) +
  boot_id` — the identity tuple is half-built; add `exe` (symlink-
  resolved) and the `verify()` closure.
- `Supervisor::root_pgid(&SessionId)` is the containment key for
  E507's `killpg` ladder; escaped rows (`escaped_group=1`) must be
  EXCLUDED from group kills (per-pid ladder only) — the flag is the
  contract.
- Helper `pgid` in the ledger is the LIVE value each tick (a process
  CAN migrate groups after spawn; the row records the latest, the
  escaped flag is sticky-OR).
- The spec's `pre_exec setsid` wording is superseded by the stronger
  workspace law (`unsafe_code` forbidden): `process_group(0)` is the
  safe stable twin — group.rs's module doc is the mechanism pin.
- Windows: group ops return `UnsupportedPlatform` and the piped spawn
  logs pid-only tracking; the Job Object seam is M7-5 — do NOT add a
  silent fallback.

# E5 — Process Ownership & Cleanup (Epic Tasks)
> Settled basis: D-01 (Clean is trust proof; zero false kills is success metric), D-08 (notify-only default, Clean-safe opt-in **per Account**, verified-owned only, every kill audit-logged; ADR-0010 required), D-13 (fleet-shaped from day one), D-15 (this file is implementation-ready task spec only; product code starts under separate approval). Canonical demand: `docs/expected-worfklow.md:15,17-18`. Design source: `docs/review/pass2/05-process-trees-zombies-and-cleanup.md` (§§2-14). Do NOT re-litigate D-08 tiers as per-Provider: pass2 §10 says per-Provider, decision-log D-08 overrides to per-Account; per-Provider MAY be a UI grouping, storage key SHALL be `account_id`.
> Sequencing: ledger+PGID+identity (E501-E503) before any tree-kill; read-only Centre (E506) before ladder; tiers last. No one-click kill. UI vocabulary Provider/Account/Instance/Helper/Orphan; never label T2/T3 "zombie" (pass2 §2).

### AJ-E501 Persistent spawn ledger with intent-before-spawn | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E501/manifest.md) | Priority P0 | Effort L | Depends: none

**Goal**: Replace root-only journal memory with a queryable `process_ledger` surviving root exit, app exit, and reboot, so orphan detection is possible at all.

**User story**: As a developer whose vite server outlived its Instance, I want Juggler to still know that helper was mine after the root (and Juggler) exited, so it can be listed and cleaned instead of forgotten.

**Requirements**:
1. The system SHALL add a SQLite `process_ledger` table (WAL store) with columns per pass2 §5: `pid,pgid,started_at` (OS start time), `exe_path,cmdline_hash,cmdline_trunc,boot_id,session_id,instance_id NULL,provider,account_id,project_id,machine_id,root_pid,parent_pid,depth,first_seen,last_seen,role,state,listens_on,cwd,peak_rss,peak_cpu,exit_kind,reaped_by_us` and indexes on `(session_id,state)`, `(pid,started_at,boot_id)`, `(state,last_seen)`.
2. The system SHALL define `LedgerStore: Send+Sync` in supervisor (`intent/spawned/upsert_helpers/candidates`, pass2 §12) so `juggler-supervisor` stays storage-agnostic; the SQLite impl SHALL live in `juggler-core` storage.
3. The system SHALL write an `Intent` row (`state=Starting`, no PID) BEFORE spawn, patch PID/PGID/start-time immediately after spawn, fixing the spawn-precedes-journal ordering at `supervisor.rs:107-114`; ledger failure SHALL NOT fail the spawn — it SHALL set a `ledger_degraded` health flag and emit a lifecycle warning.
4. The system SHALL upsert helper rows batched in ONE transaction per reconciler tick (interface owned by AJ-E504), at most once per 2 s per Instance.
5. The system SHALL retain exited-helper rows 7 days, never prune rows for still-running processes, and prune via the existing journal-prune job.
6. The system SHALL record `machine_id` from `SessionRecord.machine` (`model.rs:398`, `MachineId` `model.rs:63-66`) on every row (D-13); local rows SHALL use the stable local machine id, never hard-coded `"this-mac"` literals at call sites (`commands.rs:699` pattern forbidden in new code).

**UI/UX reasoning**: No new surface; this task SHALL surface only the `ledger_degraded` banner in Cleanup Centre header: copy `Process tracking degraded — cleanup is read-only until restart.` (NN/g: visibility of system status; error prevention over error messages). No modal, no spawn blocking.

**Performance budget**: Intent+patch ≤5 ms p95 on local SSD; batched upsert of 500 helpers ≤20 ms; ledger adds ≤10 MB per 10k rows; zero per-spawn fsync beyond WAL defaults.

**Quality gates**: `cargo test -p juggler-supervisor -p juggler-core` green; clippy `-D warnings`; migration test upgrades a v-current DB fixture and rolls back cleanly on failure; no new `unsafe`.

**Evidence/validation proof**: New `crates/juggler-supervisor/tests/ledger.rs`: fake harness spawns 3 helpers → assert ledger rows (root+helpers, correct account/project/machine) within 2 ticks; kill -9 Juggler-side supervisor, reopen store → rows persist. Store output in `docs/qa/evidence/E5/AJ-E501/` (test log + `sqlite3 .dump process_ledger` excerpt). Pass = all rows present, 0 missing helpers.

**Visual testing**: Screenshot only the degraded banner (1440×900, light+dark) in Cleanup Centre stub; pass = banner visible, no layout shift >4 px, axe-core 0 violations.

**Files to touch**: `crates/juggler-supervisor/src/tree.rs` (new: ledger types), `crates/juggler-supervisor/src/lib.rs:14-24`, `crates/juggler-supervisor/src/supervisor.rs:107-140`, `crates/juggler-core/src/storage/mod.rs` (migration), `crates/juggler-core/src/storage/process_ledger.rs` (new), `crates/juggler-core/src/model.rs:389-416`

**Research basis**: pass2/05 §5 (lines 192-227), §3 journal gap (`journal.rs:34-48` roots only); decision-log D-13:131-137.

**Handover note**: Trap: do not extend the JSONL journal into a query store — ledger is SQLite, journal stays crash log. Trap: `started_at` is OS process start time, NOT wall-clock spawn time (E503 depends on it).

*Implementation mapping (corrected in this task's PR per the claiming protocol)*:
- Schema v12: `process_ledger` (all pass2 §5 columns + the 3 indexes) in juggler-core storage; `ProcessLedgerRepo` behind its OWN WAL connection + mutex (spawn threads write without the app storage lock); exited rows prune at startup (7 days; running never).
- Seam: `LedgerStore` trait (`boot_id/intent/spawned/upsert_helpers/mark_exited/candidates/prune`) in supervisor `tree.rs`; SQL lives only in core (`CoreLedgerStore` bridges).
- Intent-first: `spawn`/`spawn_desktop` take a `SpawnContext` (provider/account/project/machine/role) and write the `starting` row BEFORE the fork; PID + OS start time patch after; every ledger error sets `ledger_degraded` (banner copy exact, rendered on Overview via `StatsInfo.ledger_degraded`) — spawns never fail for ledger reasons.
- Helpers: `LiveStatsCollector::tick` → `ledger_upserts` (one batched transaction per 2 s per Instance, update-first so no duplicates; peaks refreshed).
- Machine identity: `local_machine_id()` (generated once in app_settings; no new `"this-mac"` literals); `boot_id` = OS boot time (sysinfo) on every row/exit patch.
- Evidence: `docs/qa/evidence/AJ-E501/` — root+3-helpers fixture (0 missing, exit patch `signal:15`, reopen persistence), sqlite dump excerpt, log; gallery `_gallery/17-ledger/` banner captures both themes.

### AJ-E502 Process-group ownership at spawn (Unix PGID, Windows seam) | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E502/manifest.md) | Priority P0 | Effort M | Depends: AJ-E501

**Goal**: Create ownership at spawn time (dedicated process group per Instance) instead of inferring it later from racy ppid walks.

**User story**: As a developer, I want every helper my Instance spawns to stay reachable as one owned group even if it double-forks, so "terminate the tree" is deterministic rather than a snapshot race.

**Requirements**:
1. On Unix, the piped spawn path (`supervisor.rs:129-140`, bare `Command` today) SHALL use `pre_exec` with a single async-signal-safe `setsid` call only; the PTY path (`supervisor.rs:194-219`) SHALL explicitly record the child's PGID via `child.process_id()` instead of assuming PTY session grouping.
2. The system SHALL capture PGID into `ChildHandle.identity` and the ledger `spawned()` patch (AJ-E501) before returning from `spawn`.
3. The system SHALL define a `GroupOwnership` seam (trait or cfg-gated module) whose Windows implementation is a documented no-op returning `UnsupportedPlatform` for group ops, leaving the Job Object (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) seam per pass2 §4.2/EPICS M7-5 — Windows SHALL NOT silently fall back to PID-only kill.
4. Helpers that themselves `setsid` SHALL be detected (PGID ≠ root PGID at reconcile time) and ledger-flagged `escaped_group=true`; the system SHALL NOT claim containment for them.
5. `Supervisor::stop` signature SHALL remain source-compatible for existing callers (`commands.rs:740`, `terminal.rs:200`); internals delegate in AJ-E507.

**UI/UX reasoning**: No UI except the Cleanup Centre evidence line `escaped its process group` on escaped helpers (pass2 §4 macOS note). Guideline: NN/g error prevention — honest containment claims prevent false-confidence kills (FMEA C2).

**Performance budget**: `setsid` adds <0.1 ms per spawn; PGID capture ≤1 `getpgid` syscall per root; zero added polling.

**Quality gates**: Supervisor test suite green on macOS + Linux CI; `grep -R "process_group\|setsid" crates/juggler-supervisor/src` shows exactly the spawn sites; no new `unsafe` beyond the `pre_exec` closure (documented async-signal-safety comment).

**Evidence/validation proof**: Extend `crates/juggler-supervisor/tests/supervisor.rs`: fixture shell spawns a double-forked grandchild in-group → assert grandchild PGID == root PGID in ledger; fixture with `setsid` escapee → assert `escaped_group=true`. Evidence in `docs/qa/evidence/E5/AJ-E502/`. Pass = both assertions green on macOS and Linux.

**Visual testing**: None (backend); capture terminal log screenshot not required. Verify escaped-group badge text appears in AJ-E506 screenshots later.

**Files to touch**: `crates/juggler-supervisor/src/supervisor.rs:122-266`, `crates/juggler-supervisor/src/tree.rs`, `crates/juggler-supervisor/Cargo.toml` (existing `nix` dep only)

**Research basis**: pass2/05 §4 (lines 161-190), §3 "no setpgid/setsid/killpg under crates/" (lines 115-123); deep/08 §5 F-9.

**Handover note**: Trap: a helper forking between ppid-walk and signal escapes tree-kill — that is why PGID, not `descendants_of` (`supervisor.rs:557-579`), is the kill primitive. Do not "fix" F-9 by bolting a ppid walk onto `stop()`.

*Implementation mapping (recorded in this task's PR per the claiming protocol)*:
- Seam: new `crates/juggler-supervisor/src/group.rs` — `make_group_leader` via the STABLE-STANDARD `CommandExt::process_group(0)` (the spec's `pre_exec setsid` intent, zero `unsafe`: the workspace's absolute `unsafe_code = "forbid"` is the stronger law and the module doc is the mechanism pin) + `pgid_of` (`nix::unistd::getpgid`); `GroupError::{UnsupportedPlatform, ProcessGone}`; `cfg(not(unix))` returns `UnsupportedPlatform` (Job Object seam stays M7-5, never a silent PID-only fallback — the piped site logs the degradation).
- Capture: `ChildExtras`/`ChildHandle`/`SpawnedProcess` gained `pgid`; piped path calls `make_group_leader` BEFORE `.spawn()`; PTY + desktop paths record one `getpgid` each; the shared `spawn()` wrapper patches `ledger.spawned(..., spawned.pgid, ...)` before returning; new `Supervisor::root_pgid(&SessionId)` reads the cached handle (the E507 killpg key). `Supervisor::stop` signature unchanged (Req.5).
- Detection: `LiveStatsCollector::ledger_upserts` records each helper's LIVE pgid and flags `escaped_group` when it differs from the root's (one `getpgid` per helper riding the existing 2 s tick — zero new polling; probe failure claims nothing).
- Schema v13: `ALTER TABLE process_ledger ADD COLUMN escaped_group INTEGER NOT NULL DEFAULT 0`; sticky via `MAX(escaped_group, ?)` in the helper upsert (a clean re-tick never revokes the no-containment verdict); single ALTER owner = the migration runner (the repo's standalone CREATE is born v13-shaped; production always opens Storage before the ledger repo).
- Evidence: `docs/qa/evidence/AJ-E502/` — root leader + in-group double-forked grandchild (PGID == root PGID) + `setsid` escapee (`escaped_group=1`, PGID ≠ root) in `supervisor-ledger-groups-test.log` + `evidence-db-dump.txt`; v12→v13 upgrade test in `core-storage-v13-test.log`. UI untouched (badge wording lands with AJ-E506).

### AJ-E503 ProcessIdentity and fail-closed verify-before-kill | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E503/manifest.md) | Priority P0 | Effort M | Depends: AJ-E501

**Goal**: Make PID reuse (F-2/T5) structurally un-killable: a PID is a name, not an identity.

**User story**: As a developer, I want Juggler to refuse to touch a process whose identity no longer matches what it recorded, so a recycled PID belonging to my own editor or database can never be killed by mistake.

**Requirements**:
1. The system SHALL define `ProcessIdentity { pid, pgid, start_time: i64, exe: PathBuf, boot_id: String }` and `pub fn verify(pid, expect: &ProcessIdentity) -> Verification` (pass2 §12) in `tree.rs`.
2. Verification SHALL require ALL of: (a) OS start-time equals recorded start-time, (b) symlink-resolved exe path equals recorded exe, (c) `boot_id` equals current boot id. Any mismatch or probe error SHALL return `Verification::Unverifiable` (fail closed, never `Verified` on error).
3. The system SHALL add `SupervisorError::Unverifiable { pid }` and `::LedgerDegraded` to the closed enum at `error.rs:10-39`; callers SHALL handle refusal as a typed outcome, never as `UnknownSession`.
4. Adoption (`journal.rs:188-216`, today liveness-only via `alive_pids`) SHALL use `verify()` for roots; every kill path in AJ-E507/E508 SHALL re-verify in the same tick as the signal.
5. A row failing verification SHALL flip ledger `state=Unverifiable` permanently and be removed from every kill path and auto-tier candidate set.

**UI/UX reasoning**: Unverifiable rows show badge `Unverifiable — will not be touched` (pass2 §2 T5 label) with NO kill affordance at any tier. Guideline: WCAG 2.2 + Nielsen H5 (error prevention): removing the control beats disabling it with a tooltip (pass2 §9.2 step 1).

**Performance budget**: ≤1 `stat`/`proc_pidinfo` per candidate per tick (≤2 ms per 100 candidates); verification on kill path ≤10 ms p95.

**Quality gates**: Unit tests for each single-field mismatch (start-time, exe, boot_id) → `Unverifiable`; probe-error injection → `Unverifiable`; zero `Verified` on error paths.

**Evidence/validation proof**: PID-reuse drill in `crates/juggler-supervisor/tests/identity.rs`: spawn helper, record identity, kill it, spawn stranger (simulated mismatch via forged expected identity where real PID reuse is not forceable) → assert `Unverifiable` and that `terminate_group` refuses. Evidence `docs/qa/evidence/E5/AJ-E503/`. Pass = 0 signals sent to mismatched PID (assert via fixture's signal-trap log).

**Visual testing**: Screenshot Unverifiable row + confirmation-sheet explanation state in AJ-E506/E507 captures (1440×900); pass = no kill button in DOM (assert via accessibility tree dump).

**Files to touch**: `crates/juggler-supervisor/src/tree.rs`, `crates/juggler-supervisor/src/error.rs:10-39`, `crates/juggler-supervisor/src/journal.rs:151-216`, `crates/juggler-supervisor/src/supervisor.rs:432-438`

**Research basis**: pass2/05 §6.2 (lines 254-268, incl. man7 proc(5) PID-reuse citation), §2 T5; deep/08 §5 F-2 (`journal.rs:188-196`).

**Handover note**: Trap: helpers, not roots, are where PID reuse bites during long sessions — verify at kill time for EVERY candidate, not only at adoption. Trap: clock changes must not mass-flip rows; `boot_id` gate runs first (FMEA C8).

*Implementation mapping (recorded in this task's PR per the claiming protocol)*:
- Primitives (Req.1/2): `tree.rs` — `ProcessIdentity {pid, pgid, start_time, exe, boot_id}`, `Verification {Verified, Unverifiable}`, `pub fn verify` (real probe) + `verify_with` (injectable `IdentityProbe` seam; tests forge) + `verify_any` (adoption replay); boot gate FIRST (FMEA C8); probe error and missing-live-exe are `Unverifiable`, never `Verified`.
- Errors (Req.3): `SupervisorError::Unverifiable { pid }` + `::LedgerDegraded` (typed refusal, distinct from `UnknownSession`).
- Adoption (Req.4): `Journal::adopt_orphans` verifies roots against BOTH journal identity records — `spawned`'s spawn-time probe AND the new `identity_settled` entry (+250 ms settle thread re-probes the kernel exe after wrappers exec their interpreter; a start-time disagreement mid-settle leaves the record untouched). Pre-E503 lines (serde-default) → process left alone, fail closed. Kill-path in-tick re-verification is E507/E508's own spec (their ladders); `Supervisor::root_identity()` exposes the input.
- Permanent flip (Req.5): `STATE_UNVERIFIABLE` + `ProcessLedgerRepo::mark_unverifiable` + `LedgerStore::mark_unverifiable`; candidates are running-only and the prune exited-only → an unverifiable row can never re-enter either, ever (drill + roundtrip assert both).
- Supervisor wiring: `os_probed` (one targeted refresh, retry on table lag) builds spawn identities at all three paths; `SpawnedProcess`/`ChildHandle`/`ChildExtras` carry it; the ledger's `started_at` now comes from the same probe (one probe, not two — `os_start_time` deleted).
- Evidence: `docs/qa/evidence/AJ-E503/` — the PID-reuse drill (forged single-field strangers, 0-signal trap-log witness, dead-pid arm, control arm), 6 unit tests per quality gate, adoption-restart test, storage roundtrip, sqlite dump of the flipped row.

### AJ-E504 Reconciler loop, orphan detection, and children-map eviction | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E504/manifest.md) | Priority P0 | Effort L | Depends: AJ-E501, AJ-E502, AJ-E503

**Goal**:
One shared 2-second reconciler replaces per-call `System::new()` scans, maintains the ledger for live and exited Instances, and marks true orphans (T2) only after grace and two-tick confirmation.

**User story**:
As a developer running many Instances, I want helper processes tracked continuously without Juggler burning CPU on full process scans per query, and I want brief helper flicker during builds never to appear as a false orphan alert.

**Requirements**:
1. The system SHALL own ONE shared `sysinfo::System` refreshed on the SPEC §5.2 cadence (~2 s supervised trees, `docs/SPEC.md:231`), replacing per-call `System::new()` at `supervisor.rs:557-579` and `journal.rs:200-216`; `descendants()` SHALL be reimplemented on the reconciler's cached ppid map with no fresh full refresh per query.
2. Each tick the reconciler SHALL: build the ppid map once in memory; walk live roots and upsert helpers (AJ-E501 batch); refresh RSS/CPU and ports (AJ-E505); re-verify `Running|Orphaned|Adopted` rows via AJ-E503 (failure → `Unverifiable`, permanently un-killable).
3. When a root exits, helpers whose ppid becomes 1 or whose ancestor chain no longer reaches a live Juggler-owned root SHALL become `Orphaned` only after `orphan_grace` (default 30 s, per-Provider tunable stored in settings) AND two consecutive agreeing ticks; helpers exiting inside grace SHALL journal as normal exits and never surface in the Cleanup Centre.
4. The system SHALL evict `ChildHandle`s from the insert-only `children` map (`supervisor.rs:285-287`, F-6) on `Exited` after the output-retention window, so stopped Instances leave no phantom `Running` rows and rings do not grow for the process lifetime.
5. The reconciler SHALL emit lifecycle deltas as events; the UI SHALL NOT poll `ps`/`lsof` itself. Role guesses (`Harness|Subagent| DevServer|McpServer|TerminalShell|Watcher|BuildTool|Unknown`) SHALL be stored as guesses and labelled as such everywhere.
6. Fork storms (e.g. `cargo build` spawning dozens of short-lived `rustc` helpers) SHALL NOT flood the ledger UI: single-tick sightings stay hidden (Requirement 3) and healthy trees render collapsed by default (AJ-E506).

**UI/UX reasoning**:
No direct UI; this task guarantees the Cleanup Centre list is stable
(no flickering rows). Nielsen H1 (visibility of system status) plus
NN/g alert-fatigue guidance: two-tick confirmation exists specifically so users learn to trust every row that does appear. Copy for the grace state in row tooltip: `Waiting to confirm it was left behind…`.

**Performance budget**:
Reconciler at 50 Instances SHALL keep supervisor CPU <1% of one core
(SPEC `docs/SPEC.md:24` budget, pass2 §14.6); tick wall time ≤100 ms p95 at 50 Instances / 2,000 helpers; zero `System::new()` per query
(regression-counted, see Evidence); RSS bookkeeping ≤8 MB.

**Quality gates**:
`cargo test -p juggler-supervisor` green incl. new reconciler tests; a regression test SHALL count `System` constructions per tick and fail if >1; clippy `-D warnings`; no ledger write outside the batched transaction except Intent/spawned (AJ-E501).

**Evidence/validation proof**:
`crates/juggler-supervisor/tests/reconciler.rs`: (a) fake Instance spawns vite-like helper, root exits cleanly → helper invisible at grace−1 s, `Orphaned` at grace+2 ticks with correct Provider/Account/
Project; (b) helper exiting 200 ms after root never appears;
(c) 50-instance soak fixture → CPU/tick log. Store logs in
`docs/qa/evidence/E5/AJ-E504/`. Pass = (a)(b) exact, (c) CPU <1%.

**Visual testing**:
Record a 10 s screen capture of Cleanup Centre during a `cargo build` fork storm (1440×900): pass = zero rows flicker in/out, frame rate
≥30 fps, no row reorders more than once per tick.

**Files to touch**:
`crates/juggler-supervisor/src/tree.rs` (reconciler),
`crates/juggler-supervisor/src/supervisor.rs:285-318,347-356,557-579`,
`crates/juggler-supervisor/src/journal.rs:199-216`,
`crates/juggler-supervisor/src/lifecycle.rs`,
`crates/juggler-supervisor/tests/reconciler.rs` (new)

**Research basis**: pass2/05 §6.1-6.3 (lines 231-280), §3 children-map leak (lines
146-149); SPEC §5.2 `docs/SPEC.md:231-234`; deep/08 §5 F-6.

**Handover note**:
Trap: `sysinfo` CPU% needs two samples — first tick after spawn shows
0% by design, do not "fix" by refreshing faster. Trap: do not lower
`orphan_grace` to make demos snappier; the grace is the false-alert budget. Next agent: AJ-E505 ports join this same tick, not a new loop.

*Implementation mapping (recorded in this task's PR per the claiming protocol)*:
- ONE shared table (Req.1): `LiveStatsCollector::tick` builds ONE `ProcessTable` (ppid map + per-pid identity facts `start_time`/`exe`) from its persistent `sysinfo::System` and publishes it to `Supervisor`; `descendants()` walks the cached snapshot (counted cold fallback only before the first tick); every `System::new()` in the crate routes through `system_construction_count()` — the regression gate (soak observed 0/tick, gate ≤1).
- Reconcile pass (Req.2): after the batched helper upsert (now recording the REAL kernel start time + exe — never wall-clock/name; legacy name-exe rows healed once inside the upsert transaction), `candidates(machine)` for `running|orphaned` rows are re-verified against the tick's OWN table: boot gate first (FMEA C8), start-time all rows, exe KERNEL-vs-kernel for helpers, roots against the ChildHandle's kernel-recorded identity (the launch program may be kernel-aliased, e.g. macOS /bin/sh → bash); mismatch → `mark_unverifiable` (permanent, E503). Helper exits patch by TWO-tick absence (`unknown`). Ports join in E505 on this same tick.
- Orphans (Req.3): running helpers of an EXITED root, past `orphan_grace` (`DEFAULT_ORPHAN_GRACE_SECS = 30`, per-Provider via `set_orphan_grace`; `app_settings.orphan_grace_secs` pushed at init), whose ppid chain (`detached_from`, 64-hop guard) no longer reaches a live owned root → `mark_orphaned` only after TWO consecutive agreeing ticks (flap counters reset on disagreement); `HelperOrphaned { session, pid }` event emitted; in-grace exits journal as `exited` and never surface (asserted).
- Eviction (Req.4): `emit_exit` stamps `exit_at_ms`; each tick `sweep_finished` drops finished handles past `OUTPUT_RETENTION_AFTER_EXIT_MS` (120 s) — soak asserts 25/25 stopped evicted, 25/25 live untouched; ledger rows keep the history.
- Ledger vocabulary: `STATE_ORPHANED` — orphaned rows STAY candidates (re-verified per tick) and resolve to `exited` on death; `candidates` selects `running|orphaned`; `mark_exited` accepts both.
- Evidence: `docs/qa/evidence/AJ-E504/` — grace drill + early-exit drill + 50-instance soak (avg 12 ms/tick, max 25 ms, self CPU 0.34%, 0 constructions) + the orphaned-row sqlite dump with full attribution.

### AJ-E505 Port occupancy map and leftover inventory data | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E505/manifest.md) | Priority P0 | Effort M | Depends: AJ-E504

**Goal**:
Join ledger ownership to the OS listener table so every left-behind process can be explained as "PID 8123 holds :5173 for Instance X", and launch can warn before `EADDRINUSE` instead of after.

**User story**:
As a developer about to start a dev server, I want to know that :5173 is already held by an orphan from Tuesday's Instance (not by my own unmanaged `npm run dev`), so I free the right process and never kill a stranger by port guesswork.

**Requirements**:
1. The system SHALL implement a `PortProbe` trait in a new `crates/juggler-supervisor/src/ports.rs` (`netstat2` on macOS/Linux, `GetExtendedTcpTable` seam on Windows); it SHALL NOT shell out to `lsof` (slow, permission-flaky, unparsable under load, pass2 §7). Platforms without a probe SHALL report `ports unknown on this platform`, never an empty map presented as truth.
2. Each reconciler tick (AJ-E504) SHALL join listeners to ledger PIDs and store `listens_on` per row; well-known dev ports (3000, 4173, 5173, 8000, 8080, 9229) MAY be hint-labelled but SHALL NEVER be used as identity evidence.
3. The inventory SHALL classify every listener as exactly one of: `Managed orphan` (T2), `Managed running`, `Re-adopted` (T3), or `Not managed by Juggler` (user's own processes, display-only, no kill affordance ever — FMEA/F3 collateral guard).
4. The launch path SHALL warn, before spawn, when a target project's expected dev port is held by a managed orphan: exact copy `:5173 is already held by an orphan from {Instance} ({age}).` with actions `Free this port` (routes to AJ-E507 verified sheet) and `Launch anyway`; launching SHALL never be blocked (SPEC §5.2 advisor informs, never blocks).
5. Leftover inventory SHALL also include hog facts needed by AJ-E506: per-helper RSS/CPU, per-Instance tree rollup, peak RSS/CPU in the ledger; a helper SHALL be flagged `hog` only after sustained breach over `hog_window` (default 60 s): tree RSS > per-Provider learned p95, OR single helper >2 GiB, OR >90% of one core for the window.

**UI/UX reasoning**:
Port rows read `port → PID → Instance → Provider · Account · Project
· listener age` with a collision badge (pass2 §7). Nielsen H2 (match between system and real world): users think in ports, so the port is the headline and the PID is evidence. `Not managed by Juggler` rows are visually muted and have no buttons (Apple HIG: don't offer actions that cannot be taken safely).

**Performance budget**:
Listener enumeration ≤30 ms p95 per tick on macOS/Linux; inventory query for Cleanup Centre ≤50 ms at 2,000 rows; probe failure SHALL degrade in ≤1 tick, never retry-storm (min 2 s between retries).

**Quality gates**:
Unit tests with recorded listener-table fixtures (including a port held by an unmanaged PID and a recycled PID); `cargo test
-p juggler-supervisor` green; no `lsof`/`netstat` shell-out anywhere
(`grep -R "lsof" crates/` clean except docs/comments explaining ban).

**Evidence/validation proof**:
Fixture: vite-like helper binds :5173, root exits → within 2 ticks inventory shows `Managed orphan … holds :5173`; start an unmanaged
`python3 -m http.server 8000` → shows `Not managed`, no kill control in the API response. Evidence (API JSON dumps) in
`docs/qa/evidence/E5/AJ-E505/`. Pass = both rows correct, unmanaged row exposes zero kill command ids.

**Visual testing**:
Screenshots (1440×900 and 1024×700, light+dark): port-collision launch warning and inventory with all four classifications. Pass = collision badge and muted unmanaged row distinguishable without colour alone
(WCAG 1.4.1: icon+text, not colour only); axe-core 0 violations.

**Files to touch**:
`crates/juggler-supervisor/src/ports.rs` (new),
`crates/juggler-supervisor/src/tree.rs`,
`crates/juggler-supervisor/src/lib.rs:14-24`,
`crates/juggler-supervisor/Cargo.toml:19` (`sysinfo`, add `netstat2`),
`crates/juggler-app/src/commands.rs` (launch pre-check),
`crates/juggler-core/src/model.rs` (inventory DTOs if core-owned)

**Research basis**: pass2/05 §7 (lines 282-304), §8 hog rules (lines 306-330), §3
"no port dimension at all" (lines 131-136); SPEC §5.2
`docs/SPEC.md:234,243`.

**Handover note**:
Trap: a port hint list is a display hint, never ownership proof — identity still comes from AJ-E503. Trap: do not add a second polling loop for ports; AJ-E504's tick is the only cadence. Windows probe is a seam in this task; do not fake it with `netstat.exe` parsing.

*Implementation mapping (recorded in this task's PR per the claiming protocol)*:
- Probe (Req.1): `crates/juggler-supervisor/src/ports.rs` — `PortProbe` trait, `NetstatProbe` (netstat2, ONE pass over listening TCP both families, pids; 5-6 ms for ~119 listeners), `UnsupportedProbe` on non-unix (`ports unknown on this platform`, the GetExtendedTcpTable seam stays M7-5); no lsof/netstat shell-outs anywhere.
- Join (Req.2): rides the E504 tick — JOIN every tick on the CACHED listener table, ENUMERATION every 5th tick (`PORT_PROBE_EVERY`; loaded-box cost ~90 ms vs the ≤100 ms tick budget — the budget governs); `apply_ports` = one batched transaction per tick onto the newest running-or-orphaned row; candidates fetched once per tick and shared with reconcile (duplicate query stalled under disk pressure); hints display-only.
- Classification (Req.3): pure `classify_listeners` — pid→newest running-or-orphaned row; unverifiable/exited rows grant nothing (recycled-pid fixture); helper-of-dead-root = Managed orphan; adopted (excluding currently-supervised — bug found by the fixture, fixed in `alive_adopted_sessions`) = Re-adopted; no match = Not managed (`killable:false`, zero kill ids in the DTO — the F3 guard lives in the API).
- Warning (Req.4): `launch_port_warning(project)` = learned ports (ledger `listens_on` history) ∩ current orphan-held; copy-exact message + humanized age; the App pre-checks before spawn and NEVER blocks (Launch anyway proceeds; probe failure launches).
- Hogs (Req.5): `hog_facts` — 60 s sustained window over the collector rings: helper >2 GiB, tree >90% core, tree > provider-p95 (≥3-peer floor); returned by `port_inventory`.
- Evidence: `docs/qa/evidence/AJ-E505/` — full API JSON dumps (live/orphan/warning phases, unmanaged :8000 with zero kill ids), unit tests (unmanaged + recycled fixtures), gallery `_gallery/18-ports/` (inventory 4 classifications + collision sheet, both themes/sizes; icon+text never colour alone).

### AJ-E506 Cleanup Centre (read-only) and per-Instance Processes view | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E506/manifest.md) | Priority P0 | Effort L | Depends: AJ-E504, AJ-E505

**Goal**:
Ship the global Cleanup Centre as a read-only inventory (Tier 0,
D-08 default) plus the same rows embedded in the Instance panel, so tracking ships before any new kill capability exists.

**User story**:
As a developer, I want one place that answers "what did my Instances leave behind, what is it costing, and why does Juggler think it is mine?" without Juggler touching anything yet.

**Requirements**:
1. The system SHALL add a global Cleanup Centre surface reached from the top-right global menus (ADR-0007 global/setup mode, `docs/adr/0007-instance-centric-information-architecture.md:20-23`), NOT as a rail peer of Projects/Instances/Sessions.
2. The list SHALL be one row per process, grouped by Instance, healthy trees collapsed by default. Columns SHALL include: state badge using pass2 §2 labels verbatim (`Still running after its Instance closed` T2, `Running — re-adopted` T3, `Unverifiable — will not be touched` T5, plus Running/Exited), role guess labelled `guess`, exe/cmdline (truncated, expandable), PID/PGID, age, live RSS/CPU, ports, `Provider · Account · Project`, machine (AJ-E511), and the verification badge from AJ-E503.
3. Filters SHALL be exactly: `Needs attention` (default: Orphaned + hogs + port collisions), `All managed`, `Re-adopted`, `History (7 days)` (reads AJ-E509 audit/ledger history).
4. Every row SHALL expand `Why does Juggler think this is mine?` showing the evidence chain: spawned-by root, first-seen ppid, PGID match/mismatch, exe/start-time check result, first/last seen. Rows without a complete chain SHALL NOT show kill affordances in AJ-E507.
5. The empty state SHALL be a success state with exact copy: `Nothing left behind — {N} Instances running clean.` The ledger-degraded state (AJ-E501) SHALL show the banner from AJ-E501 and the list SHALL be labelled read-only.
6. The Instance panel Processes tab (D-06 six-tab inspector, Processes tab) SHALL embed the same row component filtered to that Instance, with no second data path or divergent state labels.
7. This task SHALL ship zero signalling: no kill/terminate command id SHALL be registered, rendered, or reachable via keyboard.

**UI/UX reasoning**:
Nielsen H1/H6: state badge + evidence expansion makes ownership recognisable rather than recalled. Apple HIG (clarity/deference): healthy trees collapsed so attention lands on orphans. WCAG 2.2: badges use icon+text (never colour alone, 1.4.1), rows fully keyboard navigable (2.1.1), focus visible (2.4.7), filter changes announced via
`aria-live=polite` (4.1.3). Shortcut: `⌘⇧C` opens Cleanup Centre;
`⌘F` focuses its filter. Glossary tooltip on first use of orphan/ helper per D-09 ("zombie" plain-language glossary duty).

**Performance budget**:
First paint ≤300 ms at 2,000 rows (virtualised list, ≤60 DOM rows mounted); delta update ≤16 ms per tick (no full re-render); filter switch ≤100 ms; UI stays 60 fps during fork-storm capture (AJ-E504).

**Quality gates**:
Component tests assert zero kill command ids in the rendered tree;
`npm run build`/typecheck in `ui/` green; axe-core 0 violations in
Storybook-equivalent fixture states (orphan, hog, unverifiable, empty, degraded).

**Evidence/validation proof**:
Live dogfood: run a fake Instance spawning a vite-like helper on
:5173, close the Instance, wait out grace → capture Cleanup Centre row with correct Provider/Account/Project/port; expand evidence and archive the expansion JSON. Evidence in
`docs/qa/evidence/E5/AJ-E506/`. Pass = row appears ≤2 ticks after grace, evidence chain complete, zero kill controls in DOM snapshot.

**Visual testing**:
Screenshots at 1440×900 and 1024×700, light+dark, for states:
Needs-attention (orphan+hog+collision), All-managed, Re-adopted,
Unverifiable, empty-success, ledger-degraded. Method: pixel-diff against committed baselines (≤0.5% differing pixels, anti-aliasing excluded) + manual badge-legibility check. Pass = diffs within threshold AND badges pass WCAG contrast ≥4.5:1 (checked via axe).

**Files to touch**:
`ui/src/screens/CleanupCentre.tsx` (new),
`ui/src/components/ProcessRow.tsx` (new, shared with Instance panel),
`ui/src/App.tsx`, `ui/src/components/WorkflowRail.tsx` (global-menu entry only), `ui/src/screens/WorkPanels.tsx` (Instance Processes tab),
`crates/juggler-app/src/commands.rs` (read-only inventory commands),
`crates/juggler-app/src/state.rs`

**Research basis**: pass2/05 §9.1 (lines 340-356), §2 labels (lines 84-92), §8
(lines 306-330); decision-log D-06:60-68, D-09:88-95;
`docs/expected-worfklow.md:15,18`.

**Handover note**:
Trap: do not "preview" kill buttons in this task — a rendered-but- disabled kill trains the wrong trust model and breaks the Tier-0 guardrail test in AJ-E508. Trap: the Instance panel embedding must reuse `ProcessRow`, not a copy; divergence here is how T3 survivors later get mislabelled as orphans.

*Implementation mapping (recorded in this task's PR per the claiming protocol)*:
- Backend: `cleanup_inventory(filter)` read command — rows grouped per Instance from the ledger (all states; history = exited within the 7-day retention), enriched with live RSS/CPU (the E504 collector), ports (`listens_on`), hog flags (E505 `hog_facts`), the pass2 §2 verbatim labels (`cleanup_state_label` — the ONE vocabulary), and the AJ-E503 query-time verification verdict; `clean_instances` drives the copy-exact empty state; `ledger_degraded` renders the E501 banner.
- Shared row: `ui/src/components/ProcessRow.tsx` — icon+text state badge, hog badge, cmdline (truncated; full on expand), PID/PGID/age/RSS/CPU/ports, and the `Why does Juggler think this is mine?` expansion (root pid, first-seen parent, PGID contained/escaped, identity check, first/last seen). The Instance Processes tab embeds IT via the same bridge (`InstanceCleanupRows`) — no second data path.
- Centre: `ui/src/screens/CleanupCentre.tsx` — global-nav entry (top-right, ADR-0007; never a rail peer), the EXACT four filters (needs-attention default), healthy trees collapsed by default, `aria-live=polite` filter announcements, copy-exact `Nothing left behind — {N} Instances running clean.`, the degraded banner + read-only chip, the D-09 glossary footer, ⌘⇧C/⌘F shortcuts.
- Zero signalling (Req.7): the DTOs carry no kill fields by construction; the dogfood JSON is scanned for kill-ish keys (zero); the rendered tree is walked for stop/kill/terminate buttons (zero); `ProcessRow` imports no control primitives.
- Evidence: `docs/qa/evidence/AJ-E506/` — live+orphan API JSON (full attribution + evidence chain), 163/163 UI, 137/137 visuals with `_gallery/19-cleanup/` (24 captures, six states × sizes × themes).
- Known gap (documented): helpers whose root dies inside one tick interval before their first sighting are missed by the frozen tree (E510's journal recovery re-discovers them).

### AJ-E507 Verify-before-kill sheet and manual TERM→KILL ladder | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E507/manifest.md) | Priority P0 | Effort L | Depends: AJ-E503, AJ-E506

**Goal**:
Add the only manual kill path: a three-step staged, verified, audited TERM→grace→KILL ladder over a PGID or single PID, with per-process outcomes and port re-probe.

**User story**:
As a developer freeing :5173, I want Juggler to re-verify the exact process at click time, show me its full command line, cwd, ports and
PGID siblings, and then terminate with a visible grace countdown — never a silent one-click SIGKILL.

**Requirements**:
1. Killing SHALL be exactly three steps (pass2 §9.2): (1) Verify — re-run AJ-E503 identity checks NOW (not from last tick), re-read exe/cmdline/cwd/ports, show the confirmation sheet; (2) Choose scope — `This process only` / `This process and its helpers` (PGID) / `Whole Instance tree`, defaulting to the narrowest scope covering the stated goal (`free :5173` defaults to process-only); (3) Confirm — the button SHALL name the outcome, e.g. `Terminate vite (PID 8123) and free :5173`. There SHALL be no one-click kill anywhere, and no `Don't ask again` for kills.
2. If verification fails, the sheet SHALL become an explanation (`This PID now belongs to a different program — Juggler will not touch it.`) with the kill button REMOVED from the DOM, not disabled-with-tooltip. T5/Unverifiable and `Not managed` rows SHALL never open a kill sheet.
3. The ladder SHALL TERM the verified PGID (or single PID), show a live countdown with per-process exit ticks, and KILL only survivors after grace (default 5 s, matching `stop_session` grace at `commands.rs:733-743`). The user SHALL be able to cancel during grace. A helper ignoring TERM SHALL be labelled `Ignored SIGTERM` before any KILL is offered. On Windows the ladder SHALL be `TerminateProcess`-only and the sheet SHALL say so verbatim.
4. `terminate_group(pgid, grace) -> LadderOutcome` (pass2 §12) SHALL be the primitive; `Supervisor::stop` (`supervisor.rs:296-318`) SHALL delegate to it over the verified tree, fixing F-9 at the primitive. Post-ladder, held ports SHALL be re-probed; an escaped helper still holding the port SHALL be reported as still holding it (never claim success, FMEA C2), then offered PID-only kill after fresh re-verification.
5. The confirm sheet SHALL list every PGID member (FMEA C3: shared- PGID shell collateral warning) and any member whose role guess is `McpServer|TerminalShell` SHALL be named with exact copy: `Includes a {role} that may hold unsaved state.`
6. Outcomes SHALL be reported per process (`exited on TERM` / `needed KILL` / `already gone` / `refused`), `already gone` being success, and SHALL be written to the AJ-E509 audit record.

**UI/UX reasoning**:
Apple HIG (confirmation for irreversible actions) + Nielsen H5: consequence-named buttons, scope defaulting to narrowest, grace countdown makes the destructive window visible and cancellable.
Keyboard: `Esc` cancels at any step incl. during grace; `⌘⏎` confirms only on step 3. WCAG 2.2: countdown updates via
`aria-live=polite` at 1 s intervals (not per-frame), focus returns to the originating row on close (2.4.3).

**Performance budget**:
Verify-on-open ≤150 ms p95; ladder TERM→exit median ≤1 s for cooperative helpers; countdown UI at ≤4 updates/s; port re-probe
≤30 ms; sheet open→interactive ≤200 ms at 2,000-row inventory.

**Quality gates**:
Supervisor integration tests assert helper death (extending
`tests/supervisor.rs:153-173`, which today never asserts it); guardrail test: no Tauri command signals a process without a prior same-tick `verify()==Verified` and an audit intent row (AJ-E509); clippy/typecheck green.

**Evidence/validation proof**:
Fixtures (SPEC §10 harness extended, pass2 §14): vite-like
(port-holding), TERM-ignoring, and `setsid`-escaping helpers. Run all three through the sheet headlessly + one live dogfood each; assert
:5173 free after (1), `Ignored SIGTERM` before KILL in (2), honest still-holding report in (3). Evidence (incl. signal-trap logs) in
`docs/qa/evidence/E5/AJ-E507/`. Pass = 3/3 fixtures, 0 signals to unverified PIDs.

**Visual testing**:
Screenshots 1440×900 + 1024×700, light+dark: verify sheet (each scope), verification-failed explanation, grace countdown at 3 s, outcome report, Windows-wording state (mocked platform). Method: baseline pixel-diff ≤0.5% + axe-core 0 violations + keyboard-only walkthrough (open→cancel via `Esc`) recorded. Pass = all states captured, no kill button in failed-state DOM dump.

**Files to touch**:
`crates/juggler-supervisor/src/tree.rs` (`terminate_group`),
`crates/juggler-supervisor/src/supervisor.rs:290-318`,
`crates/juggler-app/src/commands.rs:733-743` (`stop_session` delegates),
`ui/src/components/KillSheet.tsx` (new),
`ui/src/components/ProcessRow.tsx`, `ui/src/screens/CleanupCentre.tsx`,
`crates/juggler-supervisor/tests/supervisor.rs:153-173`

**Research basis**: pass2/05 §9.2-9.3 (lines 358-394), §12 (lines 462-505), §13 C2/C3
(lines 512-513); deep/08 §5 F-9 (`supervisor.rs:149-151,234-236`).

**Handover note**:
Trap: extending the root-only ladder in `stop()` without the PGID primitive re-creates the fork-escape race (AJ-E502 note). Trap:
`already gone` is success — do not surface it as an error toast or users will stop trusting outcome reports. Next: AJ-E508 tiers may only call this ladder, never signal directly.


**Implementation-mapping (AJ-E507)**:
- `tree.rs`: `terminate_group` (TERM → cancellable grace → KILL survivors → 2 s wait), zombie-aware `member_gone` (an unreaped direct child answers `kill -0` as alive — the desktop suite's self-group bug and the ignorer false-`refused` were both this class), `ProcessOutcome` incl. `TermSent` (cancel leaves the E509 intent PENDING — "TERM'd, outcome unknown"), `pgid_members`, `live_details` (re-read-NOW cmdline/cwd), `verify_start_time` (E504's exec-aliasing rule formalized for roots).
- `supervisor.rs`: `stop()` delegates — the ROOT keeps the exact reaped-flag path; the group's OTHER members run the ladder, and ONLY when the child leads its OWN group (`pgid == pid`, the E502 deterministic-kill group): a PTY/desktop child that never became a leader shares a FOREIGN group and must never be group-signalled (the desktop suite caught the original version TERm'ing the test process itself). `tests/supervisor.rs` now asserts group death (F-9's missing assertion) + the ladder vocabulary (4 tests).
- `commands.rs`: `kill_verify` (click-time E503 verify + exe/cmdline/cwd/ports + PGID collateral + role guesses, pass2 §5), `kill_execute` (shared `verify_kill_target` gate → `kill_intent_record` → audit intent → ladder → honest still-holding re-probe → audit completion; goal-port aware), `kill_cancel` (the AppState cancel registry), `kill_liveness` (grace-window per-process ticks). Guardrails proven: NO AUDIT ⇒ NO SIGNAL (FailingAudit seam, trap-log witness empty) and stale identity ⇒ refusal copy + zero signals + NO audit row.
- UI: `KillSheet.tsx` (the 3 steps, consequence-named confirm, aria-live 1 s countdown, per-process ticks, cancel, `TerminateProcess only` verbatim, re-offer); `ProcessRow` killable gate (verified+live only — Unverifiable/exited render NO affordance); `CleanupCentre` wiring + focus return (2.4.3); bridge + fixture seams.
- Fixtures (SPEC §10): vite-like `python3 -m http.server` (ephemeral port, collision-proof) freed on TERM with audit row; TERM-ignorer escalated with trap-log witness `term`; `setsid` escapee honestly still-holding → pid-only re-offer frees it. Evidence: `docs/qa/evidence/AJ-E507/` (kill-tests, stop-delegation, ui-tests, manifest) + `_gallery/20-kill/` (32 captures) + axe gate (2 contrast fixes).
- Test premises honestly updated to E507 semantics (group-killing `stop()`): reconciler + cleanup-dogfood + ports now kill the root by EXTERNAL SIGKILL (the crash shape) to preserve their orphan-detection subjects.

### AJ-E508 Cleanup tiers Tier 0-3: notify-only default, per-Account opt-in | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E508/manifest.md) | Priority P0 | Effort M | Depends: AJ-E507, AJ-E509

**Goal**:
Implement D-08 exactly: Tier 0 observe-only by default, Tier 1/2 auto-tidy only for Accounts the user explicitly opted in, Tier 3 never automatic, with a global pause kill-switch and ADR-0010.

**User story**:
As a developer, I want Juggler to only ever alert me by default, and
— for Accounts where I explicitly opted in — to quietly tidy verified dev-server orphans after clean exits, while never auto-killing after a crash, a shell, an MCP server, or for resource use alone.

**Requirements**:
1. The system SHALL implement tiers per pass2 §10 as amended by D-08: Tier 0 Observe (default ON: detect/list/alert/pre-stage, never signals); Tier 1 Tidy verified orphans (opt-in **per Account**): after `orphan_grace` + same-tick re-verification, TERM→KILL via AJ-E507 ladder only helpers whose root exited cleanly (code 0) and whose role is `DevServer|Watcher`; Tier 2 Tidy after crash (separate per-Account opt-in): as Tier 1 but crash-exited roots, after 5 min AND one user-visible notification with an Undo window (≥30 s) before signalling; Tier 3 Hog shed SHALL have no automatic implementation — alert + staged sheet only.
2. Hard exclusions at every tier SHALL include: roots, `McpServer`, `TerminalShell`, any DB-role guess, T3 `Adopted` survivors, T5 `Unverifiable`, escaped-group helpers, and any process on a non-opted-in Account or a different `machine_id` (AJ-E511).
3. Every automatic action SHALL write the AJ-E509 audit intent record BEFORE signalling, SHALL re-verify identity in the same tick as the signal, and on any verification failure SHALL silently demote the candidate to Tier 0 (list-only) with an audit note.
4. A global `Pause automatic cleanup` kill-switch SHALL exist in Settings AND in the Cleanup Centre header; pausing SHALL take effect within one tick and SHALL be visible as a persistent banner `Automatic cleanup is paused.` while active. Tier settings SHALL live per Account (launch-settings area, D-10 wording) with exact labels `Automatically tidy left-behind dev servers` (Tier 1) and `Also tidy after a crash (with notification)` (Tier 2), both off by default, never bundled into a single "auto-cleanup" toggle.
5. Tiers 1-2 SHALL be suspended for 24 h after any Juggler crash recovery (AJ-E510) until the user re-confirms in the Cleanup Centre; the suspension SHALL be shown, not silent.
6. This task SHALL deliver ADR-0010 (cleanup tier model + the narrow written amendment to the SPEC §5/`docs/SPEC.md:26` passive- automation invariant, per D-08) and the SPEC tier-wording change; no tier code SHALL merge before ADR-0010 is accepted.

**UI/UX reasoning**:
Nielsen H3/H4 + Apple HIG (opt-in for consequential automation): defaults are the product's promise, so both toggles start off, are separately labelled by consequence, and the Tier-2 notification names the exact process and offers `Undo`/`Keep running`. NN/g: the Undo window, not a settings page, is the real consent surface at crash time. Shortcut: none for enabling tiers (deliberately mouse-only,
Settings-deep-link from Cleanup Centre header).

**Performance budget**:
Tier evaluation ≤5 ms per candidate per tick; Tier-2 notification
≤1 per crash-orphan (no repeats); pause propagation ≤2 s (one tick); zero signals when paused/suspended (asserted, see Evidence).

**Quality gates**:
Guardrail-style test alongside `docs/SPEC.md:26` invariant: default install (Tier 0 everywhere) signals nothing across the full fixture suite; per-Account isolation test (opt-in on Account A never cleans
Account B's orphan); ADR-0010 present and linked from SPEC §5.

**Evidence/validation proof**:
Tier-1 drill (pass2 §14.5): clean-exit tree with one dev-server and one MCP-role helper, Tier 1 on → dev-server auto-TERMed after grace with pre-written audit record, MCP helper untouched; rerun with pause ON → zero signals; rerun opted-out → zero signals. Logs + audit rows in `docs/qa/evidence/E5/AJ-E508/`. Pass = exact signal sets in all three runs, 0 false kills.

**Visual testing**:
Screenshots 1440×900, light+dark: Settings per-Account tier toggles
(off/on), Cleanup Centre paused banner, suspended-after-crash banner, Tier-2 notification with Undo (at appearance and at 5 s remaining). Pass = axe 0 violations, toggle labels pass WCAG 4.5:1, pixel-diff vs baselines ≤0.5%.

**Files to touch**:
`docs/adr/0010-cleanup-tiers.md` (new), `docs/SPEC.md:26,216-252`,
`crates/juggler-supervisor/src/tree.rs` (tier evaluator),
`crates/juggler-core/src/storage/cleanup_settings.rs` (new, keyed by
`account_id`), `crates/juggler-app/src/commands.rs`,
`ui/src/screens/CleanupCentre.tsx`, `ui/src/screens/WorkPanels.tsx`
(Account launch-settings), `crates/juggler-app/tests/guardrails.rs`
(or nearest equivalent — confirm path at implementation)

**Research basis**: decision-log D-08:79-86 (overrides pass2 per-Provider wording); pass2/05 §10 (lines 396-422); SPEC automation invariant
`docs/SPEC.md:26`; pass2 §1.2 P9 gated note in pass2/00:111.

**Handover note**:
Trap: pass2 §10 says "opt-in per Provider" — D-08 settled per
Account; implement per Account or you fail the decision log. Trap: role is a GUESS (C4) — that is why only DevServer/Watcher are ever auto-eligible and MCP/shell/DB are hard-excluded. Never widen the eligible role set without a new decision-log entry.

### AJ-E509 Append-only cleanup audit log (intent before signal) | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E509/manifest.md) | Priority P0 | Effort M | Depends: AJ-E501

**Goal**:
Make every kill — manual or automatic — accountable: an append-only audit record written before the first signal and completed after, exportable, and the data source for Cleanup Centre History.

**User story**:
As a developer, I want to answer "what did Juggler kill, when, why, and what happened?" for the last 7 days, especially after an automatic tidy I did not watch happen.

**Requirements**:
1. The system SHALL add a `cleanup_audit` append-only table (SQLite, plus JSONL export from Settings) with: timestamp, actor (`user:{command}` | `auto:tier-1` | `auto:tier-2`), candidate PID/PGID/exe/cmdline-hash, `session_id,provider,account_id, project_id,machine_id`, identity-check results, scope chosen, signals sent with per-PID outcomes, ports before/after (pass2 §11.1).
2. The audit intent record SHALL be written BEFORE the first signal and completed with outcomes after the ladder; a crash mid-ladder SHALL leave a truthful `TERM'd PGID {n}, outcome unknown` record. Command lines SHALL be stored hashed + truncated only — full cmdline SHALL NOT persist beyond the confirm sheet.
3. No audit, no kill: if the intent write fails, manual kills SHALL warn-and-require-retry (kill does not proceed silently) and automatic tiers SHALL stand down for that candidate (AJ-E508 Requirement 3). This SHALL be a typed error surfaced to the UI, never a swallowed `let _ =`.
4. The Cleanup Centre `History (7 days)` filter (AJ-E506) SHALL read this log; retention SHALL match ledger retention (7 days, running processes' records never pruned). Export SHALL be user-initiated only and SHALL include a redaction note that cmdlines are hashed.
5. Audit writes SHALL be atomic with the existing WAL store and SHALL NOT be deletable from any UI surface in this epic.

**UI/UX reasoning**:
History rows mirror the outcome vocabulary of AJ-E507 (`exited on
TERM` etc.) so users learn one language (Nielsen H4 consistency).
Settings export follows Apple HIG file-export patterns (standard save sheet, default filename `juggler-cleanup-audit-{date}.jsonl`).
Empty history copy: `No cleanups yet — automatic cleanup is off by default.` (reinforces D-08 at the moment of curiosity).

**Performance budget**:
Intent write ≤5 ms p95 (must not delay the ladder perceptibly); history query ≤50 ms at 10k rows (indexed by timestamp/account); export of 10k rows ≤500 ms streaming, ≤20 MB file.

**Quality gates**:
Fault-injection test: audit write forced to fail → manual kill refused with typed error, auto tier stands down, zero signals; crash-mid-ladder test (kill the app between TERM and KILL) → intent record present with `outcome unknown`; append-only enforced by a storage-layer test (UPDATE/DELETE helpers do not exist).

**Evidence/validation proof**:
Run AJ-E507 vite fixture + AJ-E508 Tier-1 drill, then dump
`cleanup_audit` rows + exported JSONL and archive in
`docs/qa/evidence/E5/AJ-E509/`. Pass = one intent+outcome pair per signal, ports before/after differ as expected, exported file parses as JSONL 100% lines valid.

**Visual testing**:
Screenshots 1440×900, light+dark: History filter populated (manual + auto actors distinguishable by icon+text, WCAG 1.4.1), empty-history state, export save sheet. Pass = axe 0 violations, baseline pixel- diff ≤0.5%.

**Files to touch**:
`crates/juggler-core/src/storage/mod.rs` (migration),
`crates/juggler-core/src/storage/cleanup_audit.rs` (new),
`crates/juggler-supervisor/src/tree.rs` (intent/outcome hooks),
`crates/juggler-app/src/commands.rs` (history + export commands),
`ui/src/screens/CleanupCentre.tsx`, `ui/src/screens/WorkPanels.tsx`
(Settings export, if Settings lives there — confirm at build)

**Research basis**: pass2/05 §11.1 (lines 426-437), §13 C7 (line 517); decision-log D-08:83 ("every kill audit-logged").

**Handover note**:
Trap: AJ-E507/E508 depend on this contract even though they are listed around it — land the table + intent API before enabling any tier, or retrofit will tempt someone into post-hoc logging. Trap: cmdline may contain tokens in args; hash+truncate is the privacy boundary, do not "temporarily" store full cmdlines for debugging.

*Implementation mapping (recorded in this task's PR per the claiming protocol)*:
- Schema v14: `cleanup_audit` (timestamp, actor, pid/pgid/exe/cmdline-HASH+trunc, attribution, identity results, scope, outcome, ports before/after) + `at` and `account,at` indexes — landed BEFORE any tier exists (the handover trap).
- Repo (`cleanup_audit.rs`): APPEND-ONLY surface — `intent` (pending, BEFORE the signal), `complete` (the only post-insert mutation, guarded `WHERE outcome=pending`), `history` (7-day window, newest first; pending reads as `TERM'd, outcome unknown`), `rows_since` (export), `prune` (7-day); its OWN WAL connection (intent ≤5 ms must not queue).
- NO AUDIT, NO KILL: the `AuditWriter` seam — a failed intent returns the typed `AppError::AuditUnavailable` and the manual kill REFUSES (fault-injection test: trap log empty, process alive, zero signals; recovery with the real repo completes `gone`). The existing `orphan_stop` is now audited end to end.
- History + export: `cleanup_inventory("history")` reads the AUDIT (the E506 ledger-exits mapping retired in the same change — one History, one source); copy-exact `No cleanups yet — automatic cleanup is off by default.`; `cleanup_audit_export` = JSONL with the redaction note as a valid first line, user-initiated download `juggler-cleanup-audit-{date}.jsonl`.
- Evidence: `docs/qa/evidence/AJ-E509/` — audit/history/export dumps + the fault-injection proof; 165/165 UI; 141/141 visuals incl. 4 new history captures.


**Implementation-mapping (AJ-E508)**:
- ADR-0010's E508 amendment + SPEC §5.2 (Tier 2/3, exclusions, demotion, pause, 24 h suspension) landed with the code, per Req.6.
- Storage: `cleanup_settings` (v15; per-Account `tier1_opt_in`/`tier2_opt_in`, TWO toggles, both off; the E410 prototype flag seeds Tier 1 only). Evaluator: `tree.rs::evaluate_tier` — pure; eligible = `DevServer|Watcher` helpers ONLY (role gates read the ledger's TRUE command line — `ProcessRow.cmd` now rides the tick's existing refresh; helper rows gain a ≤10 s exec-settle refinement window, then identity freezes).
- Sweep (`auto_cleanup_sweep`): candidates from the LEDGER's orphan verdict (the E410 tracker DROPS clean exits by design and cannot serve Tier 1); gates = pause → 24 h suspension → evaluator → pacing (60 s auto-grace; Tier 2: 5 min + ONE notice + ≥30 s Undo) → same-tick re-verify (failure ⇒ DEMOTE to Tier 0 with an audit note) → the E507 ladder ONLY (`kill_execute_as`, actors `auto:tier-1|2`).
- Commands/UI: `cleanup_status/pause_set/tier2_decide/resume_after_crash/tiers_get/tiers_set`; `TierToggles` in the account panel (exact labels), the Centre's pause button + `Automatic cleanup is paused.` banner + suspended banner + Re-confirm, Tier-2 notice cards with aria-live countdown and Undo/Keep.
- Tests/evidence: `tests/tiers.rs` 8/8 (default-guardrail, drill with MCP untouched, pause, per-Account isolation, tier2 undo+lapse, demote, suspension, toggle persistence) + evaluator matrix; UI 180/180; `21-tiers` gallery (12) + 2 axe gates (one contrast fix); `docs/qa/evidence/AJ-E508/`.

### AJ-E510 Juggler-crash recovery: re-adoption with stop control | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E510/manifest.md) | Priority P0 | Effort L | Depends: AJ-E501, AJ-E503, AJ-E504

**Goal**: On relaunch after a Juggler crash, re-adopt surviving roots WITH a working verified stop path, surface helpers orphaned while Juggler was down, and tell the user once what happened.

**User story**: As a developer whose Juggler crashed overnight, I want survivors re-adopted (not killed), down-period leftovers listed separately, and one honest summary — silence here is how trust dies.

**Requirements**:
1. On startup the system SHALL load ledger rows in `Running|Orphaned|Adopted`, identity-verify each (AJ-E503); failures SHALL become `Unverifiable` (listed, never killed). It SHALL rebuild adopted handles (output unavailable, stop = verified PGID ladder via AJ-E507), closing F-3 (`alive_adopted_sessions` `supervisor.rs:432-438` returns ids only today; `stop` returns `UnknownSession` `supervisor.rs:296-303`), and SHALL reconcile with `state.rs:53-72` session reconciliation without marking adopted roots `Stopped`.
2. Helpers of roots that died while Juggler was down SHALL enter `Orphaned` with original first-seen attribution, labelled distinctly (`left behind while Juggler was closed`).
3. The system SHALL show ONE recovery summary with exact template: `{a} Instances re-adopted, {b} helpers left behind, {c} processes could not be verified and were left alone.` (shown once, dismissible, logged to audit as `actor=system:recovery`); auto-tiers SHALL stay suspended per AJ-E508 Requirement 5 until re-confirmed.
4. Adopted roots SHALL expose in UI: `Running — re-adopted`, terminal detached notice, and stop via the AJ-E507 sheet only.

**UI/UX reasoning**: Nielsen H1 + NN/g trust: one summary, not per-process toasts; `Re-adopted` filter in Cleanup Centre. WCAG: summary dialog focus-trapped, `Esc` dismisses, focus returns to board. No shortcut to dismiss permanently.

**Performance budget**: Recovery scan ≤1 s at 500 ledger rows, ≤3 s at 5,000; startup board interactive ≤500 ms after summary data ready (SPEC startup <1.5 s); verification batched, no per-row UI block.

**Quality gates**: Crash drill test (below) green on macOS+Linux; adopted `stop` no longer returns `UnknownSession` for verified adopted roots; unverified adopted roots return typed `Unverifiable`.

**Evidence/validation proof**: Extend pattern of `crates/juggler-supervisor/tests/supervisor.rs:118-151` (today lone `sleep 300`, no children): spawn tree with port helper, drop supervisor without stop, restart → assert roots re-adopted with stop control, down-period helpers listed as left-behind, summary counts exact (2/1/1 fixture incl. one forged-identity row). Archive in `docs/qa/evidence/E5/AJ-E510/`. Pass = counts exact, stop kills verified tree, forged row untouched.

**Visual testing**: Screenshots 1440×900 light+dark: recovery summary dialog, Re-adopted filter, adopted Instance panel (detached notice). Pass = axe 0 violations, pixel-diff ≤0.5% vs baselines, summary shown exactly once on second relaunch (no re-show).

**Files to touch**: `crates/juggler-supervisor/src/journal.rs:151-196`, `crates/juggler-supervisor/src/supervisor.rs:432-438`, `crates/juggler-app/src/state.rs:53-72`, `crates/juggler-app/src/commands.rs`, `ui/src/screens/CleanupCentre.tsx`, `ui/src/App.tsx`

**Research basis**: pass2/05 §11.2 (lines 439-460), §3 F-1/F-3; SPEC §5.3 `docs/SPEC.md:251`; deep/08 §5 F-1/F-3.

**Handover note**: Trap: T3 survivors are BY DESIGN (SPEC §5.3) — never route them through orphan tiers. Trap: identity data is least trustworthy right after a crash; that is why tier suspension is mandatory, not a nicety.

*Implementation mapping (recorded in this task's PR per the claiming protocol)*:
- Recovery pass (`supervisor.rs::recover` + `state.rs::run_startup_recovery`): candidate SESSIONS from the ledger's live-row scan, then each session's FULL rows (a root that died while Juggler was down is already `exited` — detached watcher threads keep exit-patching the durable ledger — so the live-row scan alone would drop its left-behind helpers). Roots verify by boot+start-time ONLY (the E504 aliasing rule — exe legitimately moves across `exec`); helpers verify the full triple. Verdicts: live+verified root → `register_adopted` (ChildHandle with no ring readers, no PTY, `force_kill` = verified PGID ladder — F-3 CLOSED, `stop` no longer `UnknownSession`); alive-but-unverifiable → `Unverifiable` (listed, never killed); dead → `exited (unknown)`.
- Down-period helpers (Req.2): schema v16 `process_ledger.down_period` (durable across any number of further restarts; `mark_orphaned_down` on the `LedgerStore` trait writes it) — distinct from grace-based orphans; original first-seen attribution preserved; label override `left behind while Juggler was closed` in `cleanup_inventory`.
- Summary (Req.3): exact template persisted to `app_settings` (`recovery_summary` + `recovery_summary_consumed`), read by `recovery_summary_get` (None once consumed — shown exactly once), dismissed only via `recovery_summary_dismiss`; audited as `actor=system:recovery`; ANY finding suspends Tiers 1-2 for 24 h (E508 Req.5).
- Reconciliation: adopted-marker sessions (`alive_adopted_sessions` = the `adopted` map, the old journal-minus-supervised heuristic retired) stay Running; non-adopted running rows reconcile to Stopped as before.
- UI: `InstanceInfo.adopted` → `↻ re-adopted` chip + `Running — re-adopted after restart` label; `RecoverySummaryDialog` (focus-on-open, Esc/Got-it dismiss, focus returns to the board, exact text — same string asserted in the Rust drill); Processes tab leads with the detached-terminal notice; Cleanup Centre `Re-adopted` filter + left-behind label; adopted rows killable through the E507 sheet only.
- Tests/evidence: `tests/recovery.rs` crash drill (2/1/1 exact incl. forged-identity row, stop kills the verified tree, B stays adopted, forged row untouched, audit row, suspension, dismiss-once) in `docs/qa/evidence/AJ-E510/recovery-drill.txt`; UI 190/190; `22-recovery` gallery (6 captures) + axe gates; one contrast fix (dialog h2 `text-ink`).

### AJ-E511 Fleet-shaped ownership: machine identity on every process record | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E511/manifest.md) | Priority P1 | Effort M | Depends: AJ-E501

**Goal**: Make every ownership, audit, and cleanup decision machine-scoped from day one (D-13), so fleet control later never has to guess which machine a PID belongs to.

**User story**: As a developer with two machines, I want cleanup on this Mac to never signal — or even offer to signal — a process owned by my other machine, and I want every process row to name its machine.

**Requirements**:
1. Every ledger row, audit row, inventory DTO, and kill-sheet payload SHALL carry `machine_id: MachineId` (`model.rs:63-66,398`); the local value SHALL come from one persisted local-machine identity (generated once, stored in the app state dir), NOT the hard-coded `"this-mac"` at `commands.rs:699,1175` — those call sites SHALL migrate to the shared identity provider in this task.
2. `verify()` and `terminate_group` SHALL refuse (typed `WrongMachine`) any candidate whose `machine_id` != the executing node's machine id; auto-tiers SHALL skip foreign-machine rows silently (audit note only). No cross-machine signal path SHALL exist in this epic (fleet transport is `juggler-link`, SPEC §11, out of scope here).
3. Cleanup Centre SHALL show a Machine column and a machine filter (`This machine` default); foreign rows (future sync) SHALL render display-only with no kill sheet. Session/project attribution SHALL remain joinable across machines via `session_id+machine_id` composite key in all new queries.
4. A Fleet architecture note/ADR stub (D-13/D-15 docs scope) SHALL record: identity, sync/transport, remote-ownership, offline, and security open questions as beta-exit criteria, without implementing transport.

**UI/UX reasoning**: Apple HIG: machine as a quiet column/filter, defaulting to This machine so local users see no fleet complexity. Nielsen H2: name machines by user label (`This Mac`, editable label) with id only in evidence expansion. WCAG: filter is a labelled combobox, keyboard reachable.

**Performance budget**: Machine filter adds ≤5 ms to inventory query (indexed); identity load ≤10 ms at startup; zero network calls in this task.

**Quality gates**: Tests: forged foreign-machine candidate → `WrongMachine`, zero signals; composite-key query test with same `session_id` on two machines returns disjoint rows; grep shows no new `"this-mac"` literals.

**Evidence/validation proof**: Unit + integration tests in `crates/juggler-supervisor/tests/fleet_identity.rs` (new) and storage tests; dump two-machine fixture inventory JSON in `docs/qa/evidence/E5/AJ-E511/`. Pass = foreign candidate refused, local candidate unaffected, `"this-mac"` grep clean in touched files.

**Visual testing**: Screenshots 1440×900 light+dark: Machine column + `This machine` filter, foreign-row display-only state (fixture). Pass = no kill control on foreign row in DOM dump, axe 0 violations.

**Files to touch**: `crates/juggler-core/src/model.rs:63-66,389-416`, `crates/juggler-core/src/storage/process_ledger.rs`, `crates/juggler-app/src/state.rs:91-97`, `crates/juggler-app/src/commands.rs:699,1175`, `crates/juggler-supervisor/src/tree.rs`, `ui/src/components/ProcessRow.tsx`, `docs/adr/0013-fleet-architecture.md` (new stub; number to confirm vs ADR-0012 at filing)

**Research basis**: decision-log D-13:126-137; SPEC §11 `docs/SPEC.md:362-393`; deep/08 §2.3 machine column critique (`model.rs:398` bare TEXT, no FK).

**Handover note**: Trap: PID/PGID are only unique per machine+boot — any future sync that merges rows without `machine_id+boot_id` re-creates the false-kill bug at fleet scale. Trap: do not build transport here; shaping the data is the whole task.

*Implementation mapping (recorded in this task's PR per the claiming protocol)*:
- Storage: v17 index `process_ledger(session_id, machine_id)`; repo `rows_for_session_on_machine` + `live_session_machines` (the composite-key reads); both on the `LedgerStore` trait.
- Identity: the persisted `local_machine_id` (state.rs) now feeds every session creation (the three hard-coded `"this-mac"` launch/handover sites migrated; the remaining literal is a TEST fixture).
- Kill paths: `SupervisorError::WrongMachine { recorded, local }` (typed) + `KILL_WRONG_MACHINE_COPY`; guardrail-0 in `kill_execute_as` (before identity — a foreign row must not read as PID-reuse) and the typed early-refusal in `kill_verify_inner`; `verify_kill_target` machine-gates; the sweep stays machine-scoped at candidate read.
- The composite leak the new test CAUGHT: `local_ledger_rows` read full session rows — a foreign row of a shared session leaked into the local scope; now composite-keyed (exactly the trap the handover note warns about).
- Inventory: `CleanupRow.machine_id` + `CleanupInventoryInfo.local_machine`; groups keyed `(session, machine)`; `cleanup_inventory(filter, machine_scope)` — "This machine" default, "all" widens via the composite walk.
- UI: quiet Machine cell (`This Mac` / the machine id), labelled `Machine filter` combobox (default This machine), foreign rows display-only (no kill affordance in the DOM); the kill sheet names the target's machine; the `__AIJ_CLEANUP__` seam mirrors the backend scope.
- ADR-0013: the five open-questions areas (identity, sync/transport, remote ownership, offline, security) recorded as beta-exit criteria — no transport built.
- Tests: `tests/fleet_identity.rs` 3/3 (composite-key disjoint rows on two machines, machine-scoped candidates, typed refusal naming both machines) + the guardrails foreign end-to-end (refused, zero signals, no audit row, sweep-invisible, listed display-only, local tree unaffected).

### AJ-E512 False-kill collateral fixtures, hog alerts, and gate evidence | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E512/manifest.md) | Priority P0 | Effort M | Depends: AJ-E507, AJ-E508, AJ-E510

**Goal**: Prove, with adversarial fixtures, that the epic's success metric — zero false cleanup kills (D-01) — holds, and ship the passive hog-alert surface that must never become a kill trigger.

**User story**: As a developer, I want evidence, not promises, that Juggler will not kill my own dev server, my shell, an MCP server with unsaved state, or a stranger on a recycled PID — even when I have opted into automatic tidy.

**Requirements**:
1. The fixture harness (SPEC §10, `docs/SPEC.md:439`, extended per pass2 §14) SHALL provide scriptable fake helpers: vite-like port-holder, TERM-ignorer, `setsid`-escapee, MCP-role holder, shared-PGID shell sibling, unmanaged user server, and PID-reuse stranger (forged identity). Each fixture SHALL trap and log every signal received to a per-fixture file.
2. A collateral suite SHALL run every kill path (manual ladder scopes, Tier 1, Tier 2, crash recovery) against all fixtures and SHALL assert: unmanaged/shell/MCP/Unverifiable/foreign fixtures receive ZERO signals; only the intended verified DevServer/Watcher orphans receive TERM (and KILL only after grace if TERM-ignoring). Any deviation SHALL fail CI as P0.
3. Hog alerts (AJ-E505 flags) SHALL surface as passive Cleanup Centre badges + one staged sheet pre-fill (SPEC `docs/SPEC.md:26`: app prepares, user executes) with exact copy: `{helper} under {Instance} holds {RSS} — review in Cleanup Centre.`; a hog SHALL never be auto-signalled at any tier (Tier 3, AJ-E508), and idle-orphan nudges SHALL fire at most once per orphan after 1 h idle (CPU≈0 + age only).
4. The epic SHALL close with a gate report `docs/qa/gate-E5.md` collecting AJ-E501-E511 evidence links, the collateral-suite result, reconciler CPU at 50 Instances (<1%), and the live-dogfood record (real Claude/Codex Instance → real helper → verified cleanup) — no gate, no "done" claim.

**UI/UX reasoning**: NN/g alert design: hog alerts name the top helper, not just the Instance (`node (vite) under Instance X holds 3.1 GiB` is actionable), severity low, once-only for idle nudges to prevent nag fatigue. WCAG: alerts are status messages (`role=status`), not focus-stealing dialogs.

**Performance budget**: Collateral suite ≤5 min in CI; hog evaluation on existing tick (≤2 ms extra); gate soak: 50 Instances, supervisor CPU <1%, Cleanup Centre first paint ≤300 ms (AJ-E506 budget re-measured, not assumed).

**Quality gates**: Collateral suite is a required CI job (macOS+Linux); zero-signal assertions use fixture trap logs, not UI state; clippy/typecheck/component tests all green; `docs/qa/gate-E5.md` reviewed before epic close.

**Evidence/validation proof**: Run `cargo test -p juggler-supervisor --test collateral` (new) + full fixture matrix from pass2 §14 items 1-6; archive trap logs, audit dumps, CPU soak log, and dogfood transcript in `docs/qa/evidence/E5/AJ-E512/`. Pass threshold: 0 false kills across ALL fixtures and tiers, 100% intended orphans cleaned, :5173 verifiably free after intended kills.

**Visual testing**: Screenshots 1440×900 + 1024×700 light+dark: hog badge + alert, idle-orphan nudge, gate-day Cleanup Centre with mixed fixture states. Method: baseline pixel-diff ≤0.5%, axe-core 0 violations, plus a recorded keyboard-only pass (open Centre → review hog → stage sheet → `Esc`). Pass = all captures archived and a11y clean.

**Files to touch**: `crates/juggler-supervisor/tests/collateral.rs` (new), `crates/juggler-supervisor/tests/fixtures/` (new fake-helper scripts), `crates/juggler-app/tests/guardrails.rs`, `ui/src/screens/CleanupCentre.tsx` (hog badge/alert), `docs/qa/gate-E5.md` (new)

**Research basis**: pass2/05 §13 FMEA C1-C8 (lines 509-518), §14 acceptance 1-6 (lines 520-545), §8 (lines 306-330); decision-log D-01:17-19 ("zero false cleanup kills"); SPEC §10 `docs/SPEC.md:439`.

**Handover note**: Trap: a suite that only tests intended kills proves nothing — the value is the zero-signal assertions on collateral fixtures; never weaken a trap-log assertion to make CI pass. If a fixture is flaky, fix the fixture synchronisation (wait on trap-file readiness), not the assertion.

*Implementation mapping (recorded in this task's PR per the claiming protocol)*:
- Fixture harness `tests/fixtures/`: ONE python helper (`trap_helper.py`) with modes (port-holder / TERM-ignorer / setsid-escapee / MCP argv words / default sleeper) + a sh sibling line; every signal logged to a per-fixture file BEFORE acting; ready-file handshakes everywhere (the handover-note law). `spawn_detached` wraps standalone spawns in an exiting `sh` (clippy's zombie-process law); fixtures self-clean after 5 min.
- Suite split along the crate seam the task text draws: `juggler-supervisor/tests/collateral.rs` = the ladder's signal GEOMETRY (stop() own-group guard, post-grace KILL, escapee/neighbour/unmanaged zero-signal, port freed); `juggler-app/tests/guardrails.rs` = every kill path (Tier 1, Tier 2, manual pid/pgid, crash recovery, stranger refusal, TERM-ignorer drill) × the full fixture matrix — all zero-signal assertions on trap logs. Serialized via a static mutex: real-process tests flake under parallel sysinfo contention (a harness artefact — rows flip `unverifiable` under starvation — never a product verdict); ~60 s total.
- The E507-law exception is asserted, not assumed: manual pgid/tree scope TERMs the CONFIRMED group only after the sheet NAMES every shell/MCP member (`Includes a {role} that may hold unsaved state.`); everything outside the confirmed group stays at zero signals. That is the one path where shell/MCP take a TERM — user-confirmed, warned, audited.
- Hog law (Req.3): the sweep stands down for hog pids AND hog-flagged trees (hog facts computed on the REAL clock — the sweep's schedule clock, often fake-future in tests, empties the 60 s ring window); alerts carry the exact template naming the TOP-RSS helper (`Python under acc-guard holds 11 MiB — review in Cleanup Centre.`); thresholds settings-backed for tests (`hog_helper_rss_limit_mb`, `hog_cpu_pct`, `idle_nudge_min_age_ms`); idle nudges once-per-orphan via persisted flags; the ONLY kill path from an alert is staging the E507 sheet.
- UI: the Centre's alerts region is `role=status`/`aria-live=polite` (never a dialog); `Stage Terminate…` re-uses the row's verified sheet path; the recorded keyboard-only pass found and fixed a real gap (KillSheet now focuses its root from mount — Esc works during the verify phase).
- Gate `docs/qa/gate-E5.md`: E501-E510+E512 evidence table (E511 TODO marked), collateral results, soak `self cpu 0.03%` at 50 Instances on a loaded box, the real-claude dogfood record, two honest open items (authenticated-helper dogfood = gate-M1's standing re-login; E511). CI: `Collateral suite (AJ-E512)` step added to build-macos; Linux rides `verify`.

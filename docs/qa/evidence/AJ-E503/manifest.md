# Evidence manifest — AJ-E503 (ProcessIdentity and fail-closed verify-before-kill)

Task: `docs/tasks/E5-process-ownership-and-cleanup.md` AJ-E503.
Branch: `stage2/e5-identity`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Rust suites run in CI (verify job); the
drill runs everywhere (no GUI, no grants).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 `ProcessIdentity` + `verify(pid, expect)` in `tree.rs` | `cargo test -p juggler-supervisor --lib -- verify_tests` → `verify-unit-tests.txt` | struct `{pid, pgid, start_time, exe, boot_id}`; free `verify` fn (pass2 §12) | `tree.rs`: `ProcessIdentity`, `Verification {Verified, Unverifiable}`, `pub fn verify` (real OS probe) + `pub fn verify_with` (injectable `IdentityProbe` seam — tests forge) + `verify_any` (adoption replay helper); `OsIdentityProbe` = one targeted sysinfo refresh (`ProcessRefreshKind::nothing().with_exe(Always)` — the §6.2 `stat`/`proc_pidinfo` cost) + boot clock | PASS | 2026-10-13 | zcode-primary |
| Req.2 ALL three facts; mismatch or probe error → `Unverifiable` | `verify-unit-tests.txt` (6/6) | each single-field mismatch → `Unverifiable`; probe error → `Unverifiable`; zero `Verified` on error paths | `start_time_mismatch_is_unverifiable`, `exe_mismatch_is_unverifiable`, `boot_id_mismatch_is_unverifiable`, `probe_error_is_unverifiable_never_verified` (forged `FakeProbe`, `None` = error), `all_three_facts_agree_verifies` (the ONLY path to `Verified`), `empty_or_partial_identity_sets_do_not_verify` (no record = refusal); boot gate runs FIRST (FMEA C8 — a reboot invalidates start-time comparisons as a class); a missing live exe is a probe error, never a pass | PASS | 2026-10-13 | zcode-primary |
| Req.3 typed errors, never `UnknownSession` | code review (`error.rs`) | `SupervisorError::Unverifiable { pid }` + `::LedgerDegraded` in the closed enum | both variants added with distinct `Display` strings; `#[non_exhaustive]` preserved; callers that surface supervisor errors string-format them (no caller maps a refusal into `UnknownSession` — the refusal is its own typed outcome for E507/E508's kill paths) | PASS | 2026-10-13 | zcode-primary |
| Req.4 adoption uses `verify()` for roots | `cargo test -p juggler-supervisor --test supervisor children_survive` → `adoption-restart-test.txt` | adoption = liveness AND identity | `Journal::adopt_orphans` replays `spawned` (+ its identity) AND the new `identity_settled` entries, then `verify_any`s every root before returning it; pre-E503 journal lines carry no identity (`#[serde(default)]`) → their processes are LEFT ALONE (fail closed — better an unadopted orphan than a signalled stranger); the exec-settle design: `sh -c` execs its interpreter, so the spawn-time exe record alone would fail verification — each spawn starts a settle thread (+250 ms) that re-probes the kernel exe and appends the authoritative journal entry + refines the handle (a start-time disagreement between the two probes = pid recycled mid-settle → record left untouched) | PASS | 2026-10-13 | zcode-primary |
| Req.5 unverifiable rows permanently out of kill paths + candidates | `identity-drill-test.txt` + `core-storage-test.txt` + `evidence-db-dump.txt` | state flips permanently; excluded everywhere | `ProcessLedgerRepo::mark_unverifiable` (newest running row for `(pid, boot_id)`) + `STATE_UNVERIFIABLE` + `LedgerStore::mark_unverifiable` trait method; `candidates` selects `running` only → unverifiable never re-enters; prune deletes `exited` only → unverifiable is immune (drill: flip → candidates exclude → `prune(i64::MAX)` → row still present; roundtrip step 5b repeats both arms; dump shows the real flipped row `state=unverifiable`) | PASS | 2026-10-13 | zcode-primary |
| Evidence drill (the task's proof) | `cargo test -p juggler-supervisor --test identity -- --nocapture` (2/2) → `identity-drill-test.txt` | forged expected identity → `Unverifiable`; 0 signals to the mismatched pid (trap-log witness) | fixture = TERM-trapping `sh` writing `signal >> trap.log` on every signal: true record → `Verified`; forged start-time/exe/boot each → `Unverifiable` alone; after every refusal + the ledger flip the trap log is ABSENT and the fixture ALIVE (0 signals); dead-pid arm (kill -9 → `wait_gone`) → same recorded identity now `Unverifiable` (probe error ≠ acquittal); control arm: true identity stops cleanly through the normal ladder (the refusals are the gate, not a broken kill path) | PASS | 2026-10-13 | zcode-primary |
| Kill-path wiring scope (per Req.4's own text) | code review | E507/E508 re-verify "in the same tick as the signal" | `Supervisor::root_identity(&SessionId)` exposes the recorded identity (handle-refined) as the kill-path input; `Supervisor::stop`'s internals delegate to the E507 ladder (E502 Req.5) — this task delivers the primitive + adoption gate + permanent flip; the reconciler (E504) re-verifies candidates per its Req.2 | PASS (scoped per spec) | 2026-10-13 | zcode-primary |
| Performance budget | construction + drill timing | ≤1 `stat`/`proc_pidinfo` per candidate; verify ≤10 ms p95 | the probe is ONE targeted sysinfo refresh per pid (start time + exe in the same pass); spawn paths call it once (5×20 ms retry only on table lag — none observed: drill recorded on first pass); `verify` = 1 probe + 2 integer/string compares; drill incl. both arms finishes in 0.66 s | PASS | 2026-10-13 | zcode-primary |
| Regression | full suites | prior gates intact | `just verify` green (fmt, clippy `-D warnings`, workspace tests, docs, evidence, terminology): supervisor 9/9 (adoption test now sleeps past the settle window — commented), ledger 2/2 (E502), identity 2/2, core 24/24; no UI change (visual suite untouched); `Unverifiable — will not be touched` badge wording lands with the Cleanup Centre UI (AJ-E506) | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- E504 (reconciler loop) next: its Req.2 re-verifies `Running|
  Orphaned|Adopted` candidates — call `tree::verify(pid, &identity)`
  from the shared tick; the row identity = ledger `started_at` +
  `exe_path` + `boot_id` (helpers' exe from the sysinfo NAME is
  approximate — enrich per E501's notes first).
- E507's `terminate_group` MUST call `verify` in the same tick as the
  signal (its own spec) and flip the row via
  `LedgerStore::mark_unverifiable` on refusal, returning
  `SupervisorError::Unverifiable { pid }` (typed, distinct from
  `UnknownSession`).
- The journal now has FOUR entry kinds; `identity_settled` rides
  after `spawned` (+250 ms). Old journals (pre-E503) parse fine
  (`#[serde(default)]`) but their processes are left alone at
  adoption — by design; the E506 recovery summary should say so
  ("1 process could not be verified and was left alone", pass2 §11).
- Trap: NEVER compare wall-clock spawn time — `start_time` is the
  kernel's fork time (E501 pinned this; verify depends on it).
- Trap: `verify`'s probe treats a missing live exe as an ERROR (fail
  closed) — do not "helpfully" fall back to the recorded program
  path.

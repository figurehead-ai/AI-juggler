# Gate V0 — M0: Foundation & CI

**Date:** 2026-10-12 · **Milestone head:** PR #13 merge (`702fcb5`) + this report PR · **Tracking issue:** #1 · **Gate issue:** #7

## 1. Automated

- **CI matrix: green.** All four required jobs (`verify`, `build-macos` incl. x86_64 cross-check, `build-linux`, `build-windows`) passed on every M0 PR (#8, #9, #11, #12, #13) and on `main`. Enforcement is real: probe PR #10 (deliberate lint error) failed `verify` and was unmergeable even with admin credentials; admin enforcement is on, linear history required.
- **Coverage (llvm-cov, this PR's tree):** 32/32 tests, 1,098/1,378 lines (79.6% overall).

  | Crate | Coverage | Target | Status |
  |---|---|---|---|
  | juggler-adapter | 96.2% | ≥85% | ✅ |
  | juggler-core | 89.4% | ≥85% | ✅ |
  | juggler-supervisor | 85.0% | ≥80% | ✅ |
  | juggler-adapters | 86.4% excl. bin (52.1% raw) | ≥85% | ✅ with structural note |

  *Structural note:* 139 of `juggler-adapters`' lines are the `fake-harness` **binary**, which every adapter/supervisor suite executes as a child process; llvm-cov cannot instrument child processes. Excluding the bin, the library code is at 86.4%. The bin's behavior is covered functionally by the 13 adapter + 6 supervisor tests that drive it.
- **cargo-deny:** clean (licenses/bans/sources/advisories run in every `verify` job; zero dependencies outside the allowlist).
- **Fuzzing:** not started — scheduled to begin with the first real parser (M1-5 transcript parser) per QUALITY.md §5. No parser code exists yet at M0.
- **Benchmarks:** none yet (first hot-path parser benches land with M1-5); no performance budgets applicable to M0 code.

## 2. Functional (acceptance criteria, with evidence)

| Task | Criterion | Evidence | Verdict |
|---|---|---|---|
| M0-1 | Fresh clone → `just verify` zero warnings | Re-verified from a fresh `git clone` at milestone head: fmt + clippy `-D warnings` + tests + docs all green | PASS |
| M0-1 | Crates documented, `unsafe_code` forbidden, MSRV pinned | Workspace lints deny `missing_docs`, forbid `unsafe_code`; toolchain 1.99.0 / MSRV 1.99 | PASS |
| M0-2 | All jobs green before merge; broken PR rejected | Required checks on `main`; probe PR #10 blocked by policy | PASS |
| M0-3 | Round-trip property tests per entity | proptest suites for Account/Profile/Session (+ update paths) | PASS |
| M0-3 | Migration lifecycle from empty schema | Fresh-open migrates; reopen no-op; future version → `SchemaTooNew` (forward-only policy documented) | PASS |
| M0-3 | No panics on corrupt DB | Garbage file → `NotADatabase`; unknown stored values → typed errors | PASS |
| M0-4 | Fake harness discovered → launched → watched → resumed e2e | `fake_harness_e2e.rs` (8 tests) incl. degrade paths (missing binary, non-executable, missing watch dir) | PASS |
| M0-5 | Spawn/capture/stop cleanly (piped + PTY) | `supervisor.rs` tests 1 & 5 | PASS |
| M0-5 | SIGKILL child → exit event + journal entry; excluded from adoption | test 2 | PASS |
| M0-5 | Simulated app restart → children re-adopted via journal | test 3 (supervisor dropped, child survives, `adopt_orphans` recovers) | PASS |

## 3. Quality audit (vs QUALITY.md)

Findings identified during the audit pass and **fixed before this gate** (no deferrals):

1. Coverage below §5 targets on first measurement (core 79.2%, adapter 62%, adapters-lib 77.5%) → added repo update/delete tests, launch-constructor tests, registry trait-object test, and adapter degrade-path tests; all crates now meet targets.
2. CI coverage artifact not appearing on `main` runs (upload step silent when llvm-cov fails) → llvm-cov now succeeds with tests present; artifact wiring re-checked on this PR's run. Carry-forward item below if it recurs.

Audit conformance: typed errors everywhere (`CoreError`, `AdapterError`, `SupervisorError`); no `unwrap`/`expect`/`panic` in production paths (CI-enforced); crate docs point at SPEC sections; layering respected (supervisor ↔ adapter contract only, core I/O-minimal); CHANGELOG entries per milestone; ADR-0002 reflected in the actual adapter trait.

## 4. Known issues carried forward

- **CI coverage artifact reliability** — the `verify` job's artifact upload produced nothing on early main runs. If this PR's run still lacks the artifact, tracked for M1 (owner: M1-2 CI hardening).
- **Supervisor lifecycle suites are unix-scoped** — cross-platform fixture suites arrive with M2's stuck-detection corpus (Windows/macOS currently build-checked only).
- **PTY exit kinds approximate** — portable-pty reports a single exit code; exact signal detail exists on the piped path only (documented in `supervisor.rs`).
- **PTY `from_env()` constructor untested** — edition-2024 forbids env mutation in tests under our `unsafe_code` forbid; covered indirectly, direct test scheduled with M1 app wiring.

## 5. Confirmation

- Maintainer sign-off: **the maintainer — PENDING** (this gate requires explicit confirmation on issue #7 before M1 starts; per RUNBOOK the next milestone's issues are not created until then).

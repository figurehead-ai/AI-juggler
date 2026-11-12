# ADR-0025: The heavy gates run on GitHub's runners; a fast local preflight

**Status:** Accepted
**Date:** 2026-11-02
**Deciders:** the maintainer (ruling of 2026-11-02, on the release-speed measurements)

## Context

Releases took 4 to 12 hours from the last feature commit to beta, and most of it was before the pull request:

- **The local preflight ran all of CI on the maintainer's Mac, one step after another, in about 55 minutes.** About 38 of those minutes were two Playwright files, each on one worker: the visual gate's `regression.spec.ts` and the accessibility gate's `capture.spec.ts`.
- **24 runs in four days gave 5 passes.**
  - Cheap checks (fmt, clippy, rustdoc, lints, cargo-deny, the release-notes wording) failed after up to 40 minutes.
  - Timing-sensitive tests failed on a loaded machine.
  - Editing the tree during a run voided it.
- **The Mac is shared with other projects** (about 28 containers, load average 7–36). Extra local parallelism would slow them, and their load slows and flakes ours.
- **CI took 32–38 minutes a round.**
  - `verify` was one job: tests 15 min, coverage 8 min (never gating), the dev-bridge build 3 min.
  - The visual job ran on one worker.
  - The a11y shards were 348/174/171/15 tests.
  - After every merge, the push to main ran all of it again on the same tree.
  - Promote ran the a11y gate again on a commit CI had already scanned.
  - The nightly compiled both architectures one after the other on one runner (24–26 min).
- **The repository is public.** GitHub's standard hosted runners are free for it: Linux x64 and arm64 with 4 vCPU and 16 GB, Windows, and macOS. The organisation plan allows 500 concurrent jobs, 50 of them macOS, and 256 per matrix. Larger runners are billed, even for public repositories.

## Decision

1. **The heavy gates run on GitHub's standard runners, split across jobs. Larger runners are never used.** `ci.yml`:
   - **`static`:** the release notes, the project lints, fmt, clippy (3 variants), rustdoc, cargo-deny (`just verify-static`).
   - **Rust tests with cargo-nextest:**
     - `tests (build)` builds every test binary once (a nextest archive); the doctests run beside it, with the dev-bridge tests.
     - `tests (k/6)` run that archive in six partitions.
     - `.config/nextest.toml` gives each binary that used an in-process mutex its own serial group (nextest runs each test in its own process, so a mutex no longer separates them). Wall-clock budgets run alone. The timing-sensitive tests run one at a time. Retries are 0.
   - **`dev-bridge`** and **`coverage`** run on their own runners. Coverage stays non-gating and is off the critical path.
   - **Visual:** `fullyParallel`, in four shards, in the same pinned Playwright container (x64, by digest), compared with the same baselines. The reports merge into one.
   - **a11y:** six shards, split by test.
   - **The required checks keep their names.** `verify`, `visual` and `a11y` are jobs that need their parts and fail unless every part passed, so the ruleset applies unchanged: verify, ui, visual, build-macos, build-linux, build-windows.
2. **The local preflight is fast and low-load by default** (`just preflight`):
   - **What runs, in order:**
     1. the release notes;
     2. the static checks, which stop the run if they fail;
     3. the tests of the packages the change can affect (`scripts/affected_crates.py`; every package when the build as a whole changed);
     4. vitest;
     5. the UI build.
   - **Load:** it runs under `nice` with 4 build jobs and 4 test threads.
   - **What it checks:** a snapshot worktree of the commit. It holds a lock (one preflight at a time) and uses its own target folder.
   - **What it leaves:** each step's log and `summary.txt`.
   - The visual, a11y and Linux-clippy containers no longer run locally. `--full` keeps the old whole-of-CI run for when GitHub can't be used.
   - The pre-push hook accepts either mode's stamp.
3. **CI trims:**
   - **A push to main** runs nothing again when its tree is the tree its pull request's head passed every required check with. A nightly run on main does everything (weekly until AJ-E2020 R5, 2026-11-11, which also warms the caches with it).
   - **The nightly** compiles aarch64 and x86_64 at the same time, on clean and uncached runners, and then joins them with lipo and bundles once. Signing and notarization are unchanged.
   - **A beta promotion** reuses CI's full a11y scan of the same tree: the commit, or its pull request's head when the trees are identical, with every shard passed. A stable promotion always scans again.
4. **Release pull requests will use merge commits** (`gh pr merge --merge --match-head-commit <sha>`) once main's ruleset allows them. That ends the back-merge after each squash. Until then, releases squash as before (`docs/RELEASING.md`).

**Note (2026-11-11): CI is the heavy gate, and builds the release candidates.** Local release builds filled the shared Mac's disk and swap, so:

- **CI's `rc-dev-app` job builds the release candidate** (AI Juggler Dev, release profile, arm64, unsigned) with `scripts/build_dev_app.sh`, the script `just app` runs, and the release build's pinned toolchain (`.github/actions/macos-app-toolchain`, shared with `build-macos.yml`). It checks the bundle and the first open after an update, then uploads `rc-dev-app-<commit>`. `verify` needs it, so a red release candidate blocks the merge. Walks fetch it with `scripts/qa/fetch_rc.sh <commit>`.
- **`just preflight-light`** stamps a commit after `just verify-static`, the tests of only the packages the change touches, vitest for the changed UI files, and the UI's lint and `tsc`. The fast mode and `--full` are unchanged.

**Note (2026-11-11, D-99 point 2, AJ-E2020 R2): a pull request runs what its change needs.**

- **`plan` lists the jobs** (`scripts/ci_scope.py`), and each filtered job asks for its own id; there is no workflow-level path filter, so every job reports. Rust sources run the Rust static checks, the builds and the release candidate; the test slices run the tests of the packages the change can affect (`scripts/affected_crates.py`, as the preflight); `ui/` and the files its tests read run `ui`, visual and a11y; documentation alone runs `static`'s lints. Anything else (`.github/`, `scripts/`, the toolchain, `Cargo.lock`, an unknown path) runs every job.
- **`verify` is the one aggregate.** Outside a full run it accepts a skip only for a job plan left out; a full run (every push, the weekly run, release pull requests) skips none. Pull requests into main are no longer full runs.
- **A push to main skips only a tree a full run passed**: the pull request's head needs the `full run` check, which only a full run's passing `verify` gives. A merged partial pull request runs every job on main.

## Why this doesn't weaken a gate

- **The same checks run.** They use the same thresholds, the same baselines (rendered on the same image and architecture), the same pinned tools and the same `-D warnings`. No test is skipped or retried.
- **A merge still needs every required check green on the exact head commit.** The checks now come from GitHub, which runs every one the change needs (all of them on a full run), not only the parts a local run reached.
- **Nothing is reused unless the content is the same.** A skip on main or in Promote needs an identical git tree, and the earlier run's checks passed on it. The same tree, workflow and pinned images give the same answer. Promote reuses only a real scan: an `a11y` that passed because nothing in the UI changed doesn't count.
- **nextest is stricter than `cargo test`, not looser:**
  - each test runs in its own process;
  - shared resources are serialised by declaration, not by an in-process lock;
  - JUnit records every result.
- **The local fast mode can skip tests GitHub then runs.** At worst a failure is found in CI instead of locally, never merged.

## Alternatives considered

- **Keep every gate local and make it parallel.** That puts more load on a shared Mac, which is exactly what the maintainer ruled out. It also leaves the flakes that load causes.
- **Larger GitHub runners.** They are billed even for public repositories.
- **Self-hosted or third-party runners.** They cost money and expose the signing flow. The free runners cover it.
- **Automatic retries in CI.** They hide races (#198). Flakes are fixed or quarantined with an issue instead.
- **Native arm64 for the visual gate.** It would need every baseline re-rendered and approved (Playwright renders arm64 and x64 differently). That is not needed while the gate runs on x64 on GitHub.

## Consequences

- **CI does more in total but finishes sooner.** A pull request starts about a dozen more jobs. Its wall time is bounded by the test build plus one partition, not by the sum of every step.
- **A local run is minutes, not an hour.** The full answer arrives on the pull request.
- **More of the test grouping is now explicit.** New tests that share a real resource (a socket, a process-global counter measured across tests, a wall-clock budget) need a group in `.config/nextest.toml`, and `scripts/affected_crates.py`'s `READ_BY` has to name the files outside `crates/` that tests read.
- **The main-push skip and the Promote reuse rely on the check-runs API** (`scripts/gate_passed.py`, with a self-test).
- **The preflight's target folder adds one more build folder** (`target/preflight-build`) to watch for disk.

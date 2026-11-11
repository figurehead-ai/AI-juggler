# Release speed: what comparable projects do (2026-11-11)

> Research behind D-99 and AJ-E2020. Method: web search plus direct reads
> of real workflow files and release APIs. Where a source gave no
> measurement, none is claimed. Savings for AI Juggler are estimates from
> the shape of its CI, not measurements; the first step of AJ-E2020 is to
> measure per-job timings and the cache hit rate (`gh run view --json jobs`).
> Faster paid runners were considered and ruled out by the maintainer
> (D-99); they are not covered here.

## 1. Beta promotes the tested nightly (no rebuild)

- Zed: a daily nightly (untested by design), a weekly Preview, and a Stable
  that is last week's Preview promoted; patches are cherry-picked onto the
  preview and stable lines. 2026 releases ran on a 7-day cycle.
  https://zed.dev/faq , https://zed.dev/docs/development/releases
- GitButler: stable releases a median of about 6 days apart (mean about 10)
  from June to November 2026, with several same-day patches (GitHub
  releases API). Cap (Tauri): median about 17 days (range 3 to 48).
- Pattern: a release is the promotion of an already-built, already-tested
  artifact, and the validation ring is the nightly/preview channel. A
  rebuild plus a full manual re-walk per release is not what these projects
  do.
- For AI Juggler: beta = the exact signed nightly, no rebuild; walk and
  validate on beta and fix forward in the next nightly; the full hand-walk
  stays for Stable. Keep `scripts/release_scope.py` as a CI check.
- Estimate: removes one build and one full recheck per beta (hours).
  Effort M (process plus workflow). Cost: free.

## 2. Change detection plus one aggregate required check

- Zed `run_tests.yml`: an `orchestrate` job maps changed `crates/*` to
  packages through `cargo metadata` and builds a nextest `rdeps(...)`
  filterset; toolchain, `.cargo`, `.github` or `Cargo.lock` changes run
  everything; docs-only changes skip Rust tests; one `tests_pass` job
  aggregates for branch protection.
  https://raw.githubusercontent.com/zed-industries/zed/main/.github/workflows/run_tests.yml
- Ruff `ci.yaml`: a `determine_changes` job with per-area outputs (code,
  linter, formatter, ty, fuzz, benchmarks, release), a `no-test` label
  escape hatch, and a `required-checks-passed` aggregator.
  https://raw.githubusercontent.com/astral-sh/ruff/main/.github/workflows/ci.yaml
- GitButler: `dorny/paths-filter`, GUI crates excluded from most Rust jobs.
  https://raw.githubusercontent.com/gitbutlerapp/gitbutler/master/.github/workflows/push.yaml
- Caveat (GitHub): never path-filter a required workflow at the workflow
  level, because the job never reports and the merge blocks. Filter at job
  level and make one aggregate job the required check.
  https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/troubleshooting-required-status-checks
- For AI Juggler: UI-only changes skip the Rust slices; Rust-only changes
  skip vitest, Playwright and axe; docs-only changes keep lint.
- Measured: none published. Estimate: 30 to 70% of CI minutes on most PRs.
  Effort M (rdeps filterset) or S (path filter only). Cost: free.

## 3. Build once, test from a nextest archive in N slices

- `cargo nextest archive --archive-file x.tar.zst`, then
  `run --archive-file` with `--partition slice:i/n` in the other jobs; no
  longer experimental. https://nexte.st/book/reusing-builds ,
  https://nexte.st/book/partitioning
- Ruff uses the same build-once, run-many split (upload with one-day
  retention and compression level 1).
- Logic, not a measurement: if each of the six slices compiles and links
  the test binaries, the cost is N links plus N cache restores; an archive
  is one build plus N downloads.
- Caveats: build and run jobs need the same OS and architecture;
  `slice` partitions by count, not time, so use `hash` or per-crate groups
  if one test crate (juggler-app) dominates.
- Estimate: 5 to 15 minutes of wall-clock per run. Effort M. Cost: free.

## 4. CI profile settings

- Ruff CI: `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, `OPT_LEVEL=0`,
  `LTO=false`, `CARGO_INCREMENTAL=0`, `CARGO_NET_RETRY=10`.
- Measured elsewhere: line-tables-only took a clean dev build from 6m12s to
  5m02s (about 19%) in one forum report; `debug=0` reached about 3m16s in
  another; Kobzol reports 30 to 40% faster incremental builds with
  debuginfo off.
  https://kobzol.github.io/rust/rustc/2025/05/20/disable-debuginfo-to-improve-rust-compile-times
- On macOS, `split-debuginfo=unpacked` skips `dsymutil` (Databend).
  https://rustmagazine.org/issue-3/optimizing-compilation-for-databend
- Smaller debuginfo also shrinks the cache and the disk use of `target/`.
- Estimate: 10 to 25% of Rust build time, 30 to 50% smaller `target/`.
  Effort S (environment variables). Cost: free.

## 5. Cache hygiene, cancel superseded runs, timeouts

- Ruff and GitButler: `Swatinem/rust-cache` with `save-if` on the main
  branch only, a `shared-key` reused across jobs, `CARGO_INCREMENTAL=0`. A
  job that times out saves no cache, so the next run starts cold
  (GitButler's note).
- Zed uses sccache backed by Cloudflare R2 and wipes `target/` above a size
  threshold. Depot's benchmark of Zed's workflow (28m36s baseline): the
  cargo cache alone 8% faster; sccache cold 16.8% slower, warm 11.5%
  faster; nextest on top of a warm sccache 35% faster in total.
  https://depot.dev/blog/guide-to-faster-rust-builds-in-ci
- GitHub's 10 GB cache limit per repository is the real constraint with a
  big workspace on macOS and Linux; pull-request caches can evict main's.
  sccache does not cache linking or build scripts well, so rust-cache stays
  better for dependencies.
- Concurrency: Ruff, Zed and GitButler cancel superseded runs on pull
  requests and run to completion on main (the group includes the SHA).
  Free runners cap concurrency (5 macOS jobs on free plans, 20 Linux), so
  stale runs queue the live ones.
- Scheduled warm: a nightly job on main that builds the release and Tauri
  caches so a beta or nightly build starts warm.
- Estimate: 3 to 10 minutes per run if the cache misses or thrashes today
  (unknown until the hit rate is checked). Effort S to M. Cost: free.

## 6. Visual baselines out of git (Argos)

- Argos (open source; Playwright, Storybook, Vitest) keeps baselines
  remotely, diffs them in the pull request and approves with a click, with
  no PNGs committed. Migration: swap the capture helper and delete the
  baseline images.
  https://argos-ci.com/blog/playwright-visual-regression-testing-ci
- Playwright sharding with blob reports and `merge-reports` gives one
  report. https://playwright.dev/docs/test-sharding . There is no "update
  only changed snapshots"; scope updates with `--grep` or spec paths.
- Lost Pixel is archived (April 2026): avoid. Chromatic and Percy are the
  paid equivalents.
- Pricing was not found in the sources. The maintainer asked for a check of
  whether it is free for open source; the result is recorded in AJ-E2020 R6.
- For AI Juggler: this removes the regenerate, eyeball, commit, re-push loop
  and the Mac-versus-Linux rendering mismatch; pair it with change
  detection so captures run only when `ui/**` or shared CSS changes. The
  rule that every image is opened before it ships still applies; Argos is
  the viewer.
- Estimate: a baseline cycle from hours to minutes. Effort M.

## Not recommended (measured or sourced)

- mold or lld: 0.7% slower on Zed release builds (Depot); macOS ld-prime is
  already fast.
- Cranelift (failed on Zed), BuildJet (shut down 31 March 2026), Lost Pixel
  (archived), nightly-only compiler flags such as `-Zshare-generics`.
- Merge queue: little value for a solo `release/<version>` flow.
- cargo-hakari: no CI benchmark found, 0 to 10% by estimate, and its benefit
  shrinks once the archive build exists; revisit after items 2 to 5.
- Faster, paid runners: ruled out by the maintainer (D-99).

## Gaps

No measured benchmarks were found for hakari in CI, sccache against
rust-cache head to head, tauri-action macOS times, Argos pricing or
merge-queue cost and benefit. The current cache hit rate and per-job
timings are the first thing to measure.

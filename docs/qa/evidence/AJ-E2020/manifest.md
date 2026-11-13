# Evidence manifest — AJ-E2020

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2020, release speed (P1). Ruling: D-99.
Status: **IN-PROGRESS**. R1, R3, R4 and R5 pass; R2 waits on the one-check ruleset (the maintainer's setting) and R6 on the Happo amendment. 2026-11-13, claude-opus.

Run ids are GitHub Actions runs of figurehead-ai/AI-juggler
(`https://github.com/figurehead-ai/AI-juggler/actions/runs/<id>`). Durations
come from the jobs API (`started_at`/`completed_at`). A Sonnet draft gathered
the runs; every verdict below was checked again on Opus against the API
(the ruleset, the cache usage, the run durations, the scheduled run).

## Proof

| Requirement | Proof | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| R1 Beta is a promotion of the tested nightly, no rebuild; `release_scope.py` reports on the release PR, the nightly and the beta promotion, and fails only the Stable promotion; the workflows and RELEASING.md say so | #220. `promote.yml` (nothing rebuilt: the channel's `latest.json` is pointed at the version's own release; `release_scope.py check "$VERSION" --channel "$CHANNEL"`); `release_scope.py --selftest` (beta exits 0 with open scope, stable exits 1); `ci.yml` reports it on release PRs, `nightly.yml` before a new minor; `docs/RELEASING.md` | Beta promotion run 43304684626 (0.4.3, 2026-11-12) has no build job; the beta feed serves the nightly's own `AI-Juggler-0.4.3-universal.app.tar.gz`, whose signature the nightly's sign job checked (run 43298877205). The Stable failure is shown by the self-test only (no Stable promotion has run) | PASS | 2026-11-13 | claude-opus |
| R2 Change detection: per-crate and per-area outputs filter jobs (no workflow path filter on a required check); one aggregate required check; the toolchain, `Cargo.lock` or `.github` run everything | #221. `scripts/ci_scope.py` (areas, `affected_crates.py`; `--check` holds its job list to `ci.yml`'s `if:`s; `--selftest`: docs-only, UI-only, Rust-only and the run-everything paths); `ci.yml` `plan` and `verify` (`if: always()`; a skip passes only for a job `plan` left out); no `paths:` on `ci.yml` | Filtering and the aggregate work: run 43102802387 (`.github` changed: every job ran, `verify` passed); run 43298114369 (a push of a tree its PR already passed: 15 jobs skipped, `verify` passed); run 43340145489 (`verify` failed when visual and a11y shards failed). **Not met:** main's ruleset still requires six checks (verify, ui, build-macos, build-linux, build-windows, visual), not the one aggregate — a repository setting, the maintainer's (2026-11-13: on the coordinating session's list); and no docs-only, UI-only or Rust-only pull request has run since #221, so the three kinds are shown by the self-test only | PARTIAL | 2026-11-13 | claude-opus |
| R3 Build once with `cargo nextest archive`; the slices run from the archive on the same OS and architecture, balanced | #209 (ded7253a7). `ci.yml` `rust-build` (`cargo nextest archive --workspace`, uploaded) and `rust-test` (six `slice:N/6` partitions, ubuntu, from the archive); `.config/nextest.toml` | Slices of run 43290507006: 5.5, 4.3, 4.2, 5.6, 4.3, 5.5 min (no slice dominates) | PASS | 2026-11-13 | claude-opus |
| R4 CI profile environment: `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, `CARGO_INCREMENTAL=0`, `split-debuginfo=unpacked` on macOS | #222. `ci.yml` workflow `env` (the first two); `build-macos` `CARGO_PROFILE_DEV_SPLIT_DEBUGINFO: unpacked` | All three set. Four jobs set debug info to `0` on purpose (rust-build, dev-bridge, floor-gallery, build-linux: the archive was 5.4 GB with line tables), as their comments say | PASS | 2026-11-13 | claude-opus |
| R5 Cache hygiene: caches saved on main only, shared keys, a scheduled warm, `cancel-in-progress` on pull requests and a timeout on every job; before and after timings and the cache hit rate | #223. `ci.yml`'s caches restore everywhere and save only on `refs/heads/main`; the schedule `41 5 * * *` (run 43374179390, 2026-11-13 05:46Z, passed); `cancel-in-progress` only for `pull_request` (`ci.yml`, `happo.yml`); every job of every workflow has `timeout-minutes` (`ci.yml` 19 of 19 before this change, 21 of 21 after) | **Before and after (pull-request CI, wall-clock):** median 43.1 min before (runs 43078016815, 43085370285, 43085512402, 43108510790, 43209659889), 43.7 min after (43239670856, 43239663581, 43239659943, 43277068305, 43290507006); all ten full runs. **Why no gain:** `rc-dev-app` (~40 min) was the critical path: in 43290507006 its app build took 19.2 min and the v49 test build after it 20.2 min. **Caches:** 12.3 GB in 28 entries against GitHub's 10 GB (2026-11-12), 4.4 GB of it six per-target fuzz builds saved from pull requests too; the Linux test cache missed in 43277068305 and 43290507006. **Fixed in this change (release/0.5), proof on its first CI run:** the fuzz targets share one build cache, saved from main by one target (`fuzz.yml`); the two release builds run side by side (`rc-dev-app`, `rc-v49`, then `rc-first-open`), so the critical path drops: 27 min in run 43403799722 (below) | PASS | 2026-11-13 | claude-opus |
| R6 Visual baselines on a service free for open source, with no PNGs in git; the free-tier check recorded first | #225. `happo.yml` (OIDC sign-in, no stored secret; advisory); `docs/RELEASING.md` § Happo's visual review (resolving a review from the Mac uses a key in the maintainer's Keychain, not CI's sign-in) | Happo, not Argos: the maintainer's ruling to check free-for-open-source tools (D-99 point 6) led to Happo's open-source plan (granted to the organisation 2026-11-11). It runs on main and on pull requests touching `ui/`, Home's states only. **Not met:** the committed PNG baselines are still CI's visual gate, and the requirement still names Argos | PARTIAL | 2026-11-13 | claude-opus |

## The runs that prove R2 and R5 (2026-11-13)

**R2, the three change kinds.** Three throwaway draft pull requests from main, one change each, closed unmerged with their branches deleted (#232, #233, #234). Each ran main's `ci.yml`.

| Change | Run | Ran | Left out | verify | Wall-clock |
|---|---|---|---|---|---|
| docs-only (`docs/research/`) | 43403933817 | plan, static, the visual and a11y aggregates | every build, test, UI and release-candidate job | PASS | 4 min |
| UI-only (`ui/src/lib/window-title.ts`) | 43403942121 | tests (build) and slices (`juggler-app`, `juggler-agentbridge`), ui, visual shards, a11y shards, rc-dev-app | build-macos, build-linux, build-windows, static (Rust), dev-bridge, coverage, floor-gallery | FAIL: one gallery capture (agent-tools question, dark, 6 415 px) that passed in 43403799722 on the same UI code, a flaky capture; it also shows `verify` failing when a filtered-in job fails | 44 min |
| Rust-only (`crates/juggler-vm-stats`) | 43403953672 | static (Rust), tests (build) and slices, dev-bridge, coverage, build-macos, build-linux, build-windows, rc-dev-app | ui, visual shards, a11y shards, floor-gallery | PASS | 42 min |

Each matches `scripts/ci_scope.py BASE HEAD` for its change. What R2 still lacks is main's ruleset requiring `verify` alone.

**R5, the critical path.** release/0.5's draft release pull request (#231), a full run, run **43403799722**: **27 min** wall-clock, all green, against a median of 43.1 min before (the five full runs above). `rc-dev-app` 21.2 min and `rc-v49` 24.8 min ran side by side, then `rc-first-open` 1.6 min uploaded `rc-dev-app-<commit>`. `rc-v49`'s own cache isn't saved until the change reaches main (caches save on main only), so 24.8 min is its cold time. The UI-only and Rust-only probes, on main's `ci.yml` with the two builds one after the other, took 42 and 44 min.

| Requirement | Proof | Verdict | Date | Reviewer |
|---|---|---|---|---|
| R2 (runs) | 43403933817, 43403942121, 43403953672: each change kind ran its expected jobs and left out the rest; the aggregate passed and failed as it should | PARTIAL (the ruleset) | 2026-11-13 | claude-opus |
| R5 (after) | 43403799722: 27 min against a 43.1 min median, the release builds side by side | PASS | 2026-11-13 | claude-opus |

## What closes it

1. R2: the ruleset requires `verify` alone (the maintainer's click).
2. R5, after it reaches main: the cache total under 10 GB a day later, and `rc-v49` warm.
3. R6: the requirement's text amended to Happo (with the ruling's D-number), and either Happo takes over the visual gate's screens (the PNGs leave git) or this requirement is narrowed by the maintainer.

Last updated: 2026-11-13.

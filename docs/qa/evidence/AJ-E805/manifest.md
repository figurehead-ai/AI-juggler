# Evidence manifest — AJ-E805

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E805
(performance budgets and load proof at 10 Accounts / 50 Instances).
2026-10-19, reviewer claude-opus.

Machine: Apple M5 Ultra, 30 cores, 96 GB, macOS 27.0.1. UI timings run
in headless Chromium against the production build (`pnpm preview`) with
the mocked bridge; backend timings run in debug test builds (slower than
release, so conservative).

Fixtures: `ui/tests/visual/fixtures.ts` `atScale` (10 Accounts over
three Providers, 50 Instances in every status, 20 Projects; sha256
`7334cefed416…`), `crates/juggler-app/tests/load_budgets.rs` `seed()`
(the same shape in a real store), and the 500-turn pack built from the
handover `pack_v1` fixture in `crates/juggler-handover/tests/pack_load.rs`.

Logs: [`load-gate.txt`](load-gate.txt) (one `just load-gate` run),
[`ui-load.json`](ui-load.json) (the UI table, every run),
[`regression-selftest.txt`](regression-selftest.txt) (the >10 % rule
bites), [`polling-audit.txt`](polling-audit.txt), captures under
[`visual/`](visual/).

This is the first run of the fixture, so it is the baseline
(`ui/tests/visual/load-baseline.json`). Earlier scale figures in the
reviews were estimates, and nothing here is quoted from them.

## Measured budgets

| Budget (Req.1, Req.3) | Target | Measured (median of 5 unless noted) | Verdict |
|---|---|---|---|
| Cold start to interactive | < 2.5 s | Backend share 17 ms (open and migrate the seeded store, read the 50 Instances and 20 Projects) + webview share 64 ms (fresh browser context each run, navigation to painted rail) = **~81 ms**. The release app in WKWebView on the reference Mac is not measured yet (maintainer step below) | PASS (Inferred) |
| Rail render, 50 rows | < 300 ms | **41 ms** (Instances tab click to all 50 rows painted); scroll to the last Instance **16 ms** | PASS |
| Inspector selection to paint | < 200 ms | **34 ms** (row click to the inspector showing that Instance's Project; each run lands on a different Project) | PASS |
| Search over 50 Instances | (rail budget) | **8 ms** (typing to a filtered board) | PASS |
| Cleanup inventory | (rail budget) | **33 ms** (open the Cleanup centre at scale) | PASS |
| Terminal input echo, local | < 50 ms | **0.32 ms** median, 0.33 ms p95 over 20 writes (dock write → bytes back through a real PTY → readable by the terminal snapshot) | PASS |
| Switch pack assembly, 500 turns | < 5 s | **7.4 ms** full pack, **4.4 ms** distilled (1.36 MB of turns, redacted, written to disk; no credential pattern in the output) | PASS |
| Orphan detection | < 10 s after exit | Surfaced **10.13 s** after the harness exited: the settled 10 s grace (AJ-E410) plus **0.13 s** detection. See the note below | PASS after grace / conflict flagged |
| Idle CPU, 10 live Instances | < 2 % | **0.39 %** (10 quiet PTY Instances under the supervisor plus the app's 2 s per-tree tick for 30 s; every tree asserted sampled) | PASS (backend proxy) |
| Memory excluding children | < 600 MB | **20 MB** resident for that backend process | PASS (backend proxy) |

## Claims

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Req.2: a load fixture seeds 10 Accounts, 50 Instances (mixed statuses), 20 Projects and 500-turn transcripts, and an automated test drives rail scroll, inspector switching, search and the cleanup inventory | Req.2 | `ui/tests/visual/load.spec.ts` (10 tests), `load_budgets.rs` (3), `pack_load.rs` (1); all in `just load-gate`. The UI spec also runs in every full Playwright run, holding the SPEC budgets | PASS | 2026-10-19 | claude-opus |
| Req.3: no watcher or poll under 1 s without a named requirement | Req.3 | `scripts/polling_audit.py`, run by `just verify`: 36 timers and sleeps, 11 under 1 s, each with a `poll:` note naming its requirement and bound (stop ladders, retry loops, the pairing exchange while the wizard is open, the AJ-E405 300 ms recovery drain); 0 unexplained | PASS | 2026-10-19 | claude-opus |
| Req.3: idle CPU and memory measured and recorded | Req.3 | Backend proxy above. The release app with 10 live provider Instances is a maintainer step | PARTIAL | 2026-10-19 | claude-opus |
| Req.4: a regression over 10 % fails unless waivered with a task ID and expiry | Req.4 | `regression-selftest.txt`: baseline 20 ms against a real 42 ms → FAIL; with an unexpired AJ-E805 waiver → PASS; same waiver expired → FAIL; baseline restored → PASS. A 5 ms floor keeps sub-10 ms readings from failing on timer jitter. The rule applies only under `just load-gate` (the baseline is this Mac's); other runs hold the SPEC budgets | PASS | 2026-10-19 | claude-opus |
| Req.5: the gate report shows measured values | Req.5 | The table above is measured. `docs/plan/QUALITY.md` lists only the targets and points here | PASS | 2026-10-19 | claude-opus |
| Visual: rail and inspector under the load fixture at 1440x900 and 1024x680, both themes; no layout collapse, skeletons resolve | Visual | `visual/load-{dark,light}-{1440x900,1024x680}.png`; each asserts no horizontal overflow, rows over 150 px and the inspector over 280 px wide, no `aria-busy` left | PASS | 2026-10-19 | claude-opus |

## Defects found and fixed by this proof

1. **A live Instance's process rows blinked empty every other tick**
   (`crates/juggler-supervisor/src/supervisor.rs` `refresh_pids`). The
   2 s targeted refresh listed the root pid twice, once as the root and
   once from its tree's own row. sysinfo drops a process that it is
   asked to refresh twice in one call: the second visit finds its
   "updated" mark already used. Rows per tick were `[1,1,1,0,1,0,1,0,1,1,0,1]`,
   so the Processes tab, CPU/RAM stats and the ledger's helper rows
   flickered for every Instance. The fix lists each pid once. Regression
   test: `live_root_stays_sampled_across_targeted_ticks`. The idle proof
   first read 0.39 % with no tree sampled; it now asserts every tree was
   sampled, and reads 0.39–0.51 %.
2. **The Instances tab's accessible name was "!20"** at the default
   rail width (labels hidden, the needs-you badge text won).
   It is now "Instances, 20 need you", with the badge `aria-hidden`
   (`ui/src/components/WorkflowRail.tsx`).
3. The polling audit's first version missed a sleep spelled with the
   full `std::time::Duration` path. The pattern was widened, and the
   200 ms recovery drain it then found is annotated.

## Notes and maintainer items

- **Orphan budget conflict.** E805 Req.1 says orphan detection < 10 s
  after process exit. AJ-E410 settled a 10 s grace before a leftover
  is listed, and pass2/05 §6.3 proposed 30 s. An orphan cannot be listed
  sooner than its grace, so the literal reading cannot pass with any
  grace of 10 s or more. Measured: 10.13 s after exit, which is 0.13 s
  after the grace ended. Recorded as "< 10 s after the grace window" in
  QUALITY.md, pending the maintainer's confirmation.
- **Maintainer steps (reference Mac, release build):** cold start of
  the signed app to an interactive rail; idle CPU and memory with 10 live
  provider Instances over 10 minutes (Activity Monitor or `ps`). Record
  them here.
- The UI timings are taken in Chromium. The app's WKWebView may differ;
  the margins are 5–10× under every budget.
- Every test row in the fixture shows "A0" as the Account initials,
  because the fixture's labels are all "Account NN". This comes from
  the fixture's names, not a product defect.

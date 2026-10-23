# Evidence manifest — AJ-E806 (gates + CI enforcement)

| Claim / Req ID | Command / fixture | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 gate vocabulary | justfile + QUALITY.md + CI | names identical in all three | verify/ui/evidence wired; visual/a11y/perf named with owners | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Seeded unit failure | seeded-unit-red.log (+ removal green) | gate red, then green after removal | FAILED captured; green restored | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Seeded visual failure | seeded-visual-red.log (tampered capture, byte compare) | compare red | non-zero detected | PASS | (this PR) | 2026-10-13 | zcode-primary |
| Seeded a11y failure | axe harness | gate red on seeded violation | DEFERRED → AJ-E804 (harness not yet built; gate named, not silently passed) | DEFERRED | — | 2026-10-13 | zcode-primary |
| Evidence in CI | `.github/workflows/ci.yml` verify job | evidence step present, no new job | step added inside verify | PASS | (this PR) | 2026-10-13 | zcode-primary |

## Notes

- **CI record, 2026-10-18 (fixed).** On PR #27, `backup_restore ::
  export_throughput_keeps_a_gigabyte_inside_a_minute` failed on the
  Linux runner: about 82 s/GB against the 60 s budget, while the same
  test ran about 25 s/GB locally. The unoptimised debug build was being
  measured, mostly the hand-rolled redaction scan plus hashing and
  compression. Fix: those crates (`juggler-handover`, `sha2`,
  `miniz_oxide`, `flate2`, `crc32fast`, `adler2`) are optimised in the
  dev profile too, as the vault cipher already was, so the budget
  measures what users get. Locally: export about 2.3 s/GB, verify about
  0.9 s/GB. The budget is unchanged.

- **Flake record, 2026-10-18 (fixed).** `juggler-supervisor --test
  reconciler :: fifty_instance_soak_and_construction_regression` failed
  under the full `just verify` on the E10 branch: p95 tick 1,360 ms
  against the 100 ms budget, avg 202 ms, load 7.4/9.8/12.7 on 30 cores.
  The box was busy with Spotlight, a VM and unrelated work, and the
  binary's three tests were also running in parallel, all spawning real
  process trees. Alone, the soak passes at p95 33 ms. This is the second
  occurrence of this known class (the first is in `gate-M3.md`). Fix: the
  binary's tests now run one at a time (a shared lock), so the budget
  measures the reconciler, not its siblings. The 100 ms budget and the
  load quarantine are unchanged. Verified 3/3 standalone and in the
  full gate re-run; CI stays the arbiter.
- **Flake record, 2026-10-19 (recurrence; open).** The same soak failed
  again during `just verify` on the dependency-upgrade branch: p95 tick
  1,304 ms against 100 ms, load 7.66/9.22/9.69 on 30 cores, with about
  1,240 processes on the machine (another project's browser tests and 48
  containers). It failed 2 of the next 3 runs in that window, then passed
  in 7 of 7 later runs on the same branch (p95 30–42 ms) and on unmodified
  `main` (p95 33 ms). The test binary was already serialised, so the
  2026-10-18 fix wasn't the cause. The stall is machine-wide and doesn't
  show in the load average the quarantine reads; the p95 tick is the
  full process-table scan, which slows with every other process on the
  box. Nothing changed: same budget, same quarantine, CI is the arbiter.
  Open question for the maintainer: measure this tick as CPU time, as
  the flush and outbox budgets already are (a re-threshold that needs a
  recorded decision), or keep wall time and accept rare local misses.
- **Flake record, 2026-10-19 (fixed).** `juggler-supervisor --test
  supervisor :: twenty_streaming_sessions_fan_out_without_deadlock`
  failed in CI `verify` on PR #42 (release channels, which doesn't touch
  the supervisor): `stream for s-stream-7 never delivered its final
  line`, 1.17 s into the run. Cause: a race in the test, not a deadlock.
  It spawned each shell and only then attached a live tap; a tap sees
  only output written after it attaches, so on a busy runner a shell
  could print all 50 lines and exit first. Fix:
  `Supervisor::tap_with_backlog` takes the retained output and the live
  tap under one ring lock, so nothing falls between them or arrives
  twice, and the test reads both. New regression test
  `a_tap_attached_after_the_process_finished_still_gets_its_output`
  reproduces the race deterministically. Verified 25/25 repeated runs
  and the full `just verify`. The 30 s budget and the assertion are
  unchanged.
- **Flake record, 2026-10-19 (known class; no change).** While
  verifying the fix above, `juggler-app --test tiers ::
  tier2_crash_flow_notification_undo_and_lapse` failed once in the full
  `just verify` with `crash orphans never surfaced`, at 25.2 s against
  its 25 s wait. Load was 15.4 (5-minute average) on 30 cores: a
  parallel session was rendering the visual gallery in containers.
  This is the same class as the reconciler soak above: each wait tick is
  a full process-table scan, which slows with every other process on the
  machine. It passed 5/5 standalone straight after and in the gate
  re-run. Budget, wait and assertion are unchanged; CI is the arbiter.
- **Flake record, 2026-10-19 (known class; no change).** The core
  visual gate on the D-23 editions branch failed one cell, `history —
  light-comfortable-1280x800`: `locator.click: Test timeout of 30000ms
  exceeded` before any screenshot, so no pixels were compared. Load was
  about 15 on 30 cores (a parallel session rendering in containers), and
  each cell took 10–18 s instead of about 3 s. Re-run in the same pinned
  container, all eight History cells passed 3/3 (24/24) with no pixel
  difference. Timeouts and thresholds are unchanged; CI is the arbiter.
- **Flake record, 2026-10-22 (fixed; a product bug).** `juggler-app --test
  parked_reset :: not_now_snoozes_once_then_stays_parked_quietly` failed in
  #126's CI (run 39764249611): "database is locked" (SQLITE_BUSY) storing
  usage events. Cause: usage writes opened a deferred transaction, which
  begins as a read, and #126's store reads before it writes. When another
  connection on the same file (here the process ledger, writing at launch)
  committed meanwhile, SQLite answered busy at once instead of waiting.
  Fix: usage writes take the write lock first (`BEGIN IMMEDIATE`), so they
  wait under the busy timeout. `juggler-core --test busy_timeout ::
  a_usage_write_waits_for_another_connection_that_changes_the_database`
  reproduces the exact error without the fix and passes with it;
  `parked_reset` passed 5/5 after.
- **Flake record, 2026-10-19 (fixed).** `juggler-app --test parked_reset
  :: a_parked_instance_is_offered_back_once_at_the_reset_and_resumes_the_original`
  failed once in `just verify` on the AJ-E1012 branch: the resume's
  arguments read as `--resume` without the session id. Cause: the test's
  stub wrote `last-args` with `printf … > file`, which creates the file
  before writing it, and the test polls for the file, so it could read it
  half-written. Fix: the stub writes a temporary file and renames it into
  place. It passed 15/15 standalone before the fix; the assertion is
  unchanged.
- **Flake record, 2026-10-20 (infrastructure; no change).** On PR #59
  (AJ-E1301) the CI `ui` job failed in its first step, `corepack prepare
  pnpm@10 --activate`: "Internal Error: Error when performing the request
  to https://registry.npmjs.org/pnpm/10.34.7" (run 39438483215, job
  118681742197). No project code had run; install, component tests and
  build were skipped. The job is re-run unchanged; CI is the arbiter.
- **Flake record, 2026-10-20 (known class; no change).** On PR #61
  (AJ-E1303, Rust-only — no `ui/` change) the CI `ui` job failed one of
  466 vitest tests: `first-run-policy.test.tsx` › "Decide later writes
  NOTHING, stages no pack, and Summary says so" — `Test timed out in
  5000ms` (5014 ms). The same UI code passed on #59 and #60 the same
  hour. The failed job is re-run unchanged; the 5 s budget stays; CI is
  the arbiter.
- **Defect record, 2026-10-20 (found by review; fixed).** PR #60's shared
  import walk (`claude_environment.rs`) used `DirEntry::metadata`, which
  does not follow links, and then copied every non-directory with
  `std::fs::copy`, which does: a linked folder failed the whole import, a
  link to a file outside the home (e.g. a key) was copied in, and the
  size ceiling counted the link, not its target. This affected Claude
  imports too. Fixed on main by #73 (a linked folder is kept as a link, a
  linked file copied by content, a broken link left out); #60's own
  skip-the-link fix was replaced by that shared rule.
  The same review found Codex's `auth.json` copy would share a
  single-use refresh token (now kept back).
- **Defect record, 2026-10-20 (found by review; fixed before merge).**
  PR #61's transcript linking (`handover_source` → `oldest_since`) took
  the oldest file in the session's folder created after its start and
  WROTE it down — even from a preflight. Two sessions of one account in
  one folder (Codex writes a rollout only on the first turn) could be
  given the other's conversation; Claude sessions too. Now: files linked
  to other sessions are excluded, two matches find nothing, another live
  unlinked session in the same folder makes it ambiguous (the switch
  asks), only the switch itself persists, and the scan runs outside the
  storage lock. Tests for Codex and Claude.
- **Defect record, 2026-10-20 (found by review; fixed before merge).**
  PR #64: `codex_handover::install_bundle` joined a peer-supplied file
  name and session id into paths unchecked (path traversal from a paired
  Mac); a replay whose newest turn exceeded the limit dropped every turn
  and still marked the source handed over; a fork whose file name lacked
  its id rewrote the original; `rename` could replace an existing
  rollout; a turn could forge another in the prime. Each has a test.
- **Flake record, 2026-10-20 (known class; no change).** While verifying
  the E13 stack locally, `juggler-supervisor --test reconciler ::
  fifty_instance_soak_and_construction_regression` failed once in the
  full `just verify` (p95 tick 179 ms against its 100 ms budget, load
  average 18.6 / 20.5 / 23.3 while an a11y gate ran in parallel). It
  passed 3/3 alone at the same load and in the full `just verify` re-run
  straight after (load 9.8). Budget and assertion unchanged; CI is the
  arbiter.
- **Flake record, 2026-10-20 (known class; no change).** In #74's local
  full gate (E14 review fixes merged with main), `reconciler ::
  fifty_instance_soak_and_construction_regression` failed once on its
  wall-time budget (load average about 24, other sessions building). It
  passed 3/3 alone straight after at the same load. #74 doesn't change
  the reconciler. Budget and assertion unchanged; CI is the arbiter.
- **Flake record, 2026-10-20 (known class; no change).** In #83's
  `visual_gate.sh --update` (4 workers, linux/amd64 emulated on arm64),
  `regression.spec :: add account — light-compact-1024x680` timed out at
  30 s waiting for the Accounts nav button: the page hadn't rendered yet.
  The comparison run straight after, on the same tree, passed 137/137,
  that cell included. The test and its 30 s budget are unchanged; CI
  runs it natively on amd64.
- a11y seeded failure is recorded DEFERRED with owning task AJ-E804 —
  the gate is named in QUALITY.md vocabulary now; its absence is
  explicit, never a silent pass (per the quarantine/deferral rule).
- Flake root cause FIXED (2026-10-18, AJ-E309): the recorded
  "light-theme first-scan contrast flash" (axe color-contrast on
  freshly booted light-theme dialogs, handled so far by the one
  confirmatory re-scan) came from `styles.css`'s reduced-motion rule
  setting `transition-duration: 0.01ms` on every element — with the
  default `transition-property: all`, every element transitioned its
  inherited text colour after boot (CDP: dialog `color` = dark ink
  while `--color-ink` = light ink). Now `transition: none !important`
  under reduced motion; the wizard dialog has the correct ink on the
  first frame (evidence: `docs/qa/evidence/AJ-E309/manifest.md`). The
  re-scan guard stays; no gate threshold changed.

- Flake root cause FIXED (2026-10-18, found while verifying AJ-E704):
  `guardrails` intermittently refused a kill of the test's OWN helper
  (`foreign_machine_row_…` "local rows verify normally";
  `term_ignorer_gets_term_then_kill_only_after_grace` outcome
  `refused`) — 2 failures in 4 full-binary runs, never alone. Cause:
  discovery can record a helper between `fork` and `exec` (recorded exe
  = the wrapper `/bin/sh`, live exe = `python3`); the ledger refines
  the exe inside the 10 s exec-settle window, but upserts are throttled
  to 2 s, so a kill clicked inside that window compared the stale exe
  single-shot and refused. The reconciler's listing verdict already let
  the exe leg wait during the window; kill-time verification did not.
  Now `tree::verify_settling` applies the same rule at kill time — boot
  and start time (the PID-reuse signal) never wait, a probe error is
  still a refusal, and after the window all three facts must agree
  (`verify_tests::a_settling_helper_may_have_execd_but_never_changed_start_time_or_boot`).
  The same window bit startup recovery
  (`recovery::crash_drill_readopts_verifies_and_reports` counted 2
  unverifiable instead of 1 when the "crash" landed inside the window):
  recovery now treats a helper never observed past the window
  (`last_seen − first_seen < EXEC_SETTLE_MS`) the same way. After the
  fix: the full `guardrails` binary 3/3 and `recovery` 5/5 green. No
  gate threshold changed; nothing was retried or quarantined.
- Harness wait lengthened (2026-10-18, AJ-E705 verify): two `guardrails`
  fixtures failed with "detached fixture never became ready" while the
  machine ran at load average ~55 from unrelated work (Gatekeeper
  scanning fresh binaries, other projects' test suites) — the helper's
  cold `python3` start exceeded the harness's 10 s readiness wait. The
  same binary passed 10/10 once load dropped. The wait for a fixture to
  *start* is now 30 s; no product threshold or assertion changed.
- Real bug found through a CI failure (2026-10-18, PR #21 `verify` on
  ubuntu): `guardrails :: hog_is_never_auto_signalled…` saw the hog
  helper dead before its assertion. Cause (AJ-E512): hog sustained-ness
  = "every ring sample within 60 s of now breaches", but a finished
  tree's ring stops at root death; when more than a minute passed
  between the freeze and the sweep (a slow runner), the hog aged out,
  the single-helper RSS rule was skipped with it, and the sweep could
  auto-signal the hog. Now the window is the last minute of samples the
  tree has (a live tree's newest sample is ~now — unchanged for it).
  Regression: the same test asserts the frozen tree is still a hog ten
  minutes later (fails before the fix). No threshold changed.
- Two AJ-E708 test-design fixes (2026-10-18, AJ-E1001 verify under load):
  `fleet_sync :: a_thousand_queued_changes…` asserted every one of 1,000
  outbox writes under 10 ms and one write stalled by a loaded machine
  failed it — the budget is now the p95 of those writes (still 10 ms;
  measured well under 1 ms idle). `an_offline_console_catches_up…`
  counted edits from the test's own sync passes while the 2 s background
  tick sometimes recorded one first (199 of 200) — it now counts outbox
  sequences, which every edit takes whichever pass records it. No
  product code or budget changed.
- A real bug found through a CI failure (2026-10-18, PR #23 `verify` on
  ubuntu): `fleet_sync :: a_reset_node_gets_a_new_sync_epoch…` timed out
  — a node restarted right after Link off could not rebind its last
  port (the old listener not yet released) and fell back to another,
  stranding peers paired by address. The node now retries its last port
  for up to 2 s before falling back.
- A real latency bug found through a CI failure (2026-10-18, PR #24
  `verify` on ubuntu): `transport :: a_cancelled_handover_is_discarded…`
  missed its 1 s cancel budget. `push_handover` hashed the whole bundle
  up front on the async thread, so the push could not even start, nor
  see a cancel, until a 64 MB (debug-build) digest finished. The digest is
  now computed chunk by chunk as the bundle is sent; the budget and the
  assertion are unchanged.
- A budget measured the wrong thing (2026-10-18, found while building
  AJ-E1104): `juggler-stats :: a_week_of_counters_flushes_in_under_5_ms`
  timed the flush by wall clock, which includes the disk's write latency
  at commit. With another program saturating the disk, commits took
  8–30 ms (a Python sqlite3 control run showed the same), against 1.1 ms
  quiet. The budget is what the flush costs the app, so the test now
  takes the flush thread's CPU time (best of 5, `cpu-time` dev
  dependency) and prints wall time beside it: 0.5 ms CPU, ≈10 ms wall
  under that load. The 5 ms budget is unchanged. Also: the store now runs
  SQLite at `synchronous=NORMAL` (safe with WAL) with cached statements,
  and debug builds compile SQLite optimised (`libsqlite3-sys`
  opt-level 3).
- A test-hygiene leak fixed (2026-10-18): every `just verify` left
  `trap_helper.py` fixtures running as orphans (four after a run), and
  they accumulated across runs. The neighbour session's fixture kit in
  the app guardrail suite includes a `setsid` escapee, which no
  group-scoped stop reaches, and the cleanup killed only the first
  session's helpers; a test that failed before its cleanup leaked its
  whole kit. `FixtureSet` now kills every helper it started on drop,
  matched by the kit's own trap-log path (unique per temp dir), so it
  also runs while a failing test unwinds. After the collateral and
  guardrail suites: zero `trap_helper.py` processes left. No assertion
  changed.
- The same kind of budget, again (2026-10-18, found by the AJ-E1103
  verify run): `fleet_sync :: a_thousand_queued_changes_catch_up_inside_three_seconds`
  measured each outbox write by wall clock, which includes SQLite's
  commit to disk. On the loaded machine (load average 11–13) one full
  run saw a p95 of 19.9 ms; the test alone passes in 0.2 s. The p95 is
  now taken on the writing thread's CPU time, with wall time printed
  beside it; the 10 ms budget and the catch-up assertion are unchanged.
- A flaky test of my own, fixed before merge (2026-10-18, found by a
  verify run): `juggler-stats :: cycle::tests::leaving_share_skips_pending_reports`
  (and two others with the same shape) assembled a week an hour after
  it closed. The report's upload time is random in the 144 hours after
  close, so about 0.7 % of runs found it already due and fetched the
  config, failing "no requests". The tests now assemble at the moment
  the week closes, where the upload can't yet be due; no assertion
  changed.
- Observed again, no change (2026-10-18, twice during the E11 verify
  runs): `reconciler :: fifty_instance_soak_and_construction_regression`
  missed its wall-time p95 (416 ms and 887 ms) while other programs held
  the developer box's load at 12–16 on 30 cores, below the existing
  quarantine threshold (load above the core count). Alone it passed
  both times, as did a full `cargo test -p juggler-supervisor`; CI
  enforces the 100 ms budget strictly. The budget, the quarantine and
  the assertion are unchanged; if it recurs on an idle machine or in CI,
  it becomes a fix.
- Observed, no change (2026-10-19, rendering the D-25 baselines locally):
  `regression.spec.ts › fleet — light-compact-1024x680` and
  `settings — light-compact-1024x680` timed out in `goTo`'s first
  locator call — the last two cells of an 81-test run in the amd64
  container emulated on an arm64 Mac, with a host Playwright suite
  running beside it. Re-run alone, both passed in 30 s, and the same
  spec passed 81/81 on the host. No threshold, timeout or assertion
  changed; CI runs the gate natively. If it recurs in CI, it becomes a
  fix.
- Real bugs found through a CI failure (2026-10-20, PR #53 `verify` on
  ubuntu): `guardrails :: hog_is_never_auto_signalled…` failed its
  regression assertion ("a frozen hog tree stays a hog"). Two causes in
  the resource collector:
  1. A tick that ran after a session's root died, but before the
     supervisor marked it finished, rebuilt the tree from the dead root.
     Its helpers had been reparented, so the rows lost them, and those
     rows were then frozen as the session's last snapshot. The hog guard
     and the orphan snapshot both read them. Now a tree whose root is
     gone keeps its last live rows until it is finished.
  2. The 5-minute sample ring used `VecDeque::truncate`, which drops the
     newest sample. After 150 samples a session's ring stopped advancing:
     the sparkline froze and the hog window read 5-minute-old samples.
     It now drops the oldest. Regression:
     `supervisor :: the_sample_ring_keeps_the_newest_five_minutes`
     (fails with the old code).
  No threshold or assertion changed.
- A test-design fix (2026-10-20, a full-suite run for PR #53):
  `keyboard-flows :: switch, 1-click` tabbed toward the ranked switch
  rows while they were still loading under load ("never reached …").
  It now waits for the rows to be visible, then tabs; the keyboard path
  and its assertions are unchanged.
- A test-design fix (2026-10-20, PR #55 `ui` on ubuntu):
  `first-run-policy :: Decide later writes NOTHING…` waited with
  Testing Library's 1 s default for the wizard's summary, which follows
  two mocked imports; on a loaded runner it arrived after 1,049 ms.
  The wizard steps in that file now wait up to 5 s. No assertion changed.
- Real accessibility bugs found by the release gate (2026-10-20, Promote
  for 0.0.5, `a11y (1/4)`): the D-25 rail rows failed WCAG contrast on
  every screen. Ended rows were faded with opacity-70, which took their
  faint-grey second line to 3.29:1 (dark) and 2.84:1 (light), and the
  desktop "app" tag used light accent text at 9.5 px (4.33:1 / 3.3:1).
  Now ended rows have a quieter title instead of opacity (the real fix),
  the tag uses muted text, and the second line also moved to the muted
  grey (≥ 5.8:1; the faint grey alone passes). Corrected 2026-10-20: the
  first note blamed the faint grey; a seeded regression showed the grey
  alone passes and the opacity is what fails. The
  wizard's "unverified" chip blended to 4.22:1 on the selected row in
  light mode and now has a solid backing. 0.0.5 is not promoted; 0.0.6
  carries the fix. The gate passes locally (939 states, 0 serious). One
  capture timed out while the visual container ran beside it and passed
  twice alone.
- **Flake record, 2026-10-20 (open, local only).** vitest
  `sorting.test.tsx` › "sort+filter of 200 rows stays inside one frame
  (<16 ms)" took 89 ms once while a full Playwright run of another
  checkout (and other projects' suites) loaded the Mac (load average
  ≈ 20–25). Alone it passed 15/15 at once. The budget is untouched. If it recurs in CI, measure the
  median of several runs instead of one.
- Real bugs found by the maintainer on 0.0.7 (2026-10-20, dogfood),
  none caught by a suite:
  1. Bringing in six Claude setups failed at `skills/ego-browser`, a
     symlink to a folder: the import copied only regular files and links
     to regular files. It now recreates each link as a link (never
     followed; an absolute link inside the setup is pointed at the copy)
     and lists it in the result. Regression:
     `claude_environment :: linked_folders_stay_links_and_linked_files_are_copied`.
  2. No hand cursor anywhere: the shared Button and several rows set
     `cursor-default`. Fixed at the source and in a base rule for every
     native control. Regression: `cursor.spec.ts` checks every enabled
     clickable element on each main screen, the More menu and the legend;
     it failed with 115 elements on the old styles.
  3. The bring-in screen had no way out at first run (Close only in add
     mode, Escape only in add mode). Regression: `first-run.test.tsx ::
     first run can be left too…`.
- A real security bug found by review (2026-10-20, review of Codex PR #64,
  pre-existing in the Claude adapter since AJ-E203/E706):
  `ClaudeCodeAdapter::install_bundle` joined each bundle file's name and
  the manifest's session id into paths unchecked. A bundle can arrive from
  a paired Mac (Fleet `TakeHandover`), so a name like `../../…` or an
  absolute path could write outside the target account's folder, and an
  existing file could be replaced. Now: only one plain file name per file
  and a safe session id are accepted, refused before anything is written;
  an install never replaces a file the target already has; the replay
  brief quotes each turn so text in a turn can't pose as another turn.
  Regression: `claude_handover :: an_untrusted_bundle_cannot_write_outside_the_target`
  (fails on the old code) and `a_turn_cannot_forge_another_turn_in_the_brief`.
- Real bugs found while building AJ-E1024 (2026-10-20, the maintainer's
  bring-in report plus code reading):
  1. A launch setting with a model replaced the account's whole
     `settings.json` with `{"model": …}` on every launch, wiping hooks,
     permissions and the status line in copies. Now `claude --model`.
     Regression: `in_place_accounts :: a_copy_keeps_its_settings_file_when_a_profile_sets_a_model`.
  2. One failed setup trapped the bring-in: no Continue, "Try again"
     reran all of them and lost the others' sign-ins, and closing
     mid-way kept importing unseen. Regressions: `first-run.test.tsx`
     (Continue, Try again, Stop).
  3. A copy whose account record failed to save left its copied folder
     behind (no sweep). It is now removed at once (marked copies only).
  4. Found by the review of AJ-E1024, pre-existing since AJ-E607: the
     managed status line `command` was the bare wrapper path under
     "~/Library/Application Support/AI Juggler/…". Claude runs it through
     the shell, which split it at the space, so the wrapper never ran and
     no Measured quota sample was ever captured on a real Mac (the
     maintainer's five copies had empty `quota-spool/`). The command and
     the spool path are one quoted shell word now; old installs are
     re-pointed with the user's chained line kept. Regression:
     `quota_ingest :: the_status_line_runs_from_a_folder_with_spaces`
     (fails on the old code) and `an_unquoted_install_is_repaired_and_keeps_the_chain`.
- **Flake records, 2026-10-22 (fixed in fix batch 5).** Three tests that
  measured the machine rather than the product:
  - `ports :: port_inventory_classifies_orphan_and_unmanaged_listeners`
    bound fixed ports 5173 and 8000, and a developer's own `vite` on 5173
    made it read "not-managed". It now takes two free ports from the OS.
  - `orphan :: collector_targeted_tick_overhead_under_budget` timed the
    wall clock (32 ms against 20 ms at load 17–24). It now asserts the work
    exactly (each of our processes once a targeted tick, a listing at most
    every 2nd) and this thread's CPU time against the 1 % budget: 20 ms
    release, 30 ms debug, 40 ms on CI, measured again only on a busy
    machine (6 ms here).
  - `updates_plugin :: a_package_that_fails_its_signature_never_installs`
    required progress steps of at most 5 %; how big a step is depends on
    how the bytes arrive, and a loaded machine read several 32 KB pieces at
    once. It now requires progress that only rises, in at least 20 steps.
- **Flake record, 2026-10-22 (fixed).** `switch_back_path ::
  a_to_b_to_a_resumes_original_with_one_delta_and_no_copy_back` failed once
  in `just verify` for #130 ("exactly one delta turn": 0): its `claude` stub
  wrote `last-args` in place with `printf >`, and the test's poll could
  read it half-written, the race parked_reset's stub had (2026-10-19).
  The stub now writes a temporary file and moves it into place; so do the
  seven other tests whose stubs wrote the same way (golden_path,
  switch_door, validation_gate, switch_graph_targets, identity_guard,
  fleet_switch, redaction_policy). Assertions unchanged.
- **Flake record, 2026-10-22 (local only, again).** `processes ::
  processes_command_reports_tree_orphans_and_stops` ("never saw the child")
  failed once in `just verify` for #130 at load average 14–17 with other
  sessions building; it passed 6/6 alone at once. The test has the fake
  harness's lifetime (3 turns of 300 ms) to see its child before the
  child is orphaned, and a loaded machine can spend that before the first
  ticks. Pre-existing (the record below); no budget or assertion changed.
- **Flake record, 2026-10-20 (local only, load average 33–45 while three
  sessions ran full suites).** `processes :: processes_command_reports_tree_orphans_and_stops`
  ("never saw the child") and `reconciler :: helper_exiting_inside_grace_never_surfaces`
  timed out once each in a `cargo test --workspace` run; both passed
  alone at once (3/3 each). No budget changed.
- Observed, no change (2026-10-20, E14 review fixes on PR #67, with three
  other sessions building on the same Mac at load average 18–31):
  `reconciler :: fifty_instance_soak_and_construction_regression` missed
  its wall-time p95 (854 ms against 100 ms, load 18.1/20.2/23.4). In one
  earlier full run, `orphan :: orphan_detected_after_grace_and_stopped_gone`
  and `orphan :: stop_ladder_escalates_when_sigterm_ignored` timed out
  their waits (load 30.7). Re-run alone, `--test orphan` passed 5/5; the
  rest of the workspace passed (117 suites with `--no-fail-fast`). Neither
  binary touches the E14 change: the machine sampler is not in either. No
  budget, timeout or assertion changed; CI decides.
  Again on the #68 review fixes (load 28): those two `orphan.rs` waits plus
  `collector_targeted_tick_overhead_under_budget` failed inside the
  parallel workspace run, and passed 5/5 alone twice at load 21–23. The
  two waits had already failed the same way on #67, which doesn't have
  the collector change. The collector's added work (CPU+memory refresh of
  observed trees) is zero when no tool is observed, as in that test.
- Real bugs found by the maintainer on 0.0.8 (2026-10-20, dogfood):
  1. With setups brought in and no session run yet, the rail listed every
     project from their transcript history as a flat row with a hard-coded
     "0" under "No sessions yet". It read as broken, and Claude worktrees
     (`.claude/worktrees/…`, `~/.t3/worktrees/…`) showed as separate
     projects. Now history-only projects are a labelled, foldable
     "Projects" list of rows built like session rows (when, conversations,
     accounts); worktrees map to their repository (`worktree_repo_root`,
     at launch and in the backfill, with earlier worktree-only rows
     removed when unreferenced); clones group by their `origin` remote.
     Regressions: `projects_backfill_worktrees` (Rust) and
     `rail-projects.spec.ts` (Playwright: no "0", grouping, folding
     remembered, axe).
  2. A 0.0.7 import that failed on one of six setups had brought in the
     other five as copies, but its screen offered no way on, so the
     maintainer believed nothing had come in; 0.0.8 then listed their
     history. (The all-or-nothing screen was fixed in #76.) At the
     maintainer's request their AI Juggler data was moved aside, not
     deleted, for a fresh start.

- **Bug record, 2026-10-20 (fixed; AJ-E1010).** An older AI Juggler that
  meets data a newer one saved (schema too new) **quit at launch with no
  window and no message**: `run()` printed the refusal to stderr and
  exited. Going back from 0.0.8 to 0.0.7 hits it (0.0.8 adds schema 28),
  and `docs/RELEASING.md` claimed the old version "says so". Found by the
  rollback rehearsal (`AJ-E1010/rollback-rehearsal.md`, step 3). Fix:
  the launch now says so in a window and offers the pre-update backup
  (`newer_data.rs`; regression tests `--test newer_data`, which fail on
  the old code because nothing offers or restores). 0.0.7 itself can't
  be changed, so 0.0.8's notes carry the downgrade note (Known issues).
- **Bug record, 2026-10-20 (fixed; AJ-E1010).** Found walking the update
  UI by hand: (1) the release-notes box scrolls but had no keyboard
  focus — with notes longer than the box, axe reports
  `scrollable-region-focusable` (serious); the existing a11y test used
  two-line notes and never saw it (the new long-notes test fails on the
  old markup); (2) the Home notice said "(beta)" whatever the channel;
  (3) a refused restart read "invalid update: …"; (4) Review opened
  Settings at the top, with the update below the fold; (5) "Restart now"
  was offered during a switch or a bring-in. All fixed, each with a
  vitest or Playwright check.
- **Flake record, 2026-10-21 (fixed).** CI's `verify` on PR #120 failed
  twice in `juggler-adapter` `user_path :: a_login_shell_that_prints_its_path_is_read`
  ("left: None"). The second time it failed even with a 30 s budget, and
  the suite finished in 0.30 s, so this is not a timeout. The test writes
  a stand-in shell and runs it while other tests in the binary spawn
  processes. A child forked during that write holds the file open until it
  execs, so running the script fails at once with ETXTBSY, and `login_shell`
  reads that as no PATH. The fix is in the tests, where the race starts:
  their `script()` helper writes each stand-in from a child `sh` that exits
  before the test runs it, so this process never holds the file open for
  writing and no fork can inherit it. `login_shell` itself is unchanged
  (the app's shell is never a freshly written file). The two tests that
  check what is read also allow 30 s instead of the app's 2 s. Not
  reproduced locally; 3 of 3 local runs passed. The other 33 sites that
  write stand-in executables in tests (mostly `juggler-app/tests`) haven't
  flaked; moving them to the same pattern is in the next batch.
- **Flake record, 2026-10-21 (known class; open, next batch).** The same
  PR's CI `verify` (first attempt) failed `load_budgets ::
  monitoring_costs_under_one_percent_of_a_core` at 1.118 % of a core
  (budget 1 %). The E14 machine part, which PR #120 touched, was 0.055 %;
  the rest is the per-session collector, which it didn't change. To be
  made load-aware (CPU time and a load gate) in the next batch, with the
  guardrails flakes.
- **Flake records, 2026-10-22 (fixed, fix/maintainer-reports-3).**
  - `guardrails :: term_ignorer_gets_term_then_kill_only_after_grace`
    (CI on #121: "exited on TERM") and
    `manual_pgid_scope_is_warned_and_confined_to_the_group` (CI on #101).
    `tick_until_helpers` took the first ledger row whose command line held
    a fixture's flag. A background job's first recorded command line is
    the root shell's, which holds every fixture's flags, so the row could
    be another fixture (the cooperative vite answered the TERM). Each
    fixture now writes its own pid (`<ready>.pid`) before it is ready, and
    the tests signal exactly those pids once the ledger lists them. 10 of
    10 guardrails passed locally; the supervisor's collateral and orphan
    suites still pass.
  - `machine_lock :: a_slow_disk_tick_never_holds_the_machine_lock`
    sampled `try_lock` in a loop and could catch the tick's own short
    lock. It now waits for the tick's count to move (its locked part is
    over), then checks a blocking `lock()` returns while storage is still
    held. That can't happen if the tick held the lock across the wait.
  - `load_budgets :: monitoring_costs_under_one_percent_of_a_core`
    asserted CPU %, which measured the CI runner (1.118 %). It now asserts
    the work that holds the budget: a full scan at most every 5th tick,
    and between them only our sessions' processes
    (`LiveStatsCollector::work`). CPU is still printed.
- **Flake record, 2026-10-22: a product bug (fixed in fix/maintainer-reports-6).**
  In the batch 5 verify of 7bcd862b (fix/maintainer-reports-5), with other
  sessions' builds running,
  `fleet_sync :: an_offline_console_catches_up_with_only_what_it_missed_and_keeps_last_known_rows`
  failed with the catch-up log `[(0, 10), (31, 5)]`: the console asked
  from 31, past the node's top sequence when its Link went off. This
  record's first two readings were wrong:
  - "a late first catch-up": the test reads `first[0]` before counting;
  - "a batch already on its way": seq 31 didn't exist until the node's
    edits, which began after the console's Link was off.

  The coordinator's review found the cause. `stop()` aborted the
  reconcile loop only, and dropping its map of connections detached
  (didn't end) each connection's heartbeat and telemetry tasks. So the
  console kept streaming, and kept writing the node's changes into its
  replica, after the user had turned Link off. It shows only when the
  node's own 2 s pass sends a change; a test's direct sync pass records
  without sending.
  - **Fix:** `stop()` now ends the loop, then every connection's tasks,
    and waits for each, so nothing is applied after it returns.
  - **New test:** `a_console_with_its_link_off_applies_nothing` failed
    2 of 2 on the old code (the mark went from 1 to 5) and passes on the
    new.
  - **Original test:** its `resumed.0 <= high_water` check, which caught
    this, is unchanged; a direct check that the mark stayed at or below
    `high_water` while Link was off is added.
- **Flake record, 2026-10-22 (fixed, fix/maintainer-reports-5).** CI on
  PR #132 (run 39821992531) failed
  `guardrails :: idle_nudge_fires_exactly_once_per_orphan`, "the idle
  nudge fired", after 112 s; locally it passes, even with 60 busy
  processes on 30 cores. The fixed 1,200 ms sleep never gated anything:
  the nudge's age counts from the helper's process start, already more
  than 3 s back. What can hold it is the CPU condition (under 1 %). The
  read used the collector's last sample, and on Linux CPU time comes in
  10 ms clock ticks, so a helper that wakes every 0.2 s can catch one in
  a short window and read 1 % or more. The test now ticks the collector
  until the product's own conditions hold (orphaned, aged past the
  threshold, CPU under 1 %), within 30 s. Past that, it fails naming the
  state, age and CPU it last saw. Then one read must fire and the next
  must not. No assertion changed; the cause isn't reproduced here, so a
  repeat names the condition.
- **Hang record, 2026-10-22 (fixed, fix/maintainer-reports-5).** In
  Linux CI, `load_budgets :: monitoring_beside_a_busy_claude_app` (from
  #130) sat for an hour while the other three budgets waited their turn
  behind it. On main's own run after #130 merged (39814972300), the file
  "finished in 3605.30s". #131's first run (39830985754) was cancelled
  after 36 minutes. The stand-in Claude app (`fake-desktop`) exits on its
  own after 3,600 s. Its guard sent `/bin/kill -9 -<pgid>` and then
  waited for it. Linux's `kill` doesn't read that form as a process
  group, so the wait lasted the app's full hour; macOS reads it, which is
  why it never showed here. The guard now sends
  `kill -s KILL -- -<pgid>`, a form both systems read as a group, then
  kills the app itself through `Child::kill`, so the wait can't outlast
  it. On macOS the test passes in 1.9 s with no `sleep 600` left over.
  The Linux side is shown by CI. No product code changed:
  `juggler-supervisor/Cargo.toml` in #131 adds only a dev-dependency
  (`cpu-time`), and `sysinfo`'s features are as on main.
- **Leak record, 2026-10-22 (fixed, fix/maintainer-reports-5).** The
  coordinator found and stopped 29 orphaned test fixtures on the
  developer Mac, their temp folders already gone: 24
  `python3 -m http.server` from `juggler-app` tests/ports.rs and 5
  `shell` fixtures (`shell.traps`, up to 4 days old) from
  `juggler-supervisor` tests/collateral.rs. Their cleanup ran only on the
  success path, so a failed assertion, or a run killed outright, left
  them running. The `shell` fixture loops forever and ignores TERM by
  design. The fix is in the tests only:
  - a shared `fixtures::Reaper`, which on drop (a panic's unwind
    included) sends `kill -s KILL -- -<pgid>` to each process group
    registered with it and SIGKILL to each pid. ports.rs registers the
    supervised root's group (its helper stays in that group after the
    root dies) and the stranger; collateral.rs registers both trees and
    the unmanaged server, the moment each is spawned;
  - a lifetime cap, for a run killed outright where no `Drop` runs:
    `FIXTURE_LIFETIME_SECS` (300 s, the same as `trap_helper.py`'s
    existing cap). The `shell` fixture now beats 300 times instead of
    forever, ports.rs's servers end themselves through a timer, and the
    busy-app stand-in in load_budgets ends after 300 s instead of an hour
    (its children sleep 300 s, not 600 s);
  - `juggler-supervisor` tests/reaper.rs: a test that panics with a
    group (a shell and two children) and a lone process registered
    checks that both were ended by SIGKILL and that the group is empty.

  The collateral, ports, load_budgets (6) and guardrails (10) suites
  pass, with no fixture left behind.
- **Flake record, 2026-10-22 (fixed in fix/codex-compaction-record).**
  In #134's verify (bb39112c), at load average about 21, with other
  sessions' builds running,
  `juggler-supervisor :: machine :: tick_cost_stays_inside_the_budget`
  read p95 2,570 µs against its 2 ms budget. That is wall time, which
  counts other programs' turns and disk waits. Alone it reads p95 about
  44 µs, and it passed 4 of 4 beside 60 busy processes, so this isn't
  reproduced. The p95 budget and the under-the-lock p95 are now this
  thread's CPU time, with wall time printed beside them; the 20 ms
  wall-time maximum under the lock still catches the disk listing it
  guards against. Same approach as #131's orphan tick budget.
- **Timing sweep, 2026-10-22 (fix/timing-budgets).** The coordinator's
  load-heavy run failed release/0.1's
  `manage_projects :: relocate_repoints_a_moved_folder_and_refuses_a_missing_or_taken_one`
  at 419 ms against a 100 ms wall budget. That test is only on
  release/0.1; its `timed` helper and the fix are for the release
  session. On main, the tests that assert tight wall-clock budgets were
  swept:
  - `manage_accounts`: `BUDGET` is 200 ms over rename, remove and set
    default. These are synchronous, so the check is now this thread's
    CPU time (`within_budget`), with wall time printed beside it. They
    measure about 0.4–0.9 ms.
  - `juggler-link` `registry :: fifty_peers_load_fast_and_stay_small`:
    a 50 ms load budget, now this thread's CPU time.
  - `transport :: mutual_tls_with_pinned_keys_connects_and_serves`: the
    300 ms LAN handshake budget runs across the runtime's threads, so
    it stays wall time (what a user waits), passing if any of up to
    three tries fits.

  The budgets of a second or more (session launch, preview, cancel,
  revocation) are left alone. None was seen to flake; a repeat would
  make them the next ones.
- **Flake record, 2026-10-22 (open; a test-design weakness).** In #143's
  verify (5c3c4446), at load 34, with many Claude Code sessions running
  on this Mac,
  `load_budgets :: monitoring_beside_a_busy_claude_app` failed
  "read at the listings only: 7125 for 12 listings of 543 processes":
  the bound is 12 × (543 + 30) = 6,876. The observed trees include this
  Mac's real AI tools. Their size is taken once, at the end of the
  simulated minute, while other sessions' builds start and end processes
  in them during it, so the end size understates what each listing read.
  Alone it passed 3 of 3 (8,905–10,591 read), and the rerun was green.
  Fix proposed to the coordinator: bound each listing by the trees' size
  at that listing (or the largest seen), not the size at the end.

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


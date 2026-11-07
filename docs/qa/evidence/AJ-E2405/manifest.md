# Evidence manifest — AJ-E2405 Automatic rules shown, previewed and told

Task: `docs/tasks/E24-processes.md` AJ-E2405. Re-checked against the code
and its tests on `release/0.4.1`, 2026-11-07: the dry run and policy
(`crates/juggler-app/src/features/processes/rules.rs`), the sweep
(`features/cleanup/sweep.rs`), the marks (`features/processes/commands.rs`
`mark_impl`, `juggler-core` `storage/sightings.rs` `process_marks`), the
notification (`features/notifications/mod.rs` `left_behind`), the history
(`commands.rs` `history_impl`, `ui/src/screens/processes/HistoryView.tsx`)
and the snapshot (`ended.rs` `snapshot_json`).

Cargo and Playwright tests are cited as written and run in CI; the vitest
files were run here: `cd ui && pnpm -s exec vitest run
src/screens/system-notifications.test.tsx src/screens/processes-page.test.tsx`
→ 2 files, 17 tests passed.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the per-account tiers as they are (ADR-0010); the page shows each account's rules, pause and the crash hold | `rules.rs` `policy` (each account's `tier1`/`tier2`, `paused`, `suspended_until_ms`); `RulesPanel.tsx` (Pause all / Resume, "Held after a restart until you confirm it in Settings › Advanced."); `capture.spec.ts` "tiers — centre paused banner — ${vp.name} — ${theme}", "tiers — suspended after crash — ${vp.name} — ${theme}" (both on the Processes page); `_gallery/21-tiers/{paused,suspended}-1440x900-*.png`; `_gallery/90-processes/rules-preview-1680x1000-{dark,light}.png` | each account's two rules, Pause all, the hold | `rules-preview-1680x1000-dark.png` opened: Work Max, Personal, ChatGPT Pro each with "Tidy left-behind dev servers" and "Also after a crash", Preview beside each, Pause all; the paused and held banners are the 21-tiers captures | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R2 a dry run before enabling uses the sweep's own rule (`evaluate_tier`) and exclusions (heavy, Keep): now, later (its grace), never (why) | `cargo test -p juggler-app --lib features::processes::rules::tests::the_dry_run_uses_the_sweeps_own_rule`, `…::a_crash_needs_the_second_rule`; `processes-gallery.spec.ts` "processes — automatic cleanup's dry run — ${theme}" | now/later/never as asserted; a crash needs tier 2 | `preview` calls `juggler_supervisor::tree::evaluate_tier` with the sweep's inputs; the capture reads "Would tidy esbuild, node (nodemon), python3 (http.server) :8000 now" and "node (vite) :5173: It is heavy: you decide, never a rule", "A preview: nothing was stopped". Claude Code's background tasks: ruled 2026-11-07, see below | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R3 Keep and Remind me later are stored marks; the sweep SHALL never tidy a kept process | stored: `cargo test -p juggler-core --test sightings_storage a_reminder_lapses_and_keep_stays`; `cargo test -p juggler-app --test processes_live a_session_and_what_it_leaves_behind_on_the_one_model` (step 3: `mark_impl(.., "keep")`, `to_clean` falls by one, `is_kept`); the dry run says never for Keep (`the_dry_run_uses_the_sweeps_own_rule`, `list[4].kept`) | marks stored; a kept orphan never signalled by the sweep | the marks are stored and read back. **The sweep's own Keep check (`sweep.rs`: `if state.processes.is_kept(..) { continue; }`) has no test**: `processes_live` asserts `is_kept` but never runs `auto_cleanup_sweep`, and no `guardrails.rs` flow marks a candidate Keep. Missing: a sweep test with Tier 1 on, a kept orphan past its grace, and zero signals / no audit row **Closed 2026-11-07:** `cargo test -p juggler-app --test tiers a_kept_process_is_never_tidied` (Tier 1 on, a kept orphan past its grace: `auto_cleanup_sweep` sends zero signals and writes no audit row; Keep cleared, the same sweep tidies it) | PASS | fd4602dda | 2026-11-07 | Claude (self) |
| R4 a macOS notification when a session leaves processes running, once per session, through Settings › Notifications (a new kind, off by default) and its quiet hours | `cargo test -p juggler-app --test system_notifications a_session_that_left_processes_running_is_told_once` (off: nothing; on: one, "Left 3 processes running · Personal"), `off_by_default_nothing_is_shown`; `src/screens/system-notifications.test.tsx` "offers six kinds, all off until chosen" ("A session leaves something running"), "saves each kind on its own and says what changed" (`left_behind: false`) | off by default; told once; quiet hours | vitest pass; once per session: `processes/mod.rs` `helper_orphaned` / the `orphaned` map, told after `NOTIFY_AFTER_MS`, then removed. Quiet hours: `left_behind` goes through the same engine (`observe` → `show_due`) that `quiet_hours_hold_it_until_they_end` proves for the session kinds; no test holds a `left_behind` notification in quiet hours, nor calls it twice for one session | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R5 Cleanup history filterable by session and project, paged | `cargo test -p juggler-app --test processes_live a_session_and_what_it_leaves_behind_on_the_one_model` (step 5: `history_impl(.., Some(session), None, ..)` lists the stop); `src/screens/processes-page.test.tsx` "Ended reads how long each ran; Cleanup history names the session"; `_gallery/90-processes/history-*.png` | by session; by project; a page at a time | by session: pass (the session tab's Cleanup history passes `sessionId`). **By project: not reachable** — `history_impl` and `HistoryView` take `projectId`, but `ProcessesView.tsx:230` renders `<HistoryView sessionId={sessionId} />` only and no Project filter exists on the history view; no test filters by project. **Paging: untested** — the cursor (`next_cursor`) and the "Load more" button exist, but no test pages past the first `limit` rows **Closed 2026-11-07:** by project: `HistoryView` has a Project picker (all of history; a session's own history has none), passed by `ProcessesView`; vitest `src/screens/processes/history-filter.test.tsx` "filters the whole history by project", "a session's own history has no project filter"; paging: "pages back with Show older, from where the last page ended" (the second read carries the first page's `next_cursor`) | PASS | abe4edcb3 | 2026-11-07 | Claude (self) |
| R6 export a process snapshot: names, roles, states, numbers and keyed hashes of command lines only | `cargo test -p juggler-app --test processes_graph the_snapshot_holds_names_and_hashes_and_never_a_title_folder_or_line`; `cargo test -p juggler-app --lib features::processes::sightings::tests::the_hash_is_keyed_and_never_the_line`; the page's "Export snapshot" (`ProcessesScreen.tsx`, `processes_snapshot` → `snapshot_impl`) | no title, folder or command line; a keyed hash per process | the button shows in `attention-1680x1000-dark.png` | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| Gate: `cargo test -p juggler-app --lib features::processes::rules`, `--test system_notifications`, `--test processes_live` | the task's quality gates (run in CI) plus the vitest files above | pass | vitest 17/17 pass; cargo runs in CI | PASS | f6fb096cb | 2026-11-07 | Claude (self) |

Not measured: the dry run's budget (≤ 50 ms). `dry_run` reads the graph the
tick already built and one `session_rows` per session; no test times it.

## Ruled: Claude Code's background tasks in the dry run

**Decided 2026-11-07 by the goal (1cb26528d; recorded as an amendment to
ADR-0010; its D-number is the main session's to log): option 4 with
option 2's leader check.** A helper whose group escaped is eligible only
under "Also after a crash" (Tier 2: its notification and Undo are the
consent), and only when its group's leader is a helper the ledger recorded
for the same session on the same boot (`juggler_supervisor::tree::group_is_sessions`).
Tier 1 still never touches an escaped group. Generic (no per-tool branch).
The dry run says it in plain words: "Its session started it in a group of
its own: only after a crash (the second rule)". Tests:
`cargo test -p juggler-supervisor --lib tree::tier_tests::a_group_its_session_made_is_tidied_only_after_a_crash`,
`tree::tier_tests::a_group_is_the_sessions_only_when_its_own_helper_leads_it`
(a forged foreign leader stays excluded), `cargo test -p juggler-app --lib
features::processes::rules::tests::a_group_its_session_made_is_the_second_rules_only`.

What follows is the analysis the ruling was made from.

Listed for the maintainer in `docs/qa/0.4-completion.md` §4 item 6.

**What the code does.** Claude Code starts every Bash tool command with
Node's `detached: true` (lab 1, read in its installed program), so the
shell it runs is a new session and process-group leader, and a background
task's server is in that group. The ledger's helper upsert
(`juggler-supervisor/src/live_stats.rs` `ledger_upserts`) sets
`escaped_group = live pgid ≠ the root's pgid`, sticky (`MAX`) in
`process_ledger`. `evaluate_tier` (`juggler-supervisor/src/tree.rs`) checks,
in order: root, state `orphaned`, role `DevServer|Watcher`, **escaped
group**, machine, then the root's exit kind. The role is
`kill::guess_role(exe, cmdline)`; the Bash shell's command line holds the
command, so `zsh -c … python3 -m http.server` guesses `DevServer` (an `npm
run dev` shell guesses `TerminalShell`). Nothing in the processes or rules
code reads the transcript's `run_in_background` flag
(`juggler-adapters/src/claude_subagents/own.rs` keeps it only for the
subagents view).

**What a dry run says** for a Claude Code background task left behind (the
crash route, the only way one is left):
- the server: `never`, "It left its session's group" (at tier 1 and 2);
- its shell: `never`, "It left its session's group" when its line names a
  dev server, else "Not a dev server or a watcher";
- the sentence: "Nothing to tidy now". The sweep does the same: it never
  tidies one.

Without the group rule, the crash route's root exit (a signal) is a crash,
so only "Also after a crash" (tier 2, 5 min and the Undo) would apply;
"Tidy left-behind dev servers" (tier 1) needs a clean exit, and on a clean
exit Claude Code has already stopped its tasks itself.

**Options:**
1. **Keep the rule (D-08 as written).** No change to the sweep. Change the
   dry run's words for this case ("Claude Code runs it in a group of its
   own, so no rule stops it; use Stop…") and the lab's expectation. Safe;
   the rules then never tidy anything a Claude Code session started.
2. **Count a group led by the session's own recorded helper as the
   session's.** `escaped_group` false when the helper's live pgid is the
   pid of a helper row the ledger recorded for the same session and boot
   (with its start time), so the group leader is provably the session's
   child. Generic (no per-tool branch), the identity facts stay the
   ledger's. Needs a decision entry (ADR-0010 calls escaped-group a hard
   exclusion) and a guardrails test with a forged foreign group. Misses a
   leader that died before the 2 s tick saw it (the `nohup … &` gap).
3. **An adapter-declared fact.** The Claude Code adapter declares that its
   tool commands run detached; the evaluator waives the group rule only for
   that adapter's sessions, with the same leader check as option 2.
   Narrower, but a per-tool fact in the rule path (allowed only through the
   adapter seam).
4. **Tier 2 only.** Allow such helpers only under "Also after a crash"
   (its notification and Undo are the consent), never tier 1. Matches when
   they exist at all (crashes); still needs option 2's leader check.

## Lab 1 (2026-10-26) rows, still true

- Dry run on a real leftover (step 6): "Nothing to tidy now", both rows
  `never`, "It left its session's group", at tier 1 and tier 2; nothing
  stopped. Expected by the code (above); the checklist's "Would tidy
  python3 now" was wrong for Claude Code; the rewritten lab (AJ-E2406 R3)
  expects the ruled words.
- Keep in the preview: "You chose Keep", `never`; `tier1`/`tier2` false
  for every account. PASS.
- History by session: one row, `user:kill_sheet`, "exited on TERM",
  `ports_before` 8765, named by the folder, not the session's name
  (finding 5). PASS.

## Binary artifacts

None added; cited: `_gallery/90-processes/rules-preview-*.png`,
`history-*.png`, `_gallery/21-tiers/*.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)

# Evidence manifest — AJ-E2401 The process graph: one model of apps, sessions and processes

Task: `docs/tasks/E24-processes.md` AJ-E2401 (no amendments). Re-audited
against the code and tests on `release/0.4.1`, 2026-11-07, replacing the
2026-10-25/26 manifest from branch `e24-processes`. The graph is
`crates/juggler-app/src/features/processes/` (model, observe, query, roles,
mod, dto, commands, ended); it shipped in Release 0.2.0 (f6fb096cb) and is
built on the "processes" 2 s tick in `crates/juggler-app/src/lib.rs`
(`features::processes::tick::processes_tick` then `features::processes::tick`,
which emits `AppEvent::ProcessesChanged`).

Cargo and Playwright tests below are cited as written and **run in CI**; they
were not run for this audit (shared Mac). The vitest files were run here:
`cd ui && pnpm -s exec vitest run src/screens/processes-page.test.tsx src/screens/processes/session-rows.test.tsx`
→ 2 files, 16 tests passed (2026-11-07).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 built on the existing 2 s tick from the ledger's live rows, the collector's table/trees/rings/listeners and the live view's sessions; no scan loop of its own | `cargo test -p juggler-app --test processes_live a_session_and_what_it_leaves_behind_on_the_one_model` (runs in CI): a real fake-harness session, the graph read only through `processes::tick` (asserts "the graph said it changed"); code: `lib.rs` "processes" tick (2 s) calls `features::processes::tick` after `processes_tick`; `processes/mod.rs` `tick` reads `ledger.live_rows`, `supervisor.live_roots`, `snapshot_table`, `observe::collector_view` (one collector lock), `conversations.live_model()`; other apps' trees are handed to the collector as `set_observed_extra` (measured by its own listing); `scripts/polling_audit.py` (CI) | no scan or timer of its own | no `sysinfo` refresh or loop in `features/processes/` except `stop::read_now` (the D-41b click-time targeted read, not a tick); the test passes in CI | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R2 nodes: app (`host_of`, AI Juggler for its own) → session (ledger root / PTY / driven; theirs: a live session's verified pid) → processes in tree order | `cargo test -p juggler-app --test processes_graph whose_and_how_each_may_be_stopped` (`graph.apps[0].name == "AI Juggler"`), `needs_attention_shows_what_needs_the_user_under_its_parents` (theirs: "In a session An App runs"), `everything_running_folds_healthy_sessions_to_one_line` ("app 0, session 1, claude 2, npm 3, vite 4"), `a_desktop_app_s_shared_tree_is_counted_once_as_the_app_s`; `cargo test -p juggler-app --lib features::conversations::places::tests::a_session_a_tool_starts_is_in_the_app_running_that_tool` (`host_of`); `observe.rs` `theirs_sessions` names the app from `place.host` | as asserted | tree order and depth as asserted; the app is the host's name ("Terminal" when none) | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R3 states: Working, Idle, Left behind, Left behind while closed, Picked up after restart, Zombie (names the parent, no stop), Didn't stop (failed stop, or a session marked ended whose own process runs), Won't be touched, Ended (7 days) | `cargo test -p juggler-app --test processes_graph every_state_is_in_the_one_vocabulary` (Working, Idle, LeftBehind, PickedUp, LeftBehindClosed, Zombie with `waits_for` 400 and `StopPath::Never`, DidntStop for a failed stop, WontTouch); `--test processes_live a_session_and_what_it_leaves_behind_on_the_one_model` step 4 (the stopped child reaches the Ended view); `cargo test -p juggler-supervisor --lib zombies::tests::a_real_zombie_is_read_with_its_parent` (macOS: a real zombie is read with its parent, the lab's bug 4 fix) | every state and both Didn't-stop causes evidenced | all states but two clauses evidenced. **Missing:** (a) Didn't stop for "a session marked ended whose own process runs" (`model.rs` `is_root && session.marked_ended` → `ProcState::DidntStop`) has no test: the fixture's `s-left` is `marked_ended` but its root 200 is gone; (b) Ended's 7-day window (`commands.rs` `KEPT_MS`, `ended_page`) has no test that a row ended over 7 days ago leaves the Ended view **Closed 2026-11-07:** (a) `cargo test -p juggler-app --test processes_graph a_session_marked_ended_whose_own_process_runs_didnt_stop` (its own process alive, the session marked ended → `ProcState::DidntStop`); (b) `cargo test -p juggler-core --test storage_roundtrip ended_rows_keep_to_their_window_newest_first` (a row ended more than 7 days ago leaves the Ended view; the rest newest first) | PASS | 1f5407222 | 2026-11-07 | Claude (self) |
| R4 facts per process: role in words ("probably"), CPU now + sparkline, memory now and peak, ports, age, CPU time, whose, what started it from the tool's own events (Claude hook; Codex `item/started`), in memory only | `cargo test -p juggler-app --test processes_graph whose_and_how_each_may_be_stopped` (role words "dev server", "MCP server", "watcher"; `label == "Started by a command: npm run dev"`, none from a command line); `needs_attention_shows_what_needs_the_user_under_its_parents` (whose words); `cargo test -p juggler-app --lib features::processes::roles::tests::roles_use_the_cleanup_vocabulary`; vitest `src/screens/processes/session-rows.test.tsx` "0.3: a process's line says what started it first, its role guess last"; gallery `_gallery/90-processes/running-1680x1000-dark.png` (opened: CPU with sparkline, memory, ports, age, "Started by Bash: npm run dev · probably a dev server", owner) | each fact asserted | role, label, whose, ports evidenced; vitest passes. **Missing:** no test asserts a node's `peak_rss`, `cpu_time_ms`, `age_ms` or per-process `spark` (filled in `processes/mod.rs` `tick` from `inner.peaks`/`inner.sparks`, not by `model::build`, so `processes_graph` can't see them); and no test drives the label's two sources into the graph: the hook report (`features/runtime/reports.rs`, `PostToolUse` → `note_label("{tool}: {summary}")`) and Codex's `RuntimeEvent::ItemStarted` (`features/runtime/driver.rs` `note_command` → `"a command: …"`) **Closed 2026-11-07:** `--test processes_live a_session_and_what_it_leaves_behind_on_the_one_model` now asserts, after three ticks, the child's `peak_rss > 0`, `age_ms > 0`, a CPU line `spark.len() >= 2`, and `cpu_time_ms <= age_ms` (the system's own count, copied through; a sleeping child may honestly read 0, so its presence and bound are asserted, not a minimum) (29dcdc35f, 436b09945); the two label sources: `--lib features::runtime::driver::labels::a_command_item_names_what_it_started_and_the_rest_dont` (Codex's command item → "a command: npm run dev"; an agent message names nothing; ≤ 120 characters, ≤ 40 a session) and `--lib features::runtime::reports::label_tests::a_finished_step_names_its_tool_and_what_it_ran_and_nothing_else_does` (Claude's `PostToolUse` → "Bash: npm run dev"; `PreToolUse`, or no tool or summary, names nothing) | PASS | 436b09945 | 2026-11-07 | Claude (self) |
| R5 totals per session and app; counts (to clean, left behind, zombies, didn't stop, won't touch) the top bar and This Mac show | `cargo test -p juggler-app --test processes_graph totals_roll_up_and_the_top_bar_counts_what_to_clean` (left_behind 4, didnt_stop 1, zombies 1, wont_touch 1, to_clean 5 → 3 with Keep/Remind → 4 when the reminder lapses; session rollup and ports); `--test processes_live a_session_and_what_it_leaves_behind_on_the_one_model` (`processes.leftovers()` — This Mac's count — equals `graph.counts.to_clean`; `status_impl(..).counts.to_clean` agrees) | counts as asserted; This Mac equal | as asserted | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R6 paged by cursor (≤ 200 a page); a change published as `processes://changed` with version and counts; `HelperOrphaned` no longer dropped | `cargo test -p juggler-app --test processes_graph pages_follow_a_cursor_without_repeats_or_gaps`; `cargo test -p juggler-app --lib paging::tests::the_limit_has_a_default_and_a_ceiling` (`PageRequest::first(10_000).limit() == MAX_LIMIT`, 200; `query::page` clamps to its own `MAX_LIMIT` 200 too); `--lib events::tests::every_event_has_its_own_wire_name` (`ProcessesChanged` → `"processes://changed"`, payload `ProcessGraphChangedInfo {version, counts}`); `--test processes_live …` (Left behind via the reconciler's `HelperOrphaned`); production wiring `features/sessions/lifecycle.rs` → `processes::helper_orphaned` | every row once; ≤ 200; event named | as asserted. Note: `processes_live` forwards `HelperOrphaned` with its own pump that mirrors `lifecycle.rs`; the forwarder line itself is not exercised by a test | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R7 AI Juggler's own pid never shows; its hooks and tools hidden unless asked, never counted | `cargo test -p juggler-app --test processes_graph every_state_is_in_the_one_vocabulary` (no node has `OWN`), `everything_running_folds_healthy_sessions_to_one_line` (hook 104 only with `show_internal`), `totals_roll_up_and_the_top_bar_counts_what_to_clean` (rollup 4: "the hook is AI Juggler's own"); `--lib features::processes::roles::tests::the_hook_and_the_bridge_are_ai_jugglers_own` | as asserted | as asserted | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| Gate: the task's quality gates | vitest as above (run here); `cargo test -p juggler-app --test processes_graph --test processes_live --lib features::processes`, clippy `-D warnings`, `scripts/polling_audit.py`, `scripts/tool_literal_lint.py` (run in CI) | pass | vitest 16/16 pass; cargo/clippy/audits not run here (CI) | PASS (vitest); cargo in CI | f6fb096cb | 2026-11-07 | Claude (self) |

## Performance budget (not a numbered requirement)

- `cargo test --release -p juggler-app --test processes_graph the_graph_and_a_page_fit_the_budget_at_two_thousand_processes -- --nocapture`
  (50 sessions × 40 processes): on 2026-10-25 build 1.92 ms, Needs attention
  page 0.16 ms, Everything running page 0.52 ms, search 0.02 ms (budget: build
  ≤ 5 ms, page ≤ 50 ms). Not re-measured for this audit.
- The tick's extra CPU ≤ 1 % of a core: still **UNPROVEN** — derived only
  (1.92 ms per 2 s ≈ 0.1 %); the gathering (`observe::collector_view`, the
  slow reads every 10 s) was never timed separately.

## Live evidence (2026-10-26, `docs/qa/processes-lab.md`, [lab-1-2026-10-26.md](../AJ-E2406/lab-1-2026-10-26.md))

- Left behind within 34–36 s on a real Mac (reconciler, even with the
  lifecycle forwarder ended): still holds.
- ~~A real `Z` child never reached the graph (`zombies: 0`)~~ — **no longer
  true in code**: the collector reads zombies with `/bin/ps` at each full scan
  while a tree is watched (`juggler-supervisor/src/zombies.rs`, Release
  0.2.0), unit-tested by `zombies::tests::a_real_zombie_is_read_with_its_parent`.
  Not re-run live since.
- ~~Each left-behind and ended process listed twice~~ — fixed
  (`ProcessLedgerRepo::upsert_helpers`; the graph takes each process once):
  `processes_graph a_process_with_two_ledger_rows_is_listed_once_as_left_behind`.
- The 2 s tick died once in a debug build in sysinfo's disk listing (AJ-E1902
  lab, bug 4): not re-checked here.

## Visual

AJ-E2404's gallery `_gallery/90-processes/` (48 captures, last re-shot
6a06e7483, 2026-10-31); `running-1680x1000-dark.png` opened for this audit.

## Binary artifacts

None added; cited: `_gallery/90-processes/running-1680x1000-dark.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)

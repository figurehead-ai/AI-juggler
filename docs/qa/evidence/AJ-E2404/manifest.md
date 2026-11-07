# Evidence manifest — AJ-E2404 The Processes page

Task: `docs/tasks/E24-processes.md` AJ-E2404. Re-checked against the code,
its tests and the committed gallery on `release/0.4.1`, 2026-11-07. The
page is `ui/src/screens/processes/` on the backend's graph
(`crates/juggler-app/src/features/processes/`); a session's Processes tab,
an App's and another app's conversation page use the same `ProcessesView`.

Cargo and Playwright tests are cited as written and run in CI; the vitest
files were run here: `cd ui && pnpm -s exec vitest run
src/screens/processes-page.test.tsx src/lib/process-states.test.ts
src/components/processes-tab.test.tsx src/screens/processes/tree-fold.test.tsx
src/components/palette-evidence.test.tsx src/components/notifications.test.tsx
src/screens/this-mac-page.test.tsx` → 7 files, 78 tests passed.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 views: Needs attention (default), Everything running, Ended (7 days), Ports (every open port), Cleanup history | `src/screens/processes-page.test.tsx`: "opens on Needs attention: what to clean up, under its session, and the tallies agree", "Ports lists every open port with what it is", "Ended reads how long each ran; Cleanup history names the session"; `cargo test -p juggler-app --test processes_live a_session_and_what_it_leaves_behind_on_the_one_model` (step 4: the stopped child in the `ended` view); `processes-gallery.spec.ts` "processes — ${view.id} — ${vp.name} — ${theme}"; `_gallery/90-processes/{attention,running,ended,ports,history}-*.png` | the five views; attention selected first | vitest pass; the views in `Toolbar.tsx` `VIEWS`, `attention` the saved-view default (`ProcessesView.tsx` `savedView`); captures `attention-1680x1000-dark` and `history-1280x800-light` opened: five tabs, Needs attention selected, Ports shows another app's :4321 as "Not from AI Juggler" | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R2 tree table App → Session → process → subprocess; a healthy session folds to one line; the seven headed columns; sticky header; ≤ 60 rows in the DOM; ↑ ↓ ← → Home End, Enter | `src/lib/process-states.test.ts`: "keeps at most 60 rows in the page whatever the list's length", "moves like a treegrid: arrows, Home and End, open and close"; `src/screens/processes-page.test.tsx`: "Everything running folds a healthy session to one line until it's opened" (`/\d+ processes · .*B · /`), "a live session's own process offers Go to session, never Stop… (stopping a session is its page's)" (Enter on the row); `src/screens/processes/tree-fold.test.tsx` "the session row folds at once and tells the view, even where the backend sends it open"; `processes-gallery.spec.ts` "processes — the large Mac (2,000 processes) — ${theme}" (`≤ 60` rows asserted); `keyboard-flows.spec.ts` "processes — Stop… opens the confirm sheet; Esc returns to the row"; `cargo test -p juggler-app --test processes_graph everything_running_folds_healthy_sessions_to_one_line` | as stated | vitest pass; `ProcessTree.tsx` is a `role="treegrid"` with the header row (Process, State, CPU, Memory, Ports, Age, Owner) outside the scrolling body, so it stays put; `large-1680x1000-dark.png` present | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R3 filters all off by default (state, app, profile, project, has a port, heavy, include other apps) and search (`port:5173`) | `src/screens/processes-page.test.tsx`: "search finds a port", "Include other apps adds the rest of this Mac: names and memory only, nothing to stop"; `cargo test -p juggler-app --test processes_graph search_filters_and_one_session_narrow_the_rows` (port, words, has a port, a state) and `the_rest_of_the_mac_shows_only_when_asked_for_and_only_names_and_memory`; `_gallery/90-processes/filters-1680x1000-{dark,light}.png` | each filter there and off at first | vitest pass; `Toolbar.tsx` `NO_FILTERS` (every filter off) and the panel's states, Listening on a port, Heavy, Include other apps, Runs in (app), App (profile/account), Project | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R4 row actions: Stop… (verify sheet), Keep, Remind me in 1 h, Go to session, Reveal folder, Copy PID; a session's Clean up (N)… with a verify line and outcome per process; another app's: the D-41b stop, Go to its app, Show in Activity Monitor | `src/screens/processes-page.test.tsx`: "another app's leftover is stopped alone: no scope to widen (D-41b)", "the stop sheet: one proof line, the process's own unsaved-state warning, and the report links to Cleanup history", "Clean up what a session left warns of unsaved state for each process, alone or not", "an alert's Show it finds the process and takes focus to its row"; `src/components/processes-tab.test.tsx` "lists what a session left behind with Stop…, which opens the verified sheet"; `cargo test -p juggler-app --test processes_graph whose_and_how_each_may_be_stopped`; `processes-gallery.spec.ts` "processes — a row's menu", "processes — another app's leftover: the D-41b sheet", "processes — clean up what a session left"; `_gallery/90-processes/{menu,stop,stop-theirs,group-cleanup}-1680x1000-*.png` | each action there; the sheet verifies first | vitest pass; `menu-1680x1000-dark.png` opened: Stop…, Go to session, Reveal folder, Keep (not a leftover), Remind me in 1 h, Copy PID (8123); Go to its app and Show in Activity Monitor are `RowActions.tsx` `menu-go-app` / `menu-activity-monitor` | PASS | 657678fc8 | 2026-11-07 | Claude (self) |
| R5 Cleanup is this page's Left behind view; the nav and the top bar's pill lead here; the session Processes tab is the same component filtered to that session (any app's); This Mac links in and its Left behind agrees with the top bar | `src/shell/screens.tsx` (screen `cleanup` = Processes); `src/components/notifications.test.tsx` "without a Figurehead account: one quiet Connect, icons on every line, three groups" (Processes in the menu); `src/components/palette-evidence.test.tsx` "adds Processes with what's left running and Notifications with the unread count, only when above 0"; `src/components/processes-tab.test.tsx` (4 tests, `ProcessesView sessionId`); `src/screens/this-mac-page.test.tsx` "links what's working it hard, and leads to Cleanup for leftovers"; `this-mac-gallery.spec.ts` "Clean up leads to Processes — ${theme}"; `cargo test -p juggler-app --test processes_live a_session_and_what_it_leaves_behind_on_the_one_model` (`leftovers()` = `to_clean` = `status_impl` count) | every door opens Processes; one count | vitest pass for the nav, palette, session tab and This Mac. **The top bar's pill** (`CleanupPill`, `MainNav.tsx:78`, `nav-cleanup-pill`, `onPick("cleanup")`) **has no test or capture that clicks it**: it shows in `session-tab-1680x1000-dark.png` ("7 to clean up"), but nothing proves it leads here **Closed 2026-11-07:** vitest `src/app-keys.test.tsx` "the top bar's \"to clean up\" opens Processes (AJ-E2404 R5)" clicks `nav-cleanup-pill` and lands on Processes | PASS | abe4edcb3 | 2026-11-07 | Claude (self) |
| R6 "Zombie" explained in plain words where it shows | `src/lib/process-states.test.ts` "explains a zombie in plain words, never the code"; `src/screens/processes-page.test.tsx` "opens on Needs attention: …" ("only its parent can clear it"); `src/components/processes-tab.test.tsx` "explains a process waiting for its parent and offers no stop"; a real zombie now reaches the graph: `cargo test -p juggler-supervisor --lib zombies::tests::a_real_zombie_is_read_with_its_parent`, `live_stats::tests::a_zombie_child_is_in_its_parents_tree` (lab-1 bug 4, fixed) | the explanation with a zombie row | vitest pass; `attention-1680x1000-dark.png` opened: the "Waiting for its parent means: …" line above the list and "Finished; only codex (6600) can clear it" on the row | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| Gate: the page's tests | the vitest files above; `cargo test -p juggler-app --test processes_graph --test processes_live` (runs in CI) | pass | vitest 78/78 pass; cargo runs in CI | PASS | f6fb096cb | 2026-11-07 | Claude (self) |

## Budgets and visuals (still true; cited, not re-run here)

| Check | Evidence | Result (last run) |
|---|---|---|
| A page ≤ 50 ms | `cargo test --release -p juggler-app --test processes_graph the_graph_and_a_page_fit_the_budget_at_two_thousand_processes -- --nocapture` (runs in CI) | 0.16–0.52 ms (2026-10-25) |
| First paint ≤ 300 ms | `load.spec.ts` "the Processes page paints its first rows in under 300 ms" (runs in CI) | median 42–44 ms (2026-10-26) |
| 1280, 1680, 2000 px, dark and light | `processes-gallery.spec.ts` → `_gallery/90-processes/` (48 captures, last written 6a06e7483, 2026-10-31) | `attention-1680x1000-dark`, `menu-1680x1000-dark`, `session-tab-1680x1000-dark`, `history-1280x800-light` opened 2026-11-07 |

## Lab 1 (2026-10-26) rows, re-checked

- "Each process once in Needs attention and Ended" (was FAIL, bug 1): fixed;
  `cargo test -p juggler-app --test processes_graph a_process_with_two_ledger_rows_is_listed_once_as_left_behind`.
- "A zombie waits for its parent" (was FAIL, bug 4): fixed; the two
  supervisor tests in R6. Not yet re-seen live.
- "A leftover of a Claude Code session after Stop" (was FAIL): not a code
  fault. Claude Code stops its own background tasks on Stop and `/exit`; a
  leftover comes only by the crash route. The checklist is being rewritten
  (AJ-E2406 R3).
- The live run itself ([lab-1-2026-10-26.md](../AJ-E2406/lab-1-2026-10-26.md))
  was backend-only; a window run of the rewritten lab is still owed.

## Binary artifacts

None added; cited: `_gallery/90-processes/*.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)

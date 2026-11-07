# Evidence manifest — AJ-E2407 One source: the session tab, Cleanup and This Mac on the graph

Task: `docs/tasks/E24-processes.md` AJ-E2407. Re-checked against the code
and its tests on `release/0.4.1`, 2026-11-07.

A session's Processes tab is `ProcessesView` with that session's id
(`RightInspector.tsx:1053` for AI Juggler's, `ConversationPage.tsx:231` for
another app's, `AppPage.tsx:202` for an App's open sessions). The E410
tracker no longer feeds anything: `features/processes/tick.rs`
`record_exit_snapshot` only journals the descendant rows, and
`processes_tick` runs the collector, This Mac's tick and the sweep. Cleanup
is the Processes page; This Mac's "Left behind" is
`state.processes.leftovers()`, the graph's count (`machine_health.rs`, the
ledger's count only before the graph's first tick).

Cargo tests are cited as written and run in CI; the vitest files were run
here: `cd ui && pnpm -s exec vitest run src/components/processes-tab.test.tsx
src/components/conversation-page.test.tsx src/screens/this-mac-page.test.tsx
src/components/palette-evidence.test.tsx` → 4 files, 34 tests passed.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the session Processes tab reads the graph (its session filter), including other apps' sessions | `src/components/processes-tab.test.tsx`: "shows that session's tree alone, with roles in words and what started them", "lists what a session left behind with Stop…, which opens the verified sheet", "explains a process waiting for its parent and offers no stop", "a session with nothing running says so"; `src/components/conversation-page.test.tsx` "keeps both tabs while its token counts come and go, and the tab you're on" (another app's session has a Processes tab); `cargo test -p juggler-app --test processes_graph search_filters_and_one_session_narrow_the_rows` (`session_id = "conv-live"`, another app's session: its pids alone) and `a_session_s_tab_shows_the_app_processes_it_shares`; `_gallery/90-processes/session-tab-1680x1000-{dark,light}.png` | that session's tree only, any app's | vitest pass; `session-tab-1680x1000-dark.png` opened: the session's own tree (claude, an MCP server, npm → node (vite) → esbuild, node (tsc)), Everything running / Ended / Cleanup history. The capture is AI Juggler's session only; another app's tab is proven by the backend test and the conversation page's vitest, not captured | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R2 the E410 `OrphanTracker`'s detection (its 10 s clock, ppid snapshot) retired; leftovers are the ledger's (the reconciler's) and the sightings'; every stop through the verified path | retired: `tick.rs` (no tracker call; the AJ-E2407 note in `processes_tick`), `grep OrphanTracker crates/juggler-app` (none); ledger: `cargo test -p juggler-app --test processes_live a_session_and_what_it_leaves_behind_on_the_one_model` (the reconciler's `HelperOrphaned` → Left behind), `cargo test -p juggler-supervisor --test reconciler orphan_surfaces_only_after_grace_and_two_ticks` (the `HelperOrphaned` delta); sightings: `cargo test -p juggler-app --lib features::processes::sightings::tests::a_helper_alive_after_its_session_ended_is_left_behind`; verified stops: `cargo test -p juggler-app --test signal_paths every_signal_site_is_one_of_the_verified_paths`, `the_pid_ladder_runs_only_after_verify_and_audit`; `--test observed_stop every_refusal_sends_zero_signals`; `--test processes processes_command_reports_tree_and_the_sheet_stops_what_was_left` | no second model; every signal site verified | the app never constructs the tracker. The type itself (`juggler-supervisor/src/orphans.rs` `OrphanTracker`, `ORPHAN_GRACE_MS`) is still exported and kept alive only by its own unit tests and `juggler-supervisor/tests/orphan.rs`: dead code, a removal for later, no second model at run time | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| R3 This Mac's "Left behind" and the top bar's count come from the graph | `cargo test -p juggler-app --test processes_live a_session_and_what_it_leaves_behind_on_the_one_model` (`leftovers()` = `graph.counts.to_clean` = `status_impl(..).counts.to_clean`); `cargo test -p juggler-app --lib machine_health::tests::this_mac_splits_ours_from_theirs_and_ranks_by_memory` (the count passed through to `leftover_processes`); `src/screens/this-mac-page.test.tsx` "links what's working it hard, and leads to Cleanup for leftovers"; `src/components/palette-evidence.test.tsx` "adds Processes with what's left running and Notifications with the unread count, only when above 0" (`processesCounts().to_clean`, the hook the top bar's pill uses: `lib/cleanup-count.ts`) | one number | vitest pass; This Mac reads `state.processes.leftovers()`, the top bar `useCleanupCount` → `processesCounts().to_clean` and `onProcessesChanged`: both the graph's | PASS | f6fb096cb | 2026-11-07 | Claude (self) |
| Gate: the processes, cleanup, guardrails and collateral suites | `cargo test -p juggler-app --test processes --test processes_live --test processes_graph --test cleanup --test guardrails --test signal_paths --test observed_stop` (runs in CI); the vitest files above | pass | vitest 34/34 pass; cargo runs in CI | PASS | f6fb096cb | 2026-11-07 | Claude (self) |

Budget, no new work on the tick: by reading. `machine_tools::graph_trees`
reads the graph the tick already built and the collector's trees; This
Mac's count is `leftovers()`, a read of the same graph.

## This Mac and other apps' sessions (still true)

- `cargo test -p juggler-app --lib machine_tools::tests::other_apps_sessions_come_from_the_graph_grouped_by_app_and_tool`
  and `src/screens/this-mac-page.test.tsx` "lists another app's AI sessions
  by app, each by its title, and names it in an alert": ChatGPT and
  "Codex in Terminal" groups, heaviest first.
- By design, a session whose root a Claude tool's own row already counts
  (a `claude` inside the Claude app's tree) is left to that row and not
  listed again. That is lab 1's step 10 ("bug 5": the lab's outside Claude
  Code session ran under the Claude app and showed only as the "Claude
  app" row). The lab's expectation needs rewording, not the code.

## Lab 1 (2026-10-26) rows, still true

- The tab with another app's session (step 7): the outside session's own
  rows came back for its session id, as on the page. PASS (backend).
- Include other apps (step 10): "Other apps on this Mac", programs by name
  and memory, nothing to stop. PASS (backend).
- The counts agree (step 2): `to_clean` 2 = `leftovers()` 2 = This Mac
  `leftover_processes` 2 (a server and its shell, finding 3). PASS.

## Binary artifacts

None added; cited: `_gallery/90-processes/session-tab-1680x1000-*.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)

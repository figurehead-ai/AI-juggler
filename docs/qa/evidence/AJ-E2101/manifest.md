# Evidence manifest — AJ-E2101

Task: `docs/tasks/E21-desktop-first.md` AJ-E2101, live sessions from the
apps' own files (D-37). Branch `e21-step1` (off `feat/desktop-first`).
Status: **DONE-EVIDENCED** (agent self-check; the maintainer's full gate
runs after merging).

## What exists now

- **Adapter contract** (`crates/juggler-adapter/src/live.rs`, `adapter.rs`):
  `live_session_files` + `read_live_session` (and the provided
  `live_sessions(profile_dir)`), `app_session_files` + `read_app_session`
  (`app_sessions(instance_dir)`), `held_sessions`, `is_session_server`,
  `host_apps`. Shared helpers: `ctime_utc_secs` (Claude Code's
  `procStart`), `same_start` (pid reuse), `read_small` (no links, no large
  files).
- **Claude Code** (`juggler-adapters/src/claude_live.rs`): `<profile>/sessions/<pid>.json`,
  fields `pid, sessionId, cwd, status, procStart, hostSessionId,
  entrypoint, kind, name`; the `.key` files beside them are never listed.
  **The Claude app**: `<user-data-dir>/claude-code-sessions/*/*/local_<id>.json`,
  fields `cliSessionId, cwd, title, isArchived, lastActivityAt,
  lastFocusedAt, postTurnSummary.status_category, spawnedFrom.sessionId`
  only; `needs_action`, `status_detail` and every other text are skipped
  by serde without being kept.
- **Codex** (`codex_live.rs`): the threads whose writer lock
  (`thread-writer-locks/<id>.lock`) or rollout the app's `codex
  app-server` holds open (`lsof -p`, names only, 2 s limit); a `codex
  resume <id>` holding nothing is matched by its id.
- **No environment reads**: `running_setups::running_processes()` (sysinfo
  `with_environ`), `running_setups::config_dirs_of` and
  `features/conversations/live.rs::running_sessions` are gone. The process
  table (`features/conversations/procs.rs`) asks for executable and
  arguments only; a profile comes from where its pid file sits, a Claude
  app instance from the parent chain and `--user-data-dir`
  (`places.rs`). This Mac's tools and first run's "in use" read the same
  pid → folder map (`running_setups::with_folders`, `pid_dirs_in`).
- **Refresh** (`scan.rs`, its own `live-sessions` thread): the small files
  are listed every 2 s and re-read only when size or time changed; known
  pids every 5 s; the whole process table at most once a minute; a new
  pid's parent chain pid by pid; the Codex server's open files every 15 s.
  File events were not used: listing two small folders per profile costs
  under 3 ms (below), which the maintainer's brief allows instead.
- **Model**: `ConversationInfo` gains `places: PlaceInfo[]` (app,
  app_id, profile, instance, pid, label), `pid`, `state` (`asks_you`,
  `working`, `idle`, `done_review`), `viewed_ms`, `background`,
  `inactive`; `live` now means a verified process. `WindowIdentity` gains
  `start_time`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 per-session files, each shape | `cargo test -p juggler-adapters --lib _live` | fixtures for pid files, app files, held files pass | 6 passed | PASS | cda878fb | 2026-10-24 | agent |
| Req.1 summary text never kept | `claude_live::tests::an_app_session_keeps_its_status_word_and_never_its_summary_text` | the parsed value holds no summary text | passes | PASS | cda878fb | 2026-10-24 | agent |
| Req.2 host app from the parent chain, instance from `--user-data-dir` | `cargo test -p juggler-app --lib features::conversations::places` | Claude instance, default instance, Terminal, VS Code helper, unknown app, tmux | 4 passed | PASS | 0f77987e | 2026-10-24 | agent |
| Req.3 no environment read (guardrail) | `cargo test -p juggler-app --test trust no_process_environment_is_ever_read` and `procs::tests::the_process_read_never_asks_for_environments` | no shipped code asks for an environment; the refresh kind's `environ` is `Never` | pass | PASS | 0f77987e | 2026-10-24 | agent |
| Req.4 PID reuse | `scan::tests::a_reused_pid_or_a_stale_file_is_not_open`, `running_setups::tests::a_session_file_names_its_pid_only_while_the_start_time_matches` | a pid with another start time never counts | pass | PASS | 0f77987e | 2026-10-24 | agent |
| Req.5 cadence | `scan::tests::the_whole_table_is_read_once_a_minute_and_files_only_when_changed` | one full scan a minute; a new pid found within a tick; a gone pid within 5 s | pass | PASS | 0f77987e | 2026-10-24 | agent |
| Req.6 unknown formats fall back | `claude_live::tests::a_pid_file_of_another_shape_or_pid_is_left_out`, `app_session_words_map_and_other_shapes_are_left_out` | a file of another shape gives nothing; an unknown word is unknown | pass | PASS | cda878fb | 2026-10-24 | agent |
| States in E21's words | `scan::tests::the_states_map_to_e21s_words`, `a_running_turn_is_working_whatever_the_last_word_and_open_twice_is_one_row` | busy → Working; blocked → Asks you; review_ready → Done, review it; completed/idle → Idle | pass | PASS | 0f77987e | 2026-10-24 | agent |
| Performance budget (< 20 ms, off the main thread) | [count on this Mac](docs/qa/evidence/AJ-E2101/count-on-this-mac.txt) | refresh < 20 ms | median 1.7 ms, max 2.3 ms (first, with the full table and `lsof`: 118 ms, on the live thread) | PASS | d10d322d | 2026-10-24 | agent |
| Read-only count on the maintainer's Mac | [count on this Mac](docs/qa/evidence/AJ-E2101/count-on-this-mac.txt) | files, live processes and matches agree | 6 pid files → 6 Claude sessions in 6 instances, each with its app record; 4 Codex threads in ChatGPT | PASS | d10d322d | 2026-10-24 | agent |
| Privacy page names the new reads | `cargo test -p juggler-app --test trust` | every claim's tests exist; the guide says it word for word | 13 passed | PASS | 0f77987e | 2026-10-24 | agent |

## Not done here

- Sessions on a folder that isn't one of the user's accounts stay
  unattributed (as in 0.1): they need a conversation in the index to show.
- A Claude app window running no Code session has no known Claude Code
  folder now (the 0.1 code read it from its environment); This Mac and
  first run say nothing for it rather than guess.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout (counts only)
- [x] Transcript excerpts scrubbed of personal content (none kept)

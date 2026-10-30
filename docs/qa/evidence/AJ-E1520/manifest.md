# Evidence manifest — AJ-E1520

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1520 (the subtask count and the agent subtask tree). Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

## Done

| Requirement | Proof | Result |
|---|---|---|
| Req.1 I-08 from Claude Code's agent calls | Claude Code starts a subagent with its Task tool (named Agent in later versions); the call's `tool_result` ends it, and an error result fails it. One started in the background (`toolUseResult.status` `async_launched`) is marked "in the background" and not counted as running: its transcript doesn't say when it ends. Other tool calls aren't counted. `juggler-adapters` `claude_transcript` `subagents_are_read_from_task_and_agent_calls` (fixture `claude-code/session-subagents.jsonl`). | PASS |
| Req.1 Codex's equivalent | Codex starts one with `spawn_agent`, whose output names it (`{"agent_id", "nickname"}`). A later `wait`/`wait_agent` output says how each named agent stands (`{"status": {id: status}}`); a `close_agent` output ends the agent it targeted. A `sub_agent_activity` `completed` event ends one too, and a spawn that gave no agent failed. Done and failed are final: an agent that errored and was then closed stays failed, and a close answered with an error string changes nothing. Other calls' outputs (a shell's) aren't read at all. `codex_transcript` `subagents_are_read_from_spawn_wait_and_close` (fixture `codex/rollout-subagents.jsonl`: one finished by a wait, one closed, one failed spawn, one errored then closed, one running, a shell call); unit `done_and_failed_are_final`. | PASS |
| Req.1 the count on the status row | The row's count is how many of the session's subagents are still running. It's read on the existing 2 s tick, taking only what the transcript gained (`SubtaskTail`; a half-written line waits for the next read; a replaced file, shorter or not, is read again from the start). Readers see what the last tick published, never a map being read. It's `null` until the transcript has been read (no transcript, or not live), never a guessed 0. `juggler-app` `subtasks` `the_count_follows_a_live_transcript_and_the_list_outlives_the_session`, `a_codex_session_is_counted_the_same_way`, `a_new_transcript_path_is_read_afresh_and_its_drop_is_reported`; unit `a_line_still_being_written_waits_for_the_next_read`, `a_replaced_file_is_read_again_from_the_start`, `a_file_replaced_by_one_as_long_is_read_from_its_start`. | PASS |
| Req.1 tool knowledge stays in the adapters | Each tool's lines are read in its own parser module (`claude_transcript::subtask_line`, `codex_transcript::subtask_line`). The record (`juggler_adapters::subtasks`) and the app (`features::subtasks`) never name a tool: the format is recognised from the transcript's first line. | PASS |
| Req.2 the tree in the inspector | The session's page lists every subagent behind a "Subagents" disclosure, with the summary "2 running · 4 in all". Each row shows what it was asked (first line, 80 characters), its kind when the call named one, and where it stands: running, in the background, done, failed, or "didn't finish" once the session ended without the transcript saying so. Hidden when the session started none. The list is read again when the row's count moves and every 5 s while the session runs. An ended session's list is read from its file 16 MiB at a time. UI `subagents-section.test.tsx` (8 tests). | PASS |
| The row (D-25) | The rail row's second line adds a quiet "· 2 subagents" like "· 3 changed", and the screen reader's text "2 subagents running". `subagents-section.test.tsx` `the rail row notes running subagents quietly…`. | PASS |

## After the independent review

The review found no blockers. Fixed from its list:
- **Readers see a published view.** The tick used to take the whole map out of its lock while it read, so a list or a count in that moment saw nothing.
- **A new transcript path reports its drop.** The count before was taken after the reset, so 2 → 0 never emitted.
- **An ended session's list is read in bounded chunks.** It used to be read in one piece, with no cap.
- **Codex's final states stay final.** An errored agent that was then closed stayed failed; a close answered with an error string no longer fails its target.
- **A background Claude subagent isn't counted as running.** The count was a guess for those.
- **The open list refreshes while the session runs.** It missed changes that left the count the same.
- **Smaller fixes:**
  - a replaced file of the same length is told apart by its inode;
  - only the new bytes are scanned for a line end;
  - shell outputs aren't parsed;
  - doc comments and the "none running" wording.

## After the coordinator's review

The tick listed every session ever recorded each 2 s, and kept the live ones afterwards. It now asks storage for only the sessions that haven't ended (`SessionsRepo::live`), one query that shares its list of ended statuses with `active_cwds`. Covered by `juggler-core` `storage_roundtrip` `live_lists_only_sessions_that_havent_ended` (every status).

## Decisions

- **The count is the subagents still running**, not every one ever started: a long session that started forty would otherwise carry "40" for good. The inspector lists them all, with how each ended.
- **One level.** The tree is the session and the subagents it started. Claude Code's subagents can't start subagents of their own. Codex's can (up to `agents.max_depth`, 1 by default), but their starts are in the subagent's own rollout, not the session's. A deeper level would read those rollouts; it isn't needed for Claude and is rare for Codex.

## Research basis

Read 2026-10-23 from openai/codex `main` (`gh api`):
- `core/src/tools/handlers/multi_agents/spawn.rs`: `spawn_agent` arguments `message`/`items`/`agent_type`, result `SpawnAgentResult {agent_id, nickname}`.
- `multi_agents/wait.rs`: `WaitAgentResult {status, timed_out}`.
- `multi_agents/close_agent.rs`: `CloseAgentResult {previous_status}`, argument `target`.
- `protocol/src/protocol.rs`: `AgentStatus` (`pending_init`, `running`, `interrupted`, `completed`, `errored`, `shutdown`, `not_found`).
- `rollout/src/policy.rs`: the `collab_*` events are transient and never reach a rollout; `sub_agent_activity` reaches it only in legacy history mode.
- `protocol/src/models.rs`: `function_call_output.output` is a string or an array of text items, and a call's namespace has its own field (the name stays `spawn_agent`).

Claude Code's Task/Agent call shape (`description`, `prompt`, `subagent_type`, `tool_use_id`, `toolUseResult.status`/`agentId`) follows the transcripts already parsed here. `async_launched` for a background subagent is a research claim, not yet checked against a live transcript (AJ-E810).

The fixtures are synthetic, in these shapes.

## Visual

`docs/qa/evidence/_gallery/88-subagents/`: the row's count and the session's page with the list open, 1280×800, both themes, axe 0 critical/serious. Each capture opened and checked.

The approved baselines are unchanged: no baseline fixture has subagents, so the section and the row note don't show.

## Left

- **When a background subagent ends.** Claude Code's transcript isn't known to say when a subagent started in the background ends, so it shows "in the background" and isn't counted. To check in a live lab session (AJ-E1539).
- **Codex subagents' own subagents** (depth 2): see Decisions.

## 0.2: the Subagents tab, for every tool (2026-10-27)

The maintainer asked to see the subagents a session has, on its page, for the tools that have them (approved for 0.2.0), then for every tool AI Juggler supports, through the adapters. Status: built on `release/0.2`, IN-REVIEW until its release PR.

| Requirement | Proof | Result |
|---|---|---|
| The adapter contract, tool-neutral | `juggler_adapter::subagents`: each adapter answers `subagent_support` (supported, naming the records it reads, or not supported with the reason; the trait's default, `Unanswered`, is refused by the kit) and `subagent_reader` lists a session's subagents as one record: id, type, task, status (running, done, failed, stopped, unknown), started and ended times, parent and depth, background or not, and its own transcript. Readers refresh incrementally and in bounded work (256 MiB a refresh; a 6 GB rollout reads over several asks, each listing what was found). `juggler-adapter` `subagents::tests`. | PASS |
| Every adapter answers (AJ-E1521 check 11) | The kit holds every adapter to it: the session's three (done, failed, running) and a nested one under the running one, each once; a plain session lists only what it started; not one byte of the account's folder changes while it reads; a conversation it offers is a file in that folder; one not supported says why and offers no reader. `adapter_conformance` `*::lists_subagents_honestly` for Claude Code, Codex, OpenCode 1.x and 2.x, Kimi Code, Grok Build and the fake harness (77 kit tests pass). | PASS |
| Claude Code (and the Claude app's Code tab) | `subagents/agent-<id>.meta.json` (`agentType`, `description`, `toolUseId`, `spawnDepth`, `parentAgentId`, `requestShape`, `stoppedByUser`) and `agent-<id>.jsonl` beside the session's transcript; how each ended from the transcript that started it: the Agent call's answer, and for one in the background the `<task-notification>` (`<task-id>`, `<status>` completed/failed/killed/stopped) queued for or given to the session (never one a tool's description only mentions). A nested one's status comes from its parent subagent's transcript. Files with no meta and no start: unknown. `claude_subagents` (4 tests); `claude_transcript` lines prefiltered before parsing. Shape checked read-only on this Mac's records (field names and counts only): 10 sessions with `subagents/`, 180 subagents, every one ended as its notifications say (161 completed, 17 failed, 2 stopped); the largest session (97 subagents, 207 MB transcript) lists in 0.66 s the first time and 1 ms after. | PASS |
| Codex (and Codex in the `ChatGPT` app) | Each child thread's rollout names its parent (`session_meta.source.subagent.thread_spawn.parent_thread_id`, `depth`, `agent_path`, `agent_role`), found in the session's day folder and the days around it; the parent's rollout says how each ended, in both multi-agent shapes: the first (`spawn_agent` → `agent_id`, `wait` statuses, `close_agent`) and v2, seen in this Mac's 2026-10 rollouts (`spawn_agent` → `{task_name}`, `item_completed` `SubAgentActivity` `{agent_path, agent_thread_id, kind}`, `list_agents` `{agent_name, agent_status}` with `{"errored": …}`, `interrupt_agent`). Children of children are followed. `codex_subagents` (3 tests). Shape checked read-only: 359 rollouts, 10 child threads (depth 1), 310 `guardian` reviewers (not subagents of a session, not listed). | PASS |
| Kimi Code (and Kimi Code Desktop on the same home) | `state.json`'s `agents` map (`labels.parentAgentId` for the real parent; the top-level one is always `main`; `labels.profileName`) and the `subagent.spawned/started/completed/failed/cancelled` records (`subagentId`, `subagentName`, `description`, `runInBackground`, `time` ms) in each spawning agent's `agents/<id>/wire.jsonl` (source: MoonshotAI/kimi-code `packages/agent-core-v2/docs/wire-manifest.d.ts`, `sessionMetadata.ts`, `agentLifecycleService.ts`, read 2026-10-27). A folder with no record: unknown. `kimi_subagents` (2 tests). No Kimi home on this Mac: fixtures only. | PASS |
| Grok Build | `subagents/<id>/meta.json` in the session's folder (`subagent_type`, `description`, `status` running/completed/failed/cancelled, `started_at`, `completed_at`, `child_session_id`); a nested one from the child session's own folder (source: xai-org/grok-build `xai-grok-shell/src/agent/subagent/mod.rs`, user guide `17-sessions.md`, read 2026-10-27; about a month older than the shipping CLI). An unknown status word stays unknown. `grok_subagents` (1 test). No Grok home on this Mac: fixtures only. | PASS |
| OpenCode (and OpenCode Desktop on the same store) | Child sessions by `parent_id` (1.x `session`, 2.x `session_v2`), read-only through `opencode_store::open_read_only` (immutable when OpenCode is closed); 2.x: the child's `idle_outcome` (succeeded/failed/interrupted) at `time_idle`; 1.x: the parent's `task` tool part (`state.status`, `state.time`), or for one in the background the child's last message (an error, or `time.completed` with a `finish` other than `tool-calls`). The store is unchanged after a read, and no log file appears. `opencode_subagents` (2 tests). No OpenCode store on this Mac: fixtures only. | PASS |
| Commands, paged, redacted | `subagents_list` (a keyset page, newest first, each followed by the ones it started; a cursor from elsewhere is refused) and `subagents_conversation` (the same last-turns reader as the session's own, paged, followed while it runs, "unchanged" from the file's stamp; a subagent's every line is its conversation), for any session the app shows by its account and the tool's id for it. Only the transcript its reader found is read, never a path from the page. Tasks and turns redacted (D-14); off with last messages in Settings. "Running" only while the session is open, else "unknown" (D-06). `juggler-app` `features::subagents::tests` (7 tests: 200 subagents across pages, each once, a nested one right after its parent, unknown starts last; running vs unknown; redaction; read-only; a cursor from elsewhere). `last_turns` `a_subagents_own_transcript_is_its_conversation`. | PASS |
| The Subagents tab | Shown only when the session's tool records subagents and it started some (one small read when the page opens, again every 15 s while it runs), on the session page AI Juggler's sessions use and the one for sessions another app runs. Rows in the task row's shape: type, task (the whole task on hover), status as a word in a pill, started, how long it took, "in the background"; nested ones indented under their parent; a skeleton at once; a page at a time. A row whose tool keeps its conversation opens it in the session page's own chat view, the session's asks under "The session asked", with a way back (Esc too); a tool that keeps it elsewhere says so once over the list. The rail row's "+N agents" (View › Sub-agents) opens the session on this tab. UI `subagents-tab.test.tsx` (7 tests). | PASS |

### Gallery

`docs/qa/evidence/_gallery/88-subagents/` gains `session-subagents-tab-*` and `subagent-conversation-*` (1280×800, both themes, axe 0 critical/serious), from `subagents-gallery.spec.ts`; captured locally, opened and checked, not committed (0.2's captures and baselines come with the maintainer's approval). The approved baselines don't change: no baseline fixture has subagents, so the tab doesn't show.

### Decisions

- **One contract, every adapter.** A tool's layout lives only in its adapter's `*_subagents.rs`; the app pages, orders, redacts and says running/unknown without naming a tool.
- **Routes (FINDINGS.md: the sanctioned reader first).** Claude Code and Grok Build: their files (no reader API). Codex: the rollouts, not the app-server's `thread/list`/`thread/read`, which would start a Codex process on the user's home (it writes caches there) for a read-only view of another app's session. Kimi Code: the documented session folder; its server's transcript API resumes a cold session (writes). OpenCode: the store read-only; `session list --format json` lists top-level sessions only, `export` one session without its children, both run migrations and write logs, and `GET /session/:id/children` needs a server on the user's store.
- **Conversations.** Claude Code and Codex keep each subagent's conversation in a transcript the session page's reader reads, so it opens. Kimi Code (an event journal), Grok Build (an ACP update stream) and OpenCode (its store) keep it in a form the session page doesn't read for their own sessions either; the tab says so instead of guessing.
- **Stopped is its own status.** Claude Code's `killed`/`stopped`, Codex's `interrupted`, Kimi's `cancelled`, Grok's `cancelled` and OpenCode 2.x's `interrupted` are neither done nor failed.
- **Platforms.** Every reader finds its files from the conversation's own record (beside it, or the store it names) with path joins; no macOS path or command in shared code.

### Left

- Kimi Code, Grok Build and OpenCode subagents' conversations: they open once the session page reads those tools' own conversations (Kimi's `wire.jsonl` reader is being written for its state and tokens).
- A live lab check of Kimi, Grok and OpenCode subagent records on a real setup (none of those tools is set up on this Mac).
- ~~The Overview's "Subagents" disclosure still shows beside the new tab.~~ Retired into the tab (D-48): see below.

## One Subagents view: the older list retired, one source for the count (2026-10-27)

The maintainer, 2026-10-27: "Yes, fold the Overview's older Subagents list into the new Subagents tab and retire the old one." (D-48, Req.6.) Status: built on `release/0.2`, IN-REVIEW until its release PR.

| Requirement | Proof | Result |
|---|---|---|
| The older list is gone | Removed `SubagentsSection.tsx` and its test, `lib/ipc/subtasks.ts`, the generated `SubtasksInfo`/`SubtaskInfo`/`SubtaskStateInfo`, `features::subtasks` (its tick, `subtasks_list`, its DTOs), the command's registration, the dev bridge's route and `scripts/ipc_audit.allow`'s line for it. The 0.1 rows above that cite `juggler-app` `subtasks` and `subagents-section.test.tsx` now have their behaviour guarded on the new path (below). | PASS |
| One quiet line in its place | Wireframe screen 05 (approved 2026-10-13) names the count in the session's inspector ("Subtasks · 2 sub-agents"), so the session's notes keep one line when it started any: "6 subagents · 2 running › Subagents"; the button opens the tab (its name, "Open its subagents: 6 subagents · 2 running", carries the visible word: axe's label-in-name check). Quiet at none or before the read. Its counts come from the page's one `subagents_list` read (limit 1: `page.total`, `running`), the read that already decides whether the tab shows. vitest `subagents-tab.test.tsx` "the session's line to its Subagents tab" (3 tests). | PASS |
| The count from the adapter contract, every tool | `features::subagents::count`: a row's `subtask_count` is the running subagents its tool's reader lists, the reader the tab uses (one `Readers` store: the tab's last 8, plus every live session's, keyed by account, conversation and record). Any adapter whose `subagent_support` is supported is counted (Claude Code, Codex, Kimi Code, Grok Build, OpenCode); one that isn't, or a session whose conversation isn't indexed yet, is `null`. Unit `count::tests::any_tool_with_a_reader_is_counted_running_only_and_null_until_caught_up` (a fake tool's reader), `the_tabs_reader_is_the_counts_and_outlives_the_session`, `no_record_or_no_reader_is_not_counted`. | PASS |
| The rail's hot path | Its own tick ("subagents", every 2 s, the old tick's cadence) follows only the sessions that haven't ended (`SessionsRepo::live`); each reader is refreshed once a pass, reading only what its records gained within its own budget; a count is published once its reader has caught up and kept while a later pass catches up again; the UI hears only when a count changed. Rows read the published map (`count::counted`), never a file. A session that ends leaves the count (`null`: D-06) and its reader is dropped unless its tab still keeps it. Unit `a_session_that_ends_leaves_the_count_and_its_reader_goes`. | PASS |
| End to end, the old guards moved | Over a synthetic Claude Code account (in place) and a live AI Juggler session on its conversation: `null` before the first read, 0 when none run, 2 when two start, 1 when one answers, no emit when nothing changed, the tab listing the same two from the same reader, and `null` once the session ends with one unanswered (D-06). A session whose conversation isn't on this Mac: `null`, no emit. `juggler-app` `tests/subagents_count.rs` (2 tests). The parser's own guards (a half-written line waits, a replaced file is read from the start) stay in `juggler_adapters::subtasks` unit tests. | PASS |
| One source in the UI | The rail's "+N agents" reads only `subtask_count`: it no longer falls back to the session's child processes (a "+1 agent" that opened a Subagents tab with none). The card's badge says "N subagents running", the same words as the row. vitest `rail-model.test.ts` "a row's sub-agents come from one source". | PASS |

### What the count means now

Still the subagents running now, `null` until read. It now counts what the tab calls running: a subagent started in the background (until its notification says it ended) and a subagent's own subagents, which the old count left out. The old parser stays as the Claude Code and Codex readers' start-and-end log (`juggler_adapters::subtasks`, one parser); only its old-list fields (`label`, `running()`) are gone.

### Gallery

`docs/qa/evidence/_gallery/88-subagents/` (local Playwright, 1280×800, both themes, axe 0 critical/serious, each opened and checked): `row-count-*` now shows the rail with View › Sub-agents on ("+2 agents" on the session; Fake Desk no longer shows "+1 agent" from its child processes), and `session-subagents-*` shows the session's page with the one line, "6 subagents · 2 running › Subagents", instead of the open list. The spec opens the tab from the line. (0.3: `session-subagents-*` was retired with the old tab; the Subagents tab's captures are `88-subagents/tab-*` and `detail-*`; the spec still opens the tab from the session's line.) The approved baselines don't change: no baseline fixture has subagents or the View's Sub-agents on.

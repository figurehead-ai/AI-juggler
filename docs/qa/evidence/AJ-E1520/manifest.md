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

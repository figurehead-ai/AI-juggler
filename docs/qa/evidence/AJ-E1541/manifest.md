# Evidence manifest — AJ-E1541

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1541 (cross-tool continuity). Status: **IN-PROGRESS**. 2026-10-22, claude-opus.

## Done

| Requirement | Proof | Result |
|---|---|---|
| Req.2 the conversation, within a budget, nothing dropped silently | The replay keeps the newest turns that fit, and says how many earlier ones aren't included ("N earlier turns are not included."). The newest turn always travels, cut to fit only if it alone is too big (`codex_switch` `an_oversized_newest_turn_still_travels_cut_to_fit`). | PASS (by count; not by topic) |
| Req.2 each tool call in one line | `tool_summary` (`summarize`) turns each call into a line such as `(Edit calc.py)` or `(ran "python3 -m unittest")`. `codex_switch` `claude_to_codex_carries_tool_calls_the_task_list_and_the_changes` | PASS |
| Req.2 the files changed and the task list | The replay ends with "Where the work stands now": the task list as last set, and the uncommitted changes on the branch with their stat. Same test. | PASS |
| Req.3 the new session is linked to its conversation | It is found by time after the launch. When another session of the tool begins in the same folder, it is found by the replay's opening message instead (`codex_switch` `switching_back_finds_the_replays_conversation_beside_another_one`, from lab 8). | PASS |
| Req.4 switching back carries what happened away | The update names every hop and its new turns, with the diff and the task list (`switch_source` `a_chain_of_switches_returns_to_where_it_started_with_every_hop`). The user picks where to go back to: the previous account or where the work started. An account whose conversation can't be found is named rather than left out (`switching_back_says_when_it_cant_tell_which_conversation`). Live: labs 5 and 7 returned to A with the release name said only on Codex. | PASS |
| Req.5 what can't carry is listed before confirming | The switch screen's "Doesn't carry" line and the wizard's "What's carried" step come from the fidelity table (cross-tool: subagent state). | PASS |

Live cells PASS: claude→codex transcript-resume, todos, mcp-references and switch-back (`docs/qa/evidence/AJ-E1539/manifest.md`).

## Left

- **Req.1 a handover file.** The replay still goes to the target as one prompt, which the budget keeps under the argument limit. The task asks for `switches/<edge>/HANDOVER.md` and `state.json` in AI Juggler's data, with a short prompt pointing at them.
- **Req.2 the rules files in play, and older turns by topic.** The replay doesn't name the rules files yet. Older turns that don't fit are counted, not summarised by topic.
- **The gate's codex→claude cells.** The journey returns to A by switching back, not by a forward Codex → Claude switch. A forward hop is needed to decide them.

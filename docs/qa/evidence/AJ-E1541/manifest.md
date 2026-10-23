# Evidence manifest — AJ-E1541

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1541 (cross-tool continuity). Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

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

## After the independent review of #132

- **Off the main thread.** The switch and switch-back commands now run off the main thread. "Finish this turn" could hold the whole app for up to two minutes.
- **The replay's checks come first.** A replay that can't go no longer stops the source. The secrets choice and "is there a conversation" are checked before anything stops (`codex_switch` `a_replay_that_cant_go_never_stops_the_source`).
- **Every hop is named.** A hop whose session record or account is missing is named in the update, not skipped.
- **A capped lookup.** Finding a replay's conversation reads at most the 200 newest files changed since the switch.

## The handover file (2026-10-23, claude-opus)

| Requirement | Proof | Result |
|---|---|---|
| Req.1 a handover file, and a short prompt that points at it | The switch writes `switches/<edge>/HANDOVER.md` and `state.json` in AI Juggler's data folder (folder 0700, files 0600: this user only). The target starts with the replay's opening sentence (how its conversation is found again) and the file's path on a line of its own; Claude gets `--add-dir` for that folder (after the prompt: it takes several folders), Codex reads it as is. The conversation is no longer on the command line, where `ps` showed it to every local user, and no longer capped by the argument limit (24 KiB on Windows before): 96 KiB everywhere. `features::handover` tests (written for this user only; the pointer's first line); `codex_switch` `claude_to_codex_replays_the_redacted_conversation` (the conversation is not in argv), `codex_to_claude_replays_too` (`--add-dir switches/<edge>`), `an_oversized_newest_turn_still_travels_cut_to_fit` (the command line under 2 KiB, the brief within budget). Linking still works: `switching_back_finds_the_replays_conversation_beside_another_one` passes with the pointer as the conversation's first message. | PASS |
| Req.2 the rules files in play | The handover names the rules files in the folder that either tool reads ("CLAUDE.md (read by Claude Code)"), and asks the target to read the ones it doesn't load itself; `state.json` lists them. `claude_to_codex_carries_tool_calls_the_task_list_and_the_changes` | PASS |
| Req.2 older turns by topic | Turns that don't fit are counted and named by what was asked in each (its first line, up to 40), oldest first. `older_turns_that_dont_fit_are_named_by_what_was_asked`; unit `earlier_turns_are_counted_and_named_by_what_was_asked` | PASS |

## Live (labs 9 and 10, 2026-10-23)

The lab's journey now runs through the handover file: Codex starts from the pointer prompt and reads `switches/<edge>/HANDOVER.md`. Claude→Codex: Codex answered with the codename, the multiply function's name, the open task and the rule only `CLAUDE.md` states. A forward Codex→Claude hop (a task begun on Codex, `--add-dir` given): Claude B answered with the Codex task's codename, its open plan item and the rule only `AGENTS.md` states. Switching back from B to the Codex task gave Codex a release name B had only said ("Silver Finch"). Register cells PASS: claude→codex rules-carry; codex→claude transcript-resume, todos, rules-carry and switch-back; with labs 5–8's claude→codex transcript, todos, MCP and switch-back (`docs/qa/evidence/AJ-E1539/manifest.md`, `docs/qa/evidence/AJ-E808/register.md`).

## Left

- **A Mac-to-Mac replay** (`crossmachine`) still sends its brief as one prompt; it's AJ-E1515's path.
- **Subagents and file history across tools** don't carry by design; the switch screen says so before the user confirms (Req.5). Their register cells wait for drop notes.
- **The gate's codex→claude cells.** The journey returns to A by switching back, not by a forward Codex → Claude switch. A forward hop is needed to decide them.

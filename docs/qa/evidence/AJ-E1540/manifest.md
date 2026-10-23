# Evidence manifest — AJ-E1540

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1540 (same-tool switches done right). Status: **IN-PROGRESS**. 2026-10-22, claude-opus.

## Done

| Requirement | Proof | Result |
|---|---|---|
| Req.1 the freeze is real | `switch_source` `stop_now_stops_the_source_before_the_copy` and `finish_this_turn_stops_the_source_once_it_is_quiet`. The copy is read only after the source stops; "Finish this turn" waits for the file and the output to go quiet, and gives up after two minutes with a way out. | PASS |
| Req.1 a source started outside AI Juggler is never stopped, and the screen says so | `freeze_source` stops only a session AI Juggler supervises. The preflight's `source_keeps_running` follows the same rule, and the freeze choice becomes a note: "AI Juggler can't stop this session: if it's still open, it keeps going there, and the switch starts a separate continuation." (vitest "AJ-E1540 a source AI Juggler can't stop") | PASS |
| Req.2 Claude's whole session travels | `claude_handover` `a_session_with_its_folder_and_file_history_travels_whole`: the transcript, its folder (`subagents/`, `tool-results/`) and `file-history/<id>/` go to the target's matching places without overwriting. A name that climbs out of the folder is refused (`a_bundle_name_that_climbs_out_is_refused`). | PASS |
| Req.2 the task list | Rebuilt from the conversation, where its calls travel: the fidelity table says "rebuilt". Claude's newer `tasks/` folder isn't copied. Live: B named the open tasks in labs 5, 7 and 8. | PASS (rebuilt, not copied) |
| Req.3 resumes in the source's folder, linked | `resume_cwd` `claude_resumes_and_replays_in_the_work_folder` and `codex_resumes_and_replays_in_the_work_folder`; `switch_source` `a_session_that_knows_its_conversation_switches_by_it` and `a_chain_of_switches_returns_to_where_it_started_with_every_hop` | PASS |
| Req.4 a compressed Codex rollout travels | `codex_handover` `a_compressed_rollout_travels_decompressed` and `a_compressed_rollout_of_the_thread_counts_as_present` | PASS |
| Usage after a switch | The target counts only what was done there, not the carried history (P19; `usage_carried`) | PASS |

Live: the register's claude→claude transcript-resume, todos and mcp-references cells are PASS from labs 5–8 (`docs/qa/evidence/AJ-E1539/manifest.md`).

## After the independent review of #132

- **Usage after a switch (P19).** The filter now keys on the copy's own id, as installed. A copy installed beside one the target already had (a back-and-forth) leaves the target's own conversation its usage. A failed switch carries nothing (`usage_carried`: four cases, two of which fail under the old rule).
- **No usage pass mid-switch.** Between the install and the launch record, a back-and-forth's copy was still keyed by the source's id. A pass in that gap dropped the target's own history, and kept that for good because offsets are saved. No pass runs while a switch holds its slot (`usage_carried` `no_pass_reads_a_copy_while_its_switch_runs`, which fails without the guard).
- **Bundles from a paired Mac.** A bundle can't bring another conversation in beside the transcript (`claude_handover` `a_bundle_cant_bring_in_another_conversation`).

## Left

- **Codex → Codex live (Req.4, the gate).** The lab needs a second signed-in Codex home; this Mac has only `~/.codex`, and signing in another is the maintainer's to do. Then a Codex→Codex hop goes into the journey.
- **The other claude→claude cells: decided live (labs 9 and 10, 2026-10-23).** Rules, subagents, the folder and worktree edges and switch-back are PASS: B quoted the rule only `CLAUDE.md` states, the session's `subagents/` folder travelled, B continued inside a git worktree (its change landed there, not in the root), and switching straight back gave A what B had only said (`docs/qa/evidence/AJ-E1539/manifest.md`). File history stays UNVALIDATED live: the lab's first turn is headless and leaves none to carry (it travels whole in the adapter's test).

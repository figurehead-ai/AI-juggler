# Evidence manifest — AJ-E1539

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1539 (a live switching lab). Status: **DONE-EVIDENCED**. 2026-10-22, reviewer claude-opus.

## What it is

`crates/juggler-app/tests/switch_lab_live.rs`, run with `just switch-lab` (ignored and a no-op unless every `AIJ_LAB_*` variable is set, so it never runs in CI). It drives AI Juggler's own switch code over a throwaway data folder and a new throwaway project, with the real Claude Code and Codex on accounts whose owner agreed. The maintainer allowed their own accounts on 2026-10-22, in a lab project only.

The journey is the maintainer's:

1. A small coding task on Claude account A: `calc.py` with `add`, its test, and a three-item task list. The first turn is told a codename, `BLUE-HERON`.
2. Switch A → B (same tool, a transplant). A probe on B asks for the codename and the open tasks. B then adds `sub` and decides that the multiply function will be called `times`, without writing it.
3. Switch B → Codex (cross-tool, a replay). A probe asks for the codename, the name chosen on B, and the open tasks. Codex then writes `times`, and says a release name, `Copper Kettle`, only in its reply and never in a file.
4. Switch back to A, where the work started (past B). A probe asks for the codename, the release name and what happened meanwhile. Only the conversation can know the release name.

After every step the project's tests must pass. A step it can't verify fails: nothing passes by default.

With `AIJ_LAB_PROBE=<name>`, the lab also checks the switch's set-up step (AJ-E1542). Account A alone has a probe MCP server and a probe skill of that name. The server is a local standard-library script whose tool answers `KESTREL`, and only with its environment carried. The skill's word is `PEREGRINE`. Before each switch the lab sets up only those two on the target. It then asks the target for both words in a fresh session: the switched conversation already holds them, so asking there would prove nothing (lab 6 did, and is not counted). On the way back, A must not be offered what it already has. At the end, every set-up is undone.

## Runs

All on 2026-10-22, Claude Code 2.1.302 and codex-cli 0.163.0, accounts A and B (two Claude subscriptions) and the Mac's Codex home.

| Lab | Steps | Result | What it found or proved |
|---|---|---|---|
| 1–4 | 1–4 | FAIL, then fixed | A switch of a session started outside AI Juggler couldn't find its file (now found by conversation id). Switching back reached only the last hop, its update was empty, and it promised a pack that never came (now the whole chain, with every hop's turns). Codex's older rollouts are compressed (`.jsonl.zst`) and were skipped (now read). |
| 5 | 1–4 | PASS | Every probe answered from the conversation. On A: "The codename is BLUE-HERON and the release name is "Copper Kettle"; meanwhile `sub` and `times` (mul) were added with tests…" |
| 6 | 1–5 with the probe | Not counted | Every step passed, but Codex's set-up check was asked inside the switched conversation, and Codex answered from it with its tool call blocked. Fixed in the lab. Also found: Codex's config backup outlived the undo (fixed: the last undo removes it). |
| 7 | 1–5 with the probe | 7 of 8 PASS | B, from a fresh session: "KESTREL PEREGRINE". Codex's check didn't run, because a lab flag clashed (`--approve-for-me` refuses `-s`; fixed). After the undo, no backup was left in the Codex folder. |
| 8 | 1–5 with the probe | 7 of 8 PASS, then fixed | Codex, from a fresh session: "KESTREL PEREGRINE". So the server, translated into Codex's `config.toml`, and the skill, in the shared `~/.agents/skills`, both work there. The switch back to A failed. The probe's own Codex session had begun in the same folder after the replay, so finding the replay's conversation by time alone was ambiguous. Switching back then left the Codex hop out without a word (A: "I can't see any release name being picked"). Fixed: the replay's conversation is now found by its opening message, and an account whose conversation can't be found is named in the update (`codex_switch` tests `switching_back_finds_the_replays_conversation_beside_another_one` and `switching_back_says_when_it_cant_tell_which_conversation`). |

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 an env-gated lab over a throwaway data folder and project | The test refuses a non-empty data folder or an existing project folder, and refuses to run with `CLAUDE_CONFIG_DIR` or `CODEX_HOME` set. `just switch-lab` runs it. | PASS |
| Req.2 the journey, with probes only the conversation can answer | Labs 5, 7 and 8, steps 1–4 (above). The release name is never written to a file. | PASS |
| Req.3 versions, commands, probes and answers recorded; register cells | Each report records the tool versions, the switch's launch command line, every probe and its answer. Seven cells are PASS (`run: live`, Claude Code 2.1.302, codex-cli 0.163.0). Claude→Claude: transcript-resume and todos (labs 5, 7, 8), mcp-references (B in labs 7 and 8). Claude→Codex: transcript-resume and todos (labs 5, 7, 8), mcp-references (lab 8), switch-back (labs 5 and 7; lab 8's failure is above, with its fix). They're in `crates/juggler-handover/data/fidelity-register.json`, rendered in `docs/qa/evidence/AJ-E808/register.md`. A tool upgrade resets them by itself. | PASS |
| Req.4 before and after snapshots; only allowed differences | Paths, sizes and modification times of A's, B's and Codex's folders, never contents. Every difference in labs 5–8 is the lab's own session files (`projects/`, `file-history/`, Codex `sessions/`), the approved set-up and its undo, or the tools' own housekeeping from being run at all (remote settings, policy limits, logs, caches, Codex's databases and bundled skills). After each probe run, A's and B's MCP servers and skills, Codex's `config.toml` and the shared `~/.agents/skills` were compared with copies taken before. They matched, after removing one line Codex writes itself: it trusts the lab folder. | PASS |
| Req.5 no sign-in, no credentials, nothing outside the lab folder | The lab only runs `claude -p` and `codex exec` in the lab project. The interactive session each switch starts is stopped as soon as its command line has been read. The snapshot never reads contents. | PASS |

The reports themselves hold real paths and stay out of the repository.

## Not covered here

- **Codex → Codex.** It needs a second signed-in Codex home, and this Mac has one. The register's codex→codex cells stay UNVALIDATED until someone with two Codex accounts runs the lab.
- **Rules, subagents, file history, worktrees.** The journey's project has no rules file, no subagents and no worktree, so those cells stay UNVALIDATED. File history travels whole in the adapter's test (`a_session_with_its_folder_and_file_history_travels_whole`), but the live journey didn't check it.

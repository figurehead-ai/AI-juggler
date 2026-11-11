# AJ-E1907 — A task's session starts from a short pointer

Task: `docs/tasks/E19-agent-tasks.md` AJ-E1907. Status: **DONE-EVIDENCED** (Codex, live in the lab). The Claude Code pointer path is proven by its tests; it gets a live start at the release candidate (P2 under ADR-0027 condition 7b).

| Req | Command / live step | Expected | Observed | Verdict | Commit | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the first prompt names the task and where to read it; the brief is never on the command line | `tasks::tests::a_pointer_names_the_task_but_never_its_brief`; `tests/tasks.rs` | pointer only | pass | PASS | c907c19d5 | 2026-11-07 | agent |
| R2 a tool without the task tool gets the brief through its own channel, never the command line | `launch::first_prompt`'s private brief file (`<data>/task-briefs/<id>.md`, owner-only), its tests | the file, not the arguments | pass | PASS | c907c19d5 | 2026-11-07 | agent |
| R3 no brief text reaches the process's arguments: live | The R1 lab (`/tmp/aij-lab-r1`), the real Codex sign-in where it is (ADR-0027 condition 7), agent tools on. A real Codex session wrote the task with AI Juggler's `task_create` (`aj-466f`, "Lab check: count the files", its brief carrying the marker `LABBRIEF-7731`). Then **Start a session for it** through the lab write mode's `tasks_start` (approved 2026-11-07, c907c19d5) | the marker in no process's arguments | Every Codex process working in the lab (two `node`, two `codex`, by pid): marker in arguments **no**. The task's session runs driven (Codex's app-server); its first message is AI Juggler's handoff naming the task ("Handoff · Lab check: count the files"), the brief in that message, off the command line, as the task's design says. The task shows `in_progress`, held by the new session | PASS | c907c19d5 | 2026-11-07 | agent |
| Snapshot | the Codex folder's paths, sizes and times before and after (never contents) | `config.toml` and the sign-in unchanged | unchanged; no lab process left | PASS | c907c19d5 | 2026-11-07 | agent |

The check printed each process's pid, name and whether the marker was in its arguments, never the arguments themselves.

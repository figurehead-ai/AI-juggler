# Evidence manifest — AJ-E1902

Task: `docs/tasks/E19-agent-tasks.md` AJ-E1902, agent task tools (P0).
Status: **DONE-EVIDENCED** on fakes (IN-REVIEW until the release PR); the live run is `docs/qa/tasks-lab.md`. 2026-10-25, claude-opus.

Design: `crates/juggler-agentbridge/src/task_tools.rs` (the tools and their one shape each), `wire.rs` (the calls), `cli.rs` (the command line), `crates/juggler-app/src/features/tasks/agent.rs` (the decisions), ADR-0020's E19 addendum, D-43. Test output: [bridge-tests.txt](docs/qa/evidence/AJ-E1902/bridge-tests.txt), [guardrail-tests.txt](docs/qa/evidence/AJ-E1902/guardrail-tests.txt).

## What exists now

1. **Tools** (pre-allowed at launch with the rest, D-30): `tasks_list` (one page of 20, ready work first; `mine`, `ready`, `status`, `parent`; under 16 KiB), `task_get` (in full: what it asks, when it's done, links, the sessions that worked it, recent history, the issue), both annotated read-only; `task_create`, `task_claim`, `task_update` (status, words, priority, blockers, link, `release` with a note), and `task_assign` (the lead's: a member's `session_id`, `nobody`, or `new_session`, which only asks the user, D-31). Every task an agent reads says who wrote it (`written_by`) and that its words are data.
2. **Command line**: `tasks [--mine | --ready]`, `task <id>`, `task-create <title>`, `claim <id>`, `task-update <id> <status> [note]`, `release <id> [note]`, `assign <id> <to>`; exit 2 when refused or held.
3. **Decisions**: every call through `allowed(pool, action)`, audited: `read_tasks`, `create_task`, `claim_task`, `update_task`, `assign_task` (in no named level: it asks unless the user made a rule), `prepare_launch`. A change above the level is **held** for the user's OK in Needs you and applied as the agent's own when allowed (D-43 point 1).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 the six tools, read-only ones annotated | `mcp::tests::the_tools_and_one_resource_and_which_ones_write`; `task_tools::tests::each_task_tool_reads_its_one_shape`; `wire::tests::a_task_call_names_its_task_and_nothing_else` (no field names a pool, an assignee at creation, an account, a profile or a start) | PASS |
| Req.1 `task_assign` for leads only | `tasks.rs::the_lead_plans_and_gives_tasks_out_and_members_work_what_they_hold` (a member is refused; the lead's waits on the user) | PASS |
| Req.2 the same commands from `juggler-agent` | `cli::tests::the_command_line_has_no_way_to_pass_a_key` (release, claim, no account option) | PASS |
| Gate: permission tests through the real binary | `bridge.rs::the_task_commands_go_through_the_real_binary_and_the_level` (add, list, claim, review through the real `juggler-agent` and its socket at Work together; a member's `assign` refused; at Read a change held, exit 2, in Needs you; every action audited) | PASS |
| D-31: no agent starts a session or picks an account | `tasks.rs::an_agent_asks_for_a_session_and_only_the_user_starts_it`; `tasks_guardrails.rs::no_agent_starts_a_session_or_picks_an_account` | PASS |
| Performance: < 50 ms per call | `bridge.rs::calls_answer_in_under_50_ms` (with three `task_create`, `tasks_list` and `task_claim` over the socket, 8 sessions) | PASS |
| Live: real tools call the task tools without a prompt | `docs/qa/tasks-lab.md` steps 2–5, run 2026-10-26 with Claude Code 2.1.310 (the lead, in its terminal) and Codex 0.163.0 (driven), through AI Juggler's backend ([lab-2026-10-26.md](lab-2026-10-26.md)): `task_create`, `tasks_list`, `task_claim`, `task_update` and `task_assign` with no prompt; one claim of two won; changes above the level held, and Allow once, Always (second time) and Deny all worked; a member's assign refused; `new_session` only asked | PASS (the window's views: BLOCKED, for the maintainer) |
| Live: `tasks_list ready: true` lists only claimable work | the same lab, step 2: it listed the ready task **and** a story and an epic (`agent.rs`: the ready filter has no kind) | FAIL (the lab's bug 1) |
| Live: a held assignment names the member | the same lab, step 4: "«Planner» wants to give aj-… to sess-…" (the session id, in Needs you and the task's history) | FAIL (the lab's bug 2) |
| Live, rerun on 0.2 | `docs/qa/tasks-lab.md`, run 2026-10-28 on the 0.2 build (Claude Code 2.1.310, codex-cli 0.163.0) through the committed harness `tests/agent_labs_live.rs` in a clean environment ([lab-2026-10-28.md](docs/qa/evidence/AJ-E1902/lab-2026-10-28.md)). Every step PASS. Step 6 runs since `<claude-b>`'s first-run screens were accepted with the maintainer's approval: the switch with Stop now moves the task, and the new session's own `tasks_list` says `yours: true`. Only step 7's offline check is BLOCKED (Wi-Fi is a system setting). The 10-26 bugs 1–3 are fixed live (the ready list lists tasks only; a held assignment names "Builder"; Homebrew's `gh` looks up). Found and fixed: a busy terminal session outlived Stop, so a switch's Stop now gave up (the output reader blocked on a full hint channel; the terminal ladder ended with SIGHUP) | PASS (one check BLOCKED) |

Last updated: 2026-10-28.

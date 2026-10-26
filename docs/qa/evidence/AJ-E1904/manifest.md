# Evidence manifest — AJ-E1904

Task: `docs/tasks/E19-agent-tasks.md` AJ-E1904, tasks follow sessions and switches (P1).
Status: **DONE-EVIDENCED** on fakes (IN-REVIEW until the release PR). 2026-10-25, claude-opus.

Design: a task's holder is its member ref (`work:<id>` once the session has a Work item, else `session:<id>`, which a switch's Work item stands for), so nothing is carried at a switch; `task_sessions` links each session that worked a task (claimed, started for, given, worked); `features/tasks/views::epoch_task` gives each Switch Graph node its session's task (the newest node: the one its Work item holds now). `switch_graph.rs` is unchanged. Test output: [switch-test.txt](docs/qa/evidence/AJ-E1904/switch-test.txt).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 tasks linked to Work items and session epochs | `tasks_switch.rs::a_switch_keeps_its_task_and_the_history_shows_each_sessions` (the planner claims through its tools; the app's real switch to another account; the new session lists it as its own, in progress, says where it is, and both sessions are linked) | PASS |
| Req.2 the switch history shows each session's task | the same test (both nodes of `switch_graph_overview_impl` name the task); vitest `the switch history shows each session's task`; gallery `switch-graph-tasks-1680x1050-{dark,light}.png` | PASS |
| Gate: an end-to-end test where a switch keeps its task | the same test | PASS |
| Live: a switch keeps the task with real tools | `docs/qa/tasks-lab.md` step 6, 2026-10-26: not run, as the checklist says when no second lab profile is named ([AJ-E1902/lab-2026-10-26.md](docs/qa/evidence/AJ-E1902/lab-2026-10-26.md)) | UNPROVEN (needs the maintainer's second lab profile) |
| The card and the session's page follow it | `tasks.rs::a_card_shows_its_members_task`; the switch test's card check; gallery `pool-card-task-*`, `session-tasks-*` | PASS |

Last updated: 2026-10-26.

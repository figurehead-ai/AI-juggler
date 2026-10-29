# Evidence manifest — AJ-E1903

Task: `docs/tasks/E19-agent-tasks.md` AJ-E1903, the task list and tree in the app (P0).
Status: **DONE-EVIDENCED** on the preview and fakes (IN-REVIEW until the release PR). 2026-10-25, claude-opus.

Design: `ui/src/pages/tasks/` (PoolTasks, TaskView, TaskRow, TaskTree, TaskLinks, StartSession; TaskForm until D-44), `ui/src/components/TaskNeeds.tsx`, `ui/src/lib/ipc/tasks.ts`, `crates/juggler-app/src/features/tasks/{views,needs,launch,oversight,commands}.rs`. Test output: [vitest.txt](docs/qa/evidence/AJ-E1903/vitest.txt), [gallery.txt](docs/qa/evidence/AJ-E1903/gallery.txt).

## What exists now

1. **A pool's Tasks tab** (between Members and Messages; loaded when first shown): the work, ready work first, a page of 40 at a time, with how many in each state (Open, Ready, In progress, In review, Blocked, Done), a search, and **Plan**: each epic, its stories and their tasks behind folds, each fold's tasks loaded when it opens, with how many are done. The same row shape as the threads' and the rail's: title, status as a word and a mark, id, who has it, what it waits on.
2. **A task in full**: its kind and id, the story or epic it's in, its status and priority, who wrote it (an agent's words marked as not yours), the actions it offers now (D-44, below), who has it and their session (**Open its session**; a Processes link once that view registers one, `lib/task-links.ts`; or, when no session of its holder runs, "…whose session has ended" with **Free it**), what it asks, when it's done, its items, its links (read only; shown only when it has some), the issue, the sessions that worked it, the threads that name its id (they open in Messages), and its history.
3. **D-44 (2026-10-27): the app writes no task.** No New task, no Status or With menu, no link editing. A task offers only: **Approve** and **Send back…** (in review; the note, needed, reaches its holder as the user's message), **Free it** (its holder's sessions all ended or crashed), **Cancel task** (open; after a confirm) and **Start a session for it…**; each checked by the store when done and logged as the user's.
4. **"Start a session for this task"** (D-31): the user picks the launch settings (name order; only the project's own default account is picked for them); the session starts in the pool's project with the task's brief as its first prompt, joins the pool, and holds the task, in progress (D-43 point 4).
5. **Needs you**: an agent's held change (the same approval card as E18 and E23: keys, Always allow from the second time) and an agent's request for a session (the same picker, **Start session**, **Not now**).
6. **A session's Tasks tab**, shown once it holds or worked one; the pool's member card says its task.
7. All lists page; every call through `ipc::blocking`; nothing under 12 px.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 a list per pool, ready work first; the tree behind disclosure | vitest `lists ready work first…`, `filters to done ones, and shows the plan`; `tasks.rs::two_hundred_tasks_list_fast_and_page`; gallery `pool-tasks-*`, `pool-plan-*` | PASS |
| Req.2 holder, session and history; *until 2026-10-27*: create, edit, assign, close from the app | vitest `you add a task, change its status and give it to someone`, `a held task's session is one click away…`; gallery `task-held-*`, `task-new-*` (2026-10-26; the form's captures removed with it) | SUPERSEDED by D-44 |
| Req.2 *as changed 2026-10-27 (D-44)*: the app writes no task; only the four actions, each by its state, logged as the user's | `tasks_oversight.rs` (5: approve, send back with the note in a terminal holder's inbox and in `task_get`, the note delivered to a driven holder's session, free only when the holder's sessions ended or crashed, cancel only open); `tasks_guardrails.rs::the_app_writes_no_task`; store `the_users_actions_move_only_from_their_state_and_log_one_event`, rules `the_users_four_actions_are_gated_by_the_tasks_state`; vitest `writes no task…`, `a task in review: you approve it…`, `…you send it back with a note…`, `frees only a task whose holder's session ended…`, `cancels an open task only after you say so…`, `the IPC module offers no way to create, edit, assign or link a task`; gallery `task-review-*`, `task-send-back-*` (2026-10-27, run locally and checked by eye; not committed) | PASS (tests); captures not committed |
| Req.3 "Start a session for this task" with the account the user picks | vitest `starts a session for a task only on your pick…`, Needs you `an agent's request for a session…`; `tasks.rs::an_agent_asks_for_a_session_and_only_the_user_starts_it` (the fake tool launched; the task its, linked, the request closed); gallery `task-start-*`, `needs-you-*` | PASS |
| Live: "Start a session for it" with a real tool | `docs/qa/tasks-lab.md` step 5, 2026-10-26 ([AJ-E1902/lab-2026-10-26.md](docs/qa/evidence/AJ-E1902/lab-2026-10-26.md)): `tasks_start` on the lab Claude launch settings started Claude Code in the pool's folder with the brief as its first prompt; the task in progress with it, "started for", the agent's request closed; nothing started before the user's pick. The picker as drawn: BLOCKED, for the maintainer | PASS (backend) |
| Performance: the list renders 200 tasks < 100 ms | vitest `renders 200 tasks in under 100 ms` | PASS |
| Quality gates: vitest, Playwright, the a11y gate | 14 vitest cases; `tasks-gallery.spec.ts` 31 Playwright cases, axe 0 critical or serious on each. 2026-10-27: 21 vitest cases; 32 Playwright cases, axe 0 critical or serious (the in-review pill on a selected light row was 4.2:1 and is fixed in `Pill.tsx`) | PASS |
| Visual: the list, the tree and a task, both themes | `docs/qa/evidence/_gallery/95-tasks/` (30 captures, 1280, 1680 and 2000 px, dark and light, each opened and checked; 28 since D-44 removed the two `task-new-*`; the committed ones still show the 2026-10-26 task page, with its Status and With menus) | PASS (2026-10-26); D-44's look not committed |

Last updated: 2026-10-27 (D-44).

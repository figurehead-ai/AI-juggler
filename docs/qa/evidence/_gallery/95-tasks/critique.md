<!-- captures: 6a10f76ada3b -->
# 95-tasks: tasks on a pool, a session and in Needs you

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | The task list and the open task share the panel's two columns; state pills line up under each title. |
| Hierarchy | Title, state and priority, who wrote it, then What it asks, Done when and Links as section titles. |
| Spacing | List rows evenly spaced; the detail's sections on one rhythm. |
| Icons | Needs you's task cards carry their mark; Cancel task has its icon. |
| Shared controls | List · Plan is the shared Segmented; Start a session for it opens the launch settings Picker. |
| Disabled reasons | No New task and no fields to change: agents write tasks (D-44), and the panel says the words are the agent's. |
| Plain words | "Anyone can take it", "Waits on Orders API: create and fetch an order", "Written by Planner (an agent: its words, not yours)". |
| Type sizes | Body 14 px; the detail's section titles are small capitals at 12 px. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Captured at 1280, 1680 and 2000. |

## Good

- The pool's task list and detail follow the board's card style.

## Weak

- None new.

## Fixed

- Re-render: the Tasks panel takes the page's width over the canvas.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| pool-tasks-1280x800, -1680x1050, -2000x1250 | Checkout redesign's Tasks: Open 7, ready work first; "Orders API: create and fetch an order" open with an agent's request for a session waiting on you. | The panel opens over the canvas (AJ-E2808). |
| pool-plan-1280x800, -1680x1050, -2000x1250 | The plan: the epic, its stories and their tasks behind folds. | The 0.3 header and pool page. |
| needs-you-1280x800, -1680x1050, -2000x1250 | Home's Needs you: an agent's held change and its request for a session. | The 0.3 header. |
| task-held-1680x1050 | A task someone has: its session one click away and its Processes tab. | The 0.3 header. |
| task-start-1680x1050 | Start a session for it, with the launch settings picker. | The shared Picker (AJ-E2813). |
| task-review-1680x1050, task-send-back-1680x1050 | A task in review whose holder ended: Approve, Send back with its note open, Free. | The 0.3 header. |
| session-tasks-1680x1050 | A session's Tasks tab. | The 0.3 header. |
| switch-graph-tasks-1680x1050, history-switch-1280x900 | The switch history with each session's task. | The 0.3 header. |
| pool-card-task-1680x1050 | A member's card with its task. | The 0.3 inspector. |

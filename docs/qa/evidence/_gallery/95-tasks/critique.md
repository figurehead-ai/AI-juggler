<!-- captures: 4fd46ebac026 -->
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

- switch-graph-tasks-1680x1050: opens scrolled past the top of the project page, so the Open sessions, checkout and Accounts cards show only their cut-off last rows under the tabs.
- pool-tasks-2000x1250, pool-plan-2000x1250: the open task stops short of the Tasks panel's right edge, leaving an empty strip beside it.
- session-tasks-1680x1050: one task row across the full width, "6 min" far from its title, and nothing else on the page.
- needs-you-1280x800 (both themes): "Keep the old shortcuts wor…" is cut in Pick up where you left off, and the Sign in button sits half below the window's bottom edge.
- pool-card-task-1680x1050: the two ended members are both "Untitled session · ai-juggler" while their line says payments-api; the title says nothing about what they did and names the wrong project.

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

## 0.3.2

The Tasks panel and its open task stand a step off the dark canvas (in light, a white sheet on grey), and each screen has one solid violet action that matches the state: "Start a session for it…" on a ready task (pool-tasks, pool-plan), Approve on a task in review (task-review), Send back once its note is open (task-send-back), Start session in the launch row (task-start) and Allow once in Needs you. In dark the lighter accent shows on those buttons, on the header's New session, the logo mark, Needs you's "Work Max · 58% left" bars, the lit Open 7 / In review 1 tab and the note field's focus ring in task-send-back; light captures change only by a pixel in the This Mac line. The top bar stays one row everywhere, with "This Mac · Plenty of room" and "7 to clean up" whole. The state pills stay easy to scan in both themes (teal In progress, amber In review, red Blocked with its red "Waits on…" line), and What it asks and Done when read as body copy under the small-capital section titles. The older weak spots are unchanged: switch-graph-tasks still opens scrolled past the top of the project page (its rail also cuts "outside AI Juggle" with no ellipsis); at 2000 wide (pool-tasks, pool-plan) the open task stops short of the panel's right edge; session-tasks puts one task row across the full width with "6 min" far from its title; and in pool-card-task the two ended members are "Untitled session · ai-juggler" with payments-api on their line, and the members' times ("10 min ago") are in the monospace face. After the second pass, needs-you-1280 gives the Personal needs its sign-in detail two lines, so it ends whole ("…AI Juggler never sees your password."), which pushes its Sign in button to the bottom edge of the window where it is half out of view until you scroll; Pick up where you left off's "Keep the old shortcuts wor…" is still cut. In every capture the rail's Refactor the settings screen row now keeps its time whole ("ai-ju… · viewed 2 min ago" where it read "ai-juggler · viewed 2 mi…"); nothing else changed.

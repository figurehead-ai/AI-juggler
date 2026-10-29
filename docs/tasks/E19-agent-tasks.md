# E19 — Task management shared by pools

> Source: the maintainer, 2026-10-21: "Advanced task management, with user
> stories, epics, tasks, co-ordination between different agents, and all
> shared accessible and assignable to work with groups. Could be accessed
> via MCP, SDK or plugins." Research: `docs/research/pools-messaging-tasks.md`
> (Beads, Task Master and Backlog.md among the prior art).
>
> Built on E17's pools and bridge. Tasks work in a pull model: agents check
> for work when they start and when they finish a task.
>
> **Law for every task here:**
> - **Assignment never reads quota (D-31).**
> - **A task may prepare a launch, never start one;** the user picks the
>   account.
> - **Tasks follow the work.** A switch keeps the task, and the switch
>   history shows it.

### AJ-E1901 Task storage and rules | Status: DONE-EVIDENCED (IN-REVIEW until the release PR) | Priority P0 | Effort M | Depends: AJ-E1702
**Goal**: Epics, stories and tasks live in a pool, with honest states and atomic claims.
**User story**: As a lead agent, I break an epic into stories and tasks; builders claim the ready ones.
**Requirements**:
1. Each item has:
   - a kind (epic, story or task), with a parent;
   - a short id such as `aj-7f3c`;
   - a title, a body, acceptance criteria and a priority;
   - a status: to do, ready, in progress, blocked, in review, done or cancelled;
   - an assignee;
   - an optional external URL.
2. Links between items: blocks, relates to, duplicates. "Ready" means no open item blocks it.
3. A claim is one atomic transaction. Every change goes into the event log.
4. The permission table (AJ-E1702) decides who may create, assign and close.
**UI/UX reasoning**: n/a here.
**Performance budget**: listing 1,000 tasks < 100 ms.
**Quality gates**: N fake agents claim one task and exactly one wins; tests for "ready"; permission tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1901/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-core/src/storage/` (tasks).
**Research basis**: `pools-messaging-tasks.md` §2.3 (tasks).
**Handover note**: A schema bump: downgrade note and `SCHEMA_RELEASES`.
*Built 2026-10-25* (`docs/qa/evidence/AJ-E1901/manifest.md`): schema **v36**, after D-41's v35 sightings (`e24-processes`, merged before it: a v34 database gets both); the status shown is the honest one (blocked while an open task blocks it); the roles under the level (the lead plans and assigns, a member works what it holds) are D-43 point 3.

### AJ-E1902 Agent task tools | Status: DONE-EVIDENCED (IN-REVIEW until the release PR; the live lab open) | Priority P0 | Effort S | Depends: AJ-E1901, AJ-E1704
**Goal**: Agents in any tool work the pool's tasks.
**User story**: As a builder agent, I call `tasks_list` for ready work, claim one, and mark it in review when done.
**Requirements**:
1. Tools: `tasks_list`, `task_get`, `task_create`, `task_claim` and `task_update`, plus `task_assign` for leads only. Read-only tools are annotated so they can be allowed without a prompt.
2. The same commands are available from the `juggler-agent` command line.
**UI/UX reasoning**: n/a.
**Performance budget**: < 50 ms per call.
**Quality gates**: permission tests through the real binary.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1902/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-agentbridge/`.
**Research basis**: as above.
**Handover note**: —
*Built 2026-10-25* (`docs/qa/evidence/AJ-E1902/manifest.md`, ADR-0020's E19 addendum): a change above the pool's level is held for the user's OK in Needs you (D-43 point 1); `task_assign` with `new_session` only asks the user (D-31). Live: `docs/qa/tasks-lab.md`.

### AJ-E1903 The task list and tree in the app | Status: DONE-EVIDENCED (IN-REVIEW until the release PR; D-44's change evidenced by tests, its gallery captures not committed yet) | Priority P0 | Effort M | Depends: AJ-E1901
**Goal**: The user sees and tracks the agents' work in one place (D-44).
**User story**: As the user, I see the epic and stories my agents planned, what's ready, who has what and what's blocked; I approve what's in review or send it back with a note, free a task whose agent's session ended, and cancel what's no longer wanted, and I steer everything else by talking to the agents.
**Requirements**:
1. Each pool has a task list, with ready work first. The epic and story tree is behind disclosure. A board view comes later.
2. Each task shows its holder, its session and its history. The app writes no task (no creating, editing, assigning or linking: the agents do, D-44); a task offers only the user's four actions, each when its state allows and each logged as the user's: **Approve** or **Send back** (in review; the note reaches the holder), **Free** (its holder's session ended or crashed), **Cancel** (open), and requirement 3.
3. "Start a session for this task" prepares a launch with the account the user picks (D-31).
**UI/UX reasoning**: D-25: the same rows, marks and folds as the rail.
**Performance budget**: the list renders 200 tasks < 100 ms.
**Quality gates**: vitest and Playwright; the a11y gate.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1903/manifest.md`.
**Visual testing**: the list, the tree and a task, in both themes.
**Files to touch**: `ui/src/screens/` (Tasks), `commands.rs`.
**Research basis**: as above.
**Handover note**: —
*Built 2026-10-25* (`docs/qa/evidence/AJ-E1903/manifest.md`): each pool's **Tasks** tab (the list; **Plan** for the epic and story tree) and each session's **Tasks** tab, no new screen (as D-42 point 3 for messages); Needs you for what waits on the user. "Start a session for it" is D-43 point 4.
*As changed 2026-10-27* (D-44, the maintainer: tasks are for the agents' sync and orchestration, and for humans to see and track): the New task form, the Status and With menus and link editing are gone, with the window's `tasks_create`, `tasks_edit`, `tasks_assign` and `tasks_link`; links stay, read only. Added `tasks_approve`, `tasks_send_back`, `tasks_free` and `tasks_cancel`, each checked by the store (`TasksRepo::user_action`, `juggler_core::tasks::may_user`); a send-back note reaches the holder as the user's message (E18). Evidence: `crates/juggler-app/tests/tasks_oversight.rs`, `tasks_guardrails.rs::the_app_writes_no_task`, vitest `tasks.test.tsx` (21 cases), the gallery's `task-review-*` (produced by `tasks-gallery.spec.ts`, not committed with this change). The committed `task-new-*` captures showed the removed form and are gone.

### AJ-E1904 Tasks follow sessions and switches | Status: DONE-EVIDENCED (IN-REVIEW until the release PR) | Priority P1 | Effort S | Depends: AJ-E1901, AJ-E201
**Goal**: Continuing on another account or tool carries the task.
**User story**: As a user whose builder hit its limit, I continue it on another account, and the task stays with it.
**Requirements**:
1. Link tasks to Work items and session epochs.
2. The switch history shows each session's task.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: an end-to-end test where a switch keeps its task.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1904/manifest.md`.
**Visual testing**: the Switch Graph with tasks.
**Files to touch**: `crates/juggler-core/src/storage/switch_graph.rs`.
**Research basis**: as above.
**Handover note**: —
*Built 2026-10-25* (`docs/qa/evidence/AJ-E1904/manifest.md`): a task's holder is its member ref (the Work item once it has one), so nothing needs carrying at a switch; `task_sessions` links each session that worked it, and the Switch Graph's nodes show it (`switch_graph.rs` unchanged).

### AJ-E1905 A Claude Code plugin | Status: IN-PROGRESS (built and tested; not published until D-43 point 6 is ruled) | Priority P2 | Effort S | Depends: AJ-E1902
**Goal**: Claude Code users can add the bridge and a task skill in one step, even outside AI Juggler's launches.
**User story**: As a Claude Code user, I install the plugin, and my sessions can read and update the pool's tasks.
**Requirements**:
1. A plugin bundling the bridge's MCP config and a skill for working tasks. It's published with the app's releases.
2. It works only while AI Juggler is running, and says so otherwise.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: a plugin install test in a fixture home.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1905/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `plugins/claude-code/` (new).
**Research basis**: `pools-messaging-tasks.md` §1.3.
**Handover note**: —
*Built 2026-10-25* (`docs/qa/evidence/AJ-E1905/manifest.md`): the bundle and its install test in a fixture home. "Even outside AI Juggler's launches" conflicts with ADR-0020 (a key only with a launch) and D-30, so outside them each tool says why it can't; publishing it with the releases waits on D-43 point 6.

### AJ-E1906 Links to GitHub issues and Linear | Status: DONE-EVIDENCED (IN-REVIEW until the release PR) | Priority P2 | Effort S | Depends: AJ-E1901
**Goal**: A task can point to the issue it comes from.
**User story**: As a user, I paste a GitHub issue link into a task, and it shows the issue's title and state.
**Requirements**:
1. External URLs show a title and state where public, or where the user's own `gh` login allows. No sync in v1.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: offline behaviour.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1906/manifest.md`.
**Visual testing**: the task with a link.
**Files to touch**: `commands.rs`, the task view.
**Research basis**: `pools-messaging-tasks.md` §1.6.
**Handover note**: —
*Built 2026-10-25* (`docs/qa/evidence/AJ-E1906/manifest.md`): GitHub titles and states through the user's own `gh`, on the user's click only (D-43 point 5); Linear links show their id from the link.

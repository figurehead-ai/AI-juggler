# Evidence manifest — AJ-E1901

Task: `docs/tasks/E19-agent-tasks.md` AJ-E1901, task storage and rules (P0).
Status: **DONE-EVIDENCED** on fakes (IN-REVIEW until the release PR). 2026-10-25, claude-opus.

Design: `crates/juggler-core/src/tasks.rs` (the rules), `crates/juggler-core/src/storage/tasks.rs` (schema v36), `crates/juggler-app/src/features/tasks/apply.rs` (the operations), D-43 (proposed). Test output: [core-tests.txt](docs/qa/evidence/AJ-E1901/core-tests.txt), [tasks-tests.txt](docs/qa/evidence/AJ-E1901/tasks-tests.txt).

## What exists now

1. **Items (v36)**: a kind (epic, story, task) with a parent of the right kind (a story under an epic, a task under an epic or a story; no loops), a short id (`aj-` and 4 hex digits, longer when taken), a title (one line, 120), a body (4,000), acceptance criteria (2,000), a priority (urgent, high, normal, low), a stored status (to do, ready, in progress, blocked, in review, done, cancelled), an assignee (a member ref, so a switch keeps it), and an optional `https` link to the issue it comes from (GitHub and Linear named).
2. **Links**: blocks, relates to, duplicates, within one pool; no loop of blockers. **Ready** means the status is ready and no open task blocks it: every read returns the honest status (`tasks::effective`: a ready or to-do task an open task blocks reads **blocked**, and is ready again by itself when the blocker is done).
3. **A claim is one statement** (`UPDATE … WHERE status = 'ready' AND (no assignee OR the claimer) AND kind != 'epic' AND no open blocker`), logged and linked to the claimer's session in the same transaction. Every change goes into `task_events`, append-only (update and delete triggers refuse).
4. **The permission table**: every agent call goes through `allowed(pool, action)` (AJ-E1902); under the level, the task rules (`may_plan`, `may_change`): the lead plans epics and stories, gives tasks out and changes or cancels any task; a member makes tasks, works what it holds, and edits what it made that no one holds (D-43 point 3).
5. **What waits on the user** (`task_needs`): a held change, and an agent's request for a new session.
6. Pre-migration copy kept; RELEASING.md's 0.2.x row names 36 (v35 is D-41's sightings from `e24-processes`, merged first). A v34 database gets both migrations, a v35 one gets the tasks (`tasks_storage.rs`: `upgrading_from_messages_keeps_a_copy_and_everything_as_it_was`, `upgrading_from_sightings_adds_the_tasks`).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 each item's fields, kinds and parents | `tasks::tests` (8: words round-trip, short ids, clean bounded text, links named, no loops, the rules, the brief); `storage::tasks::tests::releasing_gives_it_back_and_epics_hold_stories` (a story under an epic, never under a task); `tasks.rs::the_user_plans_an_epic_with_stories_and_sees_the_tree` | PASS |
| Req.2 links; "ready" means no open item blocks it | `storage::tasks::tests::ready_is_honest_about_open_blockers`; `tasks.rs::ready_means_no_open_item_blocks_it` (blocked, then ready by itself, then claimable) | PASS |
| Req.3 a claim is one atomic transaction; every change in the event log | `storage::tasks::tests::a_claim_takes_a_ready_task_once` (the log refuses update and delete); `tasks_storage.rs::n_connections_claim_one_task_and_exactly_one_wins` (8 connections) | PASS |
| Req.4 the permission table decides who may create, assign and close | `tasks.rs::each_level_decides_and_a_change_above_it_waits_on_the_user`; `tasks.rs::the_lead_plans_and_gives_tasks_out_and_members_work_what_they_hold` | PASS |
| Gate: N fake agents claim one task and exactly one wins | `tasks.rs::n_fake_agents_claim_one_task_and_exactly_one_wins` (6 agents through the bridge's entry point, at once) | PASS |
| Performance: listing 1,000 tasks < 100 ms | `storage::tasks::tests::a_thousand_tasks_list_quickly` (all 1,000, 5 pages, with blockers, debug build) | PASS |
| Handover: schema bump, downgrade note, `SCHEMA_RELEASES` | `tasks_storage.rs::upgrading_from_messages_keeps_a_copy_and_everything_as_it_was`; `busy_timeout.rs`, `storage_roundtrip.rs`, `switch_graph.rs`, `messages_storage.rs` follow v36; RELEASING.md's row | PASS (the notes' entry is the release's) |

## Not covered here

- Merge order: `e24-processes` (v35) is on release/0.2 first; `e19-tasks` carries its real migration and adds v36 after it.

Last updated: 2026-10-25.

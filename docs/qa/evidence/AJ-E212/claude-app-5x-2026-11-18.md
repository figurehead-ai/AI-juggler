# Move to account on the Claude app 5.x: the shape check and a live round trip (2026-11-18)

D-127 / D-128. The Claude app on the reference Mac had updated from 4.x to
**5.11492.5**, and the switch screen showed Move greyed out behind a version
list (`PROVEN_APP_MAJORS = ["4"]`). D-128 replaced the version list with a
check of the records' **shape** at run time. This is the evidence that the
shape check is right on 5.x, following the 2026-11-08 proof's method
(AJ-E212 R14, `crates/juggler-adapters/tests/move_live.rs`: the real planner
and engine). Text only; no account, organisation, person or path of a real
profile is recorded here.

## What 5.x changed (read only)

Read from every Claude app profile on this Mac (key paths and file names
only, never values) and from the app's bundled code:

| Change in 5.x | What a move does now | Test |
|---|---|---|
| `startChain.firstSessionId` on side sessions (always the parent by `spawnedFrom`, depth 1 on every record seen) | a record-to-record link: re-pointed like `spawnedFrom`; dropped with the moved conversation's own parent links | `claude_move_matrix::a_new_major_whose_records_fit_moves_with_its_start_chain_and_hint` |
| `archived-sessions.idx` beside the records, `{"v":1,"archived":[…]}`: a hint the app rebuilds from the records and loads without | put aside on both sides (`HoldWhy::AppHint`), restored by Undo | same test; the live run below |
| `waiting-input/` beside the records: per record id, messages kept until the person sends them | known; a family member with an entry waits ("…; Move becomes available once it's sent") | `a_member_with_messages_waiting_to_be_sent_waits` |
| `backlog/tasks.json` (account-level task suggestions naming their origin session) | left where it is (account-level, like the project notes) | `shape::tests::the_app_5_shape_fits` |

The shape check (`claude_move::shape`) refuses anything else: a file or
folder beside the records it doesn't know, a record field of another type,
or a record naming another under a key it doesn't rebind. On this Mac every
profile fits. Before `waiting-input/` was known, the live planner refused the
move with that folder named, before anything was written.

## The live round trip (5.11492.5)

The sanctioned `aij-test` profile and a scratch profile made for the run
(`aij-move-scratch`, not signed in, removed afterwards). Every app start went
through `scripts/qa/lab_env.sh` with `CLAUDE_CONFIG_DIR` and
`--user-data-dir`; every process of each start (main, helpers, crash
handler: 10–11) was checked by `ps -E`: the lab folders, no `CLAUDE_CODE_*`,
`CLAUDECODE` or `ANTHROPIC_*`. Each quit was by the main process's pid; no
process was left. Nothing was clicked in the app; its window was read
through the accessibility tree of its pid (`scripts/qa/ax_texts.swift`).
No model prompt was sent.

1. Seeded in `aij-test`: a parent, child A, child B (archived) and a
   grandchild, forked from a lab history, with the 5.x side-session fields
   (`spawnedFrom.taskId`, `startChain`), a nested agent and a rewind point.
   Started, the 5.x app listed child A nested under the parent and the
   grandchild under child A, and wrote its `archived-sessions.idx`.
2. Moved `aij-test` → scratch: 4 members, 19 steps, no refusals, an empty
   shape list; 60/60 checks before reopening. `aij-test` started again
   listed none of the family; 36/36 checks after, 0 lines added.
3. Moved scratch → `aij-test`: 4 members, 14 steps; 55/55 before, 36/36
   after `aij-test` started, 0 lines added. The resume link sent to
   `aij-test`'s new pid opened the parent in that profile's own window,
   with child A nested under it and its one nested session (child B
   archived, hidden). The app made its archived-sessions hint again.
4. Both moves undone, the later first; the family was then removed and the
   scratch profile deleted. `aij-test`'s session records are byte-identical
   to before the run (shasum of every file), and its Claude Code folders
   hold the same entries.

Result: the 5.x records, the launch on the profile's own folders and the
hand-off by the profile's pid behave as on 4.x. No version list gates the
move any more (D-128); Move shows wherever the shape check passes.

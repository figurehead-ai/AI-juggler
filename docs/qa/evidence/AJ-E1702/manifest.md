# Evidence manifest — AJ-E1702

Task: `docs/tasks/E17-agent-pools.md` AJ-E1702 (pools, the default pool, roles and permissions). Status: **DONE-EVIDENCED**. 2026-10-21, reviewer claude-opus.

Scope: pools in the app only. Agents can't reach them yet (the bridge is AJ-E1704, after the D-28 to D-33 rulings).

## What changed

1. **Storage (schema v31, `crates/juggler-core/src/storage/pools.rs`).**
   - `pools`: a name, a kind (`default` | `user`), an optional project, `deleted_at`. The migration makes the default pool "All sessions" (`pool-default`); a partial unique index allows only one default, ever.
   - `pool_members`: `member_ref` is `work:<id>` — the Work item the session belongs to (ADR-0011) — or `session:<id>` for a session that hasn't switched yet. One active row per member (a partial unique index on `left_at IS NULL`). A session with no row is in All sessions, as a member. A session row made before a switch stands for the Work item after it (`MemberRef::anchor`), so a switch keeps the pool and the role.
   - Roles: lead, member, observer. The user is the implicit owner and isn't a row.
   - `pool_grants` holds only what the user changed; the defaults are the plan doc's table (§2.3) in `default_grant`. "See the pool" and "read their own messages" are fixed for every role and never written.
   - Deleting a pool marks it and ends its rows (`left_at`): its sessions are back in All sessions, its history stays, and the name is free again.
   - D-33 (to be ruled): `default_pool_scope(project, account)` is the one function that decides what the default pool covers — one project and one identity (the account, until the ruling says how accounts group).
2. **A switch starts its session's own Work item** (`SwitchGraphRepo::work_for_switch`). Before, every switch in a project went into the project's oldest Work item, so a pool hung off it would have taken in every switched session there. A session's later switches, its switch back and undo stay in its Work item, as before.
3. **Commands (`crates/juggler-app/src/pools.rs`)**: `pools_list`, `pool_create`, `pool_rename`, `pool_delete`, `pool_move`, `pool_set_role`, `pool_set_grant`. Each returns the whole picture. Refusals are in words: an empty, too long (> 48) or taken name (any case, "All sessions" included); renaming or deleting All sessions; a session from another project into a pool for one project; a fixed grant; a pool or session that isn't there any more.
4. **UI.**
   - A session's menu in the list (the shared `RowMenu`): under "Move to", the other pools (up to four), and **New pool…**.
   - Filters › **Group: Pool** (offered once a pool exists): pools read like project groups — the fold, the name, the count; a pool with nobody in it keeps its place; the header opens the pool. The first pool made while the list is open groups the list by pool; at start-up the list keeps its grouping. **New pool** sits under the groups.
   - Filters › **Pool**, and **Pool** on Home (remembered): one pool's sessions only. Both appear only once a pool exists.
   - **The pool sheet** (`PoolSheet.tsx`): New pool (a name, optionally one project; the session it came from goes in). A pool: rename, each session with its role, Add a session…, Take out, a D-33 line (All sessions: "Agents here will see only sessions on their own project and account."; a user pool mixing projects or accounts says so), **What each role may do** behind a disclosure ("Agents can't reach pools yet; these apply once they can"), Delete pool with what happens said first. All sessions has no rename, delete, add or take out.
5. **Downgrade**: `SCHEMA_RELEASES` names 0.0.11 for schema 29 (0.0.10 and 0.0.11 kept it); the RELEASING table has the next release's row and the CHANGELOG the "Going back to 0.0.11 or earlier" line. The release that ships schema 31 adds its own entry.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 storage: name, kind, optional project | core `pools_storage` (7 tests); app `pools :: a_pool_for_one_project_refuses_a_session_from_another_in_words` | PASS |
| Req.1 members hang off the Work item; a switch keeps them | core `a_session_row_from_before_a_switch_stands_for_the_work_item_after`; app `membership_hangs_off_the_work_item_so_a_switch_keeps_it`; end to end through the real switch (`switch_execute_impl`): `switch_door :: a_switch_keeps_the_pool_of_its_own_line_of_work_only` — **fails on the old switch code** (the other session's continuation was pulled into the pool), checked by reverting the change | PASS |
| Req.1 roles; the user the implicit owner | app `a_planner_and_two_builders_in_checkout_redesign_with_the_planner_as_lead` (the user story); an unknown role ("owner") refused | PASS |
| Req.1 grants: the plan table, UI edits only | app `grants_follow_the_plan_table_and_only_change_where_the_user_changes_them` (the full table; an override per pool; back to the default deletes the row; kept across a restart; fixed rows refused); core `grants_are_the_defaults_until_the_user_changes_them`. The only writers are the app's commands, which only the UI calls (there is no bridge) | PASS |
| Req.2 All sessions always exists, can't be deleted, one project + one identity | core `there_is_one_default_pool_and_the_repository_wont_rename_or_delete_it`; app `the_default_pool_always_exists_and_holds_every_other_session`, `scopes_count_projects_and_accounts_d33` | PASS |
| Req.3 create, rename, delete; move; roles; the pool filter on Home and the rail | vitest `pools.test.tsx` (17); Playwright `pools-gallery` (12, both themes, axe 0 critical/serious on each state) | PASS |
| Req.3 the same rows, headers and folds | Gallery `rail-by-pool-*`, `rail-pool-folded-*`; regression baselines `rail-by-pool-*`, `pool-sheet-*` | PASS |
| Req.4 the guardrail: never spawn, stop or type into a session | app `pool_operations_never_start_stop_or_type_into_a_session`: a real PTY session (its stdin logged, TERM/HUP/INT trapped) — every pool operation, refused ones too, then: still alive, nothing typed (the probe line proves the seam), no signal, no new spawn context, no ledger row, status unchanged. `the_pool_code_has_no_path_to_a_session`: neither pool module mentions the supervisor, spawn, `write_input`, terminal, launch, stop, kill, signal or a command | PASS |
| Migration with the pre-migration copy | core `upgrading_keeps_a_copy_adds_all_sessions_and_leaves_every_row` (the copy is the previous schema, table for table; the old rows stay; a second open migrates nothing); `storage_roundtrip` v4 → v31 and v12 → v31 once; the Dev build walk below | PASS |
| Performance: the rail's budget at 50 sessions | `load.spec.ts` in the full Playwright run (see below); pool lookups are per row over the members list, no new request | PASS |

Gallery: `docs/qa/evidence/_gallery/82-pools/` (24 captures).

## Gates run for this PR

See the PR description for the exact outputs: `just verify`, `cargo deny check`, vitest (all), the full Playwright suite, `scripts/a11y_gate.sh`, `scripts/visual_gate.sh --update` in Docker.

## Walked by hand

- **Browser preview** (1280×800, dark): a session's ⋯ → New pool… → "Checkout redesign" → the sheet opened on the new pool with the session in it; set it Lead; added a second session (Personal) — the sheet said the pool mixes projects or accounts; What each role may do opened, the fixed rows locked; Done → the list grouped by pool: Checkout redesign 2, All sessions 1, New pool under them; Home showed Pool: all.
- **The Dev build** (this branch's debug binary, copied aside — the shared target's binary is whichever session built last) on a throwaway HOME (`env -i HOME=… PATH=<stand-in claude/codex>`):
  - a fresh start made a v31 database with `pool-default|All sessions|default`, no members, no grants;
  - rewound to v30 (pool tables dropped, `user_version` 30, a marker setting added) and started again: it kept `juggler.sqlite.before-v30` (v30, the marker, no `pools` table), upgraded to v31 with All sessions and the marker;
  - a third start migrated nothing (no new copy).
  Its window can't be clicked from an agent session on this Mac (no Accessibility grant), so the clicks were walked in the preview against the same UI; the commands behind them are the ones the Rust tests call.

## Not covered here

- Clicking through the real window (above).
- Agents reading pools, roles or grants: AJ-E1704. Nothing reads the grants yet; the sheet says so.
- How accounts group into one identity (D-33): until the ruling, each account is its own.

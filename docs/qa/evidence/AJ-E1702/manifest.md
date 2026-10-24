# Evidence manifest — AJ-E1702

Task: `docs/tasks/E17-agent-pools.md` AJ-E1702, pools, the default pool, roles and permissions (P0).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-24, claude-opus.

Built from the parked branch `origin/claude/e17-pools` (16862f35), brought onto 0.2 and adapted, then rebuilt to the maintainer's rulings of 2026-10-24 (D-28 access levels, D-29 delivery data, D-31, D-32, D-33). Agents can't reach pools yet: the bridge is AJ-E1704.

Gallery: `docs/qa/evidence/_gallery/91-pools/` (both themes, axe 0 critical or serious on each state).

## What exists now

1. **Storage, schema v33** (0.1.0 is at 32). The pools module owns its tables (`juggler-core/src/storage/pools.rs`, `MIGRATION`), placed in the one sequence as `V33`.
   - `pools`: name, kind (`default` | `user`), optional project, `deleted_at`, and per pool: `access_level` (starts `work_together`), `custom_actions`, `hop_limit` (5), `delivery_stopped`, `spans_macs` (off). The migration makes "All sessions" (`pool-default`); a partial unique index allows one default, ever.
   - `pool_members`: `work:<id>` (the Work item, ADR-0011) or `session:<id>` before a switch; one active row per member; no row is All sessions, as a member. A session row made before a switch stands for the Work item after it.
   - `agent_rules` (action, pool, `created_at`, `made_by` = user, `revoked_at`), `agent_approvals` (each "allow once"), `agent_audit` (every agent request and its answer).
   - Deleting a pool marks it and ends its rows; its history stays and its name is free.
2. **A session's first switch starts its own Work item** (`commands::record_edge_staged`; AJ-E1541's chain reuse stays), so a pool follows one line of work, not every switch in the project.
3. **D-28 as data** (`juggler-core/src/agent_access.rs`): the action catalog (`read_roster` … `prepare_launch`), the levels as sets, and `decide`. **One decision point**: `AgentAccessRepo::allowed` (core, writes the audit) behind `features::pools::access::allowed(state, session, pool, action, hop)` (resolves membership and role). Above the level: `Ask { offer_always }`, `offer_always` from the second identical approval. An observer reads without an ask and asks for the rest. Not a member, or no pool: `Deny`. A failure to decide is a deny.
4. **D-29 data**: `deliver_to_session` only at Full or by a user rule (never a Custom set's); a deny while the pool's delivery is stopped; at the hop limit it pauses and asks, and no rule lifts that.
5. **D-33**: `default_pool_scope(project)`, the project only; a pool's `scopes` counts its projects. **D-32**: `spans_macs`, set by the user only.
6. **Commands** (`features/pools/commands.rs`, async through `ipc::blocking`): reads `pools_list` (served by the Dev bridge); writes `pools_create`, `pools_rename`, `pools_delete`, `pools_move`, `pools_set_role`, `pools_set_access`, `pools_set_delivery`, `pools_set_spans_macs`, `pools_revoke_rule` (the ask's answer is `pools::access::answer_impl`, tested, for E1704/E18 to wire to their Needs you flow). Refusals are in words.
7. **UI** (pools stay out of sight until you make one):
   - a session's menu: **Move to pool…** (the action registry), a sheet with every pool it can join, or a new pool (straight to it with no pool of yours yet);
   - the rail: **Group: Pool** once a pool exists (the first one made switches to it), a pool filter in Filters, an empty pool keeps its place, a pool's header opens its page, **New pool** under the groups;
   - Home: a pool picker in the header (remembered);
   - the pool sheet (lazy): rename, roles, add and take out, delete with its consequence said first; "All sessions" can't be renamed or deleted; what agents may do behind a disclosure;
   - agent access: the level picker, Custom's actions, the hop limit and stop-all, the rules with **Take back**; on each pool's page and in **Settings › Agent access**.
8. **Docs**: `docs/guide/pools.md`; the CHANGELOG; the Privacy page's new line (pools never act on a session); `docs/RELEASING.md` (0.1.0 = schema 32, 0.2 = 33) and `SCHEMA_RELEASES` (32 → 0.1.0).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 a pool: name, kind, optional project | `crates/juggler-core/tests/pools_storage.rs` (10); app `pools::a_pool_for_one_project_refuses_a_session_from_another_in_words` | PASS |
| Req.1 members hang off the Work item; a switch keeps them | core `a_session_row_from_before_a_switch_stands_for_the_work_item_after`; app `membership_hangs_off_the_work_item_so_a_switch_keeps_it`, `a_switch_from_a_session_starts_its_own_work_item`; through the real switch: `switch_door::a_switch_keeps_the_pool_of_its_own_line_of_work_only` | PASS |
| Req.1 roles, the user the implicit owner | app `a_planner_and_two_builders_in_checkout_redesign_with_the_planner_as_lead` (the user story) | PASS |
| Req.1 permissions (D-28): levels, ask above, rule offer from the 2nd approval, revocable at once, audit | core `agent_access::tests` (6); core `a_pool_starts_at_work_together_and_the_user_sets_its_level`, `the_second_allow_once_offers_a_rule_and_a_revoked_rule_stops_at_once`, `not_a_member_or_no_pool_is_a_deny_and_still_audited`; app `access_levels_are_the_users_and_the_decision_point_asks_above_them` (through `pools::access::allowed`) | PASS |
| D-28 hard lines | app `no_level_or_rule_can_reach_a_hard_line` (no catalog action names a pool, level, rule, account, credential, start or permission; `prepare_launch` in no named level; the decision point writes nothing but the audit) | PASS |
| D-29 hop limit and stop-all | core `delivery_is_fulls_or_a_rules_stops_at_once_and_pauses_at_the_hop_limit`, `a_pool_keeps_its_hop_limit_and_delivery_stop_d29`; app (the hop limit and stop-all through the decision point) | PASS |
| D-31 no quota | app `pools_and_agent_access_never_read_quota` | PASS |
| D-32 spans Macs, off by default | app `a_pool_spans_paired_macs_only_when_the_user_turns_it_on` | PASS |
| Req.2 All sessions always there, not deletable, one project whatever the account (D-33) | core `there_is_one_default_pool_and_the_repository_wont_rename_or_delete_it`; app `the_default_pool_always_exists_and_holds_every_other_session`, `scopes_count_projects_whatever_the_account_d33` | PASS |
| Req.3 UI: create, rename, delete, move, roles, filters, level picker | vitest `ui/src/components/pools.test.tsx` (20); Playwright `ui/tests/visual/pools-gallery.spec.ts` (13: six states × two themes with axe, and a keyboard walk) | PASS |
| Req.4 the guardrail: never spawn, stop or type into a session | app `pool_operations_never_start_stop_or_type_into_a_session` (a real PTY session, every pool operation including access changes and refused ones: still alive, nothing typed, no signal, no spawn, no ledger row, status unchanged); `the_pool_code_has_no_path_to_a_session` (every file of `features/pools` and the core module) | PASS |
| Migration from 0.1.0 with the pre-migration copy | core `upgrading_from_0_1_keeps_a_copy_adds_all_sessions_and_leaves_every_row` (built at schema 32, the copy is `before-v32` table for table, the rows stay, a second open migrates nothing); `storage_roundtrip` v4 → v33 and v12 → v33; `busy_timeout` and `switch_graph` undo v33 | PASS |
| Performance: the rail at 50 sessions | the pools are read once after first paint and when sessions change; grouping and filtering are a lookup per row over the members list, no new request per row | PASS (by construction; the load gate runs centrally) |

## Gates

Run in this worktree on 2026-10-24: `just verify`, the clippy variants, vitest (all), `npm run typecheck`, `scripts/size_budget.py`, the plain-words and terminology lints, `evidence_index.sh` and `evidence_check.sh`, and the pools gallery locally (chromium). The Docker visual and a11y gates run centrally; the gallery gate's `91-pools` baseline (`rail-by-pool-1280x800-{dark,light}.png`) is new.

## Walked by hand

In the browser preview (1280×800, dark): a session's menu › Move to pool… › New pool… › "Checkout redesign" › Make pool: the sheet opened on the pool with the session in it; What agents may do unfolded on Work together; Done: the list grouped by pool (Checkout redesign 1, All sessions 2, New pool), Home showed **Pool: all**; the pool's header opened its page; a card unfolded with "Unavailable" and why; Agent access showed the levels, Messages put into sessions and Include my paired Macs; Settings › Agent access listed both pools. Found and fixed on the way: Move to pool with no pool yet now goes straight to New pool; a doubled "What agents may do" heading; "1 min ago" wrapping.

## Not covered here

- The real window (no Accessibility grant from an agent session on this Mac); the commands behind the clicks are the ones the Rust tests call.
- The ask itself in Needs you, delivery and its loop caps (rate, duplicates, between turns, labels): E1704 and E18 call `allowed` and enforce them.
- Roster filtering by D-33 scope for agents (what an agent in All sessions sees): AJ-E1704.

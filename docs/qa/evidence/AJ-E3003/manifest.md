# Evidence manifest — AJ-E3003

Task: `docs/tasks/E30-pools-redesign.md` AJ-E3003, the pool model stored
(schema v45) and 0.3's pools moved. Built 2026-11-04. Checked on
2026-11-06 (release/0.4.1). Rust tests are cited by name; they run in the
release preflight. Status: **DONE, 1 gap open** (agent self-check,
2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | GAP | The schema has it all (`crates/juggler-core/src/storage/pools.rs` `MODEL_MIGRATION`: `pools.scope_all`, `pool_scope`, `sealed`, `skip_held`, `pool_members.level`, `pool_talk`, `tasks.project`). Tests prove most of it: additions only and `tasks.project` filled (`juggler-core/tests/pools_v45.rs` `every_0_3_pool_kind_and_its_content_moves_with_additions_only`); every member kind's ref round-trips (`pools_storage.rs` `member_refs_round_trip_and_unknown_words_are_skipped`); a member's own level is stored and applied (`juggler-agentbridge/tests/bridge.rs` `a_session_in_two_pools_lists_both_and_names_one_on_a_write`); a role-pair rule and a one-connection override (`juggler-app/tests/messages.rs` `a_pools_talk_rules_hold_or_refuse_an_agents_message_d62`); sealed (`juggler-app/tests/pools.rs` `a_switch_says_what_happens_to_each_pool_d62`). Missing: no test stores a scope of several projects ("these projects", `Pools::set_scope` or `create_scoped` with two projects) and reads both back. |
| R2 | PASS | `juggler-core/tests/pools_storage.rs` `one_active_row_per_pool_and_member_a_move_ends_the_old_one_and_a_join_keeps_it`: one session joins two pools and keeps both rows; joining again changes its row, no second one; the database refuses two active rows for one member in one pool. |
| R3 | PASS | `pools_v45.rs` `every_0_3_pool_kind_and_its_content_moves_with_additions_only`: a v39 database with every 0.3 pool kind maps as D-62 says (a default pool → its project, everyone, skip held, same id; no-project → no project; a user pool with a project → that project; one without → Every), every session lands where 0.3 had it, and the pre-migration copy is taken first. Default pools can't be renamed or deleted: `pools_storage.rs` `each_project_gets_one_default_pool_the_repository_wont_rename_or_delete`. The P9 lab's own 0.3 data folder moves whole: `the_p9_walks_0_3_data_folder_moves_whole`. |
| R4 | PASS | `PoolRole::Reviewer` beside Lead, Member and Observer (`storage/pools.rs`, `PoolRole::ALL`). `pools_storage.rs` `one_active_row_per_pool_and_member…` joins a session as Reviewer; `pools_resolver.rs` `a_project_pool_skips_sessions_named_elsewhere_but_not_ones_a_broad_member_takes` resolves a reviewer to `PoolRole::Reviewer`. |

Performance (the move < 1 s for 100 pools and 10 000 messages):
`pools_v45.rs` `the_move_takes_under_a_second_for_100_pools_and_10_000_messages`.

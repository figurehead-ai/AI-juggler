# Evidence manifest — AJ-E3102

Task: `docs/tasks/E31-everything-links.md` AJ-E3102, relations made cheap:
instance and conversation indexes, and a relation-count cache. Built
2026-11-03. Checked on 2026-11-06 (release/0.4.1). Rust tests are cited
by name; they run in the release preflight. Status: **DONE-EVIDENCED**
(agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | The instance indexes: `juggler-core/tests/relations_storage.rs` `every_relation_column_has_its_index_and_a_count_uses_it` (each column's query plan uses its index) and `an_upgrade_from_v42_adds_the_indexes_and_stamps_with_every_row_kept`. The conversation index (`features/conversations/by_relation.rs` `ByRelation`, by project, App, tool and folder; `each_conversation_is_found_by_its_project_app_tool_and_folder`) is read by the relation counts (`relations::current`) and, since 2026-11-06, by the related lists: `sessions::rows::narrowed_outside` keeps only the outside conversations the index puts in a list's project, App or folder before any row is built, through the same stamp-cached `relations::conversation_index`, so a count and its list are read from one index. `juggler-app/tests/relations.rs` `every_count_equals_the_list_it_heads_and_a_recount` (with outside conversations in the fixture), `paged_lists.rs` and the sessions lib tests pass with it. |
| R2 | PASS | `juggler-app/tests/relations.rs`: `a_change_shows_in_the_next_count` (an unchanged read is served from the cache with no rebuild; a change no event announced moves a database stamp and shows in the next count; a new transcript too); `a_cache_cleared_mid_walk_comes_back_the_same` (Settings names the cache "Counts on pages" with its size; the user's clear empties it; the next counts come back the same, built once; off keeps nothing). `relations_storage.rs` `a_stamp_moves_only_when_what_decides_a_relation_changes`. UI: `ui/src/lib/relation-counts.test.tsx` "reads an entity's counts and again after a change". |
| R3 | PASS | `relations.rs` `every_count_equals_the_list_it_heads_and_a_recount`: on 600 sessions and 60 outside conversations, every page's counts equal the list they head (all, open, ended, archived) and a recount from the rows. |

Performance (a count ≤ 5 ms from cache; 10 000 sessions rebuilt cold ≤ 1 s):
`relations.rs` `ten_thousand_sessions_count_cold_within_a_second_and_cached_within_5_ms`.

Not yet done: the K1, K2 and X1 walks by hand (E33's validation).

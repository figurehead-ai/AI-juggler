# Evidence manifest — AJ-E3004

Task: `docs/tasks/E30-pools-redesign.md` AJ-E3004, membership resolved.
Built 2026-11-04. R1's "kept in memory, rebuilt on change" was finished on
2026-11-06 (release/0.4.1). Status: **DONE-EVIDENCED** (agent self-check,
2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | **One resolver:** `juggler_core::pools::Memberships`, indexed by member kind (App, Account, Tool, Mac, project, Work item, session). **Kept in memory:** `Storage::memberships` holds it and rebuilds only when the v48 `memberships` stamp moves. Triggers move it on any change to `pools`, `pool_scope` and `pool_members`, and on a change to a Work item in `epochs`. Every caller (`pools::members::memberships`: the pool pages, cards, canvas, switching, relation counts, the agent bridge) reads it from there, so sessions are never re-read per call. **Test:** `juggler-core/tests/memberships_cache.rs` (`the_maps_are_kept_until_a_pool_or_member_changes`) proves the same maps come back until a member joins, and that sealing a pool, which the older `pools` stamp missed, rebuilds them. |
| R2 | PASS | **The most specific member wins inside a pool; each pool's rules hold across pools; a sealed pool takes its sessions out of every other:** `juggler-core/tests/pools_l1.rs` (`membership_matches_the_reference_model_in_all_28_224_cases`, against `scripts/qa/pool_oracle.py`; talk rules in all 9,216 cases), `pools_resolver.rs` (`a_project_pool_skips_sessions_named_elsewhere_but_not_ones_a_broad_member_takes`). |
| R3 | PASS | **A session left out says why and what would let it in** (`SkipReason`, e.g. `HeldElsewhere`): `pools_resolver.rs`. **An unreadable sign-in** reads "Account unknown" and the session stays in through another member: `an_unknown_sign_in_stays_in_through_any_other_member_and_says_so`. |
| R4 | PASS | **whoami lists the session's pools, and a write names its pool:** `juggler-agentbridge/tests/bridge.rs` `a_session_in_two_pools_lists_both_and_names_one_on_a_write`. whoami lists both pools with how each was joined; an unnamed write in two pools is refused ("is in 2 pools"); a named one lands there. Added 2026-11-06: a write named for a pool the session isn't in is refused with its reason ("isn't in the pool"). |

Performance (≤ 10 ms for 500 sessions and 50 pools):
`pools_resolver.rs` `five_hundred_sessions_against_fifty_pools_resolve_within_ten_ms`,
in a debug build. With R1 the maps are built once per change, not per call.

L2 on the replica (P4, P5, P11, X2) is E33's validation, tracked under
AJ-E3302–E3306.

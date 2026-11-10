# Evidence manifest: AJ-E3304

**Task:** `docs/tasks/E33-validated-by-use.md`, AJ-E3304: pools validated by the exhaustive tables, the pairs, and P1–P11 by hand.

**Branch:** `release/0.4.1`. Live rows were walked on 0.4.1 release candidates of AI Juggler Dev (ADR-0027 conditions 7 and 7b) and checked by Opus in the release session (its walk results, 2026-11-09..10). The final candidate is rc31 (35c181e2d). Walk captures show real names and stay out of the repo.

## By requirement

| Req | Result | Evidence | Commit | Date |
|---|---|---|---|---|
| R1 L1: membership (28 224), talk (9 216), Claude's channel (27) and migration tables agree with the reference model | PASS | `crates/juggler-core/tests/pools_l1.rs` against `scripts/qa/pool_oracle.py`: `membership_matches_the_reference_model_in_all_28_224_cases`, `talk_matches_the_reference_model_in_all_9_216_cases`, `claudes_own_channel_matches_the_reference_model_in_all_27_cases` (its 7 hold-only rows accept, marked not enforceable, after D-95); migration: `pools_v45.rs`, `pools_storage.rs`. Run: `cargo test -p juggler-core --test pools_l1 --test pools_resolver --test pools_storage --test pools_v45`, green in CI on the release PR | 471d8ab9b | 2026-11-09 |
| R2 L2 on the replica; L3 wizard (~60 rows) and canvas (~40 rows) pairs plus the named risky triples | PASS | **L2:** the app's pool commands (`cargo test -p juggler-app --test pools --test pools_l2 --test pools_shared_limit`, 8e2fc687e); on the replica, 316 sessions, 0 problems ([l2-replica](l2-replica-2026-11-07.md)). **L3:** wizard, all 70 rows (`ui/tests/visual/l3-wizard.spec.ts`, `docs/qa/evidence/l3/wizard.json`, 7aaf1570e); canvas, all 42 rows (`l3-canvas.spec.ts`, `canvas.json`; the group column covering a member fixed in 7dd49af7f). **The seven named triples** (`docs/qa/0.4-validation-plan.md` §L3): 1 sealed × several pools × a switch into a sealed App, `pools.rs` `a_switch_says_what_happens_to_each_pool_d62` (3eae09e4d); 2 `messages.rs` `a_watched_member_behind_ask_first_is_held_as_ask_first_and_claudes_channel_is_untouched`; 3 Account member × unreadable sign-in × a more specific member, `pools_resolver.rs` `an_unknown_sign_in_stays_in_through_any_other_member_and_says_so` (13f9ef058); 4 `pools_shared_limit.rs` `two_pools_at_full_both_wake_one_session_and_stop_all_holds_only_its_own`; 5 `tasks_oversight.rs` `a_reviewer_at_its_own_read_level_waits_on_the_person_for_its_verdict`; 6 `pools.rs` `an_app_signing_in_to_another_account_moves_its_open_session_between_account_pools_only`; 7 `tasks.rs` `a_pool_over_two_projects_keeps_one_task_list_that_the_project_filter_splits` (2, 4–7 in d935f3d9e) | 8e2fc687e, e3a332da0, 7aaf1570e, 7dd49af7f, d935f3d9e | 2026-11-07 |
| R3 L4: P1–P11 in the lab with real sessions messaging, holding, refusing and switching | PARTIAL | Walked, step by step, in the table below. **Not proven:** P1 S9, Continue on a Watched card not walked on rc29 (the card and toast were; the aij-test CLI isn't signed in); P1S6 S2, the switch check from an out-of-room writer, blocked (the lab's Codex isn't signed in); P5, all four steps covered only by the L1/L2 resolver tests, never walked (the coordinating session's ruling of 2026-11-05: no sign-in made unreadable, no second sign-in); P8 S4, Kimi Code isn't installed, so a Kimi scout doing what its shell allows isn't walked (its label is checked against the adapter, 9874d9bd9); P11 S1, two Full pools waking one session at once was never set up live (the call and hop limits are held by tests: c3aa6639b, d935f3d9e) | — | 2026-11-10 |

## By scenario (L4)

| Scenario | Sheet | Observed | Verdict | Commit | Date |
|---|---|---|---|---|---|
| P1 Writer and reviewer | [P1](../../0.4-walks/P1.md) | rc27 live pass 1: S1–S5 and S7 met. S6, the switch: the Lead hand-off fixed (b6ad1013a), re-walked on rc29 and met. S8 met (the Claude app's App row says its sessions join watched). S9: Add as watched gave the dashed card and toast on rc29; Continue not walked | PARTIAL (S9) | b6ad1013a, b00b60179 | rc27, rc29 |
| P1S6 The writer's Account runs out | [P1S6](../../0.4-walks/P1S6.md) | rc29 lab: S1 met (the out-of-room notice, chips, Accounts). S2 blocked: the lab's Codex isn't signed in. S3: the canvas's Out mark fixed (b00b60179) | PARTIAL (S2) | b00b60179 | rc29, rc30 |
| P2 Feature team | [P2](../../0.4-walks/P2.md) | rc27 live pass 1: S1–S6 met | PASS | 2674e6919, 0c213f042 | rc27 |
| P3 Across repositories | [P3](../../0.4-walks/P3.md) | rc26 lab: S1–S3 met; S4 is the agents' (tasks are agent-owned): walked with live agents in P1 and P2 (sheet), the project filter held by triple 7 | PASS | — | rc26 |
| P4 A reviewer for everything | [P4](../../0.4-walks/P4.md) | rc26 lab: S2 met. rc27 live pass 2: S1 (refused: "This pool doesn't let you message that member."), S3, S4 met | PASS | 6d8159d01 | rc26, rc27 |
| P5 Rules by account | [P5](../../0.4-walks/P5.md) | Not walked: steps 1–4 are covered by the L1/L2 resolver tests only (`pools_resolver.rs`, triples 3 and 6 above); its read-only part looked at on the real setup | PARTIAL (tests only) | 13f9ef058, d935f3d9e | 2026-11-05 |
| P6 A build Mac | [P6](../../0.4-walks/P6.md) | Walked on the preview's paired-Mac seam, 2026-11-05 (pools across Macs are 0.5); both steps met | PASS | — | 2026-11-05 |
| P7 Sealed client project | [P7](../../0.4-walks/P7.md) | rc26 lab: S1 and S5 met. rc27 live: S2 and S3 met (Claude's own SendMessage into the sealed session refused, not delivered). S4: the scenario corrected (72bf704da); the sealed-pool ask in a switch held by `a_switch_says_what_happens_to_each_pool_d62` | PASS | 57f89e1bb, a3903836e, 72bf704da | rc26, rc27 |
| P8 Scouts | [P8](../../0.4-walks/P8.md) | rc27 live pass 2: S1–S3 met (the OpenCode and Grok scouts used `send_message` and `share_doc` with no prompt). S4: Kimi Code isn't installed; only its "Shell only" claim is checked (`a_scenarios_shell_only_claim_is_the_adapters`) | PARTIAL (S4) | 9874d9bd9 | rc27 |
| P9 Moving from 0.3 | [P9](../../0.4-walks/P9.md) | rc26 lab: S1–S3 met | PASS | — | rc26 |
| P10 Set up by hand, first time | [P10](../../0.4-walks/P10.md) | rc26 lab: S1 and S3 met; S2 met on the 2026-11-05 lab | PASS | — | rc26 |
| P11 Overlap and deletion | [P11](../../0.4-walks/P11.md) | rc26 lab: S2 and S3 met. S1 (both pools wake the shared session at once) not set up live; held by `pools_shared_limit.rs` `two_pools_chains_through_one_session_each_pause_at_their_own_hop_limit` (c3aa6639b) and triple 4 | PARTIAL (S1) | c3aa6639b, d935f3d9e | rc26 |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] No captures here (the live walks' show real names)
- [x] No conversation content

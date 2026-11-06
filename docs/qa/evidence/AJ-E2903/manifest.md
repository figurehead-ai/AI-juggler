# Evidence manifest — AJ-E2903

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2903, quality
per account. Built in 0.4. Checked on 2026-11-06 (release/0.4.1). vitest
`account-quality.test.tsx` and `quality-tab.test.tsx` pass (2026-11-06);
the Rust tests named below run in the release preflight. Status:
**DONE-EVIDENCED** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | **Figures:** `crates/juggler-app/tests/account_quality.rs` `an_accounts_week_in_figures_with_their_sources` rolls two folders of one Account up over 7 days: 9 errors in 300 turns (a session with no turns left out, not zero), 3 stuck episodes from the recorded state changes, context at finish 50 % over the 2 sessions that reported it, and 2,000 tokens per finished task over the 2 Finished sessions (D-77, as AJ-E2902). `crates/juggler-core/tests/quality_rollup.rs` `stuck_episodes_and_the_last_change_come_from_the_recorded_transitions` counts stuck episodes from `instance_transitions`, and `the_end_figures_are_kept_on_the_usage_record_and_old_records_still_read` keeps the end figures when a session ends. **On screen:** `ui/src/pages/account-quality.test.tsx` "says each signal in words, with where it comes from" asserts "9 in 300 turns" (no %), "2 times", "71% on average, over 11 sessions", "412k, over 9 tasks", and "sessions it didn't run aren't counted"; "a tool without an error feed or context leaves those rows out, said once" asserts "Codex doesn't record errors or context, so those rows are left out." (the 0.4 review, 80, one line in place of a "Not reported" line on each row). **Where:** `ui/tests/visual/account-quality-gallery.spec.ts` opens an Account's page and finds the section there. |
| R2 | PASS | `account-quality.test.tsx` "says each signal in words, with where it comes from" asserts the source once under the title ("14 sessions AI Juggler ran, read from each session's own files when it ended") and the stuck row's own source ("sessions it didn't run aren't counted"). |
| R3 | PASS | `ui/src/components/quality-tab.test.tsx` "AJ-E2903: an ended session shows its context at finish and how often it got stuck" asserts "71% (142,000 / 200,000)" and "2 times, from the states AI Juggler recorded" on an ended session, and no context-at-finish line on a live one. |
| R4 | PASS | `crates/juggler-core/tests/quality_signals.rs` `no_composite_score_exists_in_crates_or_ui` (the D-07 grep gate over `crates` and `ui/src`); `account-quality.test.tsx` asserts the section has no "score"; `quality-tab.test.tsx` "no composite score exists in the UI" checks the session tab. |
| R5 | PASS | `account-quality.test.tsx` "waits until there's something to show" asserts no section with 0 sessions in the 7 days; `account_quality.rs` asserts 0 sessions a week later. |

Performance (one read ≤ 30 ms; context at finish stored, never re-read on
open): `account_quality.rs` `settling_keeps_an_ended_sessions_figures_once`
asserts an ended session's figures are kept once and not read again.

Not done from the task's evidence list: the hand count on the replica
fixture (`just replica`).

Gallery: `ui/tests/visual/account-quality-gallery.spec.ts` (family
`99-account-quality`): measured and not reported, both themes, 1280×800
and 1024×700.

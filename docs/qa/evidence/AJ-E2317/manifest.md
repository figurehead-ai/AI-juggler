# Evidence manifest: AJ-E2317

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2317: Move picks the source by the conversation's holder and the target by the App the user picked, never by account.

**Branch:** `release/0.5.1`. Fixtures only: temp data folders, a made-up Mac, scripted systems. No real profile was read or written.

## Requirements

| Requirement | Verdict | Proof | Commit | Date |
|---|---|---|---|---|
| R1 the source is the conversation's holder; two holders refused | built, evidence not recorded here | `handover::moving::tests::picking::*` | 205dd4209 | 2026-11-23 |
| R2 Move's target is the App picked, by place id through `switch_move_execute` | built, evidence not recorded here | `handover::moving::tests::picking::two_profiles_on_one_account_the_source_by_its_records_the_target_by_the_pick` | 205dd4209 | 2026-11-23 |
| R2 follow-up: the Copy hand-off takes the picked App by place id (`switch_preflight` and `switch_execute` carry `target_place_id`; `handoff::profile_for` removed; no pick with two Apps on one account picks neither) | PASS | `places::handoff::tests::two_profiles_on_one_account_each_is_the_desktop_when_picked` (failed before the fix: the last-seen profile came back for either pick); `ui/src/lib/ipc/switchExecute.test.ts` "sends the App picked, by its place id"; `ui/src/components/switch-where.test.tsx` "sends the App picked in the list, by its place id"; `tests/switch_graph_targets.rs` green | ef8c39fee | 2026-11-23 |
| R3 two profiles on one account: fixtures and a PL1 walk | left: the PL1 walk | | | |

## Close-out (0.5.1, 2026-11-23, Opus)

Each test named below was found by name and run, targeted, on `release/0.5.1` (vitest by file; cargo through `scripts/heavy.sh`, `--lib` with module filters). An OPEN line says where it closes.

R1: PASS — cargo `handover::moving::tests::picking::two_holders_are_refused_naming_both`, `one_profile_remembered_under_two_spellings_is_one_holder`, `two_profiles_on_one_account_the_source_by_its_records_the_target_by_the_pick` (205dd4209).
R2: PASS — cargo `moving::tests::picking::the_target_is_the_app_picked_by_its_id_when_two_share_an_account`, `places::handoff_tests::two_profiles_on_one_account_each_is_the_desktop_when_picked`; vitest `lib/ipc/switchExecute.test.ts`, `components/switch-where.test.tsx`, `components/outside-conversations.test.tsx` (the pick's place id) (205dd4209, 1e88bf264, ef8c39fee).
R3: OPEN — PL1 step J1. The fixtures are in place: `two_profiles_on_one_account_the_source_by_its_records_the_target_by_the_pick`.

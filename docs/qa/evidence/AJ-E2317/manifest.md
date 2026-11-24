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
R3: PASS (2026-11-24, Opus's check of the PL1 final re-walk, `docs/qa/0.5-walks/0.5.1-PL1.md` after the Launched table, 88c871a76). The fixture: cargo `handover::moving::tests::picking::two_profiles_on_one_account_the_source_by_its_records_the_target_by_the_pick` (run 2026-11-24 in `--lib features::handover::moving`: 142 passed). The walk, PL1 J1 on a60ed66ef: no two desktop profiles share an account in the lab (no desktop App for `client` or `personal-2`, and a lab profile can't be signed in: D-147), so a live Move between two profiles on one account can't be made; what the lab could show, the live Switch dialog, named the source by its records ("now on Claude Code CLI · personal") and the target by the pick ("Claude Code CLI · client"), nothing switched (capture `docs/qa/evidence/AJ-E2323/pl1/S-J1-switch-client-a60-dark.jpg`). Opus ruled R3 closed on the fixture plus that check. Related, same class: F94 (F-PL1-27: opened from a desktop App's row, the dialog names the folder's CLI App as the source and doesn't offer the desktop App) is being fixed in 0.5.1 under its own finding; **pending at this writing** (no fix commit yet), so its fix commit is to be added here when it lands.
R2/R3 follow-up (F94, 0.5.1, 2026-11-24, Opus): PASS — the Switch dialog's source is the App the conversation is in, by its id (a desktop App's Code tab conversation is that App's, never its folder's CLI App), and every target is its App by its own id (`SwitchTargetInfo.app_id`; the source's App is never one, its folder's other desktop App is). cargo `switch::source_app::tests` (2, failed before), `--lib switch::` 21 pass, the switch integration tests (10 files) pass; vitest `components/switch-source-app.test.tsx` (3 of 4 failed before) and switch*, continue-on, launch-sheet* (294) pass; clippy -D warnings, ui_lint, private_data_lint, terminology_lint pass; gate compare: no baseline changed; 28-switch re-captured (f2dc62851, 503afb7f4, 1f51d6ec3).

# Evidence manifest: AJ-E2315

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2315: Move to account counted another profile's wrapper as the moving one's (a flag's path inside its folder); widened (R5) to one shared ownership module.

**Branch:** `release/0.5.1`. Closed out 2026-11-23 (Opus): each test below was found by name and run, targeted (`scripts/heavy.sh cargo test --offline -p juggler-app --lib -- ownership handover::moving lab_scope places::tree processes::lab_scope`; `-p juggler-handover --lib`; `-p juggler-platform --lib -- real_path`). Commits: `git log --grep=AJ-E2315`.

## By requirement

R1: PASS — a flag's value never marks a process; a wrapper is the profile's by the program it runs: `ownership::tests::a_flags_path_never_makes_a_process_this_profiles`, `this_profiles_wrapper_is_its_by_the_program_it_runs`, `places::tree` `a_flags_path_alone_never_marks_a_wrapper`, `another_profiles_wrapper_naming_a_folder_inside_it_is_never_in_it` (d62f6ca9c, 00179c60a).
R2: PASS — stronger evidence wins: `ownership::tests::another_profiles_evidence_wins`, `places::tree` `another_profiles_evidence_wins_over_a_mark`, `another_profile_and_ai_juggler_are_never_in_it` (d62f6ca9c).
R3: PASS — another profile's process never blocks with a "try again": `handover::moving::tests::wait::another_profiles_wrapper_never_blocks` (the d62f6ca9c test `what_another_profile_runs_is_never_a_try_again_refusal` was folded into it by AJ-E2318, 7e2bdb424).
R4: PASS — the exact shape (another profile's disclaimer → claude with an `--add-dir` into the moving profile) is not a member and the existing members still are: the R1/R2 tests above; the move suite passes (`handover::moving*` in juggler-app, juggler-handover's lib tests).
R5: PASS — one shared ownership module every site uses: `ownership::instance` tests (`an_instance_flag_spelled_another_way_is_the_same_profile`, `a_symlinked_profile_folder_is_the_folder_it_points_to`, `an_instance_flag_the_app_might_read_otherwise_tells_nothing`), `lab_scope` `an_add_dir_into_the_lab_never_makes_a_real_session_the_labs`, `processes::lab_scope` `an_add_dir_into_the_lab_never_makes_a_process_the_labs`, juggler-handover `two_spellings_of_one_folder_are_one_lock` (4351a2227), juggler-platform `real_path` tests (f5fe28f21), the self rule `handover::moving_tests::a_move_that_would_quit_ai_juggler_refuses_before_anything_quits` (8f9da6b15), the link by evidence `places::link_tests` (AJ-E2327).

The task's quality gate also asks for a live re-walk of Move on lab PL1 with another profile's `--add-dir` hold: `docs/qa/0.5-walks/0.5.1-PL1.md` steps I3 and J2 (a gate, not a numbered requirement).

## Redaction checklist

- [x] No credentials, tokens, or keys here
- [x] No captures here
- [x] No real names or home paths

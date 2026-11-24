# Evidence manifest: AJ-E1547

**Task:** `docs/tasks/E15-spec-gaps.md`, AJ-E1547: no system prompt the user wasn't told about first (one registry of every OS prompt, priming in setup, one just-in-time gate every trigger goes through).

**Branch:** `release/0.5.1`. Built and tested offline on this Mac; the app was not run and no real system prompt was raised. The registry was built by code audit and Apple's documentation, never by resetting privacy settings (no `tccutil`).

**The registry** is the committed doc [docs/design/os-prompts.md](../../../design/os-prompts.md) (PAT-OSP-1): 16 entries, each with its id, triggers (file:function), the system's words, why, what to click, what happens when declined, when this setup needs it and what background work does. The same table lives in code (`crates/juggler-app/src/os_prompts/registry.rs`); a test fails when the two drift.

## By requirement

| Req | Result | Evidence | Commit | Date |
|---|---|---|---|---|
| R1 one registry of every OS prompt, built by audit | PASS | `docs/design/os-prompts.md`; `os_prompts::tests::every_entry_is_complete_with_a_short_stable_id`, `every_trigger_names_a_function_in_its_file`, `the_design_doc_lists_every_entry_and_its_triggers` | (this commit) | 2026-11-23 |
| R2 priming in setup: the IPC for AJ-E1545's step | PASS | `os_prompts_list` (entries with `needed`, `status`, `primed`, `dialog`), `os_prompt_prime(id)`, `os_prompt_status(id)`, `os_prompt_request(id)`; `ui/src/lib/ipc/os-prompts.ts`; the dialog's picture `ui/src/components/OsPromptPicture.tsx` (drawn from the dialog's words, theme tokens). The wizard's cards switch to it in AJ-E1545 | (this commit) | 2026-11-23 |
| R3 one just-in-time gate, sheet first, outcome said; background never prompts; Bot's helper set up before its first start | PASS | `juggler_platform::os_prompt::Gate::through`; platform tests `the_sheet_comes_before_the_call_and_the_outcome_after`, `not_now_never_calls_and_never_primes`, `background_work_never_prompts_until_primed`, `an_entry_the_system_already_declined_gets_no_sheet_since_no_dialog_follows`, `not_now_is_never_a_decline_for_good_and_continue_is_never_asked_twice`, `the_outcome_says_granted_or_declined_with_the_way_back`; app test `background_notifications_wait_until_primed`; sheet `ui/src/components/OsPromptSheet.tsx` (vitest `os-prompt-sheet.test.tsx`, 6); Bot's helper: `helper.rs` `launch_checked` starts one of our own helpers only once `helper` is primed (`StartFailure::NotSetUp`, nothing started); Bot's setup guide (D-144: Bot is AI only, "Set up Bot") says "Not checked yet" and its one primary "Check this Mac" goes through the sheet first (`models-actions` checkAgain → `os_prompt_request("helper")`), vitest `engine-setup.test.tsx` "not set up yet" | (this commit) | 2026-11-23 |
| R4 enforcement | PASS | `scripts/os_prompt_lint.py` (+ `--selftest`) in `just platform` (verify-static); its allowlist `scripts/os_prompts.allow` holds the calls other sessions are routing (only shrinks) | (this commit) | 2026-11-23 |
| R5 the walk's dialog log | UNPROVEN | The newcomer-persona walk logs each system dialog in `docs/qa/evidence/os-prompts/0.5.1.md` (PAT-OSP-5); not run yet | — | 2026-11-23 |

## Still to route (each listed in `scripts/os_prompts.allow`)

| File | Change |
|---|---|
| `crates/juggler-app/src/features/conversations/goto.rs` `Real::osascript_with` | run through `juggler_platform::os_prompt::script(SYSTEM_EVENTS / TERMINAL / ITERM, Presence::Background, ..)` |
| `crates/juggler-app/src/features/conversations/goto.rs` `Real::send_link_to` (may ask) | `juggler_platform::os_prompt::send_link(APP_LINKS, os_prompt::user(), pid, link)` |
| `crates/juggler-app/src/features/places/handoff.rs` `Real::send` | the same |

## The walk's dialog log

None yet (R5). The walk writes `docs/qa/evidence/os-prompts/0.5.1.md`, one line per system dialog by the registry's id, and ends with its verdict line.

## Close-out (0.5.1, 2026-11-23, Opus)

Each test named below was found by name and run, targeted, on `release/0.5.1` (vitest by file; cargo through `scripts/heavy.sh`, `--lib` with module filters). An OPEN line says where it closes.

R1: PASS — cargo `os_prompts::tests::every_entry_is_complete_with_a_short_stable_id`, `every_trigger_names_a_function_in_its_file`, `the_design_doc_lists_every_entry_and_its_triggers` (8ddda6b6f).
R2: PASS — vitest `setup-wizard.test.tsx` "offers the gate's prompts this setup needs, not the ones other steps prime (AJ-E1547)", "one card per prompt, through the gate: the dialog's picture and what to click…" (dde1925b0).
R3: PASS — cargo juggler-platform `os_prompt::tests` (`the_sheet_comes_before_the_call_and_the_outcome_after`, `not_now_never_calls_and_never_primes`, `background_work_never_prompts_until_primed`, `the_outcome_says_granted_or_declined_with_the_way_back`, …), juggler-app `background_notifications_wait_until_primed`; vitest `components/os-prompt-sheet.test.tsx`, `engine-setup.test.tsx` "not set up yet" (Bot's helper through the sheet).
R4: PASS — `scripts/os_prompt_lint.py --selftest` and a full run: both exit 0. Its allowlist's leftover calls are AJ-E1548's (9ab911e54).
R5: OPEN — PL1 steps K1–K4 and Z1 (`docs/qa/evidence/os-prompts/0.5.1.md`, the walk's dialog log).

### Usability answers, matched against the built gate and sheet

1. The app, not macOS, says what is coming: matches (the sheet comes before the call, platform test above).
2. The dialog in macOS's words, its picture, which button, the outcome after: matches (`OsPromptPicture`, `os-prompt-sheet.test.tsx`, `the_click_words_say_the_button_and_the_switch`, the outcome test); the sheet's look: pending critique.
3. One gate and one registry: matches (the lint, the registry tests).
4. Nothing this setup can't use; background work never prompts: matches (`isn't a step when nothing needed is missing`, `background_work_never_prompts_until_primed`).
5. The registry lists each prompt, its trigger and words: matches (`the_design_doc_lists_every_entry_and_its_triggers`).
6. Watch-fors: Not now taken as a decline for good, the same sheet twice (match: `not_now_is_never_a_decline_for_good_and_continue_is_never_asked_twice`); a declined entry gets no sheet when no dialog would follow (matches); a dialog with no sheet before it: PL1 K1.

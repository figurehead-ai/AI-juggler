# Evidence manifest — AJ-E2102 Go to it

Task: `docs/tasks/E21-desktop-first.md` AJ-E2102. Re-audited against the
code, its tests and the gallery on `release/0.4.1` (HEAD 92d6e0ce6),
2026-11-07. Status: **IN-PROGRESS**. R3–R5 pass; R1 and R2 each have
one gap (below); R6 is deferred by D-89 (the second Mac). 0.4.1 added
"Go to it opens the exact session" for the Claude app (e07c0e65f, step 1;
fe5124663, step 2). The tests prove it; the live check on the
maintainer's real setup is still to do (the maintainer).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the app instance is activated by its process (`NSRunningApplication`), never by bundle id, so the right profile comes forward | cargo `juggler-app`, `src/features/conversations/goto/tests.rs`: `a_claude_instance_comes_forward_by_its_process_and_says_where_to_look`, `a_gone_reused_or_refused_app_says_so` (a reused pid is never brought forward), `a_claude_app_session_opens_itself_by_its_own_link_sent_to_its_profile_alone`; `src/focus.rs` `tests::without_window_focus_only_the_process_is_activated`, `tests::a_stale_pid_never_brings_another_profile_of_the_same_app_forward` (two processes share the bundle id, the recorded one is gone: neither is activated, "not running"), `tests::stale_pid_is_rediscovered_by_bundle_id_when_it_runs_once`; `src/features/places/handoff_tests.rs` `a_closed_profile_is_started_then_the_session_itself_opens_in_it_alone` | activation only by the instance's pid; never another profile by its bundle id | Go to it, a place's Open and the session link act on the instance's own pid; the focus service's fallback re-finds a relaunched app by bundle id only when one process has it (fixed in 5aefb6c36, found by this audit); focus tests 17/17. Live check pending (the maintainer) | PASS | 5aefb6c36 | 2026-11-07 | Claude (self) |
| R2 the command runs off the main thread with a 1.5 s limit and shows progress at once | cargo `juggler-app` `a_slow_step_answers_within_the_limit` (go_to_impl's `within(LIMIT)`, both commands async); vitest `components/go-to-feedback.test.tsx` "says Going to it… at once, then the outcome in the same toast" (the backend held pending: the toast reads "Going to it…" before it answers, then the outcome replaces it) | off the main thread; ≤ 1.5 s; feedback before the answer | vitest 1/1; cargo runs in CI | PASS | a80efb2d6 | 2026-11-07 | Claude (self) |
| R3 exact windows (Accessibility) and terminal tabs (Automation) only when the user turned them on in Settings; a click never raises a prompt | cargo `juggler-app` `goto/tests.rs`: `window_counts_and_minimised_need_window_focus_turned_on` (off: no `osascript`), `a_terminal_tab_is_selected_only_when_tabs_are_on`, `going_to_it_never_lets_macos_ask_and_says_one_click_more_will`; `focus.rs` `tests::without_window_focus_only_the_process_is_activated` (runs in CI); vitest `ui/src/components/terminal-tabs.test.tsx` "is off until turned on, and saves at once"; `ui/src/lib/go-to-consent.test.ts` "offers to open the session itself, and only that click goes again with asking allowed" | no System Events or AppleScript without the setting (off by default); the session link is sent without the consent prompt | pass: with the settings off only `activate <pid>` runs; the link goes with kAEDoNotPromptForUserConsent, and only the user's own "Open the session itself" lets macOS ask | PASS | ac0e41dc0, fe5124663 | 2026-11-07 | Claude (self) |
| R4 the outcome names the step reached ("… pick the session in its sidebar", "(3 windows)", "minimised — click it in the Dock") | cargo `juggler-app` `goto/tests.rs`: `a_claude_instance_comes_forward_by_its_process_and_says_where_to_look`, `window_counts_and_minimised_need_window_focus_turned_on`, `an_app_that_stays_behind_is_never_said_to_be_forward`, `a_codex_thread_that_opened_behind_says_both`, `a_refused_link_says_where_to_allow_it_and_an_unrecorded_session_keeps_the_sidebar_words`, `a_claude_app_session_opens_itself_by_its_own_link_sent_to_its_profile_alone` (runs in CI); vitest `go-to-consent.test.ts` "without a way to go again, it only says what happened" | each outcome's exact words; never "forward" when it stayed behind | pass: "Brought Claude · Work forward; pick the session in its sidebar", "… forward (3 windows); …", "Claude · Work is minimised — click it in the Dock", "Opened the session in Claude · Work", "macOS didn't bring … forward. Click it in the Dock". No gallery capture shows the outcome toast: `_gallery/14-focus/panel-outcome-1280x800.png` shows none (its critique says so). Live check pending (the maintainer) | PASS | ac0e41dc0, 7a84fdf4f, fe5124663 | 2026-11-07 | Claude (self) |
| R5 Codex threads by `codex://threads/<id>`; editors by `cursor://file/<folder>` or `code <folder>`; never an editor extension's session link | cargo `juggler-app` `goto/tests.rs`: `a_codex_thread_opens_by_its_link`, `an_editor_opens_the_folder_and_never_an_extension_link`, `the_registry_has_no_extension_session_link` (runs in CI); data `features/conversations/hosts.json` | the exact links; no `vscode://` | pass: `open codex://threads/019a…`, `open cursor://file//Users/you/Dev/my%20app`, `run code /Users/you/Dev/my app`; VS Code has neither `thread_link` nor `folder_link` | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R6 a session on another Mac reads "Bring forward on <Mac>" and runs there (AJ-E711) | — (the maintainer's after-release check, `docs/qa/second-mac-check.md`) | — | not built here; needs a second Mac | DEFERRED (D-89) | — | 2026-11-07 | Claude (self) |
| Gate: the cited vitest files | `cd ui && pnpm -s exec vitest run src/lib/go-to-consent.test.ts src/components/terminal-tabs.test.tsx src/components/outside-conversations.test.tsx`; cargo and Playwright as cited run in CI / the gallery gate (not run here) | pass | 3 files, 17 tests passed | PASS | 92d6e0ce6 | 2026-11-07 | Claude (self) |

## Notes

- UI: a row's click and ↩ open its page; its ↗ and ⌘↩ go to it (the
  maintainer's 0.1.4 review). The vitest
  `outside-conversations.test.tsx` "a click or ↩ opens its page, never its
  app; ↗ and ⌘↩ go to it, never the page (the maintainer's review of
  0.1.4)" checks this, along with "AJ-E2102: Go to it leads an open one's
  menu". The old claim "an open one's click goes to it" no longer holds,
  and that test is gone.
- A session that isn't open now: Go to it starts the profile that keeps
  it, then sends the link (`places::open_session`, fe5124663). Past 1.5 s
  it says the profile is starting.
- PID reuse: `WindowIdentity.start_time` is checked before activating
  (`a_gone_reused_or_refused_app_says_so`, `focus.rs`).
- R1's gap, where a fix goes: in `focus.rs` tier (b), a stale pid should
  read as not running (`not_running(identity)`) or be re-found only by
  something unique to the instance (its pid plus start time, or its data
  dir), never "first application process whose bundle identifier". Then
  replace `stale_pid_is_rediscovered_by_bundle_id` with a test where two
  processes share the bundle id and a stale pid activates neither.

## Binary artifacts

None added. Cited: `_gallery/14-focus/` (it shows no outcome toast).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (fixture paths only)
- [x] Transcript excerpts scrubbed of personal content (none)

# Evidence manifest — AJ-E1509

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1509, Always on: keep working sessions going (P1).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). One live
check needs an installed build: registering the login item (below).

Gallery: `docs/qa/evidence/_gallery/83-keep-going/` (This Mac's line,
Settings › Sessions' Keep work going, a launch setting's own choice; both
themes).

## What exists now

1. **Restart a crashed session** (`features/sessions/restart.rs`).
   - Settings › Sessions › Keep work going: "Restart sessions that crash",
     off by default. A launch setting can choose for itself under its
     Options: As in Settings, Restart it, or Leave it ended
     (`SettingsOverlay.restart_on_crash`, no migration).
   - How a session ended comes from the lifecycle forwarder, in order: a
     clean exit, a stop the user made, a usage-limit signature, a sign-in
     signature (`signin_signature`, beside `quota_signature`), else a crash.
     A crash is also treated as a failed sign-in when the account's
     credentials are expired or half-made.
   - A crash restarts after a 2 s backoff with the same launch settings and
     folder, its conversation resumed (`--resume`) when one is known, else
     started fresh. At most 3 tries in 10 minutes per lineage (a restarted
     session counts with the ones it replaced); then the session stays
     Failed, its row says why, and so does its history.
   - Each try is in both sessions' history. A restart entry is history
     only: `lifecycle_latest` now reads only the reviewed and archived
     events, so a restart can never archive or review a session.
2. **Keep the Mac awake while sessions work** (`juggler-supervisor/src/awake.rs`,
   `features/app/awake.rs`).
   - Every 5 s: the sessions on this Mac that are running or starting (not
     waiting for the user, stuck or at a limit) hold an activity that
     disables idle system sleep, named "AI Juggler: N sessions working",
     released as soon as none work. The display may sleep; a sleep the user
     asks for (the lid, the Apple menu) is never blocked (an idle-sleep
     assertion can't block those).
   - Settings: "Keep the Mac awake while sessions work", on (when plugged
     in); "On battery too", off, and only with the main switch.
   - No `unsafe`: each activity is held by a thread inside
     `performActivityWithOptions:reason:usingBlock:`, the safe Foundation
     form, and ends when the block returns. (`beginActivity` is safe but its
     `endActivity` isn't, and dropping the token doesn't end it: probed.)
3. **No App Nap while sessions run**: while anything is supervised, a
   user-initiated activity that still allows idle sleep.
4. **AI Juggler comes back after a crash** (`features/app/crash.rs`).
   - A run marker in the data folder: `running` from the moment the data is
     opened, `clean` at a quit (the exit path; a closed state in tests).
   - A watchdog, this same binary in a watch mode, waits for the app to end.
     Ended without the clean mark, it starts AI Juggler again, at most 3
     times in 10 minutes. Release builds only, while "Reopen AI Juggler if it
     quits unexpectedly" is on (on by default); in debug builds only with
     `AIJ_WATCHDOG=1` (the crash test). It isn't launchd's `KeepAlive`:
     that covers only a process launchd started, and AI Juggler is usually
     opened from the Dock.
   - At the next start, the sessions recorded live (running, starting,
     waiting or stuck) whose processes didn't survive are stopped after a
     quit. After a crash they're Failed with "ended when AI Juggler quit
     unexpectedly", and resumed where restarting is on. Survivors are
     picked up as before.
5. **Open AI Juggler at login** (D-26, the audited `juggler-login-item`
   crate: `SMAppService`, its four `unsafe` calls behind a safe API).
   Off by default; Settings › Sessions › Keep work going and the welcome's
   Make it smoother card offer it; the system keeps the choice and AI
   Juggler reads it back (at most once a minute). A development build
   reads "unavailable": the switch is disabled and says "Only the installed
   AI Juggler can open at login."
6. **The watchdog never outlives a clean quit or an update install.** A
   quit marks the run clean and ends the watchdog in the exit path. Tauri's
   `restart()` on the main thread relaunches without that exit event, so
   an update's restart and a reopen first go through the same
   `before_exit`. Release builds carry no `AIJ_WATCHDOG` seam
   (`release_seams_check.sh`).
7. **Honest about it**: This Mac (Home's panel and the page) says "Kept
   awake while 2 AI Juggler sessions work", or why not ("On battery, so…",
   "…(off in Settings)"), with Settings › Sessions one click away. It
   shows only while sessions work, and counts only AI Juggler's own
   sessions. The page also says "AI Juggler itself: 214 MB · with its
   crash watchdog · opens at login": the watchdog is part of AI Juggler,
   never a stray process.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 never after a finish, a stop, a usage limit or a failed sign-in; how an exit maps to them | `restart.rs` unit tests: `never_after_a_finish_a_stop_a_limit_or_a_failed_sign_in`, `any_other_failure_is_a_crash` (a stop wins over what the output said) | PASS |
| Req.1 3 tries in 10 minutes, then it stays ended and says why | unit `three_tries_in_ten_minutes_then_it_stays_ended`, `a_restarted_session_counts_with_the_ones_it_replaced`; `tests/restart_crashed.rs` `on_it_restarts_three_times_in_ten_minutes_then_says_why` (a real launch of a stub that always crashes: three restarts on the same launch settings and folder, history on both sessions, the fourth stays Failed with `restart_gave_up`, nothing archived) | PASS |
| Req.1 off by default; a launch setting's own choice wins | `off_by_default_a_crash_stays_ended`, `a_launch_settings_own_choice_overrides_settings`; storage round-trip of the choice; vitest `keep-going` (the Options select saves `true`, `false`, `null`) | PASS |
| Req.2 the assertion taken, named, renamed and released | `juggler-supervisor/tests/awake_pmset.rs`: `pmset -g assertions` shows `PreventUserIdleSystemSleep named: "AI Juggler: 2 sessions working"`, then "3 sessions working" with the old name gone, then nothing once none work | PASS |
| Req.2 on power by default, on battery only when asked (and with the main switch) | `features/app/awake.rs` unit `on_power_by_default_on_battery_only_when_asked`; vitest `keep-going` (defaults; On battery too disabled without the main switch) | PASS |
| Req.3 App Nap held off while supervised, released after; never a sleep blocker | keeper unit `it_holds_what_the_plan_asks_and_lets_go_of_the_rest`; `awake_pmset` (no `PreventUserIdleSystemSleep` while only supervised) | PASS |
| Req.4 a crash found at the next start; sessions shown as they are, resumed where on; a quit stops them | crash unit `a_run_not_ended_as_asked_is_found_at_the_next_start`, `three_relaunches_in_ten_minutes_then_no_more`; `restart_crashed` `after_a_crash_its_sessions_say_so_and_resume_after_a_quit_they_stop` | PASS |
| Req.4 killed with SIGKILL, it comes back by itself | `second_launch` `killed_outright_ai_juggler_comes_back_by_itself`: the real Dev binary with its watchdog; SIGKILL; a new AI Juggler starts once, with its own watchdog, holding the data; everything ended with the group afterwards | PASS |
| Req.4 Open at login, off by default, offered in Settings and the welcome | crate test `an_unbundled_build_reads_unavailable_or_off_and_never_claims_on`; `login_item` unit tests (each status's word; a test build never reads on); vitest `keep-going` (off by default and saving on; disabled with its reason when unavailable; the welcome's row, gone once on); Playwright `keep-going` | PASS |
| Req.4 registering and unregistering on the real system | needs the installed, signed app (a development build has no bundle to register) | NEEDS AN INSTALLED BUILD |
| Req.4 the watchdog ends with a clean quit, and before an update's restart | `second_launch` `the_watchdog_ends_with_a_clean_quit_and_starts_nothing` (the real binary's watch mode: the app gone with the run clean, it ends and starts nothing), `before_a_restart_the_watchdog_ends_and_the_run_is_clean` | PASS |
| The watchdog shows as part of AI Juggler | vitest `keep-going` (This Mac's "AI Juggler itself … with its crash watchdog … opens at login"); Playwright `keep-going` | PASS |
| Req.5 the This Mac row, the setting one click away | vitest `keep-going` (silent with nothing working; kept, on battery, off; the button opens Settings › Sessions); Playwright `keep-going` (from the page to Settings, axe 0 critical/serious, both themes) | PASS |
| Nothing left running after any test | the restart tests stop their replacements; the crash test kills its group and waits until it's empty; `pgrep` clean after each run | PASS |

## Walked

On the preview at 800×600 and 1280×800: Home's This Mac panel shows "Kept
awake while 1 AI Juggler session works · Settings". Home's Running list
also shows another tool's conversation; the line says "AI Juggler
session" so the counts don't seem to disagree. Its Settings link lands on
Settings › Sessions, where Keep work going shows the four switches with
their reasons; turning one on says what changed. Accounts › a launch
setting's Options shows "If a session crashes: As in Settings" above its
environment overrides.

# Evidence manifest — AJ-E1513

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1513, Prepare the pack before quota runs out (P2).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

Gallery: `docs/qa/evidence/_gallery/89-limit-ready/` (the limit dialog with
the prepared switch ready; both themes).

## What exists now

1. **Staging** (`features/staging`), on the existing 30-second background
   quota tick (off the UI thread). A session that is live (starting,
   running, waiting, stuck) or stopped by its limit is staged once its
   account crosses the quota warning: a window at 80 % or more (a session
   at its limit is at 100 %), or a forecast under 45 minutes. Each
   account's quota is read once a tick, and nothing else is looked at for
   sessions under the warning. It stages for
   the limit dialog's own pick (the ranker's suggestion on another
   account, else its best), and writes:
   - the switch check for that target (the preflight; a pure read since
     #153). A check that would stop the switch leaves nothing ready;
   - a redacted pack of the conversation (the global secrets policy, as
     parking uses), under `data_dir/staged/<session>/pack/`, written
     beside and then swapped in, so a reader never sees half of one;
   - `staged.json`: the target, when it was prepared, what the switch
     carries in words, the transcript's size and time, and the crossed
     window's reset. No conversation text.
2. **Refresh**: at most every 5 minutes, and only when the transcript
   changed (or the pick changed).
3. **Cleanup**: the staged switch goes when the session ends (finished,
   stopped, failed, handed over; stopped by its limit keeps it), when the
   crossed window resets, when the pick goes away, and at startup for
   anything a crash left behind.
4. **Privacy**: the `staged` folder and each session's folder are 0700,
   every file 0600 (as the handover's), all under the app's data folder.
5. **The limit dialog**: "Ready to continue on Work Max · prepared 4 min
   ago" under the pick, only when the prepared switch is for that
   account. **Continue** still opens the switch screen; nothing switches
   by itself.
6. **Quota alerts re-arm** (found while building this): the alert machine
   stayed Warned forever after its first warning, so the next window
   never warned. It now records the reset of the window it fired in and
   re-arms once that passes, or once a later window is reported (30
   minutes of slack, as readings of one window jitter by seconds). With no
   reset instant it never re-arms on a guess. Machines saved before this
   load unchanged and learn the reset from the next reading.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 crossing the warning stages the pack in the background, refreshed as the session goes on | `juggler-app` `switch_staging` `crossing_the_warning_stages_the_switch_privately_and_starts_nothing` (85 % measured: staged for "Work", fidelity words), `under_the_warning_nothing_is_staged` (50 %), `a_refresh_waits_five_minutes_and_a_changed_transcript` (unchanged at 6 min: nothing; changed at 4 min: nothing; changed at 7 min: restaged) | PASS |
| Req.1 nothing switches by itself | `crossing_the_warning…`: the sessions are unchanged and the tool stand-in ran only `--version` (the check's validation verdict), never a session; `trust` `nothing_switches_on_its_own` passes unchanged (staging is not an allowed caller and calls no executor) | PASS |
| Req.2 thrown away when the session ends or the window resets | `the_staged_switch_goes_when_the_session_ends` (kept while stopped by its limit; gone, folder empty, once finished), `the_staged_switch_goes_when_its_window_resets` (not ready at the reset even before a tick; gone after it, and not staged again for a session still stopped by its limit), `startup_drops_what_a_crash_left_behind` | PASS |
| Private files | `crossing_the_warning…`: `staged` and every folder under it 0700, every file 0600, all under `data_dir` | PASS |
| Cost bound | `a_transcript_over_the_cap_is_left_to_the_switch` (over 16 MiB, a pack's own per-file cap: nothing staged); the transcript is read and parsed once per refresh for both the check and the pack (`preflight_with` / `write_session_pack` take the shared parse) | PASS |
| Performance: off the UI thread, within E2 budgets | `staging_a_large_session_stays_cheap`: 4,000 turns (3.8 MB transcript) staged in **≈250 ms** (debug build, three runs 249–251 ms, at load average ≈16) | PASS |
| Visual: the ready state in the limit dialog | vitest `limit-dialog` (the line for the pick; nothing for a prepared switch to another account); Playwright `limit-ready` (two captures, axe 0 critical/serious) | PASS |
| Alerts re-arm (the at-most-once law per window) | `juggler-core` `alert_rearm`: `warned_then_critical_in_one_window_never_warns_twice`, `the_next_window_warns_again_from_armed`, `a_later_window_reported_before_the_old_reset_re_arms_but_jitter_does_not`, `a_window_with_no_reset_instant_never_re_arms_on_a_guess`, `a_machine_saved_before_the_reset_instant_existed_still_loads`; `juggler-app` `ranking_alerts_guardrails` `the_next_window_alerts_again_once` | PASS |

## Privacy page

No new claim. "What it keeps lives in its own folder" stays true: the
staged switch is under the app's data folder, beside the handover and
parked packs.

## Notes

- A transcript over 16 MiB is not staged; the switch itself still reads
  it when the user continues. A compressed rollout is measured by its
  size on disk.
- Staging warms the installed tool versions the dialog and the switch
  screen read, so neither waits on that probe.
- Not walked in the real app: reaching 80 % on a real account is the
  maintainer's to do; the dialog was checked in the preview captures.

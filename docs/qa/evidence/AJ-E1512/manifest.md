# Evidence manifest — AJ-E1512

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1512, Sessions left and a burn-down chart (P1).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

Gallery: `docs/qa/evidence/_gallery/87-burn-down/` (Usage with sessions
left and the burn-downs; both themes, both densities).

## What exists now

1. **The runway, worked out** (`juggler_core::quota_runway::runway`), per
   window (5-hour and weekly), from:
   - the vendor's own readings of the current window (Measured
     `quota_samples` sharing its reset);
   - the tokens AI Juggler saw between the window's first and latest
     reading: percent per token (only once the window has moved 1 %);
   - the account's typical session: the median tokens of its past
     sessions (at least three) over the last week.
   A typical session's cost in percent is the median × percent per
   token; sessions left are the percent left over that, rounded down.
   The pace is the newest reading against the oldest in the last hour
   (at least 10 minutes apart); at that pace it runs out at a time shown
   only when it's before the reset. The confidence is the lowest of what
   went in, never above Computed: these figures are worked out, not read.
2. **On the wire**: each window's display carries `runway` (the readings,
   the window's start and reset, used, pace, runs-out time, a session's
   cost, sessions left, how many sessions it's the median of, and the
   confidence word). No reading of a window that hasn't reset: no runway.
3. **Usage**: under each account's runway bars, per window, "About 6
   usual sessions left (each about 10%)" with its confidence word
   ("worked out"), the burn-down (used so far, the projection dashed — in
   the warning colour and ending at a dashed "runs out" line when that's
   before the reset — and the reset as the right edge), and "At this pace
   it runs out about Fri 05:52, before the reset at Fri 06:22" or "…lasts
   past the reset at …". Too little to go on says so ("Sessions left show
   after a few sessions on this account" / "…once this window has moved a
   little"), with no confidence word next to a missing number. The chart
   is one small SVG (no chart library), described in words for screen
   readers.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 sessions left from the user's own average session cost, with the confidence label | `juggler-core` `quota_runway` `sessions_left_come_from_the_accounts_own_typical_session` (four past sessions of 100k tokens and a window that moved 40 % over 400k tokens: 10 % a session, 6 left), `the_confidence_is_the_lowest_of_what_went_in`, `too_little_to_go_on_stays_unknown`; `juggler-app` `quota_display` `a_measured_window_carries_its_runway` (stored readings and events to the wire); vitest `burn-down` (the words, and the confidence word beside the number) | PASS |
| Req.2 a burn-down per account: used over the window, the forecast to exhaustion, the reset | `the_burn_down_runs_out_before_the_reset_or_lasts_past_it`, `only_this_windows_readings_are_drawn_and_long_ones_are_thinned`, `no_measured_reading_of_a_live_window_is_no_runway`; vitest (the projection to the exhaustion, past it to the reset, or none without a pace) | PASS |
| Performance: chart render < 50 ms | vitest `renders a full window in under 50 ms` (two windows of 120 readings) | PASS |
| Usage in both themes and densities, axe | Playwright `burn-down` (four captures, axe 0 critical/serious) | PASS |

## Walked

On the preview at 800×600 with a seeded account: Usage › Work Max shows
"5-hour window About 6 usual sessions left (each about 10%) worked out",
the burn-down with its dashed projection up to the runs-out line before
the reset, and "At this pace it runs out about …, before the reset at …";
This week shows its readings and "Resets Mon 03:22" with no number where
there isn't one yet.

## Notes

- Percent per token assumes the account's sessions mix tokens alike
  (cache reads and writes counted as the launch advisor's median counts
  them); the confidence stays "worked out" for that reason.

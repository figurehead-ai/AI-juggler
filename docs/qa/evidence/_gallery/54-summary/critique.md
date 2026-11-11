<!-- captures: 50612bbeaad3 -->
# 54-summary: Home's "Since you were away" summary of what crashed, needs you, ran, finished and ended, and a session opened from it

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The "Since you were away" card stands off the darker dark canvas and the grey light canvas, and its groups (✕ Crashed, ! Needs you, ▶ Still running, ✓ Finished, ■ Ended) are bold and easy to scan. Mark all seen is a small neutral button, and New session in the bar is the only strong action; in dark it now uses the lighter violet, which reads better against the near-black bar. The rail status line now shows its age whole or not at all, so the ai-juggler row reads "Stopped with an error" where the first pass had "Stopped with an error · 7…". The rows are still noisy. "tokens: Unavailable · cost: Unavailable" repeats on almost every row, and in Needs you it drops to a second line on the first two rows but sits right-aligned on the third. The Crashed row's "· 2 leftover processes" still starts a new line with a stray leading dot. At 1440 wide the card fills only about half the main column, so a wide empty band sits to its right. The opened captures show the session page with Where it runs, Runway and Continue on another account cards, all clearly separated; there too only the lighter dark accent and the whole rail status line changed, and after the second pass the terminal's black reaches the card's edge in both themes, with the white (light) or lighter (dark) frame gone.

## Weak

- home-1440x900-dark/light: inside the Needs you group, "tokens: Unavailable · cost: Unavailable" moves between a second line and the right edge from row to row, so the rows don't line up.
- home (all four): the Crashed row wraps "· 2 leftover processes" onto its own line, starting with a dot.
- home-1440x900-dark/light: the summary card spans only about half the main column, leaving a wide empty area to its right.
- home (all four): the same "Unavailable" token and cost text on nearly every row is noise; it could say it once.

## 0.4.1 baseline pass (2026-11-10)

home-1440x900-dark rendered in the pinned container from HEAD and looked at by eye. The fixture's rows now carry every field the backend sends. `at_ms` is when a row entered its group; before, it was missing, and the rows read "waiting NaN d" and "ended Invalid Date". The product now leaves the time out of a row that has none. What changed, on purpose:
- **Real times**: Needs you reads "waiting 2 h", "waiting 1 h" and "waiting 25 min", the longest first as the backend sorts it. The ended rows say when they ended ("ended Sun 02:33 AM", "ended Sun 12:58 AM", "ended Sat 11:23 PM"), and Still running says "last change Sat 10:13 PM".
- The heading reads "Since you last looked, Sat 09:13 PM" (D-97 5). Unknown tokens are silent: only Still running shows "19,134 tokens". The Crashed row's "2 leftover processes" sits on its line.

Weak: the summary card still spans only about half of the 1440 column. The times mix a weekday clock ("Sun 02:33 AM") with durations ("waiting 2 h"), which is correct but takes a second look.

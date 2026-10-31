<!-- captures: 3f48815caef7 -->
# 54-summary: Home's "Since you were away" summary of what crashed, needs you, ran, finished and ended, and a session opened from it

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The "Since you were away" card stands off the darker dark canvas and the grey light canvas, and its groups (✕ Crashed, ! Needs you, ▶ Still running, ✓ Finished, ■ Ended) are bold and easy to scan. Mark all seen is a small neutral button, and New session in the bar is the only strong action; in dark it now uses the lighter violet, which reads better against the near-black bar. The rail status line now shows its age whole or not at all, so the ai-juggler row reads "Stopped with an error" where the first pass had "Stopped with an error · 7…". The rows are still noisy. "tokens: Unavailable · cost: Unavailable" repeats on almost every row, and in Needs you it drops to a second line on the first two rows but sits right-aligned on the third. The Crashed row's "· 2 leftover processes" still starts a new line with a stray leading dot. At 1440 wide the card fills only about half the main column, so a wide empty band sits to its right. The opened captures show the session page with Where it runs, Runway and Continue on another account cards, all clearly separated; there too only the lighter dark accent and the whole rail status line changed, and after the second pass the terminal's black reaches the card's edge in both themes, with the white (light) or lighter (dark) frame gone.

## Weak

- home-1440x900-dark/light: inside the Needs you group, "tokens: Unavailable · cost: Unavailable" moves between a second line and the right edge from row to row, so the rows don't line up.
- home (all four): the Crashed row wraps "· 2 leftover processes" onto its own line, starting with a dot.
- home-1440x900-dark/light: the summary card spans only about half the main column, leaving a wide empty area to its right.
- home (all four): the same "Unavailable" token and cost text on nearly every row is noise; it could say it once.

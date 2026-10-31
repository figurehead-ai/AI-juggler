<!-- captures: 76681b09eab6 -->
# 33-beta: the "Beta · Not proven yet" mark on a switch, and the Accounts page above "Which switches between accounts are proven"

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The Accounts captures show the 0.3.2 surfaces well: the Personal and Fake Desk cards with their profile rows stand a step off the canvas in dark, and in light the white cards sit on the grey canvas with soft shadows; the sparklines keep their account colours. The switch dialog in badge-in-situ is clear: "Step 1 of 3: Target", the title "Switch to Personal", the "Same conversation, new account · Beta · Not proven yet" chip, then two amber boxes for "This move hasn't been proven on a real setup yet" and "Doesn't carry: Subagents' work". Its Continue is now a solid violet fill in both themes, so the dialog's one forward action is the strongest thing in it, clearly ahead of the outlined Cancel. At 800×600 the top bar now stays on one row: the cleanup pill is a broom with "7" and New session sits beside it, where the first pass had them pushed onto a second row. The rail's status lines read whole at 1280 ("Stopped with an error", no cut-off age). What still misses the point: the three matrix captures (matrix-default, matrix-failed, matrix-validated) are still the same picture and stop at the "Which switches between accounts are proven" toggle, so the table itself, which is the point of the family, is below the fold in every one. All captures were re-rendered.

## Weak

- matrix-default-*, matrix-failed-*, matrix-validated-*: the same picture three times; the proven/failed/validated matrix is below the fold, so none of them shows its state.

<!-- captures: b529742dd0e0 -->
# 33-beta: the "Beta · Not proven yet" mark on a switch, and the Accounts page above "Which switches between accounts are proven"

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The Accounts captures show the 0.3.2 surfaces well: the Personal and Fake Desk cards with their profile rows stand a step off the canvas in dark, and in light the white cards sit on the grey canvas with soft shadows; the sparklines keep their account colours. The switch dialog in badge-in-situ is clear: "Step 1 of 3: Target", the title "Switch to Personal", the "Same conversation, new account · Beta · Not proven yet" chip, then two amber boxes for "This move hasn't been proven on a real setup yet" and "Doesn't carry: Subagents' work". Its Continue button, though, is still the pale outlined violet while every other primary in this release is a solid fill, so the dialog's one forward action is the weakest thing in it. The three matrix captures (matrix-default, matrix-failed, matrix-validated) are byte-identical and stop at the "Which switches between accounts are proven" toggle: the table itself, which is the point of the family, is below the fold in every one. Every capture with the rail also shows "Stopped with an error · 7…" with the age cut off. All captures were re-rendered.

## Weak

- matrix-default-*, matrix-failed-*, matrix-validated-*: the same picture three times; the proven/failed/validated matrix is below the fold, so none of them shows its state.
- badge-in-situ-*: Continue is a pale outlined button, not the solid violet primary used everywhere else in 0.3.2.
- matrix-*-1280x800-*: the rail row "Stopped with an error · 7…" cuts off the age.

<!-- captures: 378d71a1802a -->
# 33-beta: the "Beta · Not proven yet" mark on a switch, and the Accounts page above "Which switches between accounts are proven"

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The Accounts captures show the 0.3.2 surfaces well: the Personal and Fake Desk cards with their profile rows stand a step off the canvas in dark, and in light the white cards sit on the grey canvas with soft shadows; the sparklines keep their account colours. The switch dialog in badge-in-situ is clear: "Step 1 of 3: Target", the title "Switch to Personal", the "Same conversation, new account · Beta · Not proven yet" chip, then two amber boxes for "This move hasn't been proven on a real setup yet" and "Doesn't carry: Subagents' work". Its Continue is now a solid violet fill in both themes, so the dialog's one forward action is the strongest thing in it, clearly ahead of the outlined Cancel. At 800×600 the top bar now stays on one row: the cleanup pill is a broom with "7" and New session sits beside it, where the first pass had them pushed onto a second row. The rail's status lines read whole at 1280 ("Stopped with an error", no cut-off age). What still misses the point: the three matrix captures (matrix-default, matrix-failed, matrix-validated) are still the same picture and stop at the "Which switches between accounts are proven" toggle, so the table itself, which is the point of the family, is below the fold in every one. All captures were re-rendered; the second pass changed only badge-in-situ, and only behind the dialog, where the blurred terminal now runs to its card's edges.

## Weak

- matrix-default-*, matrix-failed-*, matrix-validated-*: the same picture three times; the proven/failed/validated matrix is below the fold, so none of them shows its state.

## AJ-E808 retake (2026-11-07)

Retaken for AJ-E808's visual check: the badge for a proven and an unproven pair, both themes. The matrix captures (`matrix-*`) now scroll the matrix into view: before, its disclosure sat at the Accounts screen's foot and the captures showed the Accounts cards, not the matrix. Now `matrix-validated-*` shows "Proven ✓" and "Proven: tried by hand…" on the proven row, and `matrix-default-*` shows "Beta — not proven yet" on every row: the words the register's "Settings › Beta shows" column gives (rendered by the same code, `docs/qa/evidence/AJ-E808/register.md`). `badge-in-situ-*` shows "Same conversation, another app · Beta · Not proven yet", the register's "Switch rows show" for Claude Code → Claude Code.

Fixture notes, not product: the switch wizard's shared fixture (`SWITCH_WIZARD_FIXTURE`, capture.spec.ts) says a Claude → Claude move carries "its subagents" and then "Doesn't carry: Subagents' work"; the product's Claude → Claude dispositions copy subagents, so the real sheet never says both. And the validated row's fixture names Claude → Claude "Full story, fresh start", where the product says "Same conversation, another app". The fixture is shared by the switch wizard's families, whose assertions use its drop, so it's left as it is and noted here.

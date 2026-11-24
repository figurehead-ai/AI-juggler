<!-- captures: 8d94fa28dea8 -->
# 33-beta: the "Beta · Not proven yet" mark on a switch, and the Accounts page above "Which switches between accounts are proven"

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The Accounts captures show the 0.3.2 surfaces well: the Personal and Fake Desk cards with their profile rows stand a step off the canvas in dark, and in light the white cards sit on the grey canvas with soft shadows; the sparklines keep their account colours. The switch dialog in badge-in-situ is clear: "Step 1 of 3: Target", the title "Switch to Personal", the "Same conversation, new account · Beta · Not proven yet" chip, then two amber boxes for "This move hasn't been proven on a real setup yet" and "Doesn't carry: Subagents' work". Its Continue is now a solid violet fill in both themes, so the dialog's one forward action is the strongest thing in it, clearly ahead of the outlined Cancel. At 800×600 the top bar now stays on one row: the cleanup pill is a broom with "7" and New session sits beside it, where the first pass had them pushed onto a second row. The rail's status lines read whole at 1280 ("Stopped with an error", no cut-off age). What still misses the point: the three matrix captures (matrix-default, matrix-failed, matrix-validated) are still the same picture and stop at the "Which switches between accounts are proven" toggle, so the table itself, which is the point of the family, is below the fold in every one. All captures were re-rendered; the second pass changed only badge-in-situ, and only behind the dialog, where the blurred terminal now runs to its card's edges.

## Weak

- matrix-default-*, matrix-failed-*, matrix-validated-*: the same picture three times; the proven/failed/validated matrix is below the fold, so none of them shows its state.

## AJ-E808 retake (2026-11-07)

Retaken for AJ-E808's visual check: the badge for a proven and an unproven pair, both themes. The matrix captures (`matrix-*`) now scroll the matrix into view: before, its disclosure sat at the Accounts screen's foot and the captures showed the Accounts cards, not the matrix. Now `matrix-validated-*` shows "Proven ✓" and "Proven: tried by hand…" on the proven row, and `matrix-default-*` shows "Beta — not proven yet" on every row: the words the register's "Settings › Beta shows" column gives (rendered by the same code, `docs/qa/evidence/AJ-E808/register.md`). `badge-in-situ-*` shows "Same conversation, another app · Beta · Not proven yet", the register's "Switch rows show" for Claude Code → Claude Code.

Fixture notes, not product: the switch wizard's shared fixture (`SWITCH_WIZARD_FIXTURE`, capture.spec.ts) says a Claude → Claude move carries "its subagents" and then "Doesn't carry: Subagents' work"; the product's Claude → Claude dispositions copy subagents, so the real sheet never says both. And the validated row's fixture names Claude → Claude "Full story, fresh start", where the product says "Same conversation, another app". The fixture is shared by the switch wizard's families, whose assertions use its drop, so it's left as it is and noted here.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53 and this batch's fix, both sizes and themes. The matrix captures open Accounts on "Which switches between apps are tested": Tested switches as a table (Switch, What carries, Automatic tests, Tried by hand, Status), the proven row "Tested ✓ · Proven: tried by hand 2026-10-17 on a tiny project"; badge-in-situ is the wizard's Target step with "Same conversation, another app · Beta" on the target card.

**Fixed (c3fd24f78):** "What carries" was amber on every unproven row (and green on the proven one): amber where nothing asks the user to act (PAT-COLOR) and the same Beta said twice. It is ink now; the Status column says Beta. Gate baseline `33-beta/matrix-default-1280x800-dark` re-rendered.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Which switches are tested, and how. |
| 2 | Prominence | 4 | Behind a disclosure at the page's foot (D-25). |
| 3 | Default | 4 | Closed by default; the proven row says so. |
| 4 | Mode | 4 | Calm table on its card. |
| 5 | Mock-up | 4 | No mock; the register's words. |
| 6 | Areas | 4 | One card; rows split by faint rules (a table, where tone can't separate). |
| 7 | Colour | 4 | After the fix: a hand check's Passed green and Failed red only. |
| 8 | Type | 4 | 12 px table; long Status notes wrap to three lines. |
| 9 | Actions | n/a | No actions. |
| 10 | Serious | 4 | A failed hand check lowers the promise, said under the table. |
| 11 | Doors | 4 | Settings › Switching named in words, not a link. |
| 12 | Words | 4 | Plain; "stand-ins" for the test tools. |
| 13 | States | 4 | Default, failed, validated. |
| 14 | Consistency | 4 | |
| 15 | Accessibility | 4 | Row headers scoped (a11y.spec). |
| 16 | Hand walk | 4 | Preview. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | |

Lowest: 4 after the fix.

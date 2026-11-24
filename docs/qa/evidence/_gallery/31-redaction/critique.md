<!-- captures: bb738d0364f3 -->
# 31-redaction: the switch wizard's What's carried step when the target is another tool: keys and passwords found (hide or leave in), none found, and the same-tool case

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The step reads in a sensible order (Goes along chips, Recreated on Personal (Codex), the amber Stays behind box, then the keys section), and the dark dialog and its bordered keys group stand clearly off the dimmed page; in light the white panel is crisp. In another-tool-found the keys group explains plainly what was found, masks the values ("s•••5678"), and marks "Hide them" as recommended, while the footer's "Choose what happens to the keys first." with a greyed Continue makes the blocking choice clear. At 800x600 only the first option shows and its hint is cut by the footer, so "Leave them in" and "Remember my choice" are out of sight; a checkbox also sits inside the radio group, mixing two kinds of choice in one box. another-tool-none and same-tool are short and clear, with a one-line note in place of the keys group, and their Continue is now a solid violet primary that leads Back and Cancel. In dark the current step chip and the "What does this mean?" links carry the lighter violet, and at 800x600 the page behind keeps its top bar on one row. The second pass changed only the blurred page behind the dialog (the session's terminal is now black to its card's edge); the dialogs themselves are unchanged.

## Weak

- another-tool-found-800x600-dark.png, another-tool-found-800x600-light.png: only "Hide them" is visible and its hint is cut by the footer; the other choices need a scroll with no cue.
- another-tool-found-1280x800-*.png: the "Remember my choice" checkbox sits inside the hide/leave radio group.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53 (both sizes and themes). What's carried is the 0.5.0 step: a tinted headline ("3 go along · 1 recreated · 1 stays behind") with its (i), then Goes along, Recreated on Personal and Stays behind as grouped rows in one card, each with its kind's check chip, then the keys question. Only the rail behind changed (F53).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | Says what goes, what doesn't, and what happens to keys before anything is sent. |
| 2 | Prominence | 4 | At 1280×800 another-tool-found opens on the carried list; the keys question, the one decision, is below the fold, said by the footer line "Choose what happens to the keys first." |
| 3 | Default | 5 | Hide them is recommended; same-tool asks nothing. |
| 4 | Mode | 5 | Rich. |
| 5 | Mock-up | 4 | The kit's carried list; the keys question as two choice cards. |
| 6 | Areas | 4 | The rows' card keeps a quiet edge in light (white on the white dialog, where tone can't separate). |
| 7 | Colour | 5 | Neutral (i) headline; amber only in the keys Alert. |
| 8 | Type | 5 | Row names ink, lines ink-2. |
| 9 | Actions | 5 | Next: Confirm waits with its reason before the buttons (BTN-5). |
| 10 | Serious | 5 | Values masked, what leaving them in means said. |
| 11 | Doors | 4 | None needed beyond the step's own. |
| 12 | Words | 5 | Plain. |
| 13 | States | 4 | Found, none, same tool. |
| 14 | Consistency | 5 | As 28-switch. |
| 15 | Accessibility | 4 | N1 holds; not re-walked by hand. |
| 16 | Hand walk | 4 | Preview walk. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | A newcomer sees what is hidden and why. |

Lowest: 4. No fix; nothing logged.

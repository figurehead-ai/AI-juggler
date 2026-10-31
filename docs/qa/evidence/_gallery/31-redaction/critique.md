<!-- captures: 5a8f3523d266 -->
# 31-redaction: the switch wizard's What's carried step when the target is another tool: keys and passwords found (hide or leave in), none found, and the same-tool case

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The step reads in a sensible order (Goes along chips, Recreated on Personal (Codex), the amber Stays behind box, then the keys section), and the dark dialog and its bordered keys group stand clearly off the dimmed page; in light the white panel is crisp. In another-tool-found the keys group explains plainly what was found, masks the values ("s•••5678"), and marks "Hide them" as recommended, while the footer's "Choose what happens to the keys first." with a greyed Continue makes the blocking choice clear. At 800x600 only the first option shows and its hint is cut by the footer, so "Leave them in" and "Remember my choice" are out of sight; a checkbox also sits inside the radio group, mixing two kinds of choice in one box. another-tool-none and same-tool are short and clear, with a one-line note in place of the keys group, and their Continue is now a solid violet primary that leads Back and Cancel. In dark the current step chip and the "What does this mean?" links carry the lighter violet, and at 800x600 the page behind keeps its top bar on one row.

## Weak

- another-tool-found-800x600-dark.png, another-tool-found-800x600-light.png: only "Hide them" is visible and its hint is cut by the footer; the other choices need a scroll with no cue.
- another-tool-found-1280x800-*.png: the "Remember my choice" checkbox sits inside the hide/leave radio group.

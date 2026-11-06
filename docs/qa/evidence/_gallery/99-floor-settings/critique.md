<!-- captures: 4c7271b31728 -->
<!-- captures: pending -->
# 99-floor-settings: Settings › The Floor, and Re-create

Design critique (D-57) against the approved wireframes (D-78) and the other Settings panes (99-settings-keyboard, Desktop apps). New family (AJ-E3511).

## Checklist

| Check | Finding |
|---|---|
| Grid | The Settings pane layout: a heading, a line on what the Floor is, then two cards (Opening, The office), each choice a row with its words on the left and its control on the right. |
| Hierarchy | Row names in ink, their explanation muted under them; section headings above each card. |
| Spacing | The cards' row rhythm; "More · Apps hidden from the Floor (1)" sits under the office card as a disclosure. |
| Icons | The Floor icon in the Settings list, beside the other panes'. |
| Shared controls | Switch, the −/+ stepper, Segmented (Follow the system, Full, Calm, Off; Look: Pixel art, Full resolution), secondary buttons, Sheet. |
| Disabled reasons | The stepper's − and + are disabled at 4 and 16 with the range in their labels. |
| Plain words | "Calm: half as often, no one walks. Off: nothing moves; it draws only when something changes." Look (D-80): "Pixel art: the Floor's own look, drawn in big pixels. Full resolution: smooth edges at your screen's own pixels; on a large window it takes about 2.7% of a core while animating, more than the Floor's 2% budget." The sheet says what changes (floor, walls, desks, nook, layout) and what stays (Apps, sessions, history, accounts, the main window), with an example. |
| Type sizes | 14 px rows, 12 px explanations. |
| Both themes | Checked. |
| 1280×800 and 1024×700 | Both; the rail folds at 1024 and the pane keeps its width. |

## Good

- Each choice says what it costs or saves (Full is "about 2% of one core").
- Re-create asks first, names both lists, and links to how the Floor works.
- D-80's Look row sits under Motion, the same Segmented control, Pixel art chosen; its line says the cost with the measured number, so the budget-breaking case is said where it's chosen.

## Weak

- Re-create's example names one App; a reader with many Apps can't preview their own layout before choosing.
- The Look line runs to three lines at 1280 and 1024, the longest in the card; it's the price of saying the number.

## Fixed

- Nothing on first render.
- 2026-11-05 (D-80): the Look row added; light and dark, 1280×800 and 1024×700 checked by eye: one row, nothing clipped, the control aligned with Motion's.

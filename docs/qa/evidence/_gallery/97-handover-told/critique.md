<!-- captures: 3ed37fab0ea2 -->
# 97-handover-told: what the other tool was told

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | The line sits on the done step's text column, under its paragraph. |
| Hierarchy | Title ("Switched to Personal"), what happened, then the one muted line and its link. |
| Spacing | One line, one link; no card of its own. |
| Icons | None needed: a line of facts and a text link, as the other done steps. |
| Shared controls | Not applicable (no choices). |
| Disabled reasons | Not applicable. |
| Plain words | "Hidden from Codex: 2 API keys · Left out: 12 earlier turns, 1 note". |
| Type sizes | The line and its link at the step's body size; nothing under 12 px. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Captured as elements (the done block and the graph), so the window size doesn't change them. |

## Good

- One line says what was hidden and what was left out, with Open the handover beside it.

## Weak

- The gate captures only the done block, not the whole sheet.
- graph-edge: the element crop catches the screen's footer, cut to "ed 1s ago · from AI Juggler's projects…" at the left edge; the "What Codex was told" card runs into it with no bottom edge or padding, and no Open the handover link shows below the line.
- graph-edge: "Claude · Terminal" on the Work Max node is in the monospace face, "Codex · Terminal" on the Personal node in the body face.

## Fixed

- Nothing open from the review.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| done-line | The switch's last step: "Switched to Personal", "Hidden from Codex: 2 API keys · Left out: 12 earlier turns, 1 note", Open the handover. | New. |
| done-open | The same step with the handover opened below it. | New. |
| graph-edge | The Switch Graph with an edge opened: the same line (its link falls below the frame). | New. |

## 0.3.2

done-line and done-open read in a clean order: the "Switched to Personal" title, the sentence in the brighter body level, then "Hidden from Codex: 2 API keys · Left out: 12 earlier turns, 1 note" a step quieter, and the violet Open / Hide the handover link last; in done-open the handover's From / To / Carries / Doesn't carry / Withheld table sits on a raised card with dim labels and brighter values. Only graph-edge changed with the design fixes: in dark the selected edge pill ("→ Full story, fresh start") has the lighter violet outline, which marks it clearly between the Work Max and Personal nodes. graph-edge is still the weak capture: the element crop catches the screen's footer, so its first word is cut to "ed 1s ago · from AI Juggler's projects…" at the left edge, the "What Codex was told" card runs into that footer without its bottom edge or padding, and the Open the handover link the other captures show is not in the frame. On the two node cards "Claude · Terminal" is set in the monospace face while "Codex · Terminal" beside it is in the body face, so the twin labels don't match.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

done-line, done-open and graph-edge opened, both themes. done-line and done-open are as at 0.3.2 (the done step's line "Hidden from Codex: 2 API keys · Left out: 12 earlier turns, 1 note" and Open the handover).

**Fixed (c3fd24f78), graph-edge:** the opened edge ("→ Full story, fresh start") drew an accent ring on any click (`focus:ring`, PAT-SURF-3: a selection is a fill, never an outline). An opened edge is now `SELECTED_FILL`; the ring shows only from the keyboard (`:focus-visible`). No gate state shows it.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | What the other tool was told, one line and its detail. |
| 2 | Prominence | 4 | |
| 3 | Default | 4 | Folded. |
| 4 | Mode | 4 | Calm line. |
| 5 | Mock-up | 4 | No mock. |
| 6 | Areas | 4 | graph-edge: the Switch Graph's nodes and the detail card keep outlines (32-graph's pattern, not this family's). |
| 7 | Colour | 4 | After the fix. |
| 8 | Type | 4 | |
| 9 | Actions | n/a | A link only. |
| 10 | Serious | 4 | What was withheld is said. |
| 11 | Doors | 4 | Open the handover. |
| 12 | Words | 4 | |
| 13 | States | 4 | |
| 14 | Consistency | 4 | |
| 15 | Accessibility | 4 | aria-expanded on the edge. |
| 16 | Hand walk | 4 | Preview. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | |

Lowest: 4. graph-edge's element crop still catches the footer cut at "ed 1s ago" (a capture's frame).

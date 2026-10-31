<!-- captures: 9c7855a6b8f5 -->
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

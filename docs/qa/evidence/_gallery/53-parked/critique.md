<!-- captures: 8227db057618 -->
# 53-parked: a session parked until its account's quota resets, and the "has reset" banner that offers to resume it

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The "Personal has reset" banner is slimmer, with a solid violet Resume and a quiet "Remind me in 30 min" beside it. In rail-and-offer that hierarchy works: on Home the Needs you, Parked until a reset and Runway by account cards stand off the canvas, and "Open its page" is the one strong button in the Needs you card. The inspector captures are broken. The terminal card, the largest thing on the page, renders as an empty box in both themes (dark grey in dark, plain white in light), with no Ended toolbar and no scrollback. The Where it runs card beside it is fine, with the parked state in its own outlined box. The rail in inspector is scrolled so that its search field is cut under the top. Its selected row cuts to "Parked 2h 13m · 0% left ·…", and the Runway card's second line is sliced at the column's bottom. All four captures were re-rendered.

## Weak

- inspector-1280x800-dark/light: the terminal card is empty: no toolbar, no text, no hint bar. It is the biggest element on the page and says nothing.
- inspector-1280x800-dark/light: the rail's search field is cut off under the top of the rail.
- inspector-1280x800-dark/light: the selected rail row is cut to "Parked 2h 13m · 0% left ·…", and the Runway card's second line is sliced at the column's bottom edge.

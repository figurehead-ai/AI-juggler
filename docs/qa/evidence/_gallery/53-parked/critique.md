<!-- captures: d96cf7ade7f5 -->
# 53-parked: a session parked until its account's quota resets, and the "has reset" banner that offers to resume it

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The "Personal has reset" banner is slim, with a solid violet Resume (the lighter accent in dark) and a quiet "Remind me in 30 min" beside it. In rail-and-offer that hierarchy works: on Home the Needs you, Parked until a reset and Runway by account cards stand off the canvas, and "Open its page" is the one strong button in the Needs you card. The inspector captures are fixed where it mattered most: the capture now waits for the terminal, so the largest card on the page shows its Ended toolbar (Find, Copy, A−, A+, Maximize, Own window), the "Personal · session" scrollback and the "^Tab leaves the terminal · ⌘F finds" hint, instead of an empty box. The Where it runs card beside it is fine, with the parked state in its own outlined box. The selected rail row now reads "Parked 2h 13m · 0% left" whole. The rail in inspector is still scrolled so that its search field is cut under the top, and the Runway card's second line is still sliced at the column's bottom. After the second pass the terminal's black runs to the card's edge in both themes (the lighter frame in dark and the white one in light are gone), so the scrollback has a black margin instead of starting at the edge of a framed box. All four captures were re-rendered.

## Weak

- inspector-1280x800-dark/light: the rail's search field is cut off under the top of the rail.
- inspector-1280x800-dark/light: the Runway card's second line is sliced at the column's bottom edge.
- inspector-1280x800-dark/light: the selected rail row shows a small "···" under its status icon with no visible meaning.

## AJ-E710 retake (2026-11-07)

Retaken on release/0.4.1 for the Fleet gate's screenshot set (AJ-E710), both themes, every size the family has. First pass by a Sonnet agent against a written sheet; the points marked (checked) were confirmed by eye by Opus.

Parked until a reset: "Parked 2h 13m · 0% left" in words, the reset banner with Resume.

## 0.4.1 CI #218 (2026-11-09)

inspector-1280x800-dark re-taken from CI #218's render (the gate's capture; the light one is not in the gate and stays as it was). Looked at by eye. What changed, on purpose:
- **Stop in the head**: a session at its limit is still a live process, so the head has Stop beside "Continue in another app…", and the "ran 5h · scrollback replayed" line under the terminal is gone; the terminal card takes its room (2a7061a75).
- **The rail's head stays put**: the search field and the App/Project/Account lenses are whole at the top; the list scrolls under them (8cc1b38bb). The earlier weak point "the rail's search field is cut off under the top of the rail" is fixed in dark.

Weak: the list's first visible line is cut at the lenses' bottom edge with no fade (a scrolled list's normal edge, but abrupt). The Runway card's second line is still sliced at the column's bottom.

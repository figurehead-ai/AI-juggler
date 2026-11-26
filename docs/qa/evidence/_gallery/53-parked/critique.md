<!-- captures: ac74b8c1b173 -->
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

## 0.4.1 baseline pass (2026-11-10)

inspector-1280x800-dark rendered in the pinned container from HEAD and looked at by eye. The fixture's parked session is now open at its limit, as Claude Code is (rule `limit_hit`; parking never stops a process). Before, it had ended there (`quota_exit`) while the head still offered Stop, so the terminal was held as Ended (no cursor, no ⌃Tab hint) beside a Stop for a process that was gone. What changed, on purpose:
- **A live terminal**: the toolbar says Live, the caret block shows, and the "⌃Tab leaves the terminal · Ctrl+Shift+F finds" hint sits in the strip below the rows; the terminal has focus and takes keys.
- **Stop agrees with the terminal**: Stop stays in the head because the process runs. A session that ended at its limit now has no Stop, its terminal reads Ended, and it shows "ran … · scrollback replayed" (lib/session-process `processRunning`).
- The selected rail row reads "Personal · started 1:13 AM" (W).

Weak: unchanged from before. The Runway card's second line is still sliced at the column's bottom, and the list's first visible line is cut at the lenses' edge.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: rail-and-offer-1280x800 dark and inspector-1280x800 light, beside the 0.4.1 baseline pass. Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right.

- **rail-and-offer:** Home's "Parked until a reset" between Needs you and the feed: "⏸ storefront · p…@example.com · resets in 2h 13m · Open".
- **inspector:** the parked session's page: the "Parked 2h 13m" chip, the reset banner "p…@example.com has reset. Resume the work you parked — the same conversation, on the same Account." with Resume filled and "Remind me in 30 min" quiet; the terminal Live with its caret; Stop in the head (the process runs).
- **Since 0.4.1:** both 0.4.1 weak points are gone: the Runway card's second line is no longer sliced (Home's right column), and the rail's head holds its lenses. **Weak:** "Parked until the quota resets in 2h 13m (at 4:26 AM)." is a bordered box inside the Where it runs card (box in a box; CRD-2), and the rail's list starts with its first group head half under the lenses (a scrolled list's edge).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | A session parked for its limit, found on Home and resumed in one click when room is back. |
| 2 | Prominence | 4 | Parked sits under Needs you, above the feed. |
| 3 | Default | 4 | Resume is the default; Remind me later beside it. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | No mock of its own; Home's cards and the Banner. |
| 6 | Areas | 4 | The inspector's parked note is a box in a card (weak). |
| 7 | Colour | 4 | No amber on a parked row (a fact, not a need). |
| 8 | Type | 4 | As Home. |
| 9 | Actions | 5 | Resume filled; Stop and Remind me tinted. |
| 10 | Serious | 4 | Stop beside a live parked session; Resume says it's the same conversation and Account. |
| 11 | Doors | 4 | Open goes to the session; its row in the rail says "Parked 2h 13m · 0% left". |
| 12 | Words | 4 | "resets in 2h 13m" and "at 4:26 AM": both, once each. |
| 13 | States | 4 | The reset said as soon as it comes. |
| 14 | Consistency | 4 | The Banner and Home's cards. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | Nothing to remember: the reset brings the offer. |

Lowest: 4.

**Re-captured after F55 (2026-11-24, Opus):** all four re-rendered; opened rail-and-offer-1280x800 dark and the inspector. The rail's desktop row reads "Stopped with an …" (F55); the parked row, its card and the reset offer are unchanged; the lens is F53's solid fill. Lowest stays 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-26, AJ-E3713, Opus, both themes)

Re-captured on the host; every changed capture opened (contact sheets, four to a sheet) beside its last capture and the approved story-feed mock (`AIJuggler-design-refs/story-feed/`). Captures: rail-and-offer (dark, light).

**What changed (AJ-E3713).** Home's feed below the pinned needs is the story feed now: the tabs All · Needs you · Sessions · Tasks · Pools · Switches & limits · This Mac (the chosen one the accent's soft fill), the state chips Running / Finished / Failed in their soft tints with today's counts, Showing: Every app at that row's end, and posts in place of log rows: a tone card each with the App's own icon (or AI Juggler's mark), the calm line "**Claude Code CLI** · Work Max · 10 min", the story with its thing as an accent-soft chip, a quiet gesture bar (↑ ↓ + … Mark reviewed · ↗ Open). Nothing else on these screens moved. The parked card and the rail's offer are as before; the feed under them is posts.

Scores unchanged but for: line 5 (Mock-up) 4: the feed follows the approved story mock (68-home-feed's 0.5.2 section lists the deviations); line 19 (Rich-mode colour) **4**: the feed's filters are choices, the chosen tab a soft accent fill, the state chips their own hues with a ✓ when on, the Showing picker's options with their icons; nothing chosen by a ring.

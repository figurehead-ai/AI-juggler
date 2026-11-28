<!-- captures: 0dc509a437f3 -->
<!-- captures: pending -->
# 99-rail-splitter: the rail's edge from the keyboard

Design critique (D-57) against the rail (01-rail). New family (AJ-E2909).

## Checklist

| Check | Finding |
|---|---|
| Grid | The rail's right edge, a 1 px line that thickens to the accent when focused. |
| Hierarchy | n/a. |
| Spacing | n/a. |
| Icons | None. |
| Shared controls | A focusable separator (WAI-ARIA window splitter): ← → by 8 px, ⇧ by 32, Home and End. |
| Disabled reasons | n/a. |
| Plain words | A screen reader hears its width. |
| Type sizes | n/a. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | 1280 only: at 1024 the rail folds to icons and has no edge to drag. Also the narrowest (Home, 208 px) and widest (End, 360 px), dark. |

## Good

- Focused, the edge is clearly the thing that moves.

## Weak

- Nothing on screen says the arrows move it; only a screen reader hears its role.

## Fixed

- A width changed and then left inside 200 ms (a quit, a reload) was lost: it was kept only after the pause or when the list went. It's now kept when the window goes too (pagehide); the run reloads at once and finds it.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: the grip and "Drag, or ← →" on hover and focus; at the narrowest the order menu keeps its ▾ only.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: focused-1280x800 light and narrowest-1280x800 dark. The rail's edge takes the keyboard: focused, the splitter draws the accent line along the rail's edge with its grip; at its narrowest the rows truncate, never wrap, and the Inactive apps head keeps its count ("INAC… 1 app · 2 sessions", F22's rule: the label gives way, never the count).

- **Since 0.4.1:** the calm rail (dot gutter, F22's head) and Home's two columns behind it; the focus line is the one bright line on screen, as PAT-SURF allows for focus.
- **Weak:** the focus draws two lines (the splitter's ring on both its sides, about 13 px apart); one would read cleaner, but it's the shared focus ring and the splitter's hit area, kept.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Resize the rail from the keyboard (arrows, Home/End) as with the mouse. |
| 2 | Prominence | 4 | The edge is quiet until hovered or focused. |
| 3 | Default | 4 | Its width is kept; a double-click restores the default. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | No mock; the splitter pattern (role separator, aria-valuenow). |
| 6 | Areas | 4 | Rows truncate at both stops; nothing overlaps. |
| 7 | Colour | 4 | Accent only on the focused edge. |
| 8 | Type | 4 | Unchanged rows. |
| 9 | Actions | 5 | No primary needed. |
| 10 | Serious | n/a | Nothing destructive. |
| 11 | Doors | 4 | Rows still open their sessions at every width. |
| 12 | Words | 4 | Its name says what it does ("Resize the session list"). |
| 13 | States | 4 | Width held across reloads (the spec reloads and checks 328). |
| 14 | Consistency | 4 | The shared focus ring. |
| 15 | Accessibility | 4 | Keyboard-operable separator with its value; Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | Room for long names when wanted, by mouse or keys. |

Lowest: 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The lens switch's chosen segment is a solid fill. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-26, AJ-E3713, Opus, both themes)

Re-captured on the host; every changed capture opened (contact sheets, four to a sheet) beside its last capture and the approved story-feed mock (`AIJuggler-design-refs/story-feed/`). Captures: focused at 1280 (dark, light), narrowest and widest at 1280 dark.

**What changed (AJ-E3713).** Home's feed below the pinned needs is the story feed now: the tabs All · Needs you · Sessions · Tasks · Pools · Switches & limits · This Mac (the chosen one the accent's soft fill), the state chips Running / Finished / Failed in their soft tints with today's counts, Showing: Every app at that row's end, and posts in place of log rows: a tone card each with the App's own icon (or AI Juggler's mark), the calm line "**Claude Code CLI** · Work Max · 10 min", the story with its thing as an accent-soft chip, a quiet gesture bar (↑ ↓ + … Mark reviewed · ↗ Open). Nothing else on these screens moved. The rail's edge is unchanged; the Home behind it shows the story feed.

Scores unchanged but for: line 5 (Mock-up) 4: the feed follows the approved story mock (68-home-feed's 0.5.2 section lists the deviations); line 19 (Rich-mode colour) **4**: the feed's filters are choices, the chosen tab a soft accent fill, the state chips their own hues with a ✓ when on, the Showing picker's options with their icons; nothing chosen by a ring.

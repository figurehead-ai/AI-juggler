<!-- captures: 7d3d7b6475cf -->
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

<!-- captures: 6b4e590c05b7 -->
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

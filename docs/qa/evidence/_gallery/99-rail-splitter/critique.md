<!-- captures: 3aecb21ee010 -->
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
| 1280×800 and 1024×700 | 1280 only: at 1024 the rail folds to icons and has no edge to drag. |

## Good

- Focused, the edge is clearly the thing that moves.

## Weak

- Nothing on screen says the arrows move it; only a screen reader hears its role.

## Fixed

- Nothing on first render.

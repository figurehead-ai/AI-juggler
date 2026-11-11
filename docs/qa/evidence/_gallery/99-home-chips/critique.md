<!-- captures: 35702eb78dcb -->
<!-- captures: pending -->
# 99-home-chips: Home's chips

Design critique (D-57) against Home (67-home). New family (AJ-E2910).

## Checklist

| Check | Finding |
|---|---|
| Grid | A segmented row above Needs you's items, with "n more under All" beside it when narrowed. |
| Hierarchy | All first with the total, then each kind with its count. |
| Spacing | The card's rhythm. |
| Icons | None. |
| Shared controls | The shared Segmented control. |
| Disabled reasons | n/a: a kind with nothing isn't offered. |
| Plain words | "Asks you 1", "Probably stuck 1", "4 more under All". |
| Type sizes | 12 px chips. |
| Both themes | Checked dark and light. |
| 1280×800, 1024×700 and 1440×900 | All three. At 1440 (dark) also Mark all and its toast ("Marked 2 reviewed", with Undo); no chip changes the rail (checked in the run). |

## Good

- Narrowing never hides that more waits: "4 more under All" is a link back.

## Weak

- The heading's count ("5 sessions need you") and All's count can differ when agents' asks are counted separately.

## Fixed

- Nothing on first render.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: the chips ("All 5 · Asks you 1 · Probably stuck 1 · Limit reached 1"), a chosen chip saying what it hides ("4 more under All"), Runway by account with Out, Pick up where you left off.

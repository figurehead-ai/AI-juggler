<!-- captures: 39d6fbd3ca57 -->
<!-- captures: pending -->
# 99-floor-list: the Floor list

Design critique (D-57) against the approved wireframes (D-78) and the History and Usage tables. New family (AJ-E3512).

## Checklist

| Check | Finding |
|---|---|
| Grid | One section per floor; one card per App with a header (initials, name, kind, account and usage at the right) and a table of its sessions. The columns have fixed widths, so they line up from one App to the next. |
| Hierarchy | Floor headings, then App names in bold, then sessions; a state that waits on you is bold with a warning-coloured glyph. |
| Spacing | 20 px between floors, 12 px between Apps. |
| Icons | Each state's glyph (! ▸ ? ⏸ ✕ ✓ · –) beside its word; the glyph is decoration, the word says it. |
| Shared controls | Button (ghost, small), the screen header with its action. |
| Disabled reasons | An App with no sessions says "No sessions · last active …" and still offers Show on the Floor. |
| Plain words | "2 floors · 5 Apps · 5 waiting on you", "Busy processor · 4 min", "Work Max · 62% left". |
| Type sizes | 12 px table text, 11 px column heads, 14 px App names. |
| Both themes | Checked. |
| 1280×800 and 1024×700 | Both. |

## Good

- The same facts as the canvas, in reading order, with "Show on the Floor" as the door back to it.
- A new wait is said once, politely; sessions AI Juggler runs are said by the main window's announcer, not twice.
- 150 rows render in 2.7 ms median, 15 ms p95 (AJ-E3512 timing.json), within 16 ms.

## Weak

- "Waiting on you" counts stuck, done and out-of-room sessions along with asks; that is the Floor's own meaning but wider than the word "asks" in the rows.

## Fixed

- Columns drifted between App tables (auto widths); now fixed widths.
- axe: "Show on the Floor" buttons had names that didn't start with their visible words; now "Show on the Floor: <session>".

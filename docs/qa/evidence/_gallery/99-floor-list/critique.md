<!-- captures: e6ed11482297 -->
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
- 150 rows render in 3.3 ms median, 6.4 ms p95 (AJ-E3512 timing.json, re-run 2026-11-06), within 16 ms.

## 0.4.1 (AJ-E3517)

The list now carries the Floor card's words. Resting on a row (keyboard focus, or the pointer for 250 ms, as on the Floor) reads that one session's question or latest step through the same reader as the card (its setting and redaction) and shows it in a bar at the list's foot: "Checkout copy · Asks: Should I keep the old wording on the checkout button, or use the new one from the design file?" (said-1280x800, said-1024x700, both themes). The bar is a raised white card in light and a raised surface in dark, the session's title in semibold, then the words; it is said politely to a screen reader and goes 300 ms after the row is left. Nothing is read for rows nobody rests on. The list captures (list-*) changed only in the "Waiting on you 5" chip, which now has a space before its count, so its accessible name is its visible words (axe: label-content-name-mismatch fixed).

## Weak

- said-1024x700: the bar floats over the rows at the window's foot and can cover one for the moment it shows (it never takes clicks).
- "Waiting on you" counts stuck, done and out-of-room sessions along with asks; that is the Floor's own meaning but wider than the word "asks" in the rows.

## Fixed

- axe (0.4.1): the "Waiting on you" chip's name was "Waiting on you: 5" beside its visible "Waiting on you5"; now its words.

- Columns drifted between App tables (auto widths); now fixed widths.
- axe: "Show on the Floor" buttons had names that didn't start with their visible words; now "Show on the Floor: <session>".

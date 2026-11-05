<!-- captures: a284a2da2041 -->
<!-- captures: pending -->
# 99-usage-cost: cost you can act on

Design critique (D-57) against Usage's tiles and Top sessions (94-usage) and the shared sheet. New family (AJ-E2902).

## Checklist

| Check | Finding |
|---|---|
| Grid | Two tiles side by side on the dashboard's grid, then Cost by account as a full-width table; the review sheet stacks the same parts. |
| Hierarchy | Tile label, ~figure, one line of what it counts; the table's header row over its rows; the price list's date in the section's sub line. |
| Spacing | The dashboard's card rhythm; table rows at the Top sessions height. |
| Icons | The money and tokens icons Usage's tiles use. |
| Shared controls | StatTile, Section, the shared sheet and its paged list; Top sessions' row for every session. |
| Disabled reasons | n/a. Unknown is words: "No finished tasks in this range", "Nothing went on crashed or stuck sessions", "No list price". |
| Plain words | "On 13 sessions: 10 crashed, 3 probably stuck — see them", "API-equivalent estimate — your subscription price doesn't change per token." |
| Type sizes | Tiles' figures at the tiles' size; table 14 px, sub lines 12 px. |
| Both themes | Checked; "Crashed" and "Probably stuck…" read in amber in both. |
| 1280×800 and 1024×700 | Both; the table keeps its five columns at 1024. |

## Good

- Every figure says what it's of ("65 finished · 51 other apps' not counted"), so a reader can't take an average of AI Juggler's sessions for all sessions.
- The wasted sessions are a click away and each says why it's there.
- The review puts last week beside the week before with the change, and exports just that week.

## Weak

- "Price" says "list price" on every row in the preview; the column earns its place only when a model lacks a price.
- The review's tiles repeat the dashboard's look closely; a reader may not notice they're a different week until the subtitle.

## Fixed

- First render (seen by eye): the review opened scrolled down at 1024, focus on the table's first account button; it now opens at its top.
- Axe (scrollable-region-focusable): the wasted sheet's list scrolled but couldn't be reached by keyboard when its rows held no button. A paged list that scrolls on its own is now a focusable, named region (every sheet using it).

<!-- captures: none -->
# 97-pool-canvas: the Pools screen and a pool's canvas

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | Header, canvas and inspector share the page card's edges; nodes sit on the automatic layout's rows (lead on top). |
| Hierarchy | Pool name, then its facts line; on the canvas the member's name leads, its state second; the inspector uses the app's section titles. |
| Spacing | The inspector's rows keep the 8 px rhythm; the palette's folded line is the one header in letter-spaced capitals. |
| Icons | The canvas toolbar (zoom, fit, Tidy up) and the header's Tasks and Messages carry the app's icons. |
| Shared controls | Canvas · List is the shared Segmented; Agents is the shared Picker. |
| Disabled reasons | Removing a member a pool needs says why on the palette while you drag. |
| Plain words | "1 message waiting", "Paused at the hop limit", "Let it continue", "Don't deliver". |
| Type sizes | Fixed in this branch: below 100 % the nodes draw at true size (12 px and up); full cards from 100 %. |
| Both themes | Checked dark and light; the waiting glow reads in both. |
| 1280×800 and 1024×700 | 1280 opens on the canvas at fit; under 1100 px a pool now opens in the List view unless you chose one. |

## Good

- Nodes, edges and asks read as a diagram; the inspector's sections (This pool, Live) use the app's section titles.
- The inspector on a member or an edge answers in place (Allow once · Deny, Deliver); the List view shows the same members as rows with who talks to whom.
- Tasks and Messages take the page's width; the Pools screen's cards have a live mini-map.

## Weak

- At fit the node text was about 9–10 px, under the 12 px floor, and edge labels ("Paused at the hop limit") were tiny (fixed in this branch, below).
- The palette's "ADD A SESSION" in letter-spaced capitals is the only such header on the page.
- The new pool's centred hint card floats in a large empty canvas.

## Fixed

- Re-render: Tasks and Messages over the canvas take the page's width (they sat beside the inspector, a column a few words wide); the Live feed's times stay on one line; "View as list" says "the List view".
- Polish (0.3, this branch): semantic zoom. Below 100 % the nodes keep their places but draw their words at screen size: the name at 13–14 px, the state dot and word at 12 px, the account line where the node has room, and the waiting count as a badge you click to answer its asks in place. What waits on an edge becomes a small badge ("1 waiting", "Paused") that opens Deliver, or Let it continue and Don't deliver. Full cards, with the ask answered on the card, from 100 % up. The switch fades only with reduced motion off; remembered places, the automatic first layout, Tidy up and Fit work in both forms.
- Polish (0.3, this branch): under 1100 px a pool opens in the List view unless you chose a view for it; the choice is kept per pool.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| pools-screen-1280x800, -1024x700 | Every pool as a card with its canvas in small; Checkout team says "6 members · 4 need you · 2 tasks moving"; two project pools labelled. | New screen (AJ-E2808). |
| canvas-1280x800 | Checkout team at fit: Planner (lead) on top, five members under it, a message waiting and a chain paused at the hop limit on their edges, the inspector's Live feed. | New. This branch: compact nodes at fit, edge waits as badges. |
| canvas-1024x700 | The same pool at 1024, opened on the canvas from the List view. | New. This branch: the window starts in the List view. |
| canvas-compact | At fit: names and states at true size, waiting counts as badges. | New state (this branch). |
| canvas-full-100 | At 100 %: full cards, Checkout copy pass's ask answered on its card (Allow once · Deny). | New state (this branch). |
| canvas-edge-badge-open | An edge's "Paused" badge opened: Let it continue · Don't deliver. | New state (this branch). |
| list-default-1024 | A pool opened at 1024 without a chosen view: the List view. | New state (this branch). |
| inspect-member-1280x800, -1024x700 | Checkout copy pass in the inspector: its ask with its answers, who it talks to ("To Planner"), its tasks and usage. | New. |
| inspect-edge-1280x800 | Planner → Fix the cart totals: the three messages between them. | New. |
| inspect-pool-1280x800 | The pool's settings in the inspector: what agents may do, the rule "Take a task", delivery and the hop limit of 5. | New. |
| list-1280x800 | The List view: six member rows and seven lines of who talks to whom. | New. |
| messages-panel-1280x800 | Messages over the canvas, the page's width. | New; re-render fix. |
| new-pool-1280x800 | A new, empty pool: its name field focused ("New pool"), "No one in this pool yet". | New. |

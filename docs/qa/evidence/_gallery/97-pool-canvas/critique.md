<!-- captures: a92984f7e4a1 -->
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
- The badge's panel and the edge's panel are a card inside a card (the ask's own bordered box in the panel); in canvas-badge-open and canvas-edge-badge-open (both widths) the open panel covers Tax rules research's name until it closes, and at 1024 the "Paused" panel also cuts "Spike: payment sandbox" to "Spike: payme".
- canvas-full (both widths): the ADD A SESSION palette sits over Planner's card, the 1024 zoom bar lies over the bottom row's text, and the side nodes are cut at the board's left edge ("cart totals", "ayment san…").
- inspect-member (both widths): the selected node's ⋯ button covers the end of "Checkout copy pass".
- list-1280x800, list-default-1024x700: the members' times ("2 min ago", "just now") are in the monospace face.

## Fixed

- Re-render: Tasks and Messages over the canvas take the page's width (they sat beside the inspector, a column a few words wide); the Live feed's times stay on one line; "View as list" says "the List view".
- Polish (0.3, this branch): semantic zoom. Below 100 % the nodes keep their places but draw their words at screen size: the name at 13–14 px, the state dot and word at 12 px, the account line where the node has room, and the waiting count as a badge you click to answer its asks in place. What waits on an edge becomes a small badge ("1 waiting", "Paused") that opens Deliver, or Let it continue and Don't deliver. Full cards, with the ask answered on the card, from 100 % up. The switch fades only with reduced motion off; remembered places, the automatic first layout, Tidy up and Fit work in both forms.
- Polish (0.3, this branch): under 1100 px a pool opens in the List view unless you chose a view for it; the choice is kept per pool.
- Second re-render (seen by eye): an agent's ask on an edge (answered on its sender's node, so the edge has no label) no longer takes a label's room. It had pushed the other way's "1 waiting" badge (Planner → Fix the cart totals) about 80 px off its line into empty canvas; the badge now sits on its dashed edge. Vitest: "an ask's edge takes no label's room".
- Second re-render (seen by eye): in a narrow box at fit (1024 px, 56 %, about 130 px wide) a two-line name is 12 px, so "Spike: payment sandbox" reads whole instead of "Spike: payment…"; at 1280 (69 %) names stay 13 px. Vitest: "a narrow box at fit draws a two-line name at 12 px".

- Edge routing: an edge between two members of one row that aren't neighbours (Fix the cart totals → Review the checkout steps, paused at the hop limit) ran straight behind the member between them. It read as two short edges through Checkout copy pass, and its "Paused" badge, pushed off that node, read as Checkout copy pass → Tax rules research. An edge that would cross a node now bends round it, on the side where its label has room and, where it can, no other edge runs. Its badge sits on its own line. At fit it arcs under Checkout copy pass with "Paused" on the arc; at 100 % it arcs over, so the full chip doesn't cover that node's ask. Vitest: "an edge never runs behind a node".
- One count: the header said "1 asks you" (only agents' asks), while the Pools card said "4 need you" and the node badges added up to 4. Both now count the same things in one place (asks, held deliveries, agents' questions, tasks waiting on you, and sessions whose state needs you), and the header says "4 need you". Vitest: "the pool's header says what its Pools card says".

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| pools-screen-1280x800, -1024x700 | Every pool as a card with its canvas in small; Checkout team says "6 sessions · 4 need you · 2 tasks moving" (0.4.1 rc33: was "6 members"); two project pools labelled. | New screen (AJ-E2808). |
| canvas-1280x800 | Checkout team at fit (69 %): Planner (lead) on top, five members under it, each compact — a state dot, its name (on two lines when long), "Lead · Working" or its state word, all 12 px and up; amber count badges on Checkout copy pass (1), Review the checkout steps (2) and Tax rules research (1); "1 waiting" on Planner → Fix the cart totals' dashed edge, and "Paused" on Fix the cart totals → Review the checkout steps, whose dashed edge arcs under Checkout copy pass; the header's "4 need you"; the inspector's Live feed. | New. This branch: compact nodes at fit (the cards' words were 9–10 px), edge waits as badges. |
| canvas-1024x700 | The same pool at 1024 (56 %), opened on the canvas from the List view: the same compact nodes, names wrapping to two lines at 12 px, "Spike: payment sandbox" whole. | New. This branch: compact nodes; the window starts in the List view, the canvas chosen. |
| canvas-badge-open-1280x800, -1024x700 | Checkout copy pass's badge pressed: a small panel under the node with "Checkout copy pass wants to message Planner" and Allow once · Deny, focus on Allow once. | New state (this branch). |
| canvas-edge-badge-open-1280x800, -1024x700 | The "Paused" badge of Fix the cart totals → Review the checkout steps pressed (on its arc under Checkout copy pass): "Paused at the hop limit" with Let it continue · Don't deliver. | New state (this branch). |
| canvas-full-1280x800, -1024x700 | At 100 % (the zoom button): full cards — tool, name, account · tool, state · task — the ask answered on Checkout copy pass's card, "1 message waiting · Deliver" and "Paused at the hop limit · Let it continue · Don't deliver" on their edges (the paused one arcs over Checkout copy pass, clear of its ask). | New state (this branch). |
| list-default-1024x700 | Checkout team opened at 1024 with no view chosen: the List view (six member rows, who talks to whom), List lit in the switch. | New state (this branch). |
| inspect-member-1280x800, -1024x700 | Checkout copy pass in the inspector: its ask with its answers, who it talks to ("To Planner"), its tasks and usage. | New. |
| inspect-edge-1280x800 | Planner → Fix the cart totals: the three messages between them. | New. |
| inspect-pool-1280x800 | The pool's settings in the inspector: what agents may do, the rule "Take a task", delivery and the hop limit of 5. | New. |
| list-1280x800 | The List view: six member rows and seven lines of who talks to whom. | New. |
| messages-panel-1280x800 | Messages over the canvas, the page's width. | New; re-render fix. |
| new-pool-1280x800 | A new, empty pool: its name field focused ("New pool"), "No one in this pool yet". | New. |

## 0.3.2

The canvas gains the most from the 0.3.2 surfaces: in dark the dotted board, the nodes and the inspector are three distinct steps, so Planner's violet frame and the amber "needs you" frames on Checkout copy pass, Review the checkout steps and Tax rules research read at a glance in canvas and pools-screen; in light the white nodes and Pools cards sit on a grey ground. The design fixes show in dark only: the lighter accent on the header's New session, the logo mark, Planner's frame and the violet edges, the new pool's focused name field, Change what agents may do… and inspect-member's "1 Allow once", the one filled button beside a quiet "2 Deny"; light changes only by a pixel in the This Mac line. The top bar stays one row, with "This Mac · Plenty of room" at 1280 and "7 to clean up" whole at 1024 (the list folded, no This Mac line). New session and New pool are the solid violet primaries, New message leads messages-panel, and the inspector's section labels (THIS POOL, LIVE, WAITING ON YOU) sit dimmer than their values, which makes inspect-pool's long settings column easy to scan. The old overlaps are all still there: in canvas-badge-open and canvas-edge-badge-open the open panel covers Tax rules research's name, and at 1024 the "Paused" panel cuts "Spike: payment sandbox" to "Spike: payme" (canvas-badge-open-1024, whose panel sits higher, leaves Spike whole); in canvas-full the ADD A SESSION palette sits over Planner's card, the 1024 zoom bar lies over the bottom row's text, and side nodes are cut at the board's left edge; in inspect-member the selected node's ⋯ button covers the end of "Checkout copy pass". The List view's times ("2 min ago") are in the monospace face, as before.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

A pool you made shows its scope as a chip ("Scope every project") and sealed when kept apart (9b6b9c96b). The New pool wizard (new-pool-*) is in the family: the Build Mac template says "in a later release", not "(0.5)" (9d96ccd06). At 1024 a node's name cuts at 45 % zoom (design.md, under 4, item 3).

## 0.4.1 rc33 (2026-11-11): sessions, said as sessions

The rc33 P5 walk read "0 members" on a pool whose Account member was there: the card counted sessions under the word for member rows. pools-screen (both widths, both themes) now says "6 sessions · 4 need you · 2 tasks moving", "1 session · quiet" on docs-site and storefront, the word the pool's header uses ("5 open in AI Juggler · 6 in all"). canvas, canvas-badge-open, canvas-edge-badge-open and canvas-full: the inspector's This pool row reads "Sessions 6 · lead Planner" (was "Members"). Only these 20 were retaken, in the pinned container; nothing else on them moved. The overlaps listed under Weak are unchanged.

## 0.4.6 (AJ-E3011, AJ-E3012): the setup in order, one selection, the inspector titled by it

new-pool-* (both widths, both themes) now walks eight steps instead of four: How to start (Start from scratch is the first, full card; the small "Set up by hand" link is gone), Where, Who's in, Who leads, Everyone else, Who may message whom, On their own, Review — each with a heading and a why-line, the order ticked down the side, and the foot saying what the rest is filled in with from Who's in on. new-pool-roles-* belongs to the old four-step wizard and new-pool-1280x800-* to the removed "by hand" path; both stay as the earlier evidence their manifests cite. canvas and inspect-*: the nodes stand off the board by surface, no hairline and no amber glow (a need is its word, dot and badge, L2); the inspector leads with the selection's own title, its kind and App, and a status pill, its title edge in the accent; the selected node has the one ring (2 px accent, 2 px offset) on 0.5's raised `--color-selected`; the header's picker says "On their own: Work together". inspect-pool opens the pool's own setup above its settings (who leads, how everyone else joins with the role cards, who may message whom as a line per role, what they may do on their own) and the custom actions only when Custom is chosen.

- Weak, new: the node's ⋯ on a selected card still overlaps a long name at 100 %; with four broad members the canvas fits smaller and compact names break mid-word (the compact form's `break-words`).
- Fixed (the coordinator's check): compact names break only between words, and a word wider than its room makes the name one line with an ellipsis ("Fix the c…", "Checko…"); the Lead's state line ends in an ellipsis too ("Lead · Worki" was cut mid-letter at 45 %). inspect-member-* drops the pool's "Setup 2 of 6" line: it shows only on the pool. new-pool-talk-* and new-pool-review-*: the feature-team template's Observer row reads Blocked, calm, and the review says "an Observer only reads".

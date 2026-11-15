<!-- captures: f33b856790b6 -->
# 99-pool-selection: the canvas's selection and the inspector's title (AJ-E3012)

Design critique (D-57, AJ-E2814) against 97-pool-canvas (the canvas before) and the calm UI round 2 rules. The canvas with each kind selected — nothing (the pool), a session, an App, an Account, a Tool, everyone in scope, a line, the pool's own setup opened from the checklist, and the lead's place waiting for an App's next session — dark and light, 1280×800. Fixture names only.

## Checklist

| Check | Finding |
|---|---|
| Grid | Canvas, the members' column (224 px) and the inspector (340 px) share the page card's edges; the inspector's title block aligns with its sections' 16 px gutter. |
| Hierarchy | The inspector leads with the selection's own title (15 px semibold), its kind and App as a subtitle (muted), then its status pill; the setup line and the sections follow. |
| Spacing | 12 px between member cards in the column, the stack's two edges inside it; the inspector keeps its 8 px rhythm. |
| Icons | The App's icon beside a session's title; the lead's crown on its place. |
| Shared controls | The role and own-choice Pickers, the setup's LeadPicker, JoinAs, RoleCards, TalkTable and OwnLevelChoice in the pool's own setup. |
| Disabled reasons | An App's role picker greys Lead with its reason. |
| Plain words | "Session · Claude Code · Work Max · Lead", "App · Reviewer", "Line · what passes between these two", "Lead · it leads once it starts here"; "Its sessions read messages on their next turn". |
| Type sizes | 12 px floor in the cards' wake lines and the pill. |
| Both themes | Checked dark and light: the accent ring and its canvas-coloured offset read in both; the raised surface is 0.5's `--color-selected`. |
| 1280×800 | With four broad members the column scrolls inside itself; the canvas fits beside it. |

## Good

- One selection treatment everywhere on the canvas: a solid 2 px accent ring with a 2 px offset on a raised surface; no dashes. Keyboard focus is a separate ink outline outside it.
- The selected card and the inspector are linked by the same accent on the inspector's title edge.
- Broad members read as many (a stacked card), not as a dashed outline; their names take two lines before an ellipsis.
- The checklist never vanishes: a one-line "Setup · 2 of 6 done · next: …" over a member, the full card on the pool.

## Weak

- At the 38 % fit forced by four broad members, compact node names still break mid-word ("Plann/er"), the older compact form's `break-words`; the fit, not the selection, is the cause.
- Each broad card's wake line adds a third line; with four cards the column scrolls at 800 px.
- The inspector's back control reads "Pool" (to the pool) and "Done" (from its settings): two words for one way back.

## Fixed

- (This branch) Opening a setup step scrolled the whole window (scrollIntoView); it scrolls the inspector's own list now.
- (This branch) The talk table's five columns didn't fit the inspector's 340 px; there it is a line per role, its pairs wrapping.

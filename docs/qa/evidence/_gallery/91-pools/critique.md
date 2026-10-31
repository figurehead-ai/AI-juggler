<!-- captures: dd257746b7d7 -->
# 91-pools: the rail's Pool grouping, Move to pool, Agent access

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | The Pool grouping uses the rail's group rows; Move to pool is a list of pools in one bordered list. |
| Hierarchy | Sheet title, then the session ("ai-juggler · feat/read-model · Work Max"), then the pools. |
| Spacing | The rail's rhythm; the sheet's rows evenly spaced. |
| Icons | The sheet's pools have none, unlike the row menus' lines. |
| Shared controls | The grouping is the shared Segmented; Agent access uses the shared Picker. |
| Disabled reasons | The pool it's in says "It's here" instead of greying out. |
| Plain words | "ai-juggler · all sessions" names a project's own pool. |
| Type sizes | Body 14 px, hints 12 px. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Captured at 1280 only. |

## Good

- The rail's Pool grouping appears once a pool of yours exists; a pool's header opens its page.

## Weak

- The switch shows four segments in a 264 px rail; it fits only because segments size to their words.
- Move to pool's lines carry no icon or member count.
- rail-by-pool-1280x800, settings-agent-access-1280x800: the rail row "Refactor the settings screen" reads "ai-juggler · viewed 2 mi…", the age cut mid-word.

## Fixed

- Re-render: segments size by their words (Pool cut "Account" short).
- 0.3 (this branch): the old pool page's captures (pool-cards, pool-access, pool-sheet) were retired; the pool's page is the canvas, in 97-pool-canvas.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| rail-by-pool-1280x800 | The rail grouped by Pool: Checkout redesign with its session, then Not in a pool. | The 0.3 switch above Needs you (AJ-E2802). |
| move-to-pool-1280x800 | feat/read-model's Move to pool: Checkout redesign (It's here), Docs, ai-juggler · all sessions, New pool…. | The sheet closes with × (AJ-E2813). |
| settings-agent-access-1280x800 | Settings › Agent access: every pool's level and the rules you made. | The 0.3 header and shared Picker. |

## 0.3.2

rail-by-pool fits the four-segment switch (App, Project, Account, Pool) in the rail with Pool selected, groups the rail under "Checkout redesign 1" and "Not in a pool 1", and on Home the Your apps, Needs you and Runway by account cards stand off the canvas, with Sign in the one solid violet button. After the design fixes the dark captures carry the lighter accent on the logo mark, the New session buttons, Sign in, the Work Max progress bars and the "Measure its limits ›" link, which now read clearly against the near-black; in light only the top bar's "This Mac · Plenty of room" moved by a sub-pixel, and the bar stays one row with the whole "7 to clean up" pill. The rail's "Refactor the settings screen" row still cuts its meta to "ai-juggler · viewed 2 mi…". move-to-pool still lists its three pools without icons or member counts; "It's here" is a quiet right-hand label, and New pool… and Cancel are both outlined, which is right when choosing a row is the action. settings-agent-access shows the pools as bordered rows inside a raised card, with Docs opened to its four radios (Work together selected, now the lighter violet in dark) and a "Messages put into sessions" fold; it is rows inside a card inside a card, but the names are bright, the "Work together · 1 rule ›" values a step dimmer, and nothing clips in either theme.

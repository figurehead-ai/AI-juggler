<!-- captures: 14c40f94fc03 -->
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

## Fixed

- Re-render: segments size by their words (Pool cut "Account" short).
- 0.3 (this branch): the old pool page's captures (pool-cards, pool-access, pool-sheet) were retired; the pool's page is the canvas, in 97-pool-canvas.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| rail-by-pool-1280x800 | The rail grouped by Pool: Checkout redesign with its session, then Not in a pool. | The 0.3 switch above Needs you (AJ-E2802). |
| move-to-pool-1280x800 | feat/read-model's Move to pool: Checkout redesign (It's here), Docs, ai-juggler · all sessions, New pool…. | The sheet closes with × (AJ-E2813). |
| settings-agent-access-1280x800 | Settings › Agent access: every pool's level and the rules you made. | The 0.3 header and shared Picker. |

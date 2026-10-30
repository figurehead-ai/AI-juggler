<!-- captures: c87aa9af9c4d -->
# 93-messages: a pool's Messages and messages in Needs you

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | Thread list and open thread side by side on the panel's two columns. |
| Hierarchy | Thread title, who is in it, then the messages; each bubble names its sender, kind and time. |
| Spacing | Bubbles evenly spaced; the "2 of 6 agent messages in a row without you" pill wraps inside its column. |
| Icons | Needs you's message cards carry the message mark; Pause and Stop are words. |
| Shared controls | No system controls; the search is the app's field. |
| Disabled reasons | A paused pool says why with a banner and Resume. |
| Plain words | "To Builder · Started its turn", "an agent", "Pause this pool's messages". |
| Type sizes | Body 14 px, meta 12–13 px. |
| Both themes | Checked dark and light; your own bubbles stand apart in both. |
| 1280×800 and 1024×700 | Captured at 1280, 1680 and 2000; at 800 px the reply box is below the fold. |

## Good

- Messages over the canvas use the page's width: list and thread side by side.

## Weak

- The thread's bubbles stay narrow even with the room; the reply box is below the fold at 800 px.

## Fixed

- Re-render: the panel takes the page's width (it sat beside the inspector, a few words wide), and its pill wraps inside its column.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| pool-messages-1280x800, -1680x1050, -2000x1250 | Checkout redesign's Messages over its canvas: six threads, "Start on the API" open with Planner's handoff, Builder's question and your reply. | The panel opens over the canvas, the page's width (AJ-E2808, re-render fix). |
| pool-docs-1280x800, -1680x1050, -2000x1250 | The pool's shared documents and what was withheld. | The 0.3 header and pool page. |
| pool-paused-1280x800 | The pool's messages paused (two agents kept answering each other): the banner and Resume. | The 0.3 header and pool page. |
| session-messages-1680x1050 | A session's own Messages tab. | The 0.3 header. |
| needs-you-1280x800, -1680x1050, -2000x1250 | Home's Needs you: an agent's held message and a chain at the hop limit as approval cards, a question to you, messages waiting. | The 0.3 header. |

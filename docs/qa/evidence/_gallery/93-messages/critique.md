<!-- captures: 22c059722956 -->
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
- pool-paused-1280x800: Resume messages in the paused banner and New message below it are both solid violet, two primaries a few lines apart.
- pool-docs-1280x800, -1680x1050, -2000x1250: the capture opens scrolled to the documents, so the thread above is cut at the panel's top edge (at 1280 only the reply box's "Deliver to their sessions" row and Send show; at 2000 a cut bubble, and a thread-list column empty except "Pause this pool's messages").
- pool-messages-2000x1250, pool-docs-2000x1250: the open thread stops well short of the Messages card's right edge, leaving an empty strip.

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

## 0.3.2

The messages panel, the open thread and the shared-document cards stand a step lighter off the dark canvas; in light the panel is a white sheet on grey. New message (pool-messages, pool-paused, session-messages) and Allow once / Deliver in Needs you are solid violet; on Needs you they are the only filled buttons in the content, but pool-paused adds a solid "Resume messages" in its banner just above New message, so that screen has two. The disabled Send stays pale and reads as off. The yellow outline on the Builder's question bubble and the pale amber "agent that isn't on the canvas" strip keep the "this one is for you" signal in both themes. After the design fixes the dark captures carry the lighter accent on the logo mark, New session, New message, Allow once, Deliver, the Work Max runway bars, the "Measure its limits ›" link, the unread "1" badge, the "Deliver to their sessions" checkbox and the border of your own bubble ("An empty list. A 404 would break the mobile client."), which now separates it more clearly from the agents' bubbles; the light captures changed only in the top bar, which stays one row with "This Mac · Plenty of room" and the whole "7 to clean up" pill. pool-docs at all three sizes still opens scrolled to the documents, so what is above is cut at the panel's top edge: at 1280 only the reply box's bottom row ("Deliver to their sessions", Send) shows, and at 2000 a bubble is sliced and the thread-list column holds only "Pause this pool's messages". At 2000 wide the open thread in pool-messages and pool-docs stops about a tenth short of the Messages card's right edge, leaving an empty strip, and the bubbles stay narrow. needs-you at 2000 holds its content in a centred column with wide empty margins, which is fine. After the second pass the rail's "Refactor the settings screen" row keeps its time whole in every capture ("ai-ju… · viewed 2 min ago" where it read "ai-juggler · viewed 2 mi…"); nothing else changed.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: needs-you-1280x800 light (before and after the fix), pool-paused-1280x800 dark and session-messages-1680x1050 dark (two renders). Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right.

- **needs-you:** an agent's held message ("An agent asks to send · Checkout redesign: «Reviewer» wants to message Builder", 1 Allow and deliver · 2 Allow, deliver later · 3 Always allow in this pool · 4 Deny), a paused chain (1 Deliver · 2 Don't deliver), then the cards: a question (Reply · Open the thread), a waiting message (Deliver · Read), Personal's sign-in. **Before the fix** three filled buttons: Allow and deliver, Deliver and Reply ↵. **Fixed** (b3d54ab49): one in the zone, the first card's (Allow and deliver); the second approval and the cards below are tinted. Re-captured (six) and checked by eye; home-needs-order.test holds the zone to one.
- **pool-paused:** the pool's Messages with "Messages in this pool are paused — «Builder» and «Planner» kept answering each other (8 messages without you). No agent can send or be woken until you resume them." and Resume messages filled; the thread on the right.
- **Weak:** the approval cards are an amber fill behind words with an amber border (PAT-COLOR-2, PAT-SURF-1): logged **F59** (the same card is on the pool's inspector and the driven panel). session-messages' rail rendered once scrolled ("5 new above") and once at its top: a capture race, the newer one kept.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | An agent's message that needs you is answered in place; a pool's talk is read and steered. |
| 2 | Prominence | 4 | Approvals first, then the cards by wait. |
| 3 | Default | 4 | Keys 1–4 on each approval; Enter the first. |
| 4 | Mode | 4 | Calm: the ask cards are need cards by tone. |
| 5 | Mock-up | 4 | Matches D-118's mock for the question card ("builder-1 asks you"). |
| 6 | Areas | 4 | Approvals, then cards; the thread a column. |
| 7 | Colour | 4 | F59 fixed: amber is the ask's dot and its kind's words, never a fill behind words or a border. |
| 8 | Type | 4 | Kind labels 12 px caps in amber, the summary 13 px. |
| 9 | Actions | 4 | After the fix: one filled button in Needs you (was 3). |
| 10 | Serious | 4 | Deny is plain; Always allow says "in this pool"; a hop limit is never lifted by a rule. |
| 11 | Doors | 4 | Open the thread goes to the pool's Messages at that thread. |
| 12 | Words | 4 | «Name» quotes for agents; each card says its pool and thread. |
| 13 | States | 4 | Paused said once with what resumes it. |
| 14 | Consistency | 4 | AskCard shared with the tools' asks. |
| 15 | Accessibility | 4 | Each card a focus stop with its keys. Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | No agent talks unseen; each held message is one key away. |

Lowest as first scored: 3 (line 7, F59 logged: the shared ask card's look and its baselines).

**Re-captured after F59 (2026-11-24, Opus):** all twenty-two re-rendered; opened needs-you-1280x800 in both themes and pool-messages-1680x1050 (unchanged but for F53's chosen segment). The two approval cards in Needs you now are need cards: the surface tone, no border, an amber dot before "AN AGENT ASKS TO SEND · CHECKOUT REDESIGN" (they were an amber wash with an amber border). Inside a panel that frames it (a pool's inspector, the driven panel) the card is one tone step up; no capture here shows that one.

Lowest: 4 (line 7 was 3: F59 fixed).

# Evidence manifest — AJ-E1802

Task: `docs/tasks/E18-agent-messaging.md` AJ-E1802, your inbox and compose (P0).
Status: **DONE-EVIDENCED** in the preview (IN-REVIEW until the release PR; the Docker visual gate not run here). 2026-10-25, claude-opus.

Design: `ui/src/pages/messages/` (MessagesView, ThreadView, Compose, SharedDocs, PoolLinks, PoolMessages), `ui/src/components/MessageNeeds.tsx`, `ui/src/lib/ipc/messages.ts`, `crates/juggler-app/src/features/messages/{views,needs,commands}.rs`. D-42 point 3: the "Inbox pane" is each pool's and each session's **Messages** tab plus Home's **Needs you**, no new screen (E21). Test output: [vitest.txt](docs/qa/evidence/AJ-E1802/vitest.txt), [gallery.txt](docs/qa/evidence/AJ-E1802/gallery.txt).

## What exists now

1. **A pool's Messages tab** (lazy-loaded): threads newest first (sender, time, state, new-for-you badge, search on the store's full-text index, 30 a page), the open thread as a conversation in the session page's bubbles (an agent's on the left, yours on the right; kind, time, "an agent"; where it is for each recipient; secrets withheld), Pause / Stop / Resume, earlier messages a page at a time, a reply box, **New message** to a member or the whole pool with **Deliver to their sessions** (on by default, D-42 point 6), the paused banner with **Resume messages**, and **Shared documents** (AJ-E1804). At most 1,440 px wide.
2. **A session's Messages tab**, shown once it has messages: its own threads, written to it.
3. **Needs you**: an agent's held message and a chain at the hop limit as the same approval card as a tool's ask (E23: the batch, keys 1–9, Enter), a question to you (**Reply** opens the thread), messages waiting for a session AI Juggler runs (**Deliver**: one click for all), a paused pool or thread (**Resume**). Notifications for questions: not wired (see below).
4. **Commands** (async, `ipc::blocking`, all called by the UI): `messages_threads`, `messages_thread`, `messages_send`, `messages_deliver`, `messages_deliver_waiting`, `messages_set_thread`, `messages_set_pool`, `messages_needs`, `messages_answer`, `messages_docs`, `messages_doc`, `messages_typing`, `messages_search`, `messages_links`, `messages_link`.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 each pool's threads with sender, time and state; a thread view; compose to a member or the pool | vitest `messages.test.tsx` "a pool's messages" (6); `messages.rs::the_user_writes_with_deliver_and_a_terminal_session_is_never_typed_into` | PASS |
| Req.2 a question to the user under Needs you | vitest "Needs you" (3); `needs::needs` in `messages.rs::above_the_level_…`, `…ping_pong…`, `delivery_waits_…` | PASS |
| Req.2 notifications (AJ-E1510) when on | — | UNPROVEN (not wired: a question shows in Needs you only; follow-up) |
| Performance: opens in < 200 ms with 1,000 messages | `messages.rs::a_thousand_messages_open_quickly_and_page` (the first page of threads and the newest thread: 10.4 ms) | PASS |
| Quality: vitest | [vitest.txt](docs/qa/evidence/AJ-E1802/vitest.txt): 20 passed (messages 11, ask cards 9) | PASS |
| Quality: Playwright and the a11y gate (axe 0 critical/serious) | `ui/tests/visual/messages-gallery.spec.ts`, [gallery.txt](docs/qa/evidence/AJ-E1802/gallery.txt): 23 passed, each state axe-scanned | PASS |
| Visual: the inbox, a thread and compose, both themes | `docs/qa/evidence/_gallery/93-messages/`: `pool-messages-{1280x800,1680x1050,2000x1250}-{dark,light}.png`, `pool-docs-…`, `needs-you-…`, `pool-paused-1280x800-…`, `session-messages-1680x1050-…` (22 captures, each opened and checked by eye) | PASS |
| Readability: nothing under 12 px in the new views | vitest "no text smaller than 12 px in the message views"; AskCard's own small text raised to 12 px | PASS |

## Baselines to make (Docker visual gate, not run here)

- `ui/tests/visual/__baselines__/linux/gallery/93-messages/pool-messages-1280x800-dark.png` (the family's gate state, new).
- Possibly changed by AskCard's 12 px text: `gallery` captures of E23's ask cards.

Last updated: 2026-10-25.

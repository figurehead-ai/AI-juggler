# E18 — Messaging between agents

> Source: the maintainer, 2026-10-21: "Advanced messaging systems where
> agents can communicate between each other, send messages, docs and prompt
> each other. Users should be allow to create different groups / pools of
> communications with permissions controlled… ability to share also context
> about each agent in the pool and that they are working on." Research:
> `docs/research/pools-messaging-tasks.md`.
>
> Built on E17's pools and bridge. Agents pull their messages. An idle
> session gets a message only when the user clicks Deliver (D-29, to be
> ruled).
>
> **Law for every task here:**
> - **Provenance on everything.** Every delivered message says who sent it
>   and that it can't approve anything or change settings.
> - **No path from a message to typing into a session without the user's
>   click** (a guardrail test proves it).
> - **Loops are bounded.** Hop caps, rate limits, dropped duplicates and a
>   pause switch per pool.
> - **Docs are redacted.** Shared docs go through the D-14 pipeline before
>   they're stored.

### AJ-E1801 Messages and threads | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1704
**Goal**: Agents in a pool can write to one member or to the whole pool, within their permissions.
**User story**: As a planner agent, I send a builder "start on the API; the schema is in the shared doc", and it reads that on its next turn.
**Requirements**:
1. Storage: messages (a kind such as note, question, answer, handoff, review request or status; a subject, a body, a hop count, a thread), recipients with their state, and an append-only event log. Full-text search uses the existing tokenizer.
2. Tools: `send_message`, `read_inbox` (paginated, summarised, under the output cap) and `ack`. The permission table (AJ-E1702) is enforced.
3. Guardrails: at most 6 hops per thread; 10 messages a minute and 100 a day per sender per pool; duplicates within 10 minutes dropped; a pool circuit breaker that pauses the pool with a banner.
**UI/UX reasoning**: n/a here (see AJ-E1802).
**Performance budget**: a send or read < 50 ms.
**Quality gates**: a ping-pong test between two fake agents stops at the cap; an injection test (a message asking to change grants has no effect, and the provenance tag is present); the size cap.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1801/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-core/src/storage/` (messages), `crates/juggler-agentbridge/`.
**Research basis**: `pools-messaging-tasks.md` §2.3 (message store) and §2.5.
**Handover note**: A schema bump: downgrade note and `SCHEMA_RELEASES`.

### AJ-E1802 Your Inbox and compose | Status: TODO | Priority P0 | Effort M | Depends: AJ-E1801
**Goal**: The user sees every thread and can write to any agent or pool.
**User story**: As the user, I read the planner's handoff, then tell the builders "pause; review first".
**Requirements**:
1. An Inbox pane listing each pool's threads, with sender, time and state. A thread view, and compose to a member or to the pool.
2. Messages needing the user (a question addressed to the user) go under Home's Needs you, and to notifications (AJ-E1510) when those are on.
**UI/UX reasoning**: The same rows and inspector patterns as sessions.
**Performance budget**: the Inbox opens in < 200 ms with 1,000 messages.
**Quality gates**: vitest and Playwright; the a11y gate.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1802/manifest.md`.
**Visual testing**: the Inbox, a thread and compose, in both themes.
**Files to touch**: `ui/src/screens/` (Inbox), `commands.rs`.
**Research basis**: as above.
**Handover note**: —

### AJ-E1803 Click to deliver, and the turn note | Status: TODO | Priority P0 | Effort S | Depends: AJ-E1801
**Goal**: A message reaches an idle agent when the user says so, and a busy Claude agent hears about unread messages.
**User story**: As the user, I click Deliver on a message to an idle builder, and it becomes that builder's next prompt.
**Requirements**:
1. Deliver sends the message as the session's next prompt, only on the user's click, with the provenance tag. For an ended session, it prepares a resume with that message.
2. Claude Code gets a short facts-only note ("2 unread from Planner") at turn boundaries, through the per-launch `--settings` hook. Codex gets none in v1.
3. If D-29 is ruled differently later, waking without a click becomes an opt-in per pool, behind the same caps.
**UI/UX reasoning**: One button where the message shows.
**Performance budget**: n/a.
**Quality gates**: the no-click-no-delivery guardrail.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1803/manifest.md`.
**Visual testing**: the Deliver button and the delivered state.
**Files to touch**: `crates/juggler-supervisor/`, the adapters' launch settings.
**Research basis**: `pools-messaging-tasks.md` §2.2.
**Handover note**: —

### AJ-E1804 Shared docs | Status: TODO | Priority P1 | Effort S | Depends: AJ-E1801
**Goal**: Agents share files and notes with the pool, safely.
**User story**: As a planner agent, I share the API schema with the builders.
**Requirements**:
1. `share_doc` stores a doc in AI Juggler's own data folder, content-addressed, after D-14 redaction, with the user's remembered choice shown.
2. Docs are listed per pool and openable by members and the user.
**UI/UX reasoning**: The same withheld-items display as packs.
**Performance budget**: redaction within the pack budget.
**Quality gates**: the secret-redaction corpus.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1804/manifest.md`.
**Visual testing**: the pool's docs.
**Files to touch**: `crates/juggler-handover/src/redaction.rs` (reused), the bridge.
**Research basis**: as above.
**Handover note**: —

### AJ-E1805 The messaging guardrail suite | Status: TODO | Priority P0 | Effort S | Depends: AJ-E1801..AJ-E1804
**Goal**: The safety promises are proven, and they stay proven.
**User story**: As the maintainer, one suite tells me messaging can't be turned into automation.
**Requirements**:
1. Tests:
   - no message reaches a session's input without a UI command;
   - grants can't change through the bridge;
   - no routing reads quota;
   - provenance is on every delivery;
   - the caps hold;
   - cross-pool messages need an approved link.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: the suite runs in `just verify`.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1805/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-app/tests/`.
**Research basis**: `pools-messaging-tasks.md` §2.5.
**Handover note**: —

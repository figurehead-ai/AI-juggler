# Evidence manifest — AJ-E1803

Task: `docs/tasks/E18-agent-messaging.md` AJ-E1803, delivery and the turn note (P0).
Status: **DONE-EVIDENCED** on fakes (IN-REVIEW until the release PR); the live run is `docs/qa/messaging-lab.md`. 2026-10-25, claude-opus.

Design: `crates/juggler-app/src/features/messages/delivery.rs`; the driver's boundary kick and "Started by a message from …" line (`features/runtime/driver.rs`); the hook note (`features/runtime/reports.rs`, `juggler-agentbridge/src/hook.rs`). Test output: [messages-tests.txt](docs/qa/evidence/AJ-E1803/messages-tests.txt).

## What exists now

1. **By runtime.** A driven session gets a message as its next turn through its runtime's `send_turn` with `TurnOrigin::Delivered { sender }` (the session page shows "Started by a message from Planner"; up to 5 queued messages go as one turn, each with its provenance). A session in a terminal is never typed into: it keeps its messages in its inbox, and Deliver isn't offered for it. A member not running: Deliver queues for its next session (resume), the same Work item.
2. **Only between turns.** A delivery worker looks when a turn ends, an ask closes, the session goes idle, a message is queued or the user stops typing; it delivers only when the tool is idle with nothing asked of the user and the user isn't typing in that session's box (`messages_typing`, two minutes at most). The tool also refuses a turn while one runs, so a race only leaves the message queued.
3. **Only with permission (D-29).** Queued by the user's Deliver (or their compose with Deliver), or by `allowed(pool, deliver_to_session, hops taken)`: Full or a rule. At the hop limit (5 by default; the user's click resets it) the thread pauses and Needs you asks **Deliver** / **Don't deliver**; a rule never lifts it. The pool's stop-all and pause hold pool-queued messages; the user's click still goes.
4. **The note.** A Claude Code session with the agent tools hears "AI Juggler: N unread messages from …" as `additionalContext` from its `UserPromptSubmit` and `PostToolUse` hooks, once per change, facts only. Codex gets none (as E18 says for v1).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 Deliver on the user's click becomes the next prompt, labelled | `messages.rs::delivery_waits_for_the_users_click_at_work_together` (the line "Started by a message from Planner"; one Deliver for all that wait) | PASS |
| Req.1 by itself at Full or by a rule, only between turns, never at a permission prompt or over typing | `messages.rs::at_full_it_goes_in_between_turns_never_at_a_permission_prompt_or_over_typing` (queued while the fake's approval is open and while the user types; delivered after; the fake's transcript holds the provenance word for word) | PASS |
| Req.1 an ended session: a resume with the message | `views.rs` recipient words ("Deliver gives it this as its next prompt when you resume it"); queued per member | PASS (fakes; the resume path is the existing one) |
| Req.1 never typed into a terminal | `messages.rs::the_user_writes_with_deliver_and_a_terminal_session_is_never_typed_into`; `messages_guardrails.rs::a_message_reaches_a_sessions_input_only_through_its_turn_and_only_with_permission` | PASS |
| Req.2 Claude's turn note through the `--settings` hook; facts only | `hook::tests::a_note_is_printed_only_where_it_adds_context_and_never_decides`; `messages.rs::a_claude_style_terminal_session_hears_a_facts_only_note_once_per_change` | PASS |
| Req.3 the hop limit pauses and asks; a person's click resets it; the stop-all | `messages.rs::the_hop_limit_pauses_the_chain_and_asks_and_stop_all_holds_it` | PASS |
| Req.3 rate limit and duplicates on every level | AJ-E1801's tests (the same pipeline for every sender) | PASS |
| Req.3 Claude Code's own messages between sessions | — | UNPROVEN (not used in v1: D-42 point 4) |
| Gate: no delivery without a click, Full or a rule | `messages_guardrails.rs` (source and behaviour: ten messages at Work together start no turn) | PASS |
| Visual: the Deliver button and the delivered state | `_gallery/93-messages/pool-messages-*.png` ("Started its turn", **Deliver**) | PASS |
| Live: the same with real Claude Code and Codex | `docs/qa/messaging-lab.md` steps 2–5, run 2026-10-26 through AI Juggler's backend ([lab-2026-10-26.md](docs/qa/evidence/AJ-E1803/lab-2026-10-26.md)): Deliver started Codex's turn with "Started by a message from Planner" and the provenance; at Full a message stayed queued through Codex's open approval and while the user typed, and went in 0.3 s after; the hop limit of 2 paused the third note and asked, a rule refused, Deliver reset it to 0; the terminal session was never typed into and heard "AI Juggler: 1 unread message from You in your pool…" (facts only) from its hook | PASS (backend; the window's views and keys: BLOCKED, for the maintainer) |
| Live: one Needs you card per waiting message | the same lab, step 4: while the chain was paused, the `deliver` ask and "Builder has a message waiting" both showed for one message | FAIL (UX: the lab's finding 4) |

Last updated: 2026-10-26.

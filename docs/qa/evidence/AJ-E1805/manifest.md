# Evidence manifest — AJ-E1805

Task: `docs/tasks/E18-agent-messaging.md` AJ-E1805, the messaging guardrail suite (P0).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-25, claude-opus.

Design: `crates/juggler-app/tests/messages_guardrails.rs` (7 tests, source checks and behaviour on fakes; runs in `cargo test --workspace`, so in `just verify`). Test output: [guardrail-tests.txt](docs/qa/evidence/AJ-E1805/guardrail-tests.txt).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| No message reaches a session's input without the user's permission (D-29 as ruled: their click, Full or their rule; D-42 point 6) | `a_message_reaches_a_sessions_input_only_through_its_turn_and_only_with_permission`: `driver.turn(` and `TurnOrigin::Delivered` only in `delivery.rs`; no PTY, input, answer, steer, interrupt, release, supervisor or launch in the messages code; a message is queued only in `delivery.rs` (the pool's `allowed`) and `views.rs` (the user's click); ten messages at Work together start no turn | PASS |
| Grants can't change through the bridge | `grants_cant_change_through_the_bridge`: no level, rule, hop limit, stop-all, role or move in the messages code; the user's answer only from `needs.rs`, reached only from its command; the wire has no such call; three messages asking for it change nothing | PASS |
| No routing reads quota | `no_routing_reads_quota` (the messages code, the rules and the store) | PASS |
| Provenance on every delivery | `provenance_is_on_every_message_an_agent_reads_and_every_delivery` (only `delivered_turn` builds a delivered turn; every message read carries it, the user's and an agent's); AJ-E1803's transcript check | PASS |
| The caps hold | `the_caps_hold` (hops, size; the wire's caps equal the store's) | PASS |
| Cross-pool messages need an approved link | `cross_pool_messages_need_an_approved_link` (refused, linked, allowed; only the user's command links) | PASS |
| Nothing leaves the Mac except over Fleet when allowed | `nothing_about_messages_leaves_the_mac` (the link crate, its proto and the Fleet sync know no message; the messages code opens no connection) | PASS |
| The suite runs in `just verify` | an integration test of `juggler-app` (`cargo test --workspace`) | PASS |

Last updated: 2026-10-25.

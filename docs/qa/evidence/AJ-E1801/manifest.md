# Evidence manifest — AJ-E1801

Task: `docs/tasks/E18-agent-messaging.md` AJ-E1801, messages and threads (P0).
Status: **DONE-EVIDENCED** on fakes (IN-REVIEW until the release PR); the live run is `docs/qa/messaging-lab.md`. 2026-10-25, claude-opus.

Design: `crates/juggler-core/src/messaging.rs` (the rules), `crates/juggler-core/src/storage/messages.rs` (schema v34), `crates/juggler-app/src/features/messages/` (send, agent, delivery, docs, needs, views), ADR-0020's E18 addendum, D-42 (proposed). Test output: [core-tests.txt](docs/qa/evidence/AJ-E1801/core-tests.txt), [bridge-tests.txt](docs/qa/evidence/AJ-E1801/bridge-tests.txt), [messages-tests.txt](docs/qa/evidence/AJ-E1803/messages-tests.txt).

## What exists now

1. **Storage (v34)**: threads (subject, hop count, open / paused / stopped with why), immutable messages (kind, subject, body, hash, hop, sender and its name then; an update trigger refuses changes), recipients with their state (`unread`, `read`, `acked`, `held`, `queued`, `delivered`, `refused`), an append-only event log (update and delete triggers refuse), full-text search over subject and body with the transcript index's tokenizer, shared docs, pool links, held asks and pool pauses. Lists page by cursor. Pre-migration copy kept; `SCHEMA_RELEASES` gets 34 when 0.2 ships (RELEASING.md's table updated).
2. **Tools**: `send_message`, `read_inbox` (one page of at most 10, bodies cut at 1,200 characters, under 16 KiB, each with its provenance; `message` reads one in full; reading marks read), `ack`, plus E1804's `share_doc` and `read_doc`. One argument shape per tool (`deny_unknown_fields`). Command line: `send`, `reply`, `inbox`, `read`, `ack`, `share`, `docs`, `doc`.
3. **The permission table**: every call through `allowed(pool, action)` and audited: `post_message` (an ask holds the message for the user, AJ-E1802), `read_messages` (new, in Read: D-42 point 2), `share_doc`. Recipients: a member, `pool`, `user`; another pool's member only through the user's link.
4. **Guardrails**: 6 agent messages in a row a thread (a person's word resets; past it the thread pauses and Needs you says so); 10 a minute and 100 a day per sender per pool; the same message to the same people within 10 minutes dropped; the circuit breaker pauses the pool (banner, Needs you, Resume) when two agents answer each other 8 times without a person or a sender hits its rate 3 times in 10 minutes. Size: body 4,000 characters, subject 120, doc 64 KiB.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 storage: kinds, subject, body, hop, thread, recipients and states, append-only log, FTS | `storage::messages::tests` (5: immutable messages and log, search in its pool, links and pauses, 1,000 messages paged in < 100 ms); `messages_storage.rs` (v33 → v34 keeps a copy and every pool, level and rule; 0.1.0 → v34 keeps 0.1.0's copy) | PASS |
| Req.1 schema bump: downgrade note and `SCHEMA_RELEASES` | RELEASING.md's 0.2.x row names 34; `busy_timeout.rs`, `storage_roundtrip.rs`, `switch_graph.rs` undo v34 | PASS (the notes' entry is the release's) |
| Req.2 `send_message`, `read_inbox` (paged, summarised, capped), `ack` | `messages.rs::a_message_reaches_its_recipients_inbox_with_its_provenance_and_answers_fast`; `mcp::tests::a_tools_arguments_have_one_shape`; `cli::tests`; `wire::tests::a_message_names_its_recipient_and_nothing_else` | PASS |
| Req.2 the permission table enforced | `messages.rs::above_the_level_a_message_is_held_for_the_users_ok` (Read holds it; an observer asks); audit rows for `post_message` and `read_messages` | PASS |
| Req.3 6 hops a thread | `messages.rs::a_ping_pong_between_two_fake_agents_stops_at_the_cap` (the 7th refused, the thread paused, the user's word reopens it from 0) | PASS |
| Req.3 10 a minute, 100 a day, duplicates, the breaker | `messaging::tests::the_rate_is_ten_a_minute_and_a_hundred_a_day`, `…breaker…`; `messages.rs::the_size_cap_the_rate_duplicates_and_the_breaker` | PASS |
| Gate: a ping-pong between two fake agents stops at the cap | the same test, both ways: one thread (hop cap) and new threads each time (the loop check pauses the pool, Needs you says why, Resume) | PASS |
| Gate: an injection has no effect; the provenance tag is present | `messages.rs::a_message_asking_to_change_grants_changes_nothing_and_says_who_wrote_it`; `messages_guardrails.rs::grants_cant_change_through_the_bridge` | PASS |
| Gate: the size cap | `the_size_cap_…`; `mcp::tests::a_tools_arguments_have_one_shape` (a doc over 64 KiB never reaches the app) | PASS |
| Performance: a send or read < 50 ms | `bridge.rs::calls_answer_in_under_50_ms` (sends and reads over the socket, slowest 3.4 ms with 8 sessions); in process: send 2.5 ms, read 1.0 ms | PASS |
| Live: real tools call the tools without a prompt | `docs/qa/messaging-lab.md` steps 2 and 5 | UNPROVEN (the coordinator's lab) |

## Not covered here

- Parallel branch: `e24-processes` also takes schema v34 (`sightings`). Whichever merges second renumbers its migration to v35 (for E18: `const V34` → `V35` in `storage/mod.rs`, and the 34s in `busy_timeout.rs`, `storage_roundtrip.rs`, `messages_storage.rs`).

Last updated: 2026-10-25.

# Evidence manifest: AJ-E3832 (Bot's world model, the `gyrus` crate)

Built 2026-11-26 on release/0.5.2 (Opus), to ADR-0038 (D-150) over ADR-0037. No new crate in
Cargo.lock: gyrus depends on rusqlite, serde_json and thiserror only (`crates/gyrus/tests/deps.rs`
fails on any other, and on any `juggler-*`). Migration V56 (the next after V55, checked on every
local and remote branch: none claims 56). Every world write passes ADR-0037's write gate
(`MemoryStore::write_world`, `remember_linked`); entity nodes are pointers, their state read live.

R1: PASS — commit 917014b62. `crates/gyrus`: nodes, links, kind packs (`Pack`), the SQLite store
(`SCHEMA_V1`, V56 in `storage::memory::world`), the in-memory index (`Snapshot`, links both ways by
kind, strongest first), write-through after commit (`Graph::publish`, a generation gap marks it
stale), rebuild, forget. Tests: gyrus `core.rs` (`the_index_after_every_write_equals_a_rebuild_from_sqlite`,
`the_pack_refuses_what_it_doesnt_declare`, `forget_leaves_no_trace_in_the_file_the_index_or_memory`:
27 phrases, file + WAL + every keyword term + the index: 0), `kill.rs`
(`killed_at_every_step_the_rebuild_equals_sqlite`: the writer SIGKILLed after staging, after commit,
after publish; the rebuilt index equals SQLite, the batch whole or absent), `deps.rs`, `schema.rs`
(the batch's hash pinned); juggler-core `tests/world.rs` (the gate refuses a wrong writer, an
undeclared kind or link, a secret, a dangling link, and logs codes only; a record and its node commit
together; forget reaches the graph; Clear chat history and AI Juggler's own copies leave no world node).
R2: PASS — commit 6e76883a6. `ops::seed`, `expand` (the best K per step by score, with an exact
bound so a hub's weak links are never read), `rank` (one per fact key), `matches`, `as_of` (`at` on
every read), `Graph::touch`, `explain`; gyrus `tests/ops.rs` (8 tests, ADR-0038 §3's pattern
example among them). The benchmark (`benches/workload.rs`, gate `tests/budgets.rs`, CI step "The
world model's budgets" in `build-macos`). Measured p95 at 1M links (budget): seed 3.9 (5), expand 0.5
(5), as_of 0.09 (5), rank 0.001 (1), matches 0.05 (5), touch 0.2 (3), forget 10.4 (20; 12.6–19.3 on
a busier run); 10k and 100k well inside: `crates/gyrus/BUDGETS.md`. To fit: vectors packed 64 to a
row (an exact scan of 10,000 384-dim vectors 5 → 2 ms), stop words (> 2 % of labels) left out of
keyword search, the keyword index in 32 buckets so a forget rebuilds one.
R3: PASS — commit 2c35d593a. `features/ai/world/pack.rs` (17 kinds, 15 link kinds with their ends,
walk rules, decay, tiers, reasoning questions), `link.rs` (records as `memory` nodes; aliases on the
memory node so they are forgotten with it; entity pointers; turns), `resolve.rs` (live lines);
tests `the_pack_declares_adr_0038s_kinds_and_links`, `entities_are_pointers_linked_as_the_records_relate_them`,
`records_are_linked_as_memory_nodes_and_nothing_moves` (the one-time pass, record count unchanged),
`bots_remember_flow_links_the_record_in_the_same_write`, `nodes_resolve_to_live_lines`.
R4: PASS — commit 77fc00afc. `assemble.rs` (seed from the look-up's rows, ADR-0037's recall, the
user's names, the topic; expand; rank; resolve live; questions; packed into 320 est. tokens), Bot's
ask flow (`bot/commands.rs`: the section in both routes; each turn's node part of its topic, about
the rows it rested on; uses applied in the turn's hold), the breakdown ("Linked from your work" part,
what went in and why, memories once). Tests `the_world_section_seeds_walks_and_reads_live_state`
(ADR-0038's agent11 example; a changed state read anew), `a_topics_context_seeds_from_its_own_turns`,
`bots_breakdown_shows_what_the_world_model_put_in`, `bot::answer::tests::*` (a turn marked wrong
is never sent back, as an earlier question either), vitest `bot-model.test.tsx` "the topic's totals".
**The 27-canary forget test:** `ai::world::tests::forget_leaves_none_of_27_canaries_anywhere` (notes
said in Bot, names given, questions asked, facts derived from a note, the log's sidecars; forgotten
through the app's forget and Clear chat history; searched in every file of the data folder, both
keyword indexes' terms and the in-memory index): **0 hits**.
R5: PASS — commit a21dc9fc4. `gyrus::dream::plan` (promote by use, decay, archive never what the
user stated, fold turns into topics, propose then merge duplicates and never an agent's, prune weak
idle links, bounded batches with a cursor), `features/ai/world/maintain.rs` inside ADR-0037 §2.3's
maintenance (2 min after the first paint, hourly, never while an AI job starts or answers, ≤ 200
writes a hold, ≤ 2,000 a pass, one `memory.maintained` event with counts only; dangling memory nodes
forgotten). Tests gyrus `tests/dream.rs` (5), `ai::world::tests::the_upkeep_pass_links_prunes_dreams_and_logs_counts_only`;
one idle-pass batch p95 3.4 ms at 1M links (budget 200).
R6: PENDING — the real-engine check on the reference Mac (the coordinating session's run).

## Usability answers (PAT-USE), as built (the context breakdown)

1. Someone wondering why Bot answered as it did opens "context …": match — the part reads "Linked from your work".
2. A newcomer sees sizes and up to six plain lines ("agent11 · named in your question", "shop · connected to agent11"), no internal term: match.
3. Its value in the one breakdown, nothing replaced; memories counted once: match (`with_world_memory`).
4. Warming up or nothing linked: the part reads 0, no list: match (`WorldContext::warming`, an empty section).
5. No system prompt: match (AI Juggler's own database only).
6. Only what was sent is listed (an empty section lists nothing); names only, no state words in the list; lines truncate in the 248 px popover: match.

Settings › AI › Memory: unchanged (forgetting a memory now also removes what the world model linked to it).

Left: R6; the a11y gate and an Opus critique of the breakdown in both themes with captures (the coordinating session's walk).

## Verdicts

| Req | Verdict | Date |
|---|---|---|
| R1 | PASS | 2026-11-26 |
| R2 | PASS | 2026-11-26 |
| R3 | PASS | 2026-11-26 |
| R4 | PASS | 2026-11-26 |
| R5 | PASS | 2026-11-26 |
| R6 | PENDING | 2026-11-26 |

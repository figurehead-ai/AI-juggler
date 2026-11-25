# World-model memory for the Bot: which graph engine (research, 2026-11-25)

**Recommendation in one line:** keep SQLite as the only store. Grow ADR-0037's store into a proper bi-temporal graph schema in the same file. Do not adopt a graph engine now. LadybugDB, and Grafeo after it matures, are the fallbacks to re-check if Cypher or multi-million-edge traversal is ever needed. If one is ever adopted, it holds only a rebuildable copy of the graph, never the master data.

Release dates and repo states come from the GitHub and crates.io APIs, checked 2026-11-25. AI Juggler's LICENSE is MIT. The footprint figures are estimates, not measurements.

## 1. Comparison

| Engine | License | Status (latest release) | Embedded fit | Temporal | Vector / FTS | Rust | Footprint | Risks |
|---|---|---|---|---|---|---|---|---|
| **SQLite graph** (tables, `WITH RECURSIVE`, FTS5, vectors as BLOBs or sqlite-vec) | Public domain; sqlite-vec MIT/Apache | SQLite is active; sqlite-vec v0.1.9 (2026-03-31), 0.1.10-alpha.4 (2026-05-18), no pushes since May | Already in-process; one file, one transaction, existing backup and forget | You build it in columns; ADR-0037 already has supersede and as-of | FTS5 is in use. Vectors: brute force in Rust, or sqlite-vec / sqlite-vector (Apache-2.0, active) | rusqlite 0.41 (in use) | ~0 extra (sqlite-vec is about 100 KB of C) | Multi-hop depth is limited by CTE cost; sqlite-vec is pre-1.0 and quiet |
| **Kùzu** | MIT | **Archived 2025-10-10** (v0.11.3) | Was excellent | None native | HNSW + FTS extensions | `kuzu` crate (C++ build) | Tens of MB (C++) | Dead upstream |
| ↳ **LadybugDB** (Kùzu fork) | MIT | Active, **v0.23.0 (2026-11-25)** | In-process, Cypher | None native | Vector index + FTS | `lbug` crate | Tens of MB, long C++ build | Small maintainer base; a second file outside SQLite's transactions |
| ↳ RyuGraph / Bighorn | MIT | RyuGraph v25.9.2 (2025-12-06), dormant; Bighorn **archived 2026** | – | – | – | `ryugraph` | – | Stale |
| **CozoDB** | MPL-2.0 | v0.7.6 (**2023-12-11**); last push 2024-12 | Embedded; can use SQLite as storage | **Built-in time travel** (`Validity`) | HNSW, FTS, LSH | Native Rust | Moderate | Unmaintained; Datalog is hard for small models |
| **SurrealDB** | **BSL 1.1** (core; Apache-2.0 after 4 years; no "Database Service" use) | v4.0.1 (2026-11-09) | Embedded mode exists | Versioned KV only | HNSW + FTS | Native | Large dependency tree | Not OSI open source inside an MIT app; heavy |
| **Oxigraph** | MIT OR Apache-2.0 | v0.5.13 (2026-10-22), active | Embedded `Store` | RDF 1.2 triple terms (statement metadata) | **Neither** | Native (RocksDB C++) | RocksDB adds MBs | SPARQL is a poor surface for 1–2B models; no hybrid recall |
| **DuckDB + DuckPGQ** | MIT | DuckDB v2.0.1 (2026-11-20); DuckPGQ is a research extension, opt-in grammar in 2.0 | In-process, but built for analytics (OLAP) and a single writer | None | VSS + FTS extensions | `duckdb` crate | ~20–40 MB | Extensions install by **download** (`INSTALL … FROM community`); separate file; "features may still be under development" |
| **HelixDB** | Apache-2.0 (was AGPL) | v3.7.0 (2026-11-11) | Now "built on object storage", HTTP `/v2/query`; embedded mode is thin | None | Vector + BM25 | SDK | Server-shaped | Changed direction; not a library-first design |
| **IndraDB** | MPL-2.0 | v5.0.0 (2025-08-16); quiet since | Library | None | None | Native | Small | Missing every feature we need |
| **Grafeo** (new) | Apache-2.0 | v0.6.3 (2026-11-15), very active | Embedded, ACID, pure Rust | None native | Vector, BM25, hybrid (RRF) | Native | Moderate | Version 0.x, one year old; feature profiles still changing ("embedded" profile removed in 0.7) |
| OverGraph / GraphLite (new) | Apache-2.0 / Apache-2.0 | 0.18.1 (2026-11-11) / 0.0.1 (2025-11), quiet | In-process | None | OverGraph: HNSW + sparse | Native | Small | One maintainer / abandoned-looking |

**What decides it:**

1. **No engine supports bi-temporal facts natively** except Cozo, and Cozo is unmaintained. Graphiti builds bi-temporality in its own edge columns too.
2. **A second store breaks forget and backup.** ADR-0037 §1.5 relies on one SQLite transaction with `secure_delete`, the FTS rebuild, tombstones and the restore scrub, all built and reviewed. A second engine has no shared transaction, its own WAL and compaction, and no proof of secure delete.
3. **Our graph is small and mostly anchored.** It holds thousands to low hundreds of thousands of edges, and traversals are 1–3 hops. That is the range where indexed recursive CTEs and an in-memory adjacency pass in Rust are fast enough.

## 2. Recommendation and migration path

**Keep SQLite and add a graph schema on top of it.** Reverse ADR-0037 §8.3's "no graph store" only in part. The store becomes a graph, still in SQLite: a typed, closed vocabulary instead of a graph database. Keep §3, "live facts over stored": sessions, apps, accounts, projects, pools and tasks stay in their own tables. The world model stores only what those tables cannot know: observed relations over time, people, decisions, outcomes and preferences.

**Migration, adding only** (as §1.7 requires):

1. `memory_entities` stays as the **mention** table, the bridge from a record to an entity.
2. New `wm_entity` and `wm_alias` tables hold entities that no app table owns (person, topic, bug, decision) and the aliases of all entities. The existing `alias:*` records seed `wm_alias`.
3. Widen the reserved `memory_links` into **`wm_edge`**, which gets bi-temporal columns and a closed relation list. The planned bot relations (`rested_on`, `caused`, `outcome_of`, …) become entries in that list.
4. `memory_records` stays the store for **facts and notes**. Add a nullable `valid_to_ms`; `as_of_ms` serves as valid-from.
5. Episodes are **not copied**. They are references into the existing append-only logs (`ai_log`, `agent_events`, `chat_turns`).
6. Retrieval stays behind the `Retriever` trait (§4.4). It gains a graph-expansion pass, and later vectors fused by RRF.

## 3. Schema shape

```sql
wm_entity(id, kind,            -- person|topic|bug|decision|… ; anchored kinds point to app rows
          anchor_kind, anchor_id,  -- e.g. 'session', id; NULL for world-only entities
          name, summary, created_ms, retired_ms)
wm_alias(entity_id, alias_norm, source, authority, UNIQUE(alias_norm, entity_id))
wm_episode_ref(id, stream, log_seq, at_ms)        -- pointer into ai_log / agent_events / chat_turns
wm_edge(id, subject_key,                           -- supersede unit, as in memory_records
        src_id, rel, dst_id,                       -- rel from a closed enum with domain/range kinds
        valid_from_ms, valid_to_ms,                -- world time (when true)
        recorded_ms, superseded_by,                -- system time (when we learnt / retired it)
        authority, untrusted, confidence,
        episode_id, quote_hash)                    -- provenance: which episode, which exact span
wm_fact = memory_records (+ valid_to_ms)           -- free-text facts, decisions, preferences
wm_vec(owner_kind, owner_id, model, dim, vec BLOB) -- optional, opt-in embedder
```

- **Indexes:** `(src_id, rel) WHERE superseded_by IS NULL`, `(dst_id, rel)` for the same rows, and `(valid_from_ms, valid_to_ms)`. FTS5 covers entity names, aliases and fact text.
- **Relation cardinality** is declared in Rust. For a *functional* relation (`prefers_model_for`, `owns`), a new edge closes the old one: the old edge's `valid_to` takes the new edge's `valid_from` (Graphiti's rule), and it is superseded, never deleted. Forget still hard-deletes, as §1.5 does.

**How the small model writes, safely:**

- It never writes SQL or Cypher. It fills a constrained schema: Apple Foundation Models' guided generation, or a llama.cpp grammar for Qwen3-1.7B. The fields are `{subject_mention, rel∈enum, object_mention, when_hint, quote}`.
- Code then does the rest:
  - resolves each mention (anchor id, then exact alias, then FTS candidates; an ambiguous mention is **parked**, never guessed);
  - checks the relation's domain and range;
  - checks that `quote` appears **word for word** in the source episode, the anti-fabrication rule;
  - applies the authority gate (§1.3), then supersedes by cardinality.
- Most edges should come from **deterministic observers** (session→project, pool membership, task→session, approval→decision), not from the model. Mem0 dropped typed LLM-extracted relations for a co-occurrence graph, which suggests model-extracted relations are noisy.

**How it reads:** a fixed menu of query templates:

- `neighbors(entity, rel?, at?)`
- `path(a, b, ≤3 hops)` (recursive CTE)
- `timeline(entity, from, to)`
- `recall(text, k)`: FTS ⊕ vector ⊕ one-hop graph expansion, fused by RRF, optionally re-ranked by personalized PageRank (HippoRAG) computed in Rust

Results come back as numbered refs, and the existing cite check rejects any claim without one.

**Design references (as references, not dependencies):**

| System | What its world model stores | How it consolidates and decays |
|---|---|---|
| Graphiti/Zep | Episode, entity and community subgraphs; edges with `valid_at`/`invalid_at`/`created_at`/`expired_at` | Contradictions invalidate edges, never delete them; entity resolution by embedding + BM25 candidates, then a judge |
| Mem0 | Extract-then-update with ADD/UPDATE/DELETE/NOOP; graph now native and untyped | – |
| Cognee | – | `memify` prunes stale nodes and reweights edges by use; dates extracted by default |
| Letta | Core blocks in context, plus searchable archival memory | Sleep-time agents consolidate in the background |
| A-MEM | Notes with keywords, tags and context | Linking a new note can rewrite old notes ("evolution") |
| HippoRAG 2 | OpenIE triples, phrase and passage nodes, synonym edges | Recall by personalized PageRank, triples filtered first |

What carries over to AI Juggler:

- bi-temporal invalidation (Graphiti);
- a closed write set (Mem0);
- consolidation in idle batches, as ADR-0037 §2.3's maintenance runs, rather than on the hot path (Letta, Cognee);
- PPR for associative recall (HippoRAG 2).

Not adopted: A-MEM-style rewriting of old memories by a model, which breaks the never-invent rule.

## 4. 0.5.2 versus the next version

**0.5.2** (groundwork with no model; if 0.5.2 is meant to be bug-fix only, this moves to the start of 0.6):

- One additive migration: `wm_entity`, `wm_alias`, `wm_edge`, `wm_episode_ref`, and `valid_to_ms` on `memory_records`. Check the migration number across all branches.
- Deterministic observers that fill anchored edges from existing app events, with a backfill.
- Forget, backup and restore extended to the new tables: the same transaction, tombstones by entity, the FTS rebuild, and tests mirroring the §1.5 security review.
- `Retriever` gains one-hop expansion with the `SameEntity` reason, plus graph cases in the recall harness.
- A benchmark: 200k edges with 3-hop CTE latency, as a gate that would reopen the engine question.

**Next version (0.6, the bot):**

- Model-proposed edges and facts through the constrained write path, with the parked-ambiguity review list (§8.2).
- People and topic entities with alias-based entity resolution.
- `decision`/`action`/`outcome` chains as edges.
- An opt-in on-device embedder with brute-force cosine (sqlite-vec only if the vector count exceeds about 100k).
- PPR re-ranking.
- An idle consolidation batch: merge duplicates, close stale edges, and lower the confidence of failed decision kinds.
- Re-check LadybugDB and Grafeo then, against the benchmark.

## Sources

**Engines and extensions**
- Kùzu archived: https://www.theregister.com/2025/10/14/kuzudb_abandoned/ · https://github.com/kuzudb/kuzu
- LadybugDB: https://github.com/LadybugDB/ladybug
- RyuGraph: https://github.com/predictable-labs/ryugraph
- Bighorn: https://github.com/Kineviz/bighorn
- CozoDB: https://github.com/cozodb/cozo
- SurrealDB license: https://github.com/surrealdb/surrealdb/blob/main/LICENSE · https://surrealdb.com/license
- Oxigraph: https://github.com/oxigraph/oxigraph · https://docs.rs/crate/oxigraph/0.5.13
- DuckPGQ: https://duckdb.org/community_extensions/extensions/duckpgq · https://github.com/cwida/duckpgq-extension
- HelixDB: https://github.com/HelixDB/helix-db
- IndraDB: https://github.com/indradb/indradb
- Grafeo: https://github.com/GrafeoDB/grafeo
- OverGraph: https://github.com/Bhensley5/overgraph
- GraphLite: https://github.com/GraphLite-AI/GraphLite
- sqlite-vec: https://github.com/asg017/sqlite-vec
- sqlite-vector: https://github.com/sqliteai/sqlite-vector
- Turso: https://github.com/tursodatabase/turso
- SQLite recursive CTEs: https://sqlite.org/lang_with.html
- SQLite FTS5: https://sqlite.org/fts5.html

**Memory architectures**
- Zep paper: https://arxiv.org/abs/2501.13956
- Graphiti: https://github.com/getzep/graphiti · https://blog.getzep.com/beyond-static-knowledge-graphs/
- Mem0: https://arxiv.org/abs/2504.19413 · https://docs.mem0.ai/open-source/features/graph-memory
- Cognee: https://docs.cognee.ai/guides/time-awareness · https://github.com/topoteretes/cognee
- Letta: https://docs.letta.com/guides/agents/architectures/memgpt · https://docs.letta.com/guides/agents/architectures/sleeptime
- A-MEM: https://arxiv.org/abs/2502.12110
- HippoRAG 2: https://arxiv.org/abs/2502.14802 · https://github.com/OSU-NLP-Group/HippoRAG

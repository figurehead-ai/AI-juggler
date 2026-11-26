# ADR-0038: Bot's world model: a typed graph kept in memory over SQLite, built as its own open-source crate, gyrus

**Status:** Accepted (2026-11-26, the maintainer). The name is `gyrus`. The repository goes public once the first version works inside Bot. D-150 is accepted as this ADR, and AJ-E3832 is unblocked for 0.5.2.
**Date:** 2026-11-26
**Deciders:** the maintainer (the engine, the open-source project and its name); Opus (design)
**Serves:**
- the maintainer's world-model vision (2026-11-25);
- D-133 (one memory and context system, never rewritten);
- [ADR-0037](./0037-memory-and-context.md), which this ADR **extends**, not replaces: the store, tiers, write gate, forget, assembler and log;
- [ADR-0034](./0034-ai-jobs-router.md) (the router and Chat's look-ups);
- [ADR-0030](./0030-storage-concurrency.md) (one writer lock);
- [ADR-0019](./0019-extension-architecture.md) (modules);
- D-37 (no reading another process's environment);
- D-52 (OS seams);
- D-131 (no download without a click).

## Context

### What Bot needs: the maintainer's words, as the spec

- **A map like a human mind's.** Connected nodes for:
  - the user (name, preferences, behaviour);
  - projects and tasks;
  - Apps, profiles and accounts, and the active profile;
  - sessions;
  - memories shared across Apps;
  - facts and rules.
- **Each node has rules** on how it is used and what it links to, **and reasoning questions attached to it.**
- **Context is rebuilt for every request,** never compacted. That means the recent turns up to a budget, plus the nodes found and walked, plus memories, rules and live facts. Topics are separate chats, each with its own context, and the main Bot chat runs forever.
- **Constant upkeep:**
  - tiers run immediate → short → mid → long, with promotion and demotion;
  - a "dreaming" pass while idle archives stale items, lowers relevance and merges duplicates;
  - **every lookup returns a bounded top-K,** ranked by relevance, usefulness and recency.
- **The goal, by example.** For "tell me how many tokens we used today, and tell agent11 to double-validate their work":
  1. find the phrases' nodes in the graph (today's usage, the session called agent11, its App);
  2. pull related nodes up to a limit, following each kind's rules;
  3. prune;
  4. build a small, exact context, instead of dumping everything.

### What ADR-0037 already decided, and what changes here

ADR-0037 built these, and they stay as they are:
- `memory_records` (kinds, tiers, supersede, write gate);
- "forget really deletes";
- one assembler with per-job budgets;
- the append-only log;
- live facts over stored facts.

Its §4.4 said relation-widened recall would use AI Juggler's own `relations` tables, "no graph store". **That is the one line this ADR replaces.** The vision needs typed nodes and links with properties, rules on how links are walked, pattern matching and ranked bounded walks. Joining tables per question can't do that well (measured below).

### The evidence

**1. Our benchmark** (`~/Dev/aij-graph-bench/RESULTS.md`, 2026-11-26):
- Setup: 10,000 links, 15 node kinds, 24 link types, links valid over time, Zipf hubs, 384-dimension vectors.
- Every engine returned identical results for the same seeds. Typical (p50) times in ms:

| | SQLite alone (recursive SQL) | **SQLite + typed graph in memory** | SurrealDB 4.0.1 |
|---|---|---|---|
| 3-step walk from a hub | 2.39 | **0.10** | 20.8 |
| Pattern match | 0.11 | **0.00** | 1.93 |
| Rule-guided walk | 0.07 | **0.001** | 1.0 |
| "As of" time, 2 steps | 0.12 | **0.009** | 2.5 |
| Keyword + vector + 1 step | 0.30 | **0.23** | 1.44 |
| **Context build** (3 seeds, expand, rank, top 50) | 0.42 | **0.046** | 9.75 |
| Reinforce 50 nodes, 1 transaction | 0.16 | 0.16 | 7.0 |
| Dreaming pass | 22 | **15** | 511 |
| Forget one subject | 2.3 | 2.2 | 13.8 |
| App size added | +1.87 MB (SQLite with FTS5, plus sqlite-vec) | same | +101.8 MB |
| Memory after open | 6.2 MB | 9.3 MB | 41.8 MB |
| Forget: 27 test phrases left on disk | **0** | **0** | **4**, left in its storage files after compaction |
| Crash (process killed mid-write) | 5/5 | not run (same SQLite file) | 5/5 |

- **Caveats:**
  - Small scale only: the 100,000- and million-link runs were stopped at the maintainer's request as not worth the cost.
  - Commits aren't comparable: SurrealKV flushes to disk on every commit, while our SQLite setting batches flushes.
  - LadybugDB and Grafeo didn't build.

**2. Jive's own SurrealDB 4.0.1 test** (reported to us 2026-11-25):
- About 20,000 nodes: 3-step walk p50 58 ms; vector search 9.6 ms; keyword search 27 ms; durable commits about 4 ms.
- Reported problems: shutdown hangs, a query bug, about 2× disk use, and a young 4.0 release.

**3. What others found on SQLite** (searched 2026-11-26):
- **sqlitegraph** (Rust) measured on one machine: an item lookup takes 4 µs from SQLite tables and 0.15 µs from its own adjacency store, and its in-memory cache took a busy node from 131 ns to 13 ns.
- **A vendor's third-party benchmark** (to treat cautiously): plain SQLite recursive queries take 6–28 ms at 3 steps and time out at 4–5 steps, against about 1 ms for engines that keep links in memory.
- **graphdblite** uses SQLite "strictly as a crash-safe key-value store", with its graph layer above it. That is this ADR's pattern.
- **SurrealDB's own blog:**
  - the 3.x release has no traversal benchmark, and "graph traversal optimisations" are future work;
  - its traversal post calls 2–3 steps comfortable with pruning;
  - its per-step limit keeps an **arbitrary** K in ID order, not the best-ranked K. Bot needs the best K at every step.

**4. Why a small engine beats a "dedicated" one here.**
- SurrealDB isn't a dedicated graph engine. It's a general database (documents, tables, graph, vectors) over key-value storage, with a query language to parse and plan on every call.
- Dedicated graph engines (Neo4j, Kuzu) are fast because each node's links sit directly in memory, and that is what we build.
- Our graph is small: thousands of nodes, at most about a million links per user. It lives in one process, and the operations are fixed and few. We use graph engines' central technique without paying for a general query layer.
- Research has long found relational systems competitive with dedicated graph engines when the graph part is done right (Grail, CIDR 2015).

## Decision

### 1. One engine: SQLite is the record, a typed graph in memory is the index

- **SQLite holds the truth.**
  - The world-model tables live in AI Juggler's existing database. That gives one file, one migration chain, one writer lock (ADR-0030) and one backup scrub (ADR-0016).
  - One transaction can write a memory record (ADR-0037) and its nodes and links together, and one forget reaches both.
  - Settings stay as they are: WAL, `synchronous=NORMAL`, `checkpoint_fullfsync` (AJ-E2026).
- **Memory holds a typed index**, rebuilt from SQLite at start and never written without SQLite first:
  - per node: kind, tier, priority, salience, confidence, valid-from and valid-to, last used, use count, archived;
  - links in **both directions**, grouped by link type and sorted by weight, so a hub is walked from either end;
  - each kind's rules and reasoning questions, held as compiled data.
- **Write-through, after commit.** A write commits to SQLite first, then the same change applies to the in-memory index. If the process dies between the two, the next start rebuilds the index from SQLite, so the two can never disagree after a restart. A test kills the process at every step and compares the rebuilt index with SQLite.
- **Readers never wait on the writer.** Reads see a consistent copy: the index is swapped atomically per committed batch, so a walk never sees half a batch.
- **Vectors stay on disk** in SQLite, as blobs with an exact cosine scan in Rust, not in memory. This needs no new dependency; the benchmark used sqlite-vec's exact scan, which works the same way. sqlite-vec, or an approximate index, comes in only behind the same trait, when a measured trigger fires and the maintainer approves the new dependency.
  - Exact search is fast enough up to about 100,000 vectors (0.2 ms at the benchmark's size).
  - An approximate index (HNSW) comes later behind the same trait, when a measured trigger fires.
  - The embedder is ADR-0037 §4.4's: on-device, never downloaded without the user's click (D-131).
- **Start-up is lazy.** The index builds in the background after the first paint. Estimate: 0.5–1 s at 1M links; 4 ms at the benchmark's 10,000. Until it's ready, Bot says it's warming up and uses ADR-0037's plain keyword retrieval, so the app never waits on it.

### 2. The data model

**Nodes:**
- Columns: `id`, `kind`, `key` (one canonical subject key per node, the unit of merging and forgetting), `label`, `props` (JSON), `tier`, `priority`, `salience`, `confidence`, `authority`, `untrusted`, `valid_from` and `valid_to`, `as_of`, `last_used`, `use_count`, `archived_at`.
- Node kinds in AI Juggler's pack:
  - `user`, `preference`, `behaviour`;
  - `project`, `task`;
  - `app`, `profile`, `account`, `session`, `pool`;
  - `memory` (a link to an ADR-0037 record), `fact`, `rule`, `question`;
  - `topic`, `conversation`, `turn_summary`.
- **Entity nodes are pointers, not copies.** A `session` or `app` node holds the entity's id and its links, never its state. State (running, stuck, tokens used, runway) is read live at assembly from the owning module (ADR-0037 §3: live facts win). The graph knows that agent11 is a session of the web App in the api project; the session's module says what it's doing now.

**Links:**
- Columns: `id`, `src`, `dst`, `kind`, `weight`, `confidence`, `props`, `valid_from` and `valid_to`, `provenance` (who or what wrote it, and from which turn or record).
- Links are typed: `about`, `part_of`, `runs_in`, `uses_account`, `prefers`, `asked_about`, `derived_from`, `contradicts`, `supersedes`, `governs`, `same_as`, and so on.
- Each link type is declared by the kind pack with its allowed source and target kinds. The write gate refuses anything undeclared.

**Kind specs: the per-node rules, as data.** Each kind declares:
- which link types it may have, and how many;
- **walk rules:** which links to follow from it, in which direction, how far, and the best K kept per step;
- decay half-life and tier thresholds;
- **reasoning questions**, short texts included with the node when it enters a context. Examples:
  - a `profile` node: "Is this the active profile right now?";
  - a `fact` node: "Is this still current? Check its source's live state.";
  - a `preference` node: "Does this apply to the current project?"

A `rule` node can override a kind's spec for one scope, through a `governs` link. That covers the user's own rules for Bot ("never message agents in the billing project"); its use for decisions is ADR-0037 §8.1's 0.6 `policy` kind.

### 3. The operations: a fixed set, no general query language

The crate exposes a small, typed API rather than Cypher or SQL. Each operation is bounded by design.

| Operation | What it does | Budget (p95, at 1M links, excluding the embedder) |
|---|---|---|
| `seed(query)` | Finds starting nodes: exact alias and entity matches first (the user's own names, like "agent11"); keyword search (FTS5) plus vector search, fused by reciprocal rank; at most K seeds | ≤ 5 ms |
| `expand(seeds, rules, budget)` | A best-first walk by the kind specs' rules. Each step keeps the **best** K by score (never an arbitrary K), with a total node cap and a depth cap (3 by default) | ≤ 5 ms |
| `matches(pattern)` | Small typed patterns ("sessions of the same project that share a memory, across Apps") | ≤ 5 ms |
| `as_of(t)` | Any read restricted to what was valid at time t | same as the read |
| `rank(nodes)` | Score = relevance × priority weight × 0.5^(age ÷ half-life) × path decay per step × link weight × confidence, one node per fact key (ADR-0037 §3) | ≤ 1 ms |
| `touch(nodes)` | Queued use counts, applied in the next writer hold, so assembly never writes (ADR-0037 §4.3) | ≤ 3 ms per batch |
| `dream(budget)` | Inside ADR-0037 §2.3's bounded idle batches: decay, demote, archive, merge duplicates, recompute salience | ≤ 200 ms per batch, stops when a job arrives |
| `forget(subject)` | Removes the node, its links, its index entries and its vectors with secure delete (§6) | ≤ 20 ms |
| `explain(context)` | Says why each node was chosen: seed, path, rule, score. It feeds ADR-0037 §6's receipts and Bot's context breakdown | — |

These budgets are tests: a benchmark suite runs at 10,000, 100,000 and 1,000,000 links in the crate's CI, and a regression past a budget fails the build.

### 4. Tiers, dreaming and the bounded top-K (extends ADR-0037 §2)

- **Four tiers.** Short, mid and long are ADR-0037's, and nodes and links use them too. **Immediate** is new: the working set of the current request, held only in memory and never stored.
- **Promotion and demotion** follow ADR-0037 §2.2's rules, applied to nodes as well. A node used across conversations and days rises; an idle one decays, then is archived. Archived nodes are kept, out of every walk, and a later use brings them back. They are deleted only when the user forgets them, or by ADR-0037's caps.
- **Dreaming** (the idle pass) merges duplicates:
  - **Same key:** merged automatically.
  - **Near-duplicates** (high vector similarity, the same neighbours): proposed as a `same_as` link, merged only when the rules agree. A model may suggest a merge only through ADR-0037 §2.4's confidence gate. Agent text is never a source.
  - Dreaming also folds turn summaries into topic summaries, recomputes salience and prunes weak links.
  - It writes in bounded batches with short holds and logs counts only.
- **Every read is bounded.** No operation returns more than its K. The assembler's budget picks K per job.

### 5. Context, rebuilt per request (extends ADR-0037 §5)

- The assembler gains a `world` section provider. For every turn:
  1. it seeds from the message;
  2. it expands by the rules;
  3. it ranks;
  4. it resolves live state for the entity nodes picked;
  5. it attaches their reasoning questions;
  6. it packs the result into the section's token share.
- The recent turns go in up to their own share. Older turns come back only as `turn_summary` nodes the walk picked, so **nothing is ever compacted:** each request gets a fresh context.
- **Topics are separate conversations.** Each topic chat has a `topic` node, and its context seeds from the topic's nodes as well as the message. The main Bot chat has no fixed topic and seeds from the message and the user's always-on preferences.
- **Bot shows its context:**
  - the model in use, the tokens used and the context size;
  - a hover breakdown by section (recent turns, world, memory, rules, live facts);
  - which nodes went in and why, from `explain`.

### 6. Forget and privacy (extends ADR-0037 §1.5 and §9)

- A forget removes the subject's nodes, links, FTS entries and vectors in one transaction, with `secure_delete` on, then rebuilds the FTS index and truncates the WAL. After the commit it removes the same subject from the in-memory index.
- The benchmark's check becomes a permanent test: plant 27 test phrases, forget them, then scan every file AI Juggler keeps. The result must be 0. It was 0 for SQLite and 4 for SurrealDB.
- ADR-0037's rules on copies, backups and restores apply unchanged, because the tables live in the same database.
- Nothing in the graph comes from another process's environment (D-37). Agents' text is always marked untrusted, never a source for a merge or a rule, and never an instruction.

### 7. Its own open-source crate

- **The crate is generic.** Nodes, links, kind specs, the operations, the SQLite store, the in-memory index, dreaming and forget. It doesn't know what a session or an App is.
- **AI Juggler owns its kind pack:** the kinds, link types, rules, reasoning questions and live resolvers (`features/ai/world/`). Other projects write their own.
- **Licence:** MIT OR Apache-2.0, the Rust standard and compatible with its one storage dependency, rusqlite, which AI Juggler already uses. The crate adds no new dependency.
- **Where it's built:**
  - Built from the first commit in the AI Juggler workspace as its own crate (`crates/gyrus`).
  - It has its own README, tests and benchmarks.
  - A lint forbids any `juggler-*` dependency.
- **When it goes public:** once its first version works inside Bot and its benchmarks pass, it is split with its history into its own public repository under figurehead-ai and published to crates.io. AI Juggler then depends on the published version. The public API comes from real use, not guesses. The maintainer chose the name `gyrus` (2026-11-26) and approved going public at this point.
- **Its README states its measured numbers and their limits.** It is marketed in line with the compare-page rules: sourced facts, no claims we haven't measured.

### 8. Windows and Linux (D-52)

Nothing here is macOS-only: it is SQLite and pure Rust. The on-device embedder is behind ADR-0037's OS seam.

## Alternatives considered

- **SurrealDB 4 (embedded, SurrealKV).** Not chosen, because:
  - it was slower on every operation: 6× on combined search, 40× on reinforcing nodes, and over 200× on walks and context building;
  - it adds about 100 MB to the app and 5× the memory;
  - its forget left deleted text on disk, with no secure delete, and "forget really erases" is a promise to users;
  - its per-step limit keeps an arbitrary K, not the best;
  - it is a young 4.0 release with problems Jive's own test reported.
  - Its strengths (a query language, multi-model, scaling across servers) are things Bot doesn't need.
- **Plain SQLite with recursive queries (ADR-0037 §4.4's plan).** Not chosen: 10–25× slower than the in-memory index on hub walks, sensitive to the query planner's choice (57 ms against 2.4 ms in the first probe), and no natural place for per-kind rules or a best-K per step.
- **LadybugDB or RyuGraph (Kuzu forks).** Not chosen: upstream Kuzu was archived in October 2025; the forks are young; it needs a C++ build that failed here on OpenSSL; it adds size; and it has no known secure delete.
- **mnestic (a CozoDB fork for agent memory, MPL-2.0).** Not chosen: Datalog is a general query language we'd work around; it has one maintainer and 8 releases this month; it reports 41 ms combined recall against our 15 ms budget; and it has no tiers or dreaming. We borrow its one-call fusion of graph, vector and keyword search, with spreading across topics.
- **graphdblite (MIT).** Same storage pattern as ours, with Cypher. Not chosen: no vectors, no time validity, no tiers, 3 stars, a single maintainer. We borrow its storage layout.
- **sqlitegraph:** GPL-3.0, so it can't ship inside our app. We borrow its adjacency cache and "as of" snapshots.
- **sqlite-graphrag:** a command-line tool, not a library. We borrow its typed links with strengths.
- **HelixDB:** AGPL, and a server.
- **Grafeo:** didn't build here (needs a newer toolchain); young.
- **A separate database file for the graph.** Not chosen: two files means two forgets, two backups and no single transaction with ADR-0037's records.

## Consequences

- **Easier:**
  - Bot's context builds in single-digit milliseconds, leaving the time budget to the model.
  - App size and memory stay small.
  - Forget is provable.
  - Every follow-on version builds on the same store, so no porting.
  - Others can use the crate.
- **Harder:**
  - We own the engine's code: about the operations in §3, plus the index and its rebuild.
  - There is no general query language: a new kind of question needs a new operation or pattern, written in Rust.
  - The in-memory index costs RAM. Estimate: 50–120 MB at 1M links; a few MB at today's sizes.
- **Risks, and how they're held:**
  - **The index and SQLite disagree:** write-through after commit, a rebuild at start, and the kill-at-every-step test.
  - **Scale beyond estimates:** the 100,000 and 1M benchmarks gate the crate's CI.
  - **A wrong ranking:** `explain` shows every choice, and ADR-0037 §10.2's harness judges recall on real questions with the real engine on the real Mac.

## Order of work (AJ-E3832, after acceptance)

1. **The crate's core:** nodes, links, kind specs, the SQLite store (the next migration in AI Juggler's chain), the in-memory index, write-through, rebuild, forget, and the kill-at-every-step and test-phrase tests.
2. **The operations:** `seed`, `expand`, `rank`, `matches`, `as_of`, `touch`, `explain`, plus the benchmark suite at 10,000, 100,000 and 1M links with its budgets.
3. **AI Juggler's kind pack:** the kinds, link types, rules, reasoning questions and live resolvers. ADR-0037's records are linked as `memory` nodes; existing records are linked by a one-time pass, and nothing moves.
4. **Assembly:** the `world` section, topics as separate chats, and Bot's context and memory breakdown.
5. **Dreaming and tiers:** merges, archive, salience, all inside ADR-0037's maintenance batches.
6. **A real-engine check:** fixed questions asked of Bot on the reference Mac, with the answers read and judged (`ai-features-tested-for-real`). Then the public split (§7).

## Resolved with the maintainer (2026-11-26)

1. **The name is `gyrus`:** the folds of the brain's cortex. It is free on crates.io, and no software project of that name is on GitHub. The other two names proposed were `cogmap` and `lociweave`.
2. **The repository goes public after the first version works inside Bot** (§7). It is not public from the first commit.
3. **D-150 is accepted as this ADR.** AJ-E3832 is built in 0.5.2, as D-150 placed it. Topics as separate chats and the context and memory breakdown (AJ-E3825 R9) are built on it, not on a store to port later.

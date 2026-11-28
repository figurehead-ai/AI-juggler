# gyrus

A typed graph kept in memory over SQLite, for an assistant's world model.

gyrus holds connected nodes (a person, their projects and tasks, the apps
and sessions they work in, memories, facts and rules) with typed links
between them, and answers a small fixed set of bounded questions: where to
start for this message, what is related by each kind's own rules, which
nodes matter most right now, and why each was chosen. It is meant for
building a small, exact context per request instead of dumping everything
into a prompt.

- **SQLite is the record.** The tables live in your database, applied as
  one of your own migrations (`gyrus::SCHEMA_V1`). Your own records and
  their nodes commit in one transaction.
- **A typed index lives in memory.** Links are kept in both directions,
  grouped by link kind and sorted by weight, so a hub is walked from either
  end. It is rebuilt from SQLite at start and changed only after a commit,
  so the two never disagree after a restart. Readers hold a consistent
  snapshot; a committed batch makes the next one.
- **Kind packs are data.** You declare node kinds, link kinds with their
  allowed ends, and per kind: which links a walk follows, which way, how far
  and the best K kept per step; its recency half-life and tier rules; and
  the short reasoning questions it brings into a context. Anything the pack
  doesn't declare is refused.
- **Forget really deletes.** With `secure_delete` on, a forget removes the
  node, its links, aliases, vector and keyword entry, rebuilds the keyword
  bucket that held it (so no index page separator keeps a prefix of the
  words), truncates the WAL, then removes the node from memory. A test
  plants 27 phrases, forgets them and scans the database file, the WAL,
  every keyword term and the in-memory index: 0 hits.
- **No new dependencies.** rusqlite (with its bundled SQLite and FTS5),
  serde_json and thiserror. Vectors are stored as blobs and compared by an
  exact cosine scan in Rust; no extension is loaded.

## Operations

| Operation | What it does |
|---|---|
| `ops::seed` | Starting nodes: your own matches and exact aliases first, then keyword (FTS5) and vector search fused by reciprocal rank; at most K |
| `ops::expand` | A best-first walk by the kinds' rules; each step keeps the **best** K by score (never an arbitrary K), with a node cap and a depth cap |
| `ops::rank` | The top K, one node per fact key |
| `ops::matches` | Small typed patterns ("sessions of the same project that share a memory, in different apps") |
| `Walk::at`, `Pattern::at`, `SeedQuery::at` | Any read restricted to what was valid at a time |
| `Graph::touch` | Uses queued in memory, applied in the next write, so a read never writes |
| `ops::explain` | Why each node was chosen: seed, path, rule, score, questions |
| `dream::plan` | The idle pass: decay, promote and demote between tiers, archive, merge duplicates, fold children into parents, prune weak links, in bounded batches |
| `store::forget_in`, `Graph::forget` | The forget above |

The score of a node reached from a seed is relevance × priority weight ×
0.5^(age ÷ half-life) × usefulness × confidence, times for each step the
rule's decay × the link's weight × its confidence.

## Use

```rust,ignore
let pack = Pack::builder()
    .kind(KindSpec::new("session").walk(WalkRule::out("part_of", 2, 4)))
    .kind(KindSpec::new("project"))
    .link(LinkSpec::new("part_of", &["session"], &["project"]))
    .build()?;
let graph = Graph::new(pack);
gyrus::schema::install(&conn)?;          // or apply SCHEMA_V1 as a migration
graph.build(&conn)?;                     // lazily, after start
let mut batch = Batch::new();
batch.node(NewNode::new("project", "project:api", Label::Trusted("api".into()), now))
     .node(NewNode::new("session", "session:web", Label::none(), now))
     .link(NewLink::new("session:web", "part_of", "project:api", "setup"));
graph.write(&conn, &batch, now)?;        // or store::stage in your own transaction, then publish
let snap = graph.snapshot()?;
let seeds = ops::seed(&conn, &snap, graph.pack(), &SeedQuery { text: "api", k: 8, ..Default::default() })?;
let reached = ops::expand(&snap, graph.pack(), &seeds, &Walk::new(64, now));
let top = ops::rank(&snap, &reached, 20);
let why = ops::explain(&snap, graph.pack(), &top);
```

## Budgets

Each operation has a p95 budget at 1,000,000 links (ADR-0038 §3), checked
by `tests/budgets.rs` in an optimised build at 10k, 100k and 1M links;
`cargo bench -p gyrus` prints the table. The numbers below were measured on
one Apple-silicon Mac with the benchmark's synthetic graph (15 node kinds,
24 link kinds, Zipf-like hubs, random weights, 3–6 word labels, 384-dim
vectors on up to 10,000 nodes). They are not a promise for other machines
or other graph shapes.

At 1M links (200,000 nodes) the measured p95s were: seed 3.9 ms (budget
5), expand 0.5 (5), as_of 0.09 (5), rank 0.001 (1), matches 0.05 (5), touch
0.2 (3), one idle-pass batch 3.4 (200), forget 10.4 (20; up to 19.3 on a
busier machine). The full table, the setup and what it took to
stay within budget are in [BUDGETS.md](BUDGETS.md).

## Limits

- One process, one writer: the host serializes writes (gyrus writes in the
  caller's transaction).
- The in-memory index costs RAM in proportion to the graph; a committed
  batch while a reader holds the previous snapshot copies the index once.
- Vector search is an exact scan of the stored vectors (packed 64 to a
  row): about 2 ms for 10,000 vectors of 384 dimensions without a memory
  map. Far more vectors need an approximate index, which is future work.
- Keyword search leaves out stop words (a word in more than 2 % of the
  labels) once there are more than about 12,800 labels.
- There is no query language: a new kind of question is a new operation or
  pattern, in Rust.

## Licence

MIT OR Apache-2.0, at your option.

# Measured budgets

Each operation's p50 and p95 in milliseconds, against its p95 budget
(ADR-0038 §3), from `cargo test -p gyrus --release --test budgets --
--include-ignored --nocapture --test-threads=1` on 2026-11-26.

**The machine:** one Apple-silicon Mac (30 cores), shared with other work
during the run (load average about 5). **The graph:** the benchmark's synthetic
one (`benches/workload.rs`): N links over N ÷ 5 nodes of 15 kinds and 24
link kinds, hubs drawn from a Zipf-like spread, random weights and
confidences, 3–6 word labels from a 2,000-word vocabulary, aliases on 1 % of
nodes, 384-dimension vectors on up to 10,000 nodes. SQLite in WAL mode with
`synchronous = NORMAL`, its default page cache, no memory map. **The
queries:** `seed` asks two random words plus an alias with a vector; `expand`
walks from a hub and two other nodes, depth 3, at most 200 nodes; `rank`
takes the top 50 of a walk; `matches` binds a 3-node pattern anchored on a
hub; `as_of` is `expand` at a past time; `touch` applies 50 queued uses;
`dream` plans one batch of at most 200 writes (three months on) and
writes it; `forget` removes one labelled node with its links and rebuilds its keyword
bucket, then truncates the WAL.

| Links | Nodes | Index built | Operation | p50 | p95 | Budget |
|---|---|---|---|---|---|---|
| 10,000 | 2,000 | 4 ms | seed | 0.69 | 0.87 | 5 |
| | | | expand | 0.04 | 0.07 | 5 |
| | | | as_of | 0.003 | 0.005 | 5 |
| | | | rank | 0.001 | 0.001 | 1 |
| | | | matches | 0.002 | 0.005 | 5 |
| | | | touch | 0.18 | 0.20 | 3 |
| | | | dream (one batch) | 0.82 | 0.89 | 200 |
| | | | forget | 0.99 | 1.19 | 20 |
| 100,000 | 20,000 | 51 ms | seed | 2.12 | 2.41 | 5 |
| | | | expand | 0.10 | 0.14 | 5 |
| | | | as_of | 0.012 | 0.017 | 5 |
| | | | rank | 0.001 | 0.001 | 1 |
| | | | matches | 0.018 | 0.029 | 5 |
| | | | touch | 0.21 | 0.24 | 3 |
| | | | dream (one batch) | 0.92 | 1.03 | 200 |
| | | | forget | 1.70 | 3.78 | 20 |
| 1,000,000 | 200,000 | 887 ms | seed | 2.83 | 3.90 | 5 |
| | | | expand | 0.38 | 0.49 | 5 |
| | | | as_of | 0.048 | 0.086 | 5 |
| | | | rank | 0.001 | 0.001 | 1 |
| | | | matches | 0.033 | 0.050 | 5 |
| | | | touch | 0.21 | 0.23 | 3 |
| | | | dream (one batch) | 3.13 | 3.44 | 200 |
| | | | forget | 9.22 | 10.36 | 20 |

Earlier runs the same day, with the machine busier (load 6–10), measured
the 1M-link seed at p95 4.2–4.5 ms and forget at p95 12.6–19.3 ms: forget
is the budget closest to its limit.

**What it took to stay within budget at 1M links,** and the limits that
follow:

- **Vectors are packed 64 to a row** and stored at length 1: a scan of
  10,000 384-dimension vectors read one row per vector at about 5 ms (SQLite
  reading the rows), 2 ms packed. The scan is still exact and exhaustive;
  with a memory map (`PRAGMA mmap_size`) it took 0.6 ms, which a host may
  choose.
- **Keyword search leaves out stop words:** a word in more than 2 % of the
  labels (counted in one bucket) ranks nothing and costs a score per label.
  Below about 12,800 labels every word counts.
- **The keyword index is split into 32 buckets** so a forget rebuilds one
  thirty-second of it (the rebuild clears FTS5's page separators); forget is
  the tightest budget at 1M links, and only a node whose label was ever
  indexed pays it.
- The index's build at 1M links took about 1 s (ADR-0038's estimate:
  0.5–1 s), which is why a host builds it lazily, in the background.

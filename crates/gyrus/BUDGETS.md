# Measured budgets

Each operation's p50 and p95 in milliseconds, against its p95 budget
(ADR-0038 §3), from `cargo test -p gyrus --release --test budgets --
--include-ignored --nocapture --test-threads=1` on 2026-11-26.

**The machine:** one Apple-silicon Mac (30 cores), shared with other work
during the run (load average 6–10). **The graph:** the benchmark's synthetic
one (`benches/workload.rs`): N links over N ÷ 5 nodes of 15 kinds and 24
link kinds, hubs drawn from a Zipf-like spread, random weights and
confidences, 3–6 word labels from a 2,000-word vocabulary, aliases on 1 % of
nodes, 384-dimension vectors on up to 10,000 nodes. SQLite in WAL mode with
`synchronous = NORMAL`, its default page cache, no memory map. **The
queries:** `seed` asks two random words plus an alias with a vector; `expand`
walks from a hub and two other nodes, depth 3, at most 200 nodes; `rank`
takes the top 50 of a walk; `matches` binds a 3-node pattern anchored on a
hub; `as_of` is `expand` at a past time; `touch` applies 50 queued uses;
`forget` removes one labelled node with its links and rebuilds its keyword
bucket, then truncates the WAL.

| Links | Nodes | Index built | Operation | p50 | p95 | Budget |
|---|---|---|---|---|---|---|
| 10,000 | 2,000 | 4 ms | seed | 0.85 | 1.36 | 5 |
| | | | expand | 0.05 | 0.08 | 5 |
| | | | as_of | 0.003 | 0.005 | 5 |
| | | | rank | 0.001 | 0.001 | 1 |
| | | | matches | 0.002 | 0.006 | 5 |
| | | | touch | 0.20 | 0.32 | 3 |
| | | | forget | 1.22 | 1.55 | 20 |
| 100,000 | 20,000 | 57 ms | seed | 2.57 | 3.14 | 5 |
| | | | expand | 0.11 | 0.16 | 5 |
| | | | as_of | 0.013 | 0.035 | 5 |
| | | | rank | 0.001 | 0.001 | 1 |
| | | | matches | 0.020 | 0.032 | 5 |
| | | | touch | 0.22 | 0.39 | 3 |
| | | | forget | 2.04 | 2.75 | 20 |
| 1,000,000 | 200,000 | 1,037 ms | seed | 3.25 | 4.48 | 5 |
| | | | expand | 0.49 | 0.81 | 5 |
| | | | as_of | 0.06 | 0.13 | 5 |
| | | | rank | 0.001 | 0.002 | 1 |
| | | | matches | 0.037 | 0.069 | 5 |
| | | | touch | 0.23 | 0.39 | 3 |
| | | | forget | 11.9 | 19.3 | 20 |

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
- The dreaming budget (≤ 200 ms per batch) is measured with the idle pass.

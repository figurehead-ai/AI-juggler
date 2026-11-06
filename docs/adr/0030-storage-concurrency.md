# ADR-0030: Storage concurrency: one writer lock with short holds; a reader pool only when a measured trigger fires

**Status:** Accepted
**Date:** 2026-11-06
**Deciders:** the 0.4 completion session (AJ-E2017 R3, an engineering ruling)

## Context

AJ-E2017 asked for a decision between per-repository connections and a pool, instead of one `Mutex<Storage>`.

### What there is today

- `AppState::storage` is a `Mutex<Storage>`: one SQLite connection, in WAL mode, with a busy timeout. 525 call sites take it (`lock_storage_arc` or `storage.lock()`); after AJ-E2017's split they're spread over the features, with the most in `manage.rs` (16), `history.rs` (13), `link.rs` (12) and `features/pools/model.rs` (10).
- Five repositories already have connections of their own, outside that mutex, because each had a measured reason:
  - the process ledger (the supervisor writes it on spawn and exit);
  - the cleanup audit (intent before a kill, ≤ 5 ms p95 in front of the ladder);
  - the cleanup settings;
  - the relations reader;
  - the transcript index (the conversation index's long scans).
- Commands run off the main thread (`ipc::blocking`), so a held lock never freezes the window; it delays the command or tick that waits.

### What was measured

`crates/juggler-app/tests/storage_contention.rs` (ignored by default; run with `--ignored --nocapture`) seeds 10 accounts, 50 working sessions and 2,000 ended ones. It measures how long each hot path holds the lock and how long a page's read waits for it, with the background ticks running at 10× their real rate. Figures are from a debug build on the shared development Mac, 2026-11-06; a release build is several times faster, but the shape holds.

| Path (holds the one lock) | p50 | p95 | max |
|---|---|---|---|
| The 15 s heuristics tick: reads **every** session ever recorded | 18.5 ms | 19.5 ms | 19.9 ms |
| The working set (the 5 s resources and 30 s quota ticks) | 0.9 ms | 1.6 ms | 1.6 ms |
| The sessions list's read model (each page open, each change) | 24.9 ms | 26.8 ms | 32.2 ms |
| One session's status write | 0.03 ms | 0.04 ms | 0.15 ms |
| The projects list (whole join under the lock) | 43.2 ms | 56.3 ms | 58.7 ms |

| A page's read waits for the lock | p50 | p95 | max |
|---|---|---|---|
| Nothing else running | 1.7 µs | 3 µs | 10 µs |
| The ticks running at 10× | 1.6 µs | 2.6 µs | 45 ms |
| On its own read-only connection, the ticks running (wait **and** read) | 22.9 ms | 25.6 ms | 26.5 ms |

What the figures say:

1. **Contention is rare.** Even at ten times the real tick rate, the lock is busy about 2 to 3% of the time. A page's p95 wait is microseconds. The worst case is one other hold, 45 ms here.
2. **What costs is the length of a hold, not how many threads wait.** The long holds are reads that do their whole job under the lock:
   - the projects list's join (56 ms p95);
   - the read model (27 ms);
   - the heuristics tick (20 ms). It also grows with the whole history (`sessions().list(None)`, 2,050 rows here), and it reads each live transcript's file time from disk while holding the lock.
3. **A second connection doesn't make a read faster.** On its own connection the page's read took 25.6 ms p95, the same as the read itself under the shared lock. It only removes the rare wait behind another long hold. A pool trades that rare wait for more connections, more memory and WAL checkpoint pressure. It also brings a consistency question: a command that writes then reads must read on the writer, or it may not see its own write.

## Decision

1. **The writer stays one `Mutex<Storage>`.** SQLite has one writer at a time anyway; a mutex in front of it is the simplest correct queue. Nothing in the measurements calls for more today.
2. **Holds are short, by rule.** A lock is held for the rows, not for the work:
   - read under the lock, drop it, then join, sort, format or touch the filesystem;
   - never read the whole history on a tick; read the working set (`working_records`) or a page;
   - the budget is **one hold ≤ 16 ms p95 in a release build** at the promised scale (one frame). The three long holds above are the first to fix, in this order: the heuristics tick (working set only, file times read after the lock), the projects list (rows under the lock, the join after), the read model (the same).
3. **Per-repository connections stay the pattern for a repository with its own reason.** A repository gets its own connection when it has a hot writer of its own, or a long read the app's paths must never wait behind (the ledger, the audit, the transcript index, as today). It is opened through `juggler-core` (defensive mode, busy timeout, WAL), held by the feature's own state, and never locked together with `storage`.
4. **A read pool comes when a trigger fires, not before.** It's a small pool of read-only connections (`query_only`, two to start), behind `AppState::read()`; page reads move to it and writes stay on the mutex. Any one of these triggers it:
   - a page's wait for the lock over 16 ms p95 in a release build at the promised scale, after the holds are short;
   - a background writer with long transactions (E16's cross-machine sync or a bigger import);
   - the scale target growing past 50 working sessions.
   `storage_contention.rs` is the measurement for each.

## Alternatives considered

- **A pool for everything now** (r2d2 or a hand-rolled one): the measurement shows no waiting worth removing, and the long holds would stay long on any connection. It adds a write-then-read consistency rule to every command, for no measured gain.
- **A connection per repository for all of them**: one connection each for the store's dozens of repositories, and transactions that span repositories (a switch writes sessions, edges and the graph together) would lose their atomicity or need a coordinator.
- **An async mutex or an actor thread owning the connection**: it changes every call site's shape (525) and still serialises the same work; commands are already off the main thread.

## Consequences

- No call site changes now. The rule in Decision 2 is what reviews check; the three long holds are named follow-ups (AJ-E2017's manifest lists them).
- `storage_contention.rs` stays in the tree, so the trigger is checked with the same seed and the same paths each time it's asked.
- When the pool comes, it lands behind one accessor, and only reads move. A feature whose read must see its own write keeps using the writer.

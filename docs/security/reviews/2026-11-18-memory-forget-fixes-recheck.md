# Security re-review: the fixes to the memory store's forget path (ADR-0037, AJ-E3819 step 1)

- **Reviewer:** an independent Opus session. It did not write the fixes and did not write the first review. It assumed nothing was fixed until the code and a test (or its own experiment) showed it.
- **Date:** 2026-11-18. **Branch:** `release/0.5` at `dec4c24da`.
- **Checks:** the first review `docs/security/review-memory-forget-step1.md` (f9137b6b1); the fixes bebace3b4 (store) and 01d8e83ce (backup); ADR-0037 §1.5 and §1.8 as changed by 32a3c634d; `crates/juggler-core/src/storage/memory/`; `crates/juggler-app/src/backup.rs`, `cloud_backup.rs` and `newer_data.rs`.
- **SQLite:** bundled by `libsqlite3-sys` 0.39.1, version 3.54.1, as in the first review.

## Verdicts

| Finding | Verdict | In one line |
|---|---|---|
| **H1** index separators | **PARTIAL** | Backups and the cloud copy are fixed. The live forget and a restore's tombstone pass are not: `optimize` does nothing on an index of one segment (N1). |
| **H2** turns and titles | **CLOSED** (with a condition) | Turns are redacted and titles renamed, and the test fails without either. This depends on step 2 setting `title_turn` and calling `link_turn` (N4). |
| **H3** before-restore copy | **PARTIAL** | The code is right on every exit except a crash. The named test passes with the scrub removed (its assertion is vacuous). The end-to-end test is ignored. |
| **M1** restore not atomic | **CLOSED** | Tombstones are applied to the staged copy first, and the test fails without that. H1's separator gap still applies on this path (N1). |
| **M2** self-declared authority | **PARTIAL** | Writers and `RecordText` are typed, and the gate test fails without them. The cloud filter's `untrusted`/source checks are untested, and `Writers::issue` is not taken at start yet. |
| **M3** free text as preferences | **PARTIAL** | The gate keeps a preference to 60 characters with an entity, and that is tested. It is still free text, not a typed form. The cloud-side shape filter is untested. |
| **L1** blocked WAL fold not retried | **OPEN** | It is deferred to step 3. There is no retry and no `journal_size_limit`. |
| **L2** `after_restore` WAL fold | **CLOSED** (by code) | `fold_wal` is called, but no test fails without it. |
| **L3** `REPLACE` skips the FTS delete | **CLOSED** (by code and experiment) | `recursive_triggers` is on in the hold. My experiment shows it works, but no test fails without it. |
| **L4** sidecars without a subject | **CLOSED** (by code) | The by-turn clause is tested. The by-version clause works in my experiment but is untested. |
| **L5** staging left behind | **PARTIAL** | The restore's exits are covered. A verify with no restore, or a crash, still leaves the staged backup. There is no sweep at start. |
| **L6** cascade depends on a compile flag | **CLOSED** | `foreign_keys` and explicit deletes are both used. The test fails only when both are removed. |

The author's **accepted residuals:**

- **"Old exports can't be scrubbed, so a fixed notice says so."** True in code: nothing touches an exported file. But the notice (`features::ai::memory::FORGET_AND_BACKUPS`) is a constant that no UI, IPC type or page shows yet. The forget sheet is step 4, and the backup page doesn't carry it. It must be wired, with a test that it is shown, before V54 registers.
- **"AI Juggler's own pre-update copies hold no memory."** True: `Purpose::StaysOnThisMac` maps to `OwnCopy`, which empties memory, and two tests cover it. The broader claim in ADR §1.5 ("every copy AI Juggler keeps … keeps no memory at all") is **not** true: see N2 and N3.

## How the re-review was checked

Experiments and mutations ran in a scratch copy (`git archive HEAD`) outside the repo. Each mutation edited one hunk, ran the tests, then restored the file. The worktree was never edited, and `git status` for the reviewed paths is unchanged. On the worktree itself, `cargo test -p juggler-core --lib memory` passes all 13 tests.

| # | Mutation (one fix removed) | Result |
|---|---|---|
| MUT-1 | `optimize_index` removed from `forget` | `forget_clears_the_index_page_separators` **fails**. `forget_leaves_no_trace_in_the_file` still passes, because its canary never starts a page. |
| MUT-2 | `reindex` removed from `scrub_snapshot` | `the_cloud_copy_holds_no_local_only_or_forgotten_word` **fails** |
| MUT-3 | turn redaction removed from `erase_subject` | `forget_leaves_no_trace_in_the_file` **fails** |
| MUT-4 | title rename removed | the same test **fails** |
| MUT-5a | L4's by-version sidecar clause removed | **every test passes** (gap) |
| MUT-5b | the by-turn sidecar clause removed | `forget_leaves_no_trace_in_the_file` **fails** |
| MUT-6 | `recursive_triggers` removed from `SecureHold` | **every test passes** (gap) |
| MUT-7 | `fold_wal` removed from `after_restore` | **every test passes** (gap) |
| MUT-8 | cloud filter put back to `kind IN (…) AND authority = 'user'` | **every test passes** (gap: M2's `untrusted`/source checks and M3's length/entity checks are unproven at the cloud layer) |
| MUT-9 | gate's `PreferenceShape` removed | `the_write_gate_refuses_and_logs_without_the_text` **fails** |
| MUT-10 | `covers` ignores authority and source | the same test **fails** |
| MUT-11 | `scrub_copy` with `foreign_keys = OFF`, then without the explicit `chat_turns` delete, then both | passes, passes, **fails** (either guard is enough) |
| APP-1 | `apply_tombstones` made a no-op | `a_restore_drops_the_forgotten_and_keeps_no_copy_holding_it` **fails** |
| APP-2 | `ScrubOnDrop` made a no-op | **the same test passes** (gap: the kept copy is a snapshot taken *after* the forget, so it holds no record whether or not it is scrubbed) |

| # | Experiment | Result |
|---|---|---|
| RX1 | the author's test shape (4,000 rows), forget a page-start word, then **forget another with no write between** | First forget: 8 segments become 1, and the separator is gone. **Second forget: the index is 1 segment, and the separator stays in `memory_fts_idx` and in the raw file** (`diagnosisword01853`). |
| RX2 | 4,000 rows, `optimize` first (one segment), then forget a page-start word | **the separator stays** in the index and the raw bytes |
| RX3 | a Local-scrubbed export (fresh index, one segment); this Mac forgets a page-start subject; `after_restore` on the export with the live database as `prior` | the subject is dropped, but **its separator stays** in the index and in the file that then replaces the live database |
| RX1–3 + fix | `optimize_index` replaced with `reindex` (delete-all, then re-insert current trusted rows) | all three are clean, and the whole memory suite passes |
| RX4 | `INSERT OR REPLACE` over a record's `id` inside a `SecureHold` | the old term is gone from the index (L3 works) |
| RX5 | a sidecar on an event naming a version id only (no `memory_subject`) | erased by forget (L4's by-version clause works) |
| RX6 | a conversation titled from the remembering turn with `title_turn` NULL | the turn is redacted, but **the title keeps the forgotten words** |
| RX7 | `scrub_copy` on a 3,000-row index | no word left in the raw bytes |

**Raw bytes:** `forget_leaves_no_trace_in_the_file`, `forget_clears_the_index_page_separators`, `the_cloud_copy_holds_no_local_only_or_forgotten_word` and `a_pre_migration_copy_holds_no_memory` read the file plus `-wal`, `-shm` and `-journal`. The app test's `holds()` reads the file, `-wal` and `-shm` for the staged copy. **The before-restore copy is checked only by SQL (`COUNT(*)`), not by its bytes** (and see APP-2).

**The end-to-end test** (`tests/backup_restore.rs`, `a_restore_brings_back_nothing_forgotten_and_no_kept_copy_holds_it`) was read but not run. It cannot pass before V54 is registered: `foreign::vet` compares the backup's schema with a fresh database's and refuses "a table AI Juggler doesn't have" (`memory_records`). It is the only test that covers the wiring in `backup_restore_impl` (the order of snapshot, `ScrubOnDrop`, `apply_tombstones` and `restore_from`, and `RemoveOnDrop`). The unit test calls each helper directly.

## Findings

### H1, still partial: `optimize` does nothing on a one-segment index (N1, high until fixed)

- **Where:** `erase.rs:82` (`optimize_index`); `store.rs:486` (`forget`); `backup.rs:218` (`after_restore`).
- **Failure:** FTS5's `fts5IndexOptimizeStruct` (`sqlite3.c:258135` in the bundled 3.54.1) returns early when the index is one segment with no tombstone pages. `secure-delete` never creates tombstone pages, and a delete that removes every entry adds no new segment. So `optimize` rewrites only when two or more segments exist. That leaves the same separator leak the first review found (H1) in three common cases:
  1. **A second forget with no write since the first** (RX1). The first forget's `optimize` leaves one segment. A user who cleans out several memories in one sitting hits this every time.
  2. **Any store whose index is one segment** (RX2), for example after an automerge or any earlier forget.
  3. **A restore of an older export** (RX3). Every export's scrub now builds a fresh index, which is one segment. The tombstone pass then drops the subject but keeps its separator, and that file becomes the live database.
- **Why the test misses it:** `forget_clears_the_index_page_separators` bulk-loads 4,000 rows, which gives 8 segments, and forgets one separator. The first forget always merges.
- **Fix:** in `forget` and `after_restore`, call `reindex` (delete-all, then re-insert the current trusted rows) instead of `optimize`. RX1 to RX3 then come out clean, and the suite still passes. The index is capped at 20 MB, so the cost is bounded. Add tests for two forgets in a row, a forget after an explicit `optimize`, and `after_restore` on a Local-scrubbed export, each asserting `memory_fts_idx` and the raw bytes.
- **Closed parts:** every backup scrub (Local, OwnCopy, Cloud) ends with `reindex` (MUT-2 fails without it). The cloud copy is a filtered export into an empty index.

### N2 (medium): the going-back copies hold all memory and Chat history, are never scrubbed or pruned, and are outside forget's reach

- **Where:** `crates/juggler-app/src/newer_data.rs:332` (`juggler.sqlite.from-v<N>-<ms>`, a full `snapshot_to` of the live database before going back) and `:190` (`juggler.sqlite.from-newer-<ms>`, the newer database moved aside by an older build, "never deleted").
- **Failure:** ADR-0037 §1.5 now says "the same deletion reaches every copy AI Juggler keeps, or the copy keeps no memory at all". These two copies hold every memory, turn and sidecar. Nothing scrubs or prunes them. After going back, the live data no longer holds those subjects, so the user can never forget them from the app. Once V54 is registered, going back from 0.5 to 0.4 creates `from-v54-…` this way.
- **Fix (author's call; log it as a D-number):**
  - scrub memory from `from-v<N>` as the before-restore copy is (the safe direction); or
  - name both in ADR §1.5 as the way back, say on the going-back sheet that they hold memory and Chat history, and offer to delete them.

  The `from-newer` copy is written by an older build that knows nothing of memory. It can only be named and swept by the newer build when it next starts.

### N3 (low): a crash or quit mid-backup or mid-restore leaves unscrubbed full copies in the data folder, with no sweep at start

- **Where:** `backup.rs:396` (`backup-work-<ms>/juggler.sqlite`, an unscrubbed `VACUUM INTO` until `scrub` runs; `cloud_backup.rs:190` builds through it on a schedule); `backup.rs:688–730` (the before-restore copy is unscrubbed until `ScrubOnDrop` runs at the end); `backup_verify_impl` (`restore-staging` stays when a verified restore is never confirmed).
- **Failure:** `Drop` guards don't run on a crash, a kill or a power cut. `prune_restore_copies` runs at start, but only deletes a copy older than a day, and it never scrubs. The app is meant to stay on for weeks, so a crashed restore's full copy can sit for weeks. The `backup-work-*` and `cloud-backup-work-*` folders are never swept.
- **Fix:** one sweep at start that removes `backup-work-*`, `cloud-backup-work-*` and `restore-staging`, and runs `scrub_copy` on every `juggler.sqlite.before-restore-*` before the age check. This also closes L5's remaining case.

### N4 (low): a title whose `title_turn` is NULL is never renamed, and nothing pins step 2 to setting it

- **Where:** `schema.rs` (`title_turn TEXT`, nullable); `erase.rs:63`. RX6.
- **Failure:** forget renames a title only when `title_turn` names a redacted turn. A conversation written without `title_turn` keeps the forgotten words in its title. Receipts reach forget only if step 2 calls `link_turn` for every turn that used a memory. Both are contracts on code that doesn't exist yet.
- **Fix:** make `title_turn` `NOT NULL` once Chat writes conversations, or fall back to renaming when the title equals a prefix of a redacted turn's question. Add a step-2 test that a turn using a recalled memory is linked.

### N5 (low): a forget that commits during a restore is lost and its subject comes back

- **Where:** `backup.rs:688–717`. The storage lock is released between `snapshot_to(&before_db)` and `restore_from`.
- **Failure:** a forget that commits in that window writes its tombstone to the live database only. `apply_tombstones` and `keep_this_macs_rows` read `before_db`, so the restore resurrects the subject and drops the tombstone. No forget surface exists yet, so this can't happen today.
- **Fix:** hold the storage lock from the snapshot to the end of `keep_this_macs_rows`, or apply the live database's tombstones (attach it, not the snapshot) right before `restore_from`.

### Test gaps to close (no new risk, but the fixes are unproven)

- APP-2: `ScrubOnDrop`'s test copy must hold memory before the drop (snapshot *before* the forget, or with a kept note), and it must check raw bytes.
- MUT-5a, MUT-6, MUT-7 and MUT-8 pass with their fixes removed. Add a by-version sidecar, a `REPLACE` inside the hold, a WAL check after `after_restore`, and cloud-filter rows that are a user preference marked untrusted, a free-text preference written by direct SQL, and a NoteJob-sourced preference.

### Info

1. **Text in logs.** No `tracing`, `println!` or `eprintln!` in `storage/memory/`. Refusals log codes only (`log_refusal`). `ScrubOnDrop` logs a SQLite error, not text. `NewRecord`, `Record` and `RecordText` derive `Debug`, so any future `?record` in a trace prints the words. Consider a redacting `Debug` for `RecordText`.
2. **A backup's `created_at_ms` is stamped after the scrub, not at the snapshot.** A forget that commits during a long export is in a file dated after it. The notice says "every backup made after you forget". Stamp it at the snapshot.
3. **`code_like` still allows commas** (the first review's Info 1). Unchanged.
4. **`Writers::issue` has no production caller.** Until the composition root takes it at start, "issued once" guards nothing: the first caller anywhere gets every writer.

## Cleanup

- Listed with `find`, then deleted:
  - the scratch copy `…/scratchpad/recheck` (the `git archive` tree with the RX tests);
  - the mutation fragments `…/scratchpad/m`;
  - the helper scripts `mutate.py` and `runmut.sh`;
  - the `.orig` backups.
- **Shared target:** the scratch copy built into `AIJuggler-shared-target/r05` under the same artifact hashes as this worktree (cargo fingerprints path packages relative to the package root). A later worktree run reused the scratch-built `juggler-core` test binary. I removed the two fingerprints whose content differed from the worktree: `juggler-core-d78d4b0e73291c5a` (test binary, last built with a mutation) and `juggler-app-bf7f3fd1a90e9e87` (lib test binary, last built with APP-2 and without the worktree's uncommitted `juggler-app` changes). I then rebuilt `juggler-core`'s test binary from the worktree. Its 13 tests pass. The other workspace crates rebuilt from the scratch copy are byte-for-byte HEAD, which the worktree matches for those crates. **Lesson:** a scratch copy needs its own `CARGO_TARGET_DIR`.
- Kept: this file only.

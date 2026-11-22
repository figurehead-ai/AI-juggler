# Security review: the memory store's forget and secure-delete path (ADR-0037, AJ-E3819 step 1)

- **Reviewer:** an independent Opus session, which did not write this code. It was asked to assume the code has bugs and not to rely on the author's self-review (`storage/memory/mod.rs` §"Security self-review").
- **Date:** 2026-11-18. **Branch:** `release/0.5` at `4bad21f8a`.
- **Scope:** `crates/juggler-core/src/storage/memory/` (`schema.rs`, `store.rs`, `secure.rs`, `log.rs`, `backup.rs`, `retrieval.rs`, `types.rs`, `tests.rs`); the pre-migration copy in `storage/mod.rs`; the app's backup, cloud backup and restore in `crates/juggler-app/src/backup.rs` and `cloud_backup.rs`; ADR-0037 §1.2–§1.8, §7 and §9.
- **Commits:** f6d9f03a2 and 35f07345b, then 8b101e8474, 9c85482914, ec27161385 and 96c849cf2 (rewritten from 49f21cde3, ebcf9ccea, 037c54386 and 58d430a7c with no change to their content).
- **SQLite:** bundled by `libsqlite3-sys` 0.39.1, version 3.54.1, built with `SQLITE_DEFAULT_FOREIGN_KEYS=1` and `SQLITE_ENABLE_FTS5`. FTS5's `secure-delete` option needs 3.42 or later, so it is supported.

## Summary

| Severity | Count |
|---|---|
| Blocker | 0 |
| High | 3 |
| Medium | 3 |
| Low | 6 |
| Info | 4 |

None of these blocks step 1 as merged, because V54 is not in `MIGRATIONS` yet: every hook does nothing on a real database. **H1 to H3 must be fixed before V54 is registered** (the 0.4.6 to 0.5 merge), and H2 before Chat writes `chat_turns` (step 2).

## How the review was checked

All experiments ran in a scratch copy made with `git archive HEAD`, outside the repo. The scratch copy has been deleted (see Cleanup below). Builds were offline and used the shared target.

| Experiment | Result |
|---|---|
| Baseline: every `storage::memory` test | passes |
| **M1** Remove `INSERT INTO memory_fts(memory_fts, rank) VALUES ('secure-delete', 1)` | `forget_leaves_no_trace_in_the_file` **fails** ("the forgotten words are still in the file") |
| **M2** Remove the `SecureHold` in `forget` | the same test **fails** |
| **M3** Remove `fold_wal` after `forget` | the same test **fails** |
| **E5** 300 notes of about 250 bytes each, `touch` three times so the rows grow and move between pages, then forget one | no stale copy found. B-tree rebalancing did not leave the words in page gaps in this run |
| **E6** 4,000 index rows in one segment (a multi-page segment), then forget the row whose word starts a leaf page | **the word's prefix (`diagnosisword00454` of `diagnosisword00454q`) stays in `memory_fts_idx` and in the raw file bytes.** `MATCH` finds nothing, so the existing test cannot see it (H1) |
| **E6c** The same, then `INSERT INTO memory_fts(memory_fts) VALUES ('optimize')` inside a `SecureHold`, then a checkpoint | the separator is gone from the index and from the raw bytes (H1's fix works) |
| **E6d** A mixed store (1 in 50 rows a user `preference`), cloud scrub, then `VACUUM` | **all 12 `memory_fts_idx` separators in the cloud copy name scrubbed notes**, and they are in the raw bytes of the file that is uploaded (H1) |
| **E7** A second connection holds a read open during a forget | `wal_folded = false`. After later writes and a passive checkpoint, the canary was no longer in the WAL in this run (L1) |
| **E9** `INSERT OR REPLACE` over a record's `id` | the old row's term **stays in the index**. The delete trigger does not fire because `recursive_triggers` is off (L3) |
| **E10** `KindWriter::for_kinds(&[Preference])`, authority `User`, agent-like text, `source = DigestJob` | accepted, indexed, and **kept in the cloud copy** (M2) |

## Findings

### H1 (high): FTS5 secure-delete leaves page-separator terms in `memory_fts_idx`, and they reach the cloud backup

- **Where:** `schema.rs:76-80` (the index, with `secure-delete`); `store.rs:434-477` (`forget`); `backup.rs:68-92` (the cloud scrub); `crates/juggler-app/src/backup.rs:226-230` (the `VACUUM` that copies the index as it stands).
- **Failure:** FTS5 keeps one `%_idx` row for each leaf page of a segment, holding the shortest prefix of the page's first term that sorts after the previous page. `secure-delete` removes a deleted row's entries from the leaf pages, but it does not rewrite these separators. Any word that started a leaf page therefore survives, nearly whole, after the forget. This was reproduced in E6 (18 of 19 characters, present in the raw file bytes).
  - The store writes each record in its own transaction, and automerge folds those small segments into multi-page segments as the store grows (the cap is 20 MB). So this happens in normal use, not only in a contrived test.
  - The cloud scrub deletes notes, `asked` rows and answers through the same triggers, and `VACUUM` copies `memory_fts_idx` unchanged. In E6d every separator in the uploaded copy named a scrubbed note. That breaks §1.8 and D-121 ("Chat's content never leaves the Mac"). The vault's encryption reduces the exposure but does not repair the rule.
- **Why the test misses it:** `forget_leaves_no_trace_in_the_file` writes 31 short notes, so the index is a single leaf with no `%_idx` term. It also checks `fts5vocab`, which reads the leaves and not the separators.
- **Fix:**
  1. **Cloud and local scrubs:** after the deletes in `scrub_snapshot`, run `INSERT INTO memory_fts(memory_fts) VALUES ('delete-all')`, then re-index only the current trusted rows with `INSERT INTO memory_fts(rowid, text) SELECT seq, text FROM memory_records WHERE superseded_by IS NULL AND untrusted = 0`. This avoids `rebuild`, which would also index superseded rows.
  2. **Live forget:** run `'optimize'` inside the `SecureHold` before `fold_wal`. E6c shows this clears the separators. The index is small, so the cost is low. If it proves expensive, maintenance can run it in batches, but forget must report it.
  3. **Test:** extend the raw-bytes test with a multi-page index (thousands of rows, or `pgsz` set low with `INSERT INTO memory_fts(memory_fts, rank) VALUES ('pgsz', 64)` in the test) and assert on the `memory_fts_idx` contents as well as the bytes. Add the same check to `backup_inclusion_follows_the_rule` for the cloud copy.

### H2 (high): forget never reaches the conversation that holds the same words

- **Where:** `store.rs:434-477` (forget deletes only `memory_records`, the entities through cascade, and `ai_log_text`); ADR-0037 §1.5 step 4 removes turns only when the user picks "Forget this conversation".
- **Failure:** a note is "the user's own words after 'remember…'". Those words are in `chat_turns.question` and in the app's echo in `chat_turns.shown` ("I'll remember: “…”"), and often in `chat_conversations.title` (the first question). The same holds for an `asked` row, which is the question itself.
  - After **Forget**, the words remain verbatim in Chat history.
  - They also go into every later local backup and every later copy-before-update, and they come back on every restore. `after_restore` drops records only, never turns.
  - §10.1's `forget_really_deletes` promises that the canary is gone from "a turn" after Forget, but no code path can do that: `forget` takes no conversation option and never reads `source_id`.
- **Impact:** a user who forgets a sensitive memory still has it on screen in History, and in backups. This defeats the "Forget really deletes" promise in the plainest way.
- **Fix:** forget a subject together with its sources. For every deleted version with `source_kind = 'chat_turn'`, delete the turn `source_id` names, or at least mask the remembered span in `question` and `shown`, in the same transaction and inside the `SecureHold`. Recompute the conversation's `title` when its first turn is touched. Add the turn canary to the raw-bytes test before step 2 writes any turns. If deleting turns is ruled too destructive, the forget sheet must say so plainly. That is a product decision (a D-number), not a silent gap.

### H3 (high): the before-restore copy is a full, unscrubbed database that forget can never reach and nothing deletes

- **Where:** `crates/juggler-app/src/backup.rs:682-684`. `juggler.sqlite.before-restore-<stamp>` is written with `VACUUM INTO` in the data folder.
- **Failure:** every restore leaves a complete copy of the live database: every memory, every conversation, every sidecar. Nothing prunes these copies (grep finds no cleanup), and no scrub runs on them.
  - Anything the user forgets after a restore is still in that file indefinitely.
  - ADR §1.5 says "the same deletion reaches every copy AI Juggler keeps" and names only the pre-migration copies and backups. This copy was missed.
  - Unlike a backup the user exported, the user never chose to make this one, so the backup page's wording ("Backups you made before still hold them") does not cover it.
- **Fix (one of):**
  - Run `scrub_snapshot(Local)` on it, so forgotten subjects and tombstones go. This still leaves later forgets unreached.
  - Better: give it a short life, deleted after the next successful start or after N days, and list it on the backup page as a kept copy.
  - Better still: the forget path also deletes from every `*.before-restore-*` copy, by `subject_id`, inside a `SecureHold`.
- **Also:** if the restore fails between `restore_from` and `remove_dir_all` (`backup.rs:705-718`, the `?` exits), the staging copy of the backup stays in `data_dir/STAGING_DIR`, unscrubbed (L5).

### M1 (medium): the restore and the tombstone pass are not atomic

- **Where:** `crates/juggler-app/src/backup.rs:705-706`; `backup.rs:106-134`.
- **Failure:** `restore_from` replaces the database through the backup API and commits. `after_restore` runs later, in its own transaction, from `keep_this_macs_rows`.
  - A crash or quit between the two leaves the restored database **with the forgotten subjects back and without this Mac's tombstones**, which then exist only in `before_db`. Nothing re-applies them on the next start.
  - If `after_restore` returns an error, the restore has still happened and the forgotten rows stay.
  - `after_restore` also never folds the WAL (L2).
- **Fix:** apply the tombstones to the **staged** copy before it replaces the live database. Attach the live database as `prior` and run `after_restore` on the staged file, which is migrated by then, inside a `SecureHold`, then checkpoint it. The restore then only ever copies an already-filtered database. Keep the post-restore pass as a second check.

### M2 (medium): authority and the writer capability are self-declared, so agent or Chat words can enter as "user" settings and go to the cloud

- **Where:** `types.rs:268-275` (`KindWriter::for_kinds` is `pub`, so any code can mint any writer); `store.rs:239-282` (the gate never ties `authority` or `text_untrusted` to the writer); `backup.rs:31` (`STATED_SETTING` checks only `kind` and `authority`, not `untrusted` or `source_kind`).
- **Failure:** reproduced in E10. Any module can claim `authority = User` and `text_untrusted = false` for text it derived from agent words. The row is then indexed, recalled into prompts at the top authority, and, if the kind is `preference` or `standing_request`, uploaded to the cloud.
  - A user-authority watch whose payload holds an `Untrusted` session name (`untrusted = 1`) also goes to the cloud, because `untrusted` is not checked.
  - "Untrusted by type" holds only for the payload. For `text`, it is a bool the caller sets with care.
- **Impact:** the guarantees about prompt-injection resistance and the cloud rule rest on every future caller being correct. The architecture lint covers imports, not construction.
- **Fix:**
  - Make `KindWriter` constructible only by one registry at startup (`pub(crate)`, handed out once), and give it the authorities and sources each writer may use. The gate refuses anything else.
  - Make `text` an enum (`Text::User(String)` / `Text::Untrusted(String)`) so the column is set by type.
  - Add `AND untrusted = 0 AND source_kind IN ('settings', 'watches_view', 'chat_turn')` to `STATED_SETTING`, or carry a session name in a watch as an id only.

### M3 (medium): "stated settings" are free text, so typed Chat words go to the cloud as preferences

- **Where:** `backup.rs:31`; ADR §1.1 and §2.4. The cues "from now on" and "always" store "the user's own words after the cue" as a `preference` (`focus`).
- **Failure:** §1.8 keeps notes local because they are "free words typed into Chat, which can hold anything". It sends preferences to the cloud because they are "typed settings from a closed set of slots". But only the **slot** is closed: a `focus` or `alias` row's `text` is up to 280 characters of the user's Chat words, checked only by the secret pass. "Always remember that my treatment is X" can become a `focus` preference that is uploaded.
- **Fix:** send a preference to the cloud only in a typed, bounded form: for `alias`, the entity id plus a name capped at about 60 characters; for `focus`, an entity reference, not free text. A cue that doesn't parse to an entity becomes a `note` (local only). Alternatively, the coordinating session rules on it explicitly (a D-number). Add a test that a `focus` row with a free sentence stays out of the cloud copy.

### L1 (low): a forget whose WAL fold is blocked is never retried

- **Where:** `secure.rs:40-45`; `store.rs:475`. Nothing reads `Forgotten::wal_folded`, and maintenance does not exist yet.
- **Failure:** the app opens `juggler.sqlite` four times (`storage/mod.rs`, `BUSY_TIMEOUT`'s note), so a read on another connection will regularly block `wal_checkpoint(TRUNCATE)`.
  - The frames holding the words before the forget then stay in `-wal` until a later full checkpoint.
  - Once the WAL restarts, a stale frame past the new write point survives in the file until the WAL is truncated (`journal_size_limit` is not set).
  - E7 did not leave the canary behind in a small run, but that depended on where the frame sat in the WAL.
- **Fix:** when `wal_folded` is false, retry with a bounded backoff (for example 3 tries over a few seconds). Record a pending fold that the next write or the app's idle tick runs. Set `PRAGMA journal_size_limit` so a passive checkpoint's reset truncates the WAL. Surface "still finishing" to the caller.

### L2 (low): `after_restore` and the cloud scrub do not fold the WAL themselves

- **Where:** `backup.rs:106-134`. `scrub_snapshot` relies on the caller's `VACUUM`, which the app does run (`crates/juggler-app/src/backup.rs:229`).
- **Failure:** after a restore, the original pages holding the dropped subjects stay in the main file, and the zeroed versions sit in the WAL, until the next checkpoint.
- **Fix:** call `fold_wal` at the end of `after_restore`, as `forget` and `scrub_copy` do.

### L3 (low): `INSERT OR REPLACE` / `UPDATE OR REPLACE` bypass the FTS delete trigger

- **Where:** `schema.rs:90-93`; reproduced in E9.
- **Failure:** a REPLACE conflict deletes the old row without firing `AFTER DELETE` triggers, because `recursive_triggers` is off. The old row's words stay in the index under its `seq`, and a later forget cannot reach them. No code uses REPLACE on `memory_records` today, but `memory_forgotten` already uses `INSERT OR REPLACE`, so the pattern is in this code.
- **Fix:** run `PRAGMA recursive_triggers = ON` on the store's connection (set in `Storage::initialize`), or add a clippy-style grep in `verify-static` that refuses `OR REPLACE` against `memory_records`.

### L4 (low): a sidecar without `memory_subject` is out of forget's reach

- **Where:** `log.rs:211-254`. `AiLog::append` is public, and `memory_subject` is optional.
- **Failure:** a feedback ("this is wrong") or fire event that quotes a memory but is appended without the subject's id keeps those words until its 30-day expiry. ADR §7.1 says a sidecar "goes with its subject".
- **Fix:** require `memory_subject` whenever the event's subject is a memory record or a turn that produced one. Alternatively, make forget also delete sidecars whose event `subject_id` is one of the deleted version ids (`ai_log.subject_kind = 'memory'`).

### L5 (low): a failed restore leaves the extracted backup in staging

- **Where:** `crates/juggler-app/src/backup.rs:705-718`.
- **Failure:** a `?` before `remove_dir_all(&staging)` leaves the staged database, with its Chat history and the subjects this Mac forgot, in the data folder.
- **Fix:** use a guard that removes the staging folder on every exit.

### L6 (low): the scrub of a pre-migration copy depends on a compile flag for the turns cascade

- **Where:** `backup.rs:145-163`; `storage/mod.rs:1464-1499` (`open_hardened` never sets `foreign_keys`).
- **Failure:** `DELETE FROM chat_conversations` removes `chat_turns` only through `ON DELETE CASCADE`. On this connection that works only because the bundled build sets `SQLITE_DEFAULT_FOREIGN_KEYS=1`. A system-SQLite build (Linux in 0.7) would leave every turn in the copy. `memory_entities` is in the same position.
- **Fix:** set `PRAGMA foreign_keys = ON` in `scrub_copy`, and delete `chat_turns` and `memory_entities` explicitly as well.

### Info

1. **The text-free spine accepts words.** `code_like` (`types.rs:366-372`) allows `[A-Za-z0-9_.:,$-]`, so a payload string or `correlation` such as `my,diagnosis,is,x` passes `text_free` (`log.rs:180-190`) and stays for 90 days. A typed payload struct, the plan in §7.1, is the real control. Until then, consider refusing commas and limiting lowercase runs.
2. **The ADR's own test list is only partly built.** §10.1's `forget_really_deletes` also covers a turn, "both pre-migration copies made after it was written" and "a backup archive's snapshot" read as raw bytes, plus the Clear actions. Only the database, WAL and sidecar parts exist. `a_pre_migration_copy_holds_no_memory` checks one string in the raw bytes and never the turn's text.
3. **`superseded_by` can be pointed at the row's own id** with a direct `UPDATE`, because the trigger allows NULL to any value. This leaves the subject with no current row, and the next `remember` then fails on `UNIQUE(subject_key, version)`. It is an integrity issue, not a privacy one: forget still removes every version. A `CHECK (superseded_by IS NOT id)` or a trigger requiring the target to be a newer version of the same `subject_key` would close it.
4. **Restoring a cloud backup erases this Mac's Chat history** ("a restore replaces everything", §1.8), apart from the before-restore copy. That is data loss rather than exposure. It is already deferred to step 4.

## Verified as sound

- **One transaction per forget** (`store.rs:441-474`): rows, entities (by cascade, with FK on), FTS entries, sidecars, the tombstone and the `memory.forgot` event all commit together or not at all. A crash mid-forget rolls back, so a partial forget cannot leave a half-deleted subject.
- **`secure_delete` is on for every store and log write**, not only forget (`SecureHold`), and is restored afterwards. `SecureHold` is dropped after the transaction ends, because locals drop in reverse order. M2 proves the test depends on it.
- **FTS5 `secure-delete` is really enabled** and persisted in `memory_fts_config`. M1 proves the test depends on it. Its remaining gap is H1.
- **The FTS index is keyed by an explicit `seq INTEGER PRIMARY KEY`**, so `VACUUM` and `VACUUM INTO` keep the index aligned. Untrusted and superseded rows are never indexed, and retrieval filters them again (`retrieval.rs`, `eligible`).
- **The content triggers** refuse every change to content columns. `superseded_by` can be set only once from NULL, and `supersedes` only to NULL. Reserved kinds, the `bot` authority, and `agent` without `untrusted` are refused at the database level as well as in Rust. `memory_entities` cannot be updated.
- **The log spine** cannot be updated or deleted early, and its watermarks only move forward. It names a subject only by a random `subject_id`, never by the key (a note's key is a hash of its words), so no fingerprint outlives the forget.
- **The WAL fold:** M3 proves the test depends on it. The test reads `-wal`, `-shm` and `-journal` as well as the main file, and proves its search would find the canary before the forget.
- **Tombstones:** a later local backup drops forgotten subjects and carries no tombstones. A completed restore copies this Mac's tombstones forward and drops what they name, including when the backup predates V54, because `restore_from` migrates the staged copy first.
- **The cloud copy** drops every conversation, turn, sidecar, spine row and non-stated record. The FTS triggers stay consistent and `integrity-check` passes. The app's `scrub` folds the WAL, switches to `DELETE` journal mode and runs `VACUUM`, so freed pages don't travel (apart from H1's separators). Only `cloud_backup_now_impl` uses `backup_for_cloud`. The user's export and the copy before an update are `Local` by D-133's ruling.
- **No in-memory copy outlives a forget in this step:** the store keeps no cache (`prepare_cached` holds statements, not rows), and retrieval rebuilds its candidates on every call. Receipts are ids only (§6), so they need no forgetting.

## Cleanup

- The scratch copy `…/scratchpad/aij-review` (the `git archive` tree, the experiment tests and two `.orig` files) was listed with `find`, then deleted.
- The experiments' databases were `tempfile` folders, removed when each test ended.
- Nothing was kept apart from this file. The shared target holds the scratch build's artifacts under their own hashes, as any branch build does.

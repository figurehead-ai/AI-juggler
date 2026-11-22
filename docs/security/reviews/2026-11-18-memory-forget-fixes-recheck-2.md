# Security re-review 2: the memory store's forget path after the re-check's fixes (ADR-0037, AJ-E3819 step 1)

- **Reviewer:** an independent Opus session. It wrote neither the fixes nor either earlier review. It assumed nothing was fixed until the code, and a test that fails without the fix, showed it.
- **Date:** 2026-11-18. **Branch:** `release/0.5` at `162a3e055`. HEAD moved to `7f25278a2` during the review. The commits in between touch only `docs/qa/` (the harness), not the paths reviewed here.
- **Inputs:** the first re-check `docs/security/reviews/2026-11-18-memory-forget-fixes-recheck.md` (6aa2a729a); the fixes 94071d3be (H1), 8b3bd9539 (H3, N3, N5, L5), 8218679f4 (M2), 4208c10ac (M3), 2e0b0f8f5 (N2), 7d4536b9b (N4), 9bee4ccef (L2–L4), f172bc6f5 (carried tasks), 0a6ea129f (ADR text), e3707995e (test split); ADR-0037; `crates/juggler-core/src/storage/memory/`; `crates/juggler-app/src/backup.rs` and `newer_data.rs`.

## Verdicts

| Finding | Verdict | In one line |
|---|---|---|
| **H1** index separators | **CLOSED** | `forget` and `after_restore` rebuild the index (`reindex`). All three re-check cases fail with `optimize` and check raw bytes (M-C1, M-C2). |
| **H3** before-restore copy | **CLOSED** | `ScrubOnDrop` runs inside the hold, the test checks raw bytes, and the copy holds the words before its scrub (M-A1). A crash is left to the start sweep (M-A2). |
| **M2** self-declared authority | **CLOSED** | `AppState` takes the writers at build (M-A7), and each cloud-filter clause has its own failing test (M-C7a–e). Info 3 and 4 below are notes for step 2. |
| **M3** free text as preferences | **CLOSED** | A preference must carry `RecordText::Setting(SettingLabel)` (M-C8). The cloud layer checks length and entity, not the label's shape (Info 2). |
| **L1** blocked WAL fold not retried | **OPEN** (accepted for now, with a condition) | It is deferred to step 3 (f172bc6f5). The judgement is below. |
| **L2** `after_restore` WAL fold | **CLOSED** | `a_restore_of_an_older_export_drops_the_forgotten_separator` fails on raw bytes without `fold_wal` (M-C3). |
| **L3** `REPLACE` skips the FTS delete | **CLOSED** | The test fails without `recursive_triggers` (M-C5). |
| **L4** sidecars by version | **CLOSED** | The test fails without the by-version clause (M-C6). |
| **L5** staging left behind | **PARTIAL** (low) | Crashes and a restart are covered: the sweep removes the staging folder (M-A3). Within one session, a backup that was verified but never restored stays staged. A forget made meanwhile doesn't reach it until the next verify, restore or start. |
| **N2** going-back copies | **CLOSED** (in code) | `from-v<N>` is scrubbed before anything is replaced (M-A5), and `from-newer` once going back succeeds (M-A4). The sweep scrubs both, plus `unused-` (M-A6). The user-facing words are now untrue: see **N6**. |
| **N3** crash leftovers | **CLOSED** | The sweep at start removes the work folders and scrubs every kept copy (M-A2, M-A3). It runs before `cloud_backup::spawn_schedule`. |
| **N4** NULL `title_turn` | **CLOSED** for step 1 | The fallback rename is tested (M-C4). Setting `title_turn` and calling `link_turn` are step 2's contract, recorded in E38-chat.md. |
| **N5** forget during a restore | **CLOSED** (by code and by my test, which isn't committed) | One hold covers everything from `snapshot_to` to the session stops. My deterministic test (appendix) passes on HEAD 8 times out of 8. It fails when the hold is split (M-A8) and when `ScrubOnDrop` does nothing (M-A1). |

**New findings:** **N6 (low):** the going-back sheet and the older build's launch dialog still say "nothing is deleted". **N7 (low):** reindex cost when a forget covers many subjects. Info 1–5 below.

## How it was checked

- **Scratch copy:** `git archive HEAD` into the scratchpad, every source file touched (so every workspace crate rebuilt there), and the ignored `ui/dist` and `binaries/` cloned in.
- **Scratch target:** its own `CARGO_TARGET_DIR` in the scratchpad, seeded by an APFS clone (`cp -c`) of r05's `debug/{deps,build,.fingerprint}`. The shared r05 target was only read, never built into. A test run with `-v` confirmed that the scratch tree's own files decide what rebuilds ("Dirty juggler-core … erase.rs has changed").
- **Mutations:** each one edited the scratch copy from a pristine copy, built, ran the tests, and restored the file.
- **The worktree:** never edited. `git status` lists no tracked change under `crates/`, `ui/` or `docs/adr/`.

| # | Mutation (one fix removed) | Result |
|---|---|---|
| M-C1 | `forget`'s `reindex` replaced with `optimize` | `a_second_forget_in_a_row…` and `a_forget_on_a_one_segment_index…` **fail** |
| M-C2 | `after_restore`'s `reindex` replaced with `optimize` | `a_restore_of_an_older_export_drops_the_forgotten_separator` **fails** |
| M-C3 | `fold_wal` removed from `after_restore` | the same test **fails** (on the file-bytes assertion) |
| M-C4 | N4's `title_turn IS NULL` clause removed | `forget_leaves_no_trace_in_the_file` **fails** |
| M-C5 | `recursive_triggers` removed from `SecureHold` | `a_replace_inside_the_hold_leaves_no_old_words_in_the_index` **fails** |
| M-C6 | the by-version sidecar clause removed | `a_sidecar_naming_a_version_goes_with_the_forget` **fails** |
| M-C7 | the cloud filter put back to `kind IN (…) AND authority = 'user'` | `the_cloud_filter_takes_only_typed_trusted_stated_settings` **fails** |
| M-C7a–e | one clause dropped at a time: `untrusted = 0`; the preference source; `length <= 60`; the entity `EXISTS`; the watch source | **each fails** that test |
| M-C8 | the gate's `RecordText::Setting` requirement removed | `the_write_gate_refuses_and_logs_without_the_text` **fails** |
| M-A1 | `ScrubOnDrop` made a no-op | `a_restore_drops_the_forgotten_and_keeps_no_copy_holding_it` **fails**, and so does my N5 test |
| M-A2 | the sweep's `scrub_copy` replaced with an error | both sweep tests **fail** |
| M-A3 | the sweep's work-folder removal removed | `the_sweep_at_start_leaves_no_unscrubbed_copy_or_work_folder` **fails** |
| M-A4 | `go_back`'s scrub of `from-newer` removed | `the_launch_offers_the_copy_kept_before_the_upgrade` **fails** |
| M-A5 | `going_back_prepare_at`'s scrub of `from-v<N>` removed | `the_newer_build_prepares_going_back_to_the_version_before` **fails** |
| M-A6 | `from-newer-` dropped from `KEPT_COPIES` | `the_sweep_scrubs_and_ages_out_the_going_back_copies` **fails** |
| M-A7 | `MemoryWriters::taken_at_start` holding `None` (not taking the writers) | `the_writers_are_taken_when_the_state_is_built` **fails** |
| M-A8 | the old split hold (snapshot under one lock, restore under a later one) | my N5 test **fails**: the forget commits in the gap and the restore brings the subject back |

**Baselines:** on the unmutated scratch copy, every test passes: `-p juggler-core --lib memory` (19), `-p juggler-app --lib backup:: features::ai::memory` (13, plus my N5 test), and `--test newer_data` (8).

**Raw bytes:** every "no trace" test reads raw bytes.
- The core tests read the file, `-wal`, `-shm` and `-journal`.
- The app's `holds()`, `newer_data`'s `holds_canary` and the sweep tests read the file, `-wal` and `-shm`.
- They cover the staged backup, the before-restore copy, the crashed and failed-restore copies, the three going-back copies, and the cloud and local copies.
- My N5 test scans every file under the data folder.
- `-journal` isn't read on the app side. A rollback journal is deleted when its transaction ends, and its freed blocks are file-system residue, which ADR §1.5 item 9 already puts out of reach.

## L1: is deferring it to step 3 acceptable for a 0.5 Beta?

**Yes, on one condition:** the retry and `journal_size_limit` must land before V54 is registered, or in the same release. Until V54 is registered, no real database holds memory, so there is nothing to leak. The task text in f172bc6f5 doesn't tie step 3 to V54's registration, so it could slip past it. Make it a registration gate, like the e2e test.

**What could be recovered in the meantime:**
- **What survives:** a forgotten record's words in WAL frames written before the forget. They stay in `juggler.sqlite-wal` until the next checkpoint that truncates, or until new frames overwrite them.
- **When the fold is blocked:** `wal_checkpoint(TRUNCATE)` waits up to `BUSY_TIMEOUT` (5 s) for readers. It fails only when another of the app's four connections (process ledger, cleanup audit, cleanup settings) holds a read snapshot for more than 5 s at that moment. Their transactions are short, so this is rare.
- **How long it survives:** the next forget retries the fold, and every open runs `wal_checkpoint(TRUNCATE)` (`open_steps::tidy`). The real exposure is the time until the next forget or app start, which for an always-on app can be weeks.

**Who could read it:**
- Any code running as the user can read `~/Library/Application Support/…`. That includes the coding agents AI Juggler hosts, which run as the user without a sandbox.
- A curious agent or person running `strings` on the WAL could find the words. The same reader could also have read the live database before the forget, so the extra exposure is only for someone who arrives after it.
- Time Machine could also capture the `-wal` in that window. Item 9 of ADR §1.5 already puts Time Machine copies out of reach.

**Likelihood and impact:** likelihood is low (a blocked fold, then a reader in that window). Impact is moderate: the user's own words that they asked to forget.

## Regression hunt

- **Can the start sweep delete the wrong thing?** It only matches names AI Juggler itself writes in the data folder:
  - folders `backup-work-*`, `cloud-backup-work-*` and `restore-staging`;
  - files `juggler.sqlite.{before-restore-,before-failed-restore-,from-v,from-newer-,unused-}*`, with `-wal`, `-shm` and `-journal` mapped to their base file.

  Three things it must not touch are safe:
  - **The live data:** `juggler.sqlite` and its `-wal` and `-shm` match no prefix.
  - **The update backups:** `backups/` matches nothing.
  - **The user's exports:** they go to a path the user picks (`PickKind::BackupDest`), not the data folder.

  The sweep also runs before the cloud backup schedule starts (`lib.rs` setup), so it can't remove a work folder that is in use. Removing `restore-staging` at start can break a verify → restart → confirm sequence, but the UI can't reach it: the confirmation token exists only in the frontend's verify result, and a restart loses it.
- **Can the 30-day prune race a restore?** No race found. `prune_aged_copies` runs in `backup_restore_impl` before the hold, and removes only copies older than their keep:
  - the current restore's copy is stamped now;
  - `before-failed-restore-*` is never pruned;
  - only one restore can hold the one-time token.

  `go_back` (an older build restoring a backup) writes `from-newer-<now>` right before that prune, so its own copy isn't touched.
- **What does reindex cost on a large store?** I measured it in the scratch target with an ignored timing test:
  - **Store:** 100,000 notes, 16.9 MB of text (near the 20 MB cap), a 58 MB file.
  - **Machine:** shared, load average about 20.
  - **Cost:** each `forget` took **333–373 ms** (rebuild and WAL fold). `optimize` alone took 101 ms.

  That is acceptable for one forget, but it runs under the app-wide storage mutex. See N7.
- **Do logs print record text?** No. The two new `tracing::warn!` lines (`sweep_kept_copies`, `go_back`) print only the SQLite error. `storage/memory/` still has no logging, and refusals log codes only.

## Findings

### N6 (low): the going-back words still say "nothing is deleted"

- **Where:**
  - `ui/src/components/UpdatesCard.tsx:121–123`: "The data as it is now is kept beside it in the data folder; nothing is deleted."
  - `crates/juggler-app/src/newer_data.rs:489` (the older build's launch dialog): "What the newer version saved is kept beside it in the data folder; nothing is deleted."
- **What is now true (since 2e0b0f8f5):**
  - The kept copy's memory, Chat conversations, turns and sidecars are emptied before anything is replaced (`from-v<N>`) or once going back succeeds (`from-newer`).
  - The sweep deletes the **whole** copy 30 days later: accounts brought in, sessions, settings, everything, not just memory.
- **The scope change:** the `from-newer` copy used to be "never deleted". N2 needed the memory out of these copies. It did not need the rest of the user's newer data deleted after a month. That is a retention change beyond the security fix, so it needs its own D-number.
- **Why it matters:** this is not a confidentiality issue. The risk is a promise the app no longer keeps, and data loss for someone who comes forward after 30 days and looks for the copy by hand.
- **Fix:**
  - Say on both surfaces that memory and Chat history are not kept in the copy, and how long the copy is kept.
  - Either log the 30-day removal as a D-number, or keep the scrubbed copy without an age limit (it no longer holds anything forget must reach).

### N7 (low): a forget that covers many subjects pays one reindex per subject, under the storage lock

- **Where:** `MemoryStore::forget` takes one subject key, and each call that erases something ends with a full `reindex` (about 350 ms at 100k rows).
- **The cost:** step 4's "Forget this conversation" or "Forget all memory", built as a loop of `forget` calls, holds the app-wide storage mutex for N × 350 ms. Fifty memories means about 17 s with every storage command blocked.
- **Fix:** give step 4 a batch forget (one transaction, one `reindex`, one WAL fold). "Forget all" should empty the tables and `delete-all` the index. Note it in the step-4 task.

### L5, the remaining part (low)

- **The gap:** a `restore-staging` folder from a verify that was never restored stays for the rest of the session. A forget made after the verify doesn't reach it.
- **Fix:** remove the staging folder in `forget` (or when the restore sheet is dismissed), or let it expire after a few minutes.

### Info

1. **N5 has no committed test.** The appendix test needs no production change. It runs before V54 is registered: verify stages the archive, then refuses V54's tables, and the test sets the token the way verify would. It also covers `backup_restore_impl`'s wiring end to end (`ScrubOnDrop`, `apply_tombstones`, the hold, and a raw-bytes scan of the whole data folder), which the ignored e2e test was meant to cover. I recommend committing it beside the sweep tests.
2. **The cloud filter checks less than the ADR table says.** ADR §1.8's table says a stated preference's label is a `SettingLabel` (at most 6 words, no sentence punctuation). The cloud filter checks only `length <= 60` and an entity. A row that bypassed the gate (an older build, a bug) and holds a 60-character sentence would go to the cloud. Either check the shape in SQL too, or say in the ADR that the cloud layer checks length and entity only.
3. **`MemoryWriters::take()` hands all four writers to whichever caller comes first.** Before, `Writers::issue` handed them to the first caller anywhere. Now the first caller holding `&AppState` gets them. There's no production caller yet. When step 2 wires the modules, hand each one its own writer (one take per field).
4. **The ignored e2e test will fail when it is un-ignored.** It calls `a.memory_writers.take().unwrap()`. In the `backup_restore` test binary, another test's `AppState`, built earlier or in parallel, takes the process's writers first, so this `unwrap` panics. Write the canary with direct SQL (as the unit tests now do), or use a per-test writer.
5. **The re-check's Info 2 is unchanged.** A backup is still stamped after its scrub, not at its snapshot. It was out of scope for this round.

## Appendix: the N5 test (scratch only, not committed)

Add to `crates/juggler-app/src/backup.rs`'s `mod tests` (it uses that module's `canary_note`, `CANARY`, `TOKEN_KEY`, `STAGING_DIR` and `RESTORE_COPY_PREFIX`):

```rust
/// A forget asked for while a restore holds the storage waits, then
/// applies to the restored data; the forgotten words come back nowhere in
/// the data folder. The forget starts only once the copy before the
/// restore exists (inside the hold), so the order is fixed. Fails when the
/// hold is split.
#[test]
fn a_forget_during_a_restore_applies_to_the_restored_data() {
    let dir = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let data = dir.path().to_path_buf();
    let state = Arc::new(
        AppState::init_with_registry(data.clone(), juggler_adapter::AdapterRegistry::new())
            .unwrap(),
    );
    {
        let storage = state.storage.lock().unwrap();
        storage.raw_conn().execute_batch(memory::V54).unwrap();
        canary_note(&storage);
    }
    let archive = out.path().join("before.ajbackup");
    backup_export_impl(&state, &archive, &mut |_, _| {}).unwrap();
    // Verify stages it, then refuses V54's tables until V54 is registered;
    // the confirmation is set as verify sets it.
    let _ = backup_verify_impl(&state, &archive);
    assert!(data.join(STAGING_DIR).join("db/juggler.sqlite").exists());
    state.storage.lock().unwrap().app_settings()
        .set(TOKEN_KEY, &format!("tok:{}", archive.display())).unwrap();

    let forgetter = {
        let state = Arc::clone(&state);
        let data = data.clone();
        std::thread::spawn(move || {
            let start = std::time::Instant::now();
            while !std::fs::read_dir(&data).unwrap().flatten().any(|e| {
                e.file_name().to_string_lossy().starts_with(RESTORE_COPY_PREFIX)
            }) {
                assert!(start.elapsed().as_secs() < 30, "no restore started");
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            crate::commands::lock_storage_arc(&state).unwrap()
                .memory().forget("note:gone", &Actor::User, 5).unwrap()
        })
    };
    backup_restore_impl(&state, &archive, "tok").unwrap();
    let forgot = forgetter.join().unwrap();
    assert_eq!(forgot.subjects, 1, "the forget found the restored subject");
    {
        let storage = state.storage.lock().unwrap();
        assert!(storage.memory().current("note:gone").unwrap().is_none());
        storage.raw_conn()
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(())).unwrap();
    }
    let mut stack = vec![data.clone()];
    let mut holding = Vec::new();
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if std::fs::read(&path).unwrap_or_default()
                .windows(CANARY.len()).any(|w| w == CANARY.as_bytes())
            {
                holding.push(path);
            }
        }
    }
    assert!(holding.is_empty(), "these still hold it: {holding:?}");
}
```

## Cleanup

- **Deleted,** after listing them first:
  - the scratch copy (`…/scratchpad/tree`);
  - the scratch target (`…/scratchpad/target`, APFS clones plus this review's rebuilds);
  - the mutation specs and tools (`…/scratchpad/specs`, `…/scratchpad/tools`);
  - the result logs and the two test fragments.
- **Shared target:** r05 was only read (cloned), never written.
- **Kept:** this file only.

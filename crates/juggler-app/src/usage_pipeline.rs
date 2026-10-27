//! The AJ-E607 ingest pipeline owner (deep/09 §Q8: one pipeline task,
//! single writer per account, passive): a background pass that tails
//! every sanctioned local source into the AJ-E606 store and records
//! per-source health. Sources:
//!
//! - Claude: the managed statusline spool (Measured quota samples;
//!   wrapper re-ensured idempotently each pass) + transcript `usage`
//!   lines (Measured tokens), offset-tracked per file.
//! - Codex: rollout `token_usage_record` lines (Measured tokens).
//! - `ZCode`: `model_usage` via read-only SQLite (Computed credits).
//! - A tool that keeps its usage in a store read by time (OpenCode's
//!   `opencode.db`): through its adapter's `usage_since`, read-only.
//! - Gemini: NO source exists at any grade (AJ-E605) — nothing to
//!   tail; the absence is a typed decision, not an omission.
//!
//! No HTTP, no Provider process control, no writes outside AI
//! Juggler's own managed dirs (test-pinned in the adapters crate).
//! Offsets + health persist in the settings KV as one JSON blob, so a
//! restart resumes exactly where the spool was consumed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::state::AppState;

/// Spool cadence: every pass (event→engine ≤2 s budget).
const PASS_EVERY_MS: u64 = 1_000;
/// Transcript/rollout scan cadence: every 4th pass (~4 s).
const FILE_SCAN_EVERY: u32 = 4;
/// `ZCode` SQLite re-read cadence: every 15th pass (~15 s).
const ZCODE_EVERY: u32 = 15;
/// The persisted offsets+health blob's settings key.
const INGEST_STATE_KEY: &str = "usage_ingest_state";
/// Per account: the newest credits snapshot (JSON) its tool reported (Codex,
/// from its own rollouts) — what the account's card shows as its balance.
/// Keyed by account, whatever the tool.
pub const CREDITS_KEY_PREFIX: &str = "credits:";
/// Raised when stored transcript/rollout rows were counted wrongly and
/// must be read again: the next pass re-reads every file still on disk
/// from the start, replacing its sessions' rows. 1 = 0.0.13 (Claude
/// content blocks counted per line; Codex's nested token classes and
/// missing model). 2 = Codex compactions' own usage (P17), and a message a
/// resumed copy shares credited to the session that came first. 3 = every
/// session that offered a message recorded, so a shared message's keeper
/// no longer depends on the reading order (a copy of a copy, a fork); a
/// compaction counted once wherever a pass ends. Then two branches each
/// took 4, and met at 6 (a cross-branch number, like a migration's; the
/// check is `stored < REREAD_VERSION`, so every install re-reads once with
/// all three):
/// - the history a same-tool switch carried into the target's copy,
///   counted again on the target (P19, ×1.41–1.48 on the lab's
///   conversations; release/0.1's 4);
/// - Codex's compressed rollouts read (`.jsonl.zst`): their history comes
///   in as a re-read, not as new use (main's 4);
/// - a Codex compaction that names its own model call opens no row of its
///   own (P21: one counted another thread's call again; main's 5).
const REREAD_VERSION: u32 = 6;

/// Quota samples gathered in one pass, with where each came from (the
/// all-models windows', and the weeks on one model only).
type SampleBatch = crate::features::quota::records::SourcedBatch;

/// Per-account persisted tail state (offsets by file path).
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct AccountOffsets {
    spool: u64,
    /// A copy an earlier version wired to a spool of its own (left as it
    /// is): read too, while it's there.
    #[serde(default)]
    legacy_spool: u64,
    #[serde(default)]
    files: BTreeMap<String, u64>,
    /// Per Codex rollout: what its last slice left for the next one (the
    /// model, the last total, compactions waiting for their thread).
    #[serde(default)]
    carries: BTreeMap<String, juggler_adapters::RolloutCarry>,
    /// Claude: how far its own limit records were read (the app's meter,
    /// Claude Code's last `/usage`, the look back for limit hits).
    #[serde(default)]
    records: crate::features::quota::records::RecordsState,
}

/// The whole persisted pipeline state.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct IngestState {
    #[serde(default)]
    offsets: BTreeMap<String, AccountOffsets>,
    #[serde(default)]
    health: BTreeMap<String, juggler_adapters::SourceHealth>,
    /// The [`REREAD_VERSION`] the stored rows were last read under.
    #[serde(default)]
    reread: u32,
    /// Codex accounts whose P21 leftovers were looked for (once each; see
    /// `forget_duplicated_compactions`).
    #[serde(default)]
    compactions_checked: std::collections::BTreeSet<String>,
    /// The tools' own app instance folders seen (their limit records are
    /// read there while the folder is).
    #[serde(default)]
    app_instances: std::collections::BTreeSet<String>,
}

/// A Codex account's P21 check to make, with the threads whose rollouts
/// are still there.
type CompactionCheck = (String, std::collections::BTreeSet<String>);

/// Spawns the pipeline thread (called once from app setup).
pub fn spawn(state: Arc<AppState>) {
    std::thread::spawn(move || {
        let mut tick = 0u32;
        loop {
            tick = tick.wrapping_add(1);
            let pass_started = std::time::Instant::now();
            ingest_pass(&state, tick);
            // Never a busy loop: sleep the remaining budget, at least
            // the full cadence when a pass overruns (slow disk).
            let elapsed = pass_started.elapsed();
            let sleep_for = std::time::Duration::from_millis(PASS_EVERY_MS).saturating_sub(elapsed);
            std::thread::sleep(sleep_for.max(std::time::Duration::from_millis(1)));
        }
    });
}

/// P19: while a switch runs, its copy may be installed before the launch
/// records the copy's own id, and a pass then would key the copy by the
/// source's id; offsets are saved, so that would be for good. The next
/// pass after the switch reads it.
fn switch_in_progress(state: &Arc<AppState>) -> bool {
    state
        .switching
        .lock()
        .is_ok_and(|running| !running.is_empty())
}

/// The accounts to read, and whether everything must be read again. Also
/// records which accounts' usage keys name their own session.
fn accounts_to_read(state: &Arc<AppState>) -> Option<(Vec<juggler_core::model::Account>, bool)> {
    let storage = state.storage.lock().ok()?;
    let list = storage.accounts().list().ok()?;
    // A build without the holders wrote usage rows here (gone back to,
    // then forward again): read everything again so they're whole.
    let dirty = storage.usage().derived_dirty().unwrap_or(false);
    // Accounts whose tool's usage keys name their own session: the
    // store keeps no holders for them.
    let unshared: std::collections::BTreeSet<String> = list
        .iter()
        .filter(|account| {
            crate::features::quota::adapter_has(
                account.harness.as_str(),
                juggler_adapter::Capabilities::USAGE_KEYS_PER_SESSION,
            )
        })
        .map(|account| account.id.as_str().to_owned())
        .collect();
    let _ = storage.usage().set_unshared_accounts(&unshared);
    Some((list, dirty))
}

/// One ingest pass (public for tests): accounts → sources → store.
pub fn ingest_pass(state: &Arc<AppState>, tick: u32) {
    if switch_in_progress(state) {
        return;
    }
    let Some((accounts, dirty)) = accounts_to_read(state) else {
        return;
    };
    let mut ingest = load_state(state);
    let rereading = ingest.reread < REREAD_VERSION || dirty;
    if rereading {
        for offsets in ingest.offsets.values_mut() {
            offsets.files.values_mut().for_each(|offset| *offset = 0);
            offsets.carries.clear();
        }
    }
    let mut store_batch: Vec<juggler_core::quota::UsageEvent> = Vec::new();
    let mut sample_batch = SampleBatch::default();
    let mut credits_batch: Vec<(String, juggler_adapters::CreditsSnapshot)> = Vec::new();
    let mut checks: Vec<CompactionCheck> = Vec::new();
    // Claude's own limit records, on their slower cadence.
    let instances =
        crate::features::quota::records::due_instances(state, &mut ingest.app_instances, tick);

    for account in &accounts {
        let account_id = account.id.as_str().to_owned();
        let harness = account.harness.as_str().to_owned();
        let dir = account.state_dir.clone();
        if dir.as_os_str().is_empty() {
            continue;
        }
        match harness.as_str() {
            "claude-code" => {
                // ADR-0018: an account's capture lives in AI Juggler's own
                // folder, a copy's as an in-place one's; its folder is
                // never written.
                let spool =
                    juggler_adapters::overlay_spool_path(&state.overlays_dir(), &account_id);
                ingest_claude(
                    &account_id,
                    &dir,
                    &spool,
                    &mut ingest,
                    &mut store_batch,
                    &mut sample_batch,
                    instances.as_deref(),
                );
            }
            "codex" if rereading || tick.is_multiple_of(FILE_SCAN_EVERY) => {
                ingest_codex(
                    &account_id,
                    &dir,
                    &mut ingest,
                    &mut store_batch,
                    &mut sample_batch,
                    (&mut credits_batch, &mut checks),
                );
            }
            "zcode" if tick.is_multiple_of(ZCODE_EVERY) => {
                ingest_zcode(&account_id, &dir, &mut ingest, &mut store_batch);
            }
            // ADR-0019 §2: a tool whose usage sits in a store read by time
            // (a database) reads it through its adapter.
            _ if rereading || tick.is_multiple_of(FILE_SCAN_EVERY) => {
                if let Some(adapter) = state.registry.get(&account.harness) {
                    ingest_store(adapter.as_ref(), account, &mut ingest, &mut store_batch);
                }
            }
            _ => {}
        }
    }

    // P19: the history a same-tool switch carried belongs to the account
    // it was spent on, never to the one it was copied into.
    let emptied = drop_carried(state, &mut store_batch);
    if rereading
        && !emptied.is_empty()
        && let Ok(storage) = state.storage.lock()
        && storage.usage().delete_session_events(&emptied).is_err()
    {
        return;
    }

    // Single writer, in chunks (QA batch 9: a first pass on a heavy setup
    // held storage for over a minute in one write, and every screen and
    // the conversation index waited behind it). Offsets are saved only
    // once every chunk is in: when a write fails, the next pass reads the
    // same lines again (the store dedupes what an earlier chunk kept).
    let Some(stored) = store_in_chunks(state, rereading, &store_batch) else {
        return;
    };
    if let Ok(storage) = state.storage.lock() {
        let usage = storage.usage();
        if rereading {
            ingest.reread = REREAD_VERSION;
            if usage.clear_derived_dirty().is_err() {
                return;
            }
        } else {
            // AJ-E1103: Measured tokens this pass added (D-06): a new
            // response, or the rest of one an earlier pass began. A re-read
            // is not new use.
            crate::usage_stats::tokens_ingested(&accounts, &store_batch, &stored);
        }
        if usage.insert_sourced_samples(&sample_batch.windows).is_err()
            || usage
                .insert_model_week_samples(&sample_batch.model_weeks)
                .is_err()
        {
            return;
        }
        save_credits(&storage, &credits_batch);
        forget_duplicated_compactions(&storage, &mut ingest, &checks);
        if let Ok(blob) = serde_json::to_string(&ingest) {
            let _ = storage.app_settings().set(INGEST_STATE_KEY, &blob);
        }
    }
    // D-39 (off until ruled): ask the tool itself for profiles whose own
    // records are old.
    if instances.is_some() {
        crate::features::quota::probe::tick(state, &accounts);
    }
}

/// How long a staged switch's copy is recognised by the source's id,
/// before its launch is recorded with the copy's own (P19).
const CARRY_STAGED_MS: i64 = 10 * 60 * 1000;

/// Removes from `batch` the lines a same-tool switch carried into an
/// account: in the target's copy of a switched conversation, every line
/// older than the switch. Returns the sessions it emptied, so a re-read
/// can clear the rows they stored before.
fn drop_carried(
    state: &Arc<AppState>,
    batch: &mut Vec<juggler_core::quota::UsageEvent>,
) -> Vec<(String, juggler_core::quota::UsageSource, String)> {
    let carried: std::collections::HashMap<(String, String), i64> = {
        let Ok(storage) = state.storage.lock() else {
            return Vec::new();
        };
        storage
            .switch_graph()
            .carried_into(
                juggler_core::model::Timestamp::now().millis(),
                CARRY_STAGED_MS,
            )
            .unwrap_or_default()
            .into_iter()
            .map(|(account, session, at)| ((account, session), at))
            .collect()
    };
    if carried.is_empty() {
        return Vec::new();
    }
    let cutoff = |event: &juggler_core::quota::UsageEvent| {
        event
            .session
            .as_ref()
            .and_then(|session| carried.get(&(event.account.clone(), session.clone())))
            .copied()
    };
    let mut dropped = BTreeMap::new();
    let mut kept = std::collections::BTreeSet::new();
    batch.retain(|event| {
        let key = event
            .session
            .clone()
            .map(|session| (event.account.clone(), session));
        if cutoff(event).is_some_and(|at| event.ts < at) {
            if let Some(key) = key {
                dropped.insert(key, event.source);
            }
            false
        } else {
            kept.extend(key);
            true
        }
    });
    dropped
        .into_iter()
        .filter(|(key, _)| !kept.contains(key))
        .map(|((account, session), source)| (account, source, session))
        .collect()
}

/// P21's leftovers, once per Codex account: an older build stored a
/// compaction row for another thread's model call, and when its rollout
/// is gone no re-read replaces it. Only such a row goes (its tokens equal a
/// stored record's); every other row of a gone file stays.
fn forget_duplicated_compactions(
    storage: &juggler_core::storage::Storage,
    ingest: &mut IngestState,
    checks: &[CompactionCheck],
) {
    for (account, present) in checks {
        if storage
            .usage()
            .forget_duplicated_compactions(account, present)
            .is_ok()
        {
            ingest.compactions_checked.insert(account.clone());
        }
    }
}

/// Stores each account's newest credits snapshot, only ever forward: a
/// compressed old rollout, read whole, can carry a snapshot weeks older
/// than the one already stored.
fn save_credits(
    storage: &juggler_core::storage::Storage,
    batch: &[(String, juggler_adapters::CreditsSnapshot)],
) {
    for (account, credits) in batch {
        let key = format!("{CREDITS_KEY_PREFIX}{account}");
        let stored = storage
            .app_settings()
            .get(&key)
            .ok()
            .flatten()
            .and_then(|blob| serde_json::from_str::<juggler_adapters::CreditsSnapshot>(&blob).ok());
        if stored.is_some_and(|stored| stored.ts >= credits.ts) {
            continue;
        }
        if let Ok(blob) = serde_json::to_string(credits) {
            let _ = storage.app_settings().set(&key, &blob);
        }
    }
}

/// Events per storage write: one chunk holds the lock for a moment, and
/// whatever waits on storage gets it between chunks.
const STORE_CHUNK: usize = 2_000;

/// Stores the pass's events a chunk at a time, each in its own lock and
/// transaction; `None` when a write fails. A re-read replaces each
/// session's rows in the chunk that holds all of them, so a session is
/// never left half-replaced. The results come back in `batch`'s order
/// (a re-read's order isn't used).
fn store_in_chunks(
    state: &Arc<AppState>,
    rereading: bool,
    batch: &[juggler_core::quota::UsageEvent],
) -> Option<Vec<juggler_core::storage::Stored>> {
    let chunks: Vec<Vec<juggler_core::quota::UsageEvent>> = if rereading {
        session_chunks(batch, STORE_CHUNK)
    } else {
        batch.chunks(STORE_CHUNK).map(<[_]>::to_vec).collect()
    };
    let mut stored = Vec::with_capacity(batch.len());
    for chunk in &chunks {
        let storage = state.storage.lock().ok()?;
        let usage = storage.usage();
        // Every file still on disk was read from its start: its sessions'
        // old rows go and the rows just read replace them. A session whose
        // files are gone keeps the rows it had.
        let done = if rereading {
            usage.replace_session_events(&reread_sessions(chunk), chunk)
        } else {
            usage.store_events(chunk)
        };
        stored.extend(done.ok()?);
        drop(storage);
        std::thread::sleep(CHUNK_PAUSE);
    }
    Some(stored)
}

/// The pause between two chunks' writes, so a reader waiting on storage
/// gets it in between. Storage is a `std::sync::Mutex`, which isn't fair:
/// unlocked and locked again at once, it goes back to this loop before a
/// woken waiter runs, and a screen could wait out the whole re-read (2.2 s on
/// the replica at the maintainer's size; `yield_now` still left 1.1 s). A
/// fair hand-off (`parking_lot`'s `unlock_fair`) would need every one of
/// storage's lock sites changed; until then, this pause.
const CHUNK_PAUSE: std::time::Duration = std::time::Duration::from_millis(1);

/// The events grouped so that all of one session's are in the same chunk,
/// chunks filled up to about `size` (a session bigger than that is a chunk
/// of its own).
fn session_chunks(
    batch: &[juggler_core::quota::UsageEvent],
    size: usize,
) -> Vec<Vec<juggler_core::quota::UsageEvent>> {
    let mut by_session: BTreeMap<(String, Option<String>), Vec<juggler_core::quota::UsageEvent>> =
        BTreeMap::new();
    for event in batch {
        by_session
            .entry((event.account.clone(), event.session.clone()))
            .or_default()
            .push(event.clone());
    }
    let mut chunks: Vec<Vec<juggler_core::quota::UsageEvent>> = Vec::new();
    for events in by_session.into_values() {
        match chunks.last_mut() {
            Some(last) if last.len() + events.len() <= size => last.extend(events),
            _ => chunks.push(events),
        }
    }
    chunks
}

/// Claude: spool tail every pass; transcript scan every file-scan cadence;
/// its own limit records (`records`) when `instances` is given.
fn ingest_claude(
    account: &str,
    dir: &Path,
    spool: &Path,
    ingest: &mut IngestState,
    store: &mut Vec<juggler_core::quota::UsageEvent>,
    samples: &mut SampleBatch,
    instances: Option<&[PathBuf]>,
) {
    use juggler_core::quota::UsageSource;
    // The status line spool in AI Juggler's own folder: AI Juggler's
    // launches pass it with `--settings` (ADR-0018 §3); nothing is
    // installed into the account's folder, a copy's included.
    juggler_adapters::rotate_spool_if_needed(spool);
    let offsets = ingest.offsets.entry(account.to_owned()).or_default();
    let mut outcome = juggler_adapters::tail_spool(spool, offsets.spool);
    offsets.spool = outcome.offset;
    // A copy an earlier version wired to its own spool keeps it (its
    // files are left as they are): read it too while it's there.
    let legacy = juggler_adapters::spool_path(dir);
    if juggler_adapters::claude_code::is_managed_dir(dir) && legacy.exists() {
        let older = juggler_adapters::tail_spool(&legacy, offsets.legacy_spool);
        offsets.legacy_spool = older.offset;
        outcome.samples.extend(older.samples);
        outcome.parse_errors += older.parse_errors;
    }
    for (kind, sample) in outcome.samples {
        samples
            .windows
            .push((account.to_owned(), kind, sample, UsageSource::Statusline));
        record_health(ingest, account, "statusline", Some(sample.ts), 0, "");
    }
    if outcome.parse_errors > 0 {
        record_health(
            ingest,
            account,
            "statusline",
            None,
            outcome.parse_errors,
            "spool line",
        );
    }
    // Transcripts: <dir>/projects/**/*.jsonl, offset-tracked.
    let projects = dir.join("projects");
    let mut files = Vec::new();
    let _ = collect_files(&projects, &mut files, is_jsonl);
    forget_missing(ingest, account, &projects, &files);
    // Limit hits ride in the transcripts: Claude Code's own record of a
    // window running out, with the vendor's reset.
    for path in &files {
        let outcome = tail_one(
            account,
            "transcript",
            path,
            |path, account, offset, _| juggler_adapters::tail_transcript(path, account, offset),
            ingest,
            store,
        );
        for (kind, sample) in outcome.samples {
            samples
                .windows
                .push((account.to_owned(), kind, sample, UsageSource::ApiRetry));
        }
    }
    if let Some(instances) = instances {
        let records = &mut ingest
            .offsets
            .entry(account.to_owned())
            .or_default()
            .records;
        let now_ms = juggler_core::model::Timestamp::now().millis();
        crate::features::quota::records::look_back_for_hits(
            account, &files, now_ms, records, samples,
        );
        crate::features::quota::records::read_claude_records(
            account, dir, instances, records, samples,
        );
    }
}

/// Codex: rollout files under <dir>/sessions/**, offset-tracked. Their
/// `token_count` lines also carry Codex's own limit readings: Measured
/// samples, and the latest credits snapshot.
fn ingest_codex(
    account: &str,
    dir: &Path,
    ingest: &mut IngestState,
    store: &mut Vec<juggler_core::quota::UsageEvent>,
    samples: &mut SampleBatch,
    (credits, checks): (
        &mut Vec<(String, juggler_adapters::CreditsSnapshot)>,
        &mut Vec<CompactionCheck>,
    ),
) {
    let sessions = dir.join("sessions");
    let mut files = Vec::new();
    let complete = collect_files(
        &sessions,
        &mut files,
        juggler_adapters::codex::is_rollout_file,
    );
    forget_missing(ingest, account, &sessions, &files);
    // Only from a whole listing with rollouts in it: an empty, missing or
    // unreadable folder would make every thread look gone, and the check
    // is made once. Left for a pass that reads them.
    if !ingest.compactions_checked.contains(account) && complete && !files.is_empty() {
        // Every rollout still here, by thread; a name that doesn't give
        // its thread skips the check (its session can't be told present).
        let present: Option<std::collections::BTreeSet<String>> = files
            .iter()
            .map(|path| {
                path.file_name().and_then(|name| {
                    juggler_adapters::codex_transcript::thread_id_from_name(&name.to_string_lossy())
                })
            })
            .collect();
        if let Some(present) = present {
            checks.push((account.to_owned(), present));
        }
    }
    let mut latest: Option<juggler_adapters::CreditsSnapshot> = None;
    for path in files {
        let outcome = tail_one(
            account,
            "rollout",
            &path,
            juggler_adapters::tail_rollout,
            ingest,
            store,
        );
        for (kind, sample) in outcome.samples {
            samples.windows.push((
                account.to_owned(),
                kind,
                sample,
                juggler_core::quota::UsageSource::Rollout,
            ));
        }
        if let Some(seen) = outcome.credits
            && latest.as_ref().is_none_or(|kept| kept.ts <= seen.ts)
        {
            latest = Some(seen);
        }
    }
    if let Some(latest) = latest {
        credits.push((account.to_owned(), latest));
    }
}

/// `ZCode`: the whole `model_usage` table, read-only; the store's
/// uuid dedupe (`zcode:<rowid>`) makes the full read idempotent.
fn ingest_zcode(
    account: &str,
    dir: &Path,
    ingest: &mut IngestState,
    store: &mut Vec<juggler_core::quota::UsageEvent>,
) {
    let db = dir.join(".zcode").join("cli").join("db").join("db.sqlite");
    if !db.exists() {
        return;
    }
    match juggler_adapters::zcode_events(account, &db) {
        Ok(events) => {
            let latest = events.iter().map(|event| event.ts).max();
            let count = events.len();
            store.extend(events);
            if let Some(latest) = latest {
                record_health(ingest, account, "sqlite", Some(latest), 0, "");
            }
            let _ = count;
        }
        Err(error) => {
            record_health(ingest, account, "sqlite", None, 0, &error);
        }
    }
}

/// Reads one file's next slice: its path, account, offset and carry.
type Tail = fn(&Path, &str, u64, &juggler_adapters::RolloutCarry) -> juggler_adapters::TailOutcome;

/// Forgets the offsets and carries of an account's files that are no longer
/// on disk (deleted, or moved away), so the state doesn't grow with every
/// file there ever was. Only when `root` could be listed: a folder that is
/// gone or unreadable for a moment forgets nothing (else the next pass
/// would read every file again from its start).
fn forget_missing(ingest: &mut IngestState, account: &str, root: &Path, files: &[PathBuf]) {
    if std::fs::read_dir(root).is_err() {
        return;
    }
    let Some(offsets) = ingest.offsets.get_mut(account) else {
        return;
    };
    let present: std::collections::BTreeSet<String> = files
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect();
    offsets.files.retain(|path, _| present.contains(path));
    offsets.carries.retain(|path, _| present.contains(path));
}

/// The high-water key, in an account's offsets, of a store read by time.
const STORE_SEEN: &str = "store:updated_ms";

/// How far behind the high-water mark each store read starts: a message
/// written in the same millisecond as the last read is read again (its
/// key absorbs the repeat).
const STORE_OVERLAP_MS: u64 = 60_000;

/// A store read by time (AJ-E1602: OpenCode's `opencode.db`): what changed
/// since the last pass, through the account's adapter, read-only.
fn ingest_store(
    adapter: &dyn juggler_adapter::HarnessAdapter,
    account: &juggler_core::model::Account,
    ingest: &mut IngestState,
    store: &mut Vec<juggler_core::quota::UsageEvent>,
) {
    let id = account.id.as_str();
    let seen = ingest
        .offsets
        .get(id)
        .and_then(|offsets| offsets.files.get(STORE_SEEN).copied())
        .unwrap_or(0);
    let since = i64::try_from(seen.saturating_sub(STORE_OVERLAP_MS)).unwrap_or(0);
    let Some(read) = adapter.usage_since(account, since) else {
        return;
    };
    match read {
        Ok(events) => {
            let latest = events.iter().map(|event| event.ts).max();
            if let Some(latest) = latest.and_then(|ts| u64::try_from(ts).ok()) {
                ingest
                    .offsets
                    .entry(id.to_owned())
                    .or_default()
                    .files
                    .insert(STORE_SEEN.to_owned(), seen.max(latest));
            }
            store.extend(events);
            if let Some(latest) = latest {
                record_health(ingest, id, "store", Some(latest), 0, "");
            }
        }
        Err(error) => record_health(ingest, id, "store", None, 0, &error),
    }
}

/// Tails one file from its stored offset and carry: its events go to the
/// store, the offset, carry and health are recorded; the rest of the
/// outcome (samples, credits) is returned for the caller.
fn tail_one(
    account: &str,
    source: &str,
    path: &Path,
    tail: Tail,
    ingest: &mut IngestState,
    store: &mut Vec<juggler_core::quota::UsageEvent>,
) -> juggler_adapters::TailOutcome {
    let key = path.to_string_lossy().to_string();
    let offsets = ingest.offsets.entry(account.to_owned()).or_default();
    let offset = offsets.files.get(&key).copied().unwrap_or(0);
    let carry = offsets.carries.get(&key).cloned().unwrap_or_default();
    let mut outcome = tail(path, account, offset, &carry);
    if outcome.carry == juggler_adapters::RolloutCarry::default() {
        offsets.carries.remove(&key);
    } else {
        offsets.carries.insert(key.clone(), outcome.carry.clone());
    }
    offsets.files.insert(key, outcome.offset);
    let latest = outcome
        .events
        .iter()
        .map(|event| event.ts)
        .chain(outcome.samples.iter().map(|(_, sample)| sample.ts))
        .max();
    let errors = outcome.parse_errors;
    store.append(&mut outcome.events);
    if latest.is_some() || errors > 0 {
        record_health(ingest, account, source, latest, errors, "");
    }
    if let Some(reason) = outcome.unread {
        record_health(ingest, account, source, None, 1, reason);
    }
    if outcome.dropped > 0 {
        // Damage, counted like a bad line: usage that won't be counted.
        record_health(
            ingest,
            account,
            source,
            None,
            outcome.dropped,
            "compactions dropped before their thread was known",
        );
    }
    outcome
}

/// The sessions a re-read pass read again: every transcript/rollout
/// session in its batch, once.
fn reread_sessions(
    batch: &[juggler_core::quota::UsageEvent],
) -> Vec<(String, juggler_core::quota::UsageSource, String)> {
    use juggler_core::quota::UsageSource;
    let mut sessions = BTreeMap::new();
    for event in batch {
        if matches!(event.source, UsageSource::Transcript | UsageSource::Rollout)
            && let Some(session) = &event.session
        {
            sessions
                .entry((
                    event.account.clone(),
                    event.source.as_str(),
                    session.clone(),
                ))
                .or_insert(event.source);
        }
    }
    sessions
        .into_iter()
        .map(|((account, _, session), source)| (account, source, session))
        .collect()
}

/// Merges one observation into the health map.
fn record_health(
    ingest: &mut IngestState,
    account: &str,
    source: &str,
    last_event_at: Option<i64>,
    parse_errors: u64,
    note: &str,
) {
    let key = format!("{account}:{source}");
    let health = ingest.health.entry(key).or_default();
    if let Some(ts) = last_event_at {
        health.last_event_at = health.last_event_at.max(Some(ts));
    }
    health.parse_errors += parse_errors;
    if parse_errors > 0 {
        // AJ-E1103: one degraded read of this source (never its text).
        let (provider, area) = match source {
            "statusline" => ("claude-code", "quota_feed"),
            "transcript" => ("claude-code", "transcript"),
            "rollout" => ("codex", "transcript"),
            "sqlite" => ("zcode", "transcript"),
            _ => ("other", "transcript"),
        };
        crate::usage_stats::count(
            "usage.parse_degraded",
            &[("provider", provider), ("area", area)],
        );
    }
    if !note.is_empty() {
        // The note is diagnostic only (eprintln): health stays the
        // typed observable, never a stringly blob.
        eprintln!("ai-juggler: ingest {account}/{source}: {note}");
    }
}

/// One source's health, flattened for IPC (the AJ-E610 `Feed stale`
/// banner binds to this; `last_event_at: None` + zero errors = the
/// source has simply never fired).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize)]
pub struct SourceHealthInfo {
    /// Account id (storage string form).
    pub account: String,
    /// Source name (`statusline` | `transcript` | `rollout` | `sqlite`).
    pub source: String,
    /// Last ingested event time, epoch ms, when any.
    pub last_event_at: Option<i64>,
    /// Dropped malformed lines/rows since first sight (never resets).
    pub parse_errors: u64,
}

/// The health snapshot for every observed source (Req.5).
#[must_use]
pub fn source_health(state: &Arc<AppState>) -> Vec<SourceHealthInfo> {
    load_state(state)
        .health
        .into_iter()
        .map(|(key, health)| {
            let (account, source) = key.split_once(':').unwrap_or((key.as_str(), ""));
            SourceHealthInfo {
                account: account.to_owned(),
                source: source.to_owned(),
                last_event_at: health.last_event_at,
                parse_errors: health.parse_errors,
            }
        })
        .collect()
}

/// ADR-0018: an account's folder changed (a copy switched to its
/// original): drop its read offsets so the next pass reads the new place
/// from the start; the store's `(account, msg_uuid)` dedupe absorbs repeats.
pub fn forget_account_offsets(state: &Arc<AppState>, account: &str) {
    let mut ingest = load_state(state);
    if ingest.offsets.remove(account).is_none() {
        return;
    }
    if let (Ok(storage), Ok(blob)) = (state.storage.lock(), serde_json::to_string(&ingest)) {
        let _ = storage.app_settings().set(INGEST_STATE_KEY, &blob);
    }
}

/// Reads the persisted state (missing/corrupt ⇒ fresh — worst case a
/// one-time full re-tail whose duplicates the store dedupes).
fn load_state(state: &Arc<AppState>) -> IngestState {
    let Ok(storage) = state.storage.lock() else {
        return IngestState::default();
    };
    storage
        .app_settings()
        .get(INGEST_STATE_KEY)
        .ok()
        .flatten()
        .and_then(|blob| serde_json::from_str(&blob).ok())
        .unwrap_or_default()
}

/// The files under `root` (at most [`MAX_WALK_DEPTH`] folders down) that
/// `keep` accepts: the one walk for every source the pipeline tails. What a
/// source's files look like is its adapter's to say.
fn collect_files(root: &Path, out: &mut Vec<PathBuf>, keep: fn(&Path) -> bool) -> bool {
    walk(root, out, keep, 0)
}

/// `false` when the listing may be missing files: a folder that couldn't
/// be read (the root included), an entry that couldn't, or one deeper
/// than [`MAX_WALK_DEPTH`].
fn walk(root: &Path, out: &mut Vec<PathBuf>, keep: fn(&Path) -> bool, depth: u32) -> bool {
    if depth > MAX_WALK_DEPTH {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    let mut complete = true;
    for entry in entries {
        let Ok(entry) = entry else {
            complete = false;
            continue;
        };
        let path = entry.path();
        if path.is_dir() {
            complete &= walk(&path, out, keep, depth + 1);
        } else if keep(&path) {
            out.push(path);
        }
    }
    complete
}

/// A Claude transcript (`.jsonl`).
fn is_jsonl(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "jsonl")
}

const MAX_WALK_DEPTH: u32 = 5;

#[cfg(test)]
mod chunk_tests {
    #![allow(clippy::unwrap_used)]
    use super::session_chunks;
    use juggler_core::quota::{UsageEvent, UsageSource};

    fn event(session: &str, n: usize) -> UsageEvent {
        UsageEvent {
            account: "a".into(),
            session: Some(session.into()),
            ts: i64::try_from(n).unwrap(),
            model: None,
            tokens: juggler_core::model::TokenCounts::default(),
            credits: None,
            usd_est: None,
            source: UsageSource::Transcript,
            confidence: juggler_core::model::Confidence::Measured,
            msg_uuid: Some(format!("{session}-{n}")),
        }
    }

    /// #126: a screen waiting on storage gets it between two chunks, not
    /// after the whole pass (the lock isn't fair; without the pause it went
    /// straight back to the writer).
    #[test]
    fn a_reader_gets_storage_between_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let state = std::sync::Arc::new(
            crate::state::AppState::init_with_registry(
                dir.path().join("data"),
                juggler_adapter::AdapterRegistry::new(),
            )
            .unwrap(),
        );
        let batch: Vec<UsageEvent> = (0..super::STORE_CHUNK * 20)
            .map(|n| event("s", n))
            .collect();
        let reader = {
            let state = std::sync::Arc::clone(&state);
            std::thread::spawn(move || {
                // Once the first chunk is in, wait for storage like a screen.
                loop {
                    let stored = state
                        .storage
                        .lock()
                        .unwrap()
                        .usage()
                        .events_for_account("a", 0)
                        .unwrap()
                        .len();
                    if stored > 0 {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                state
                    .storage
                    .lock()
                    .unwrap()
                    .usage()
                    .events_for_account("a", 0)
                    .unwrap()
                    .len()
            })
        };
        super::store_in_chunks(&state, false, &batch).unwrap();
        let seen = reader.join().unwrap();
        assert!(seen < batch.len(), "the reader waited for all {seen} rows");
    }

    /// QA batch 9: a re-read is written a chunk at a time, and a session's
    /// rows are replaced in one chunk, never split across two.
    #[test]
    fn a_reread_is_chunked_without_splitting_a_session() {
        let mut batch = Vec::new();
        for (session, rows) in [("s1", 3), ("s2", 4), ("s3", 9), ("s4", 2)] {
            batch.extend((0..rows).map(|n| event(session, n)));
        }
        let chunks = session_chunks(&batch, 5);
        let sessions: Vec<Vec<&str>> = chunks
            .iter()
            .map(|chunk| {
                let mut ids: Vec<&str> = chunk
                    .iter()
                    .map(|e| e.session.as_deref().unwrap())
                    .collect();
                ids.dedup();
                ids
            })
            .collect();
        assert_eq!(sessions, [vec!["s1"], vec!["s2"], vec!["s3"], vec!["s4"]]);
        assert_eq!(chunks.iter().map(Vec::len).sum::<usize>(), batch.len());
        // Small sessions share a chunk up to its size.
        let small: Vec<UsageEvent> = ["x", "y", "z"]
            .iter()
            .flat_map(|s| (0..2).map(|n| event(s, n)))
            .collect();
        assert_eq!(
            session_chunks(&small, 4)
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [4, 2]
        );
    }
}

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
/// missing model).
const REREAD_VERSION: u32 = 1;

/// Quota samples gathered in one pass: (account, window, sample).
type SampleBatch = Vec<(
    String,
    juggler_core::quota::WindowKind,
    juggler_core::quota::QuotaSample,
)>;

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
    /// Per Codex rollout: the model its last `turn_context` named, for
    /// the usage lines of the next slice that come before their own.
    #[serde(default)]
    models: BTreeMap<String, String>,
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
}

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

/// One ingest pass (public for tests): accounts → sources → store.
pub fn ingest_pass(state: &Arc<AppState>, tick: u32) {
    let accounts = {
        let Ok(storage) = state.storage.lock() else {
            return;
        };
        let Ok(list) = storage.accounts().list() else {
            return;
        };
        list
    };
    let mut ingest = load_state(state);
    let rereading = ingest.reread < REREAD_VERSION;
    if rereading {
        for offsets in ingest.offsets.values_mut() {
            offsets.files.values_mut().for_each(|offset| *offset = 0);
            offsets.models.clear();
        }
    }
    let mut store_batch: Vec<juggler_core::quota::UsageEvent> = Vec::new();
    let mut sample_batch: SampleBatch = Vec::new();
    let mut credits_batch: Vec<(String, juggler_adapters::CreditsSnapshot)> = Vec::new();

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
                );
            }
            "codex" if rereading || tick.is_multiple_of(FILE_SCAN_EVERY) => {
                ingest_codex(
                    &account_id,
                    &dir,
                    &mut ingest,
                    &mut store_batch,
                    &mut sample_batch,
                    &mut credits_batch,
                );
            }
            "zcode" if tick.is_multiple_of(ZCODE_EVERY) => {
                ingest_zcode(&account_id, &dir, &mut ingest, &mut store_batch);
            }
            _ => {}
        }
    }

    // Single writer: ONE storage lock for the whole batch.
    if let Ok(storage) = state.storage.lock() {
        let usage = storage.usage();
        // Events and their offsets go together: when the write fails,
        // nothing of this pass is kept (offsets included), and the next
        // pass reads the same lines again.
        let stored = if rereading {
            // Every file still on disk was read from its start: its
            // sessions' old rows go and the rows just read replace them, in
            // one transaction. A session whose files are gone keeps the
            // rows it had.
            usage.replace_session_events(&reread_sessions(&store_batch), &store_batch)
        } else {
            usage.store_events(&store_batch)
        };
        let Ok(stored) = stored else {
            return;
        };
        if rereading {
            ingest.reread = REREAD_VERSION;
        } else {
            // AJ-E1103: Measured tokens this pass added (D-06): a new
            // response, or the rest of one an earlier pass began. A re-read
            // is not new use.
            crate::usage_stats::tokens_ingested(&accounts, &store_batch, &stored);
        }
        if usage.insert_samples(&sample_batch).is_err() {
            return;
        }
        for (account, credits) in &credits_batch {
            if let Ok(blob) = serde_json::to_string(credits) {
                let _ = storage
                    .app_settings()
                    .set(&format!("{CREDITS_KEY_PREFIX}{account}"), &blob);
            }
        }
        if let Ok(blob) = serde_json::to_string(&ingest) {
            let _ = storage.app_settings().set(INGEST_STATE_KEY, &blob);
        }
    }
}

/// Claude: spool tail every pass; transcript scan every file-scan cadence.
fn ingest_claude(
    account: &str,
    dir: &Path,
    spool: &Path,
    ingest: &mut IngestState,
    store: &mut Vec<juggler_core::quota::UsageEvent>,
    samples: &mut SampleBatch,
) {
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
        samples.push((account.to_owned(), kind, sample));
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
    collect_jsonl(&projects, &mut files);
    ingest_files(
        account,
        "transcript",
        files,
        juggler_adapters::tail_transcript,
        ingest,
        store,
    );
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
    credits: &mut Vec<(String, juggler_adapters::CreditsSnapshot)>,
) {
    let sessions = dir.join("sessions");
    let mut files = Vec::new();
    collect_jsonl(&sessions, &mut files);
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
            samples.push((account.to_owned(), kind, sample));
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

/// Tails a set of JSONL files with per-file offsets.
fn ingest_files(
    account: &str,
    source: &str,
    files: Vec<PathBuf>,
    tail: fn(&Path, &str, u64) -> juggler_adapters::TailOutcome,
    ingest: &mut IngestState,
    store: &mut Vec<juggler_core::quota::UsageEvent>,
) {
    for path in files {
        let _ = tail_one(account, source, &path, tail, ingest, store);
    }
}

/// Tails one file from its stored offset: its events go to the store,
/// the offset and health are recorded; the rest of the outcome (samples,
/// credits) is returned for the caller.
fn tail_one(
    account: &str,
    source: &str,
    path: &Path,
    tail: fn(&Path, &str, u64) -> juggler_adapters::TailOutcome,
    ingest: &mut IngestState,
    store: &mut Vec<juggler_core::quota::UsageEvent>,
) -> juggler_adapters::TailOutcome {
    let key = path.to_string_lossy().to_string();
    let offset = ingest
        .offsets
        .get(account)
        .and_then(|offsets| offsets.files.get(&key).copied())
        .unwrap_or(0);
    let mut outcome = tail(path, account, offset);
    let offsets = ingest.offsets.entry(account.to_owned()).or_default();
    // Codex: usage before the slice's first `turn_context` ran on the
    // model the file's previous slice ended with.
    if let Some(carried) = offsets.models.get(&key) {
        for event in outcome
            .events
            .iter_mut()
            .filter(|event| event.model.is_none())
        {
            event.model = Some(carried.clone());
        }
    }
    if let Some(model) = outcome.model.clone() {
        offsets.models.insert(key.clone(), model);
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

/// Collects `*.jsonl` files under `root` (depth-capped — vendor trees
/// are shallow; a runaway walk is not part of the budget).
fn collect_jsonl(root: &Path, out: &mut Vec<PathBuf>) {
    collect_jsonl_depth(root, out, 0);
}

const MAX_WALK_DEPTH: u32 = 5;

fn collect_jsonl_depth(root: &Path, out: &mut Vec<PathBuf>, depth: u32) {
    if depth > MAX_WALK_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_depth(&path, out, depth + 1);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "jsonl")
        {
            out.push(path);
        }
    }
}

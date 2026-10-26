//! AJ-E1529: every account's conversations, the ones AI Juggler started
//! and the ones it didn't, read from each tool's own files through its
//! adapter (`conversation_files`, `read_conversation`) and kept in memory.
//!
//! The index is keyed by file: a refresh lists each account's files, and
//! re-reads only those whose size or modification time changed. Nothing
//! is written: not into a setup, and not into AI Juggler's database
//! (there is no migration; a restart rebuilds it from the files).

pub mod asks;
pub mod commands;
pub mod dto;
pub mod goto;
pub mod last_messages;
pub mod live;
pub mod places;
pub mod procs;
pub mod scan;

/// Emitted when the conversations or which are live changed (the rail
/// reads them again). Its payload says which: [`CHANGED_INDEX`] or
/// [`CHANGED_LIVE`].
pub const EVENT_CHANGED: &str = "conversations://changed";

/// The files changed (new conversations, titles, folders, counts): the
/// app's own lists (projects, sessions) are read again too.
pub const CHANGED_INDEX: &str = "index";

/// Only which are open, and their states, changed: just the
/// conversations are read again (with many open sessions this is every
/// few seconds).
pub const CHANGED_LIVE: &str = "live";

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use juggler_adapter::conversation::ConversationSummary;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// One conversation in the index: what its file says, and whose it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversation {
    /// What the file says about itself.
    pub summary: ConversationSummary,
    /// The account whose folder it's in.
    pub account_id: String,
    /// That account's tool.
    pub tool: String,
    /// The file.
    pub path: PathBuf,
    /// Its folder resolved on this Mac (as a running process reports
    /// its own), or as recorded when it's gone.
    pub folder: Option<PathBuf>,
}

/// A file as last read: its size and modification time decide whether
/// it's read again.
#[derive(Clone, Debug)]
struct Entry {
    len: u64,
    modified: Option<SystemTime>,
    conversation: Option<Conversation>,
    /// Read whole (its tokens counted), not only its quick read.
    complete: bool,
}

/// Calls a pass's `progress` every `every` that read something.
struct Reporter<'a> {
    every: std::time::Duration,
    progress: &'a dyn Fn(),
    reported: std::time::Instant,
    unreported: bool,
}

impl<'a> Reporter<'a> {
    fn new(every: std::time::Duration, progress: &'a dyn Fn()) -> Self {
        Self {
            every,
            progress,
            reported: std::time::Instant::now(),
            unreported: false,
        }
    }

    /// A file was read.
    fn read(&mut self) {
        self.unreported = true;
    }

    /// Reports what's read so far, now.
    fn flush(&mut self) {
        if self.unreported {
            (self.progress)();
            self.reported = std::time::Instant::now();
            self.unreported = false;
        }
    }

    /// Reports what's read so far, when it's time.
    fn maybe(&mut self) {
        if self.unreported && self.reported.elapsed() >= self.every {
            (self.progress)();
            self.reported = std::time::Instant::now();
            self.unreported = false;
        }
    }
}

/// Puts one file's read into the index. A conversation that records no
/// time was last written when its file was.
fn store(
    state: &Arc<AppState>,
    account: &juggler_core::model::Account,
    path: &std::path::Path,
    len: u64,
    modified: Option<SystemTime>,
    summary: Option<ConversationSummary>,
    complete: bool,
) -> AppResult<()> {
    let written = modified
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|since| i64::try_from(since.as_millis()).ok());
    let conversation = summary.map(|mut summary| {
        summary.last_ms = summary.last_ms.or(written);
        Conversation {
            folder: summary
                .cwd
                .as_ref()
                .map(|cwd| std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.clone())),
            summary,
            account_id: account.id.to_string(),
            tool: account.harness.to_string(),
            path: path.to_path_buf(),
        }
    });
    let mut files = state
        .conversations
        .files
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let conversation =
        conversation.map(|conversation| carry_tokens(conversation, complete, files.get(path)));
    files.insert(
        path.to_path_buf(),
        Entry {
            len,
            modified,
            conversation,
            complete,
        },
    );
    Ok(())
}

/// A quick read (a changed file's start and end) counts no tokens: until
/// the whole file is read again, it keeps the count its last read had, so
/// a live session's usage never flips to "not measured" and back on every
/// pass (a transcript only grows, so the last count is a floor).
fn carry_tokens(
    mut conversation: Conversation,
    complete: bool,
    before: Option<&Entry>,
) -> Conversation {
    let counted = juggler_core::model::TokenCounts::default();
    if !complete
        && conversation.summary.tokens == counted
        && let Some(before) = before.and_then(|entry| entry.conversation.as_ref())
    {
        conversation.summary.tokens = before.summary.tokens;
    }
    conversation
}

/// The in-memory index (one per app).
#[derive(Debug, Default)]
pub struct ConversationIndex {
    files: Mutex<HashMap<PathBuf, Entry>>,
    /// `(account, tool, id)` of the conversations a running session is
    /// on, with where each is open (AJ-E2101).
    live: Mutex<HashMap<live::Live, scan::LiveEntry>>,
    /// Conversations in a running app's sidebar that no process runs.
    inactive: Mutex<std::collections::HashSet<live::Live>>,
    /// What is open now, as the live refresher last saw it.
    model: Mutex<scan::LiveModel>,
    /// The live refresher (its thread alone uses it).
    scanner: Mutex<scan::Scanner>,
    /// AJ-E2107: what each open terminal session's transcript tail last
    /// said it waits on.
    tails: Mutex<asks::Tails>,
    /// The profile folders it reads, refreshed with the accounts.
    profiles: Mutex<Option<(std::time::Instant, Vec<scan::Profile>)>>,
    /// Held while a pass reads the files: one pass at a time (QA batch 9:
    /// at a start, the background pass, the conversation list and the
    /// projects backfill each read every file at once).
    pass: Mutex<()>,
}

impl ConversationIndex {
    /// A pass is reading the tools' files now (AJ-E1538's builder waits).
    #[must_use]
    pub fn is_reading(&self) -> bool {
        self.pass.try_lock().is_err()
    }

    /// Every conversation now in the index, one per account and id,
    /// newest activity first.
    ///
    /// # Errors
    ///
    /// [`AppError::StatePoisoned`] when the index's lock is poisoned.
    pub fn all(&self) -> AppResult<Vec<Conversation>> {
        let files = self.files.lock().map_err(|_| AppError::StatePoisoned)?;
        let mut out = merged(
            files
                .values()
                .filter_map(|entry| entry.conversation.clone()),
        );
        out.sort_by(|a, b| {
            b.summary
                .last_ms
                .cmp(&a.summary.last_ms)
                .then_with(|| a.path.cmp(&b.path))
        });
        Ok(out)
    }

    /// AJ-E2108: the file that stands for `account`'s conversation `id`
    /// (the newest of its files, as `merged` keeps), without copying
    /// the index.
    #[must_use]
    pub fn path_of(&self, account: &str, id: &str) -> Option<PathBuf> {
        let files = self.files.lock().ok()?;
        files
            .values()
            .filter_map(|entry| entry.conversation.as_ref())
            .filter(|c| c.account_id == account && c.summary.id == id)
            .max_by(|a, b| (a.summary.last_ms, &a.path).cmp(&(b.summary.last_ms, &b.path)))
            .map(|c| c.path.clone())
    }

    /// Whether a running session is on `account`'s `(tool, id)`.
    #[must_use]
    pub fn is_live(&self, account: &str, tool: &str, id: &str) -> bool {
        self.live.lock().is_ok_and(|live| {
            live.contains_key(&(account.to_owned(), tool.to_owned(), id.to_owned()))
        })
    }

    /// Where `account`'s `(tool, id)` is open now, and its state.
    #[must_use]
    pub fn live_entry(&self, account: &str, tool: &str, id: &str) -> Option<scan::LiveEntry> {
        self.live.lock().ok().and_then(|live| {
            live.get(&(account.to_owned(), tool.to_owned(), id.to_owned()))
                .cloned()
        })
    }

    /// Whether `account`'s `(tool, id)` is in a running app's sidebar with
    /// no process on it.
    #[must_use]
    pub fn is_inactive(&self, account: &str, tool: &str, id: &str) -> bool {
        self.inactive.lock().is_ok_and(|inactive| {
            inactive.contains(&(account.to_owned(), tool.to_owned(), id.to_owned()))
        })
    }

    /// Each verified session pid's profile folder (This Mac's tools and
    /// first run read which setup a process uses from this, never from its
    /// environment).
    #[must_use]
    pub fn pid_dirs(&self) -> std::collections::BTreeMap<u32, PathBuf> {
        self.model
            .lock()
            .map(|model| model.pid_dirs.clone())
            .unwrap_or_default()
    }
}

/// One conversation per `(account, tool, id)`. A resumed or forked thread
/// can be several files with one id (Codex writes a rollout per resume,
/// repeating its parent's rows): the newest file stands for it, from the
/// earliest start, with the most tokens any one file holds (never a sum,
/// which would count the repeated rows twice).
fn merged(conversations: impl Iterator<Item = Conversation>) -> Vec<Conversation> {
    let total = |c: &Conversation| {
        let t = &c.summary.tokens;
        t.input + t.output + t.cache_read + t.cache_write + t.reasoning
    };
    let mut by_id: HashMap<(String, String, String), Conversation> = HashMap::new();
    for conversation in conversations {
        let key = (
            conversation.account_id.clone(),
            conversation.tool.clone(),
            conversation.summary.id.clone(),
        );
        let Some(kept) = by_id.remove(&key) else {
            by_id.insert(key, conversation);
            continue;
        };
        let newer =
            (conversation.summary.last_ms, &conversation.path) > (kept.summary.last_ms, &kept.path);
        let (mut keep, other) = if newer {
            (conversation, kept)
        } else {
            (kept, conversation)
        };
        keep.summary.started_ms = match (keep.summary.started_ms, other.summary.started_ms) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        if total(&other) > total(&keep) {
            keep.summary.tokens = other.summary.tokens;
        }
        keep.summary.title = keep.summary.title.or(other.summary.title);
        by_id.insert(key, keep);
    }
    by_id.into_values().collect()
}

/// AJ-E1536: AI Juggler's sessions with their conversations' titles (the
/// tool's summary, else the first thing asked), so a session reads as
/// what it was about wherever it shows, as one started outside does.
///
/// # Errors
///
/// [`AppError::StatePoisoned`] when the index's lock is poisoned.
pub fn titled(
    state: &Arc<AppState>,
    mut rows: Vec<crate::commands::InstanceInfo>,
) -> AppResult<Vec<crate::commands::InstanceInfo>> {
    let titles: HashMap<(String, String), String> = state
        .conversations
        .all()?
        .into_iter()
        .filter_map(|c| Some(((c.account_id, c.summary.id), c.summary.title?)))
        .collect();
    for row in &mut rows {
        if let (Some(account), Some(conversation)) = (&row.account_id, &row.conversation_id) {
            row.title = titles
                .get(&(account.clone(), conversation.clone()))
                .cloned();
        }
    }
    Ok(rows)
}

/// One refresh of the files and of which conversations are live; `true`
/// when anything the UI shows changed.
///
/// # Errors
///
/// As [`refresh_impl`].
pub fn tick(state: &Arc<AppState>) -> AppResult<bool> {
    tick_with(state, PROGRESS_EVERY, &|| {})
}

/// [`tick`], telling `progress` while a long pass is still reading (QA,
/// batch 8: a first pass over a heavy setup takes minutes). Every
/// `every` (the app uses [`PROGRESS_EVERY`]) that read something, which
/// conversations are live
/// is worked out again from what's read so far, and `progress` runs; the
/// newest files are read first, so a running session shows within
/// seconds, not after the whole pass.
///
/// # Errors
///
/// As [`refresh_impl`].
pub fn tick_with(
    state: &Arc<AppState>,
    every: std::time::Duration,
    progress: &dyn Fn(),
) -> AppResult<bool> {
    // The accounts once, up front: the pass's progress never waits on
    // storage (QA batch 9: it waited behind the usage pipeline's first
    // write, and nothing showed as running for over a minute).
    let accounts = lock_storage_arc(state)?.accounts().list()?;
    let files_changed = {
        let _pass = state
            .conversations
            .pass
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        refresh_with(state, &accounts, every, &|| {
            if update_live(state).is_ok() {
                progress();
            }
        })?
    };
    if files_changed {
        // New folders become projects without the UI asking.
        crate::commands::projects_backfill_impl(state)?;
    }
    let live_changed = update_live(state)?;
    Ok(files_changed || live_changed)
}

/// How often a long pass reports what it has read so far.
pub const PROGRESS_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// Which conversations are live now, from the live refresher's last look
/// and what's indexed; `true` when that changed.
fn update_live(state: &Arc<AppState>) -> AppResult<bool> {
    let model = state
        .conversations
        .model
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .clone();
    let all = state.conversations.all()?;
    let mut now = live::live_entries(&all, &model);
    // AJ-E2107: a terminal session's tail can say it waits on the user;
    // a session its tool records no state for takes its tail's last turn.
    {
        let mut tails = state
            .conversations
            .tails
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        let say = |tool: &str, tail: &str| {
            state
                .registry
                .get(&juggler_core::model::HarnessId::new(tool.to_owned()))
                .map(|adapter| asks::Said {
                    ask: adapter.pending_ask(tail),
                    turn: adapter.tail_turn(tail),
                })
                .unwrap_or_default()
        };
        asks::apply(&mut now, &all, &mut tails, &say, SystemTime::now());
    }
    let idle = live::inactive_ids(&all, &model);
    let changed = {
        let mut live = state
            .conversations
            .live
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        let changed = *live != now;
        *live = now;
        changed
    };
    let mut inactive = state
        .conversations
        .inactive
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let idle_changed = *inactive != idle;
    *inactive = idle;
    Ok(changed || idle_changed)
}

/// How often the profile folders are listed again from the accounts.
const PROFILES_EVERY: std::time::Duration = std::time::Duration::from_secs(15);

/// The folders whose per-session files are read: every account's, the
/// folders accounts were copied from, and each tool's own default.
fn profiles(state: &Arc<AppState>) -> Vec<scan::Profile> {
    let now = std::time::Instant::now();
    if let Ok(cached) = state.conversations.profiles.lock()
        && let Some((at, profiles)) = cached.as_ref()
        && now.saturating_duration_since(*at) < PROFILES_EVERY
    {
        return profiles.clone();
    }
    // Never waits on storage: a busy lock keeps the last list.
    let accounts = match state.storage.try_lock() {
        Ok(storage) => storage.accounts().list().unwrap_or_default(),
        Err(_) => {
            return state
                .conversations
                .profiles
                .lock()
                .ok()
                .and_then(|cached| cached.as_ref().map(|(_, profiles)| profiles.clone()))
                .unwrap_or_default();
        }
    };
    let canonical =
        |path: &std::path::Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out: Vec<scan::Profile> = Vec::new();
    let mut add = |tool: &str, dir: &std::path::Path, account: Option<String>| {
        let dir = canonical(dir);
        if !out
            .iter()
            .any(|known| known.tool == tool && known.dir == dir)
        {
            out.push(scan::Profile {
                tool: tool.to_owned(),
                dir,
                account_id: account,
            });
        }
    };
    for account in &accounts {
        add(
            account.harness.as_str(),
            &account.state_dir,
            Some(account.id.to_string()),
        );
    }
    for account in &accounts {
        if let Some(source) = account.import_source_path.as_deref() {
            add(account.harness.as_str(), std::path::Path::new(source), None);
        }
    }
    for id in state.registry.ids() {
        if let Some(dir) = state.registry.get(&id).and_then(|a| a.vendor_default_dir()) {
            add(id.as_str(), &dir, None);
        }
    }
    if let Ok(mut cached) = state.conversations.profiles.lock() {
        *cached = Some((now, out.clone()));
    }
    out
}

/// AJ-E2101: one look at what is open now (the app's live thread calls
/// this every [`scan::TICK`]); `true` when what the UI shows changed.
///
/// # Errors
///
/// A poisoned lock.
pub fn live_tick(state: &Arc<AppState>) -> AppResult<bool> {
    let adapters: Vec<(String, juggler_adapter::SharedAdapter)> = state
        .registry
        .ids()
        .into_iter()
        .filter_map(|id| {
            state
                .registry
                .get(&id)
                .map(|adapter| (id.to_string(), adapter))
        })
        .collect();
    let profiles = profiles(state);
    let home = dirs::home_dir().unwrap_or_default();
    let model = state
        .conversations
        .scanner
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .tick(
            &scan::RealOs,
            &adapters,
            &profiles,
            &home,
            std::time::Instant::now(),
        );
    {
        let mut known = state
            .conversations
            .model
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        if *known == model {
            // Nothing new is open, but a tail can have gone quiet long
            // enough to ask you (AJ-E2107).
            drop(known);
            return update_live(state);
        }
        *known = model;
    }
    update_live(state)?;
    Ok(true)
}

/// Brings the index up to date with every account's files: new and
/// changed files are read, gone ones dropped. `true` when anything
/// changed (the UI is told then, and only then).
///
/// # Errors
///
/// Storage errors listing the accounts; a poisoned lock.
pub fn refresh_impl(state: &Arc<AppState>) -> AppResult<bool> {
    let accounts = lock_storage_arc(state)?.accounts().list()?;
    let _pass = state
        .conversations
        .pass
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    refresh_with(state, &accounts, PROGRESS_EVERY, &|| {})
}

/// The first read, for a command that finds the index empty: done here
/// unless a pass is already reading (then the command answers with what
/// is read so far, and the pass tells the UI as it goes) — a command never
/// waits minutes on a cold pass.
///
/// # Errors
///
/// As [`refresh_impl`].
pub fn read_if_cold(state: &Arc<AppState>) -> AppResult<()> {
    if !state.conversations.all()?.is_empty() {
        return Ok(());
    }
    let accounts = lock_storage_arc(state)?.accounts().list()?;
    let pass = match state.conversations.pass.try_lock() {
        Ok(pass) => pass,
        Err(std::sync::TryLockError::WouldBlock) => return Ok(()),
        Err(std::sync::TryLockError::Poisoned(_)) => return Err(AppError::StatePoisoned),
    };
    refresh_with(state, &accounts, PROGRESS_EVERY, &|| {})?;
    drop(pass);
    Ok(())
}

/// [`refresh_impl`], newest files first, calling `progress` every
/// `every` that read something.
fn refresh_with(
    state: &Arc<AppState>,
    accounts: &[juggler_core::model::Account],
    every: std::time::Duration,
    progress: &dyn Fn(),
) -> AppResult<bool> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut changed = false;
    // Every account's files, listed first (cheap), then read newest
    // first: a running session's file is among the newest.
    let mut found = Vec::new();
    for account in accounts {
        let Some(adapter) = state.registry.get(&account.harness) else {
            continue;
        };
        for path in adapter.conversation_files(&account.state_dir) {
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            found.push((
                account,
                std::sync::Arc::clone(&adapter),
                path,
                meta.len(),
                meta.modified().ok(),
            ));
        }
    }
    found.sort_by_key(|entry| std::cmp::Reverse(entry.4));
    let mut report = Reporter::new(every, progress);
    // Two rounds, newest first (QA batch 10: one Mac's Codex rollouts were
    // 6.6 GB, and reading each whole kept everything from showing for over
    // a minute). First every new or changed file's quick read (its start
    // and end) where its tool has one, so the list and what's live come
    // within seconds; then the whole of each file read quickly, for its
    // tokens. A file read whole in the first round is complete.
    let mut later = Vec::new();
    for (account, adapter, path, len, modified) in &found {
        report.maybe();
        seen.push(path.clone());
        let entry = state
            .conversations
            .files
            .lock()
            .map_err(|_| AppError::StatePoisoned)?
            .get(path)
            .map(|entry| {
                (
                    entry.len == *len && entry.modified == *modified,
                    entry.complete,
                )
            });
        match entry {
            Some((true, true)) => continue,
            Some((true, false)) => {
                later.push((account, adapter, path, len, modified));
                continue;
            }
            _ => {}
        }
        // Read outside the lock: a big transcript never holds up a reader.
        let (summary, complete) = match adapter.read_conversation_quick(path) {
            Some(summary) => {
                later.push((account, adapter, path, len, modified));
                (Some(summary), false)
            }
            None => (adapter.read_conversation(path), true),
        };
        store(state, account, path, *len, *modified, summary, complete)?;
        changed = true;
        report.read();
    }
    // What's listed and live, at once, before the long round.
    report.flush();
    for (account, adapter, path, len, modified) in later {
        report.maybe();
        let summary = adapter.read_conversation(path);
        store(state, account, path, *len, *modified, summary, true)?;
        changed = true;
        report.read();
    }
    let mut files = state
        .conversations
        .files
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let before = files.len();
    let seen: std::collections::HashSet<PathBuf> = seen.into_iter().collect();
    files.retain(|path, _| seen.contains(path));
    Ok(changed || files.len() != before)
}

/// The conversation `id` on `account_id`, from the index.
fn find(state: &Arc<AppState>, account_id: &str, id: &str) -> AppResult<Conversation> {
    state
        .conversations
        .all()?
        .into_iter()
        .find(|conversation| conversation.account_id == account_id && conversation.summary.id == id)
        .ok_or_else(|| AppError::Refused("that conversation isn't there any more".to_owned()))
}

/// AJ-E1540: the file that holds `account`'s conversation `id`, for a
/// session that knows which conversation it is (one picked up with
/// "Continue in AI Juggler", or recorded from outside) but has no
/// transcript link yet. The index first; then the tool's own files, those
/// whose name carries the id checked first, each confirmed by reading its
/// id (never by its name alone). Files in `taken` belong to other sessions
/// and are skipped. Read-only.
pub(crate) fn transcript_of(
    state: &Arc<AppState>,
    account: &juggler_adapter::model::Account,
    id: &str,
    taken: &[PathBuf],
) -> Option<juggler_core::model::TranscriptRef> {
    let reference = |path: PathBuf| juggler_core::model::TranscriptRef {
        path,
        native_id: Some(id.to_owned()),
    };
    if let Ok(all) = state.conversations.all()
        && let Some(found) = all.into_iter().find(|conversation| {
            conversation.account_id == account.id.as_str()
                && conversation.summary.id == id
                && !taken.contains(&conversation.path)
        })
    {
        return Some(reference(found.path));
    }
    let adapter = state.registry.get(&account.harness)?;
    let holds_it = |path: &PathBuf| {
        adapter
            .read_conversation_quick(path)
            .or_else(|| adapter.read_conversation(path))
            .is_some_and(|summary| summary.id == id)
    };
    let (named, others): (Vec<PathBuf>, Vec<PathBuf>) = adapter
        .conversation_files(&account.state_dir)
        .into_iter()
        .filter(|path| !taken.contains(path))
        .partition(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().contains(id))
        });
    // Both tools put the id in the file name. A tool that writes a new file
    // on every resume (Codex) has several: the newest holds the latest turns.
    let modified = |path: &PathBuf| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    };
    if let Some(newest) = named
        .into_iter()
        .filter(|path| holds_it(path))
        .max_by_key(|path| modified(path))
    {
        return Some(reference(newest));
    }
    others
        .into_iter()
        .find(|path| holds_it(path))
        .map(reference)
}

/// The most conversation files one replay lookup reads.
const PRIMED_SCAN_MAX: usize = 200;

/// How much of a title must match a replay's header to count as its
/// opening (a short title says too little to tell).
const OPENING_MIN: usize = 24;

/// AJ-E1541 (lab 8): the conversation a replay started, when finding it by
/// time alone is ambiguous (another session of the same tool began in the
/// same folder since). A replay's first message opens with `header`, the
/// target tool's replay header: the one conversation in `cwd` that began
/// after `after_ms` and opens with it is the one. Only files changed since
/// then are read, so a large history costs a directory listing. Several
/// matches (two replays into one folder at once) find nothing: never a
/// guess. Read-only.
pub(crate) fn primed_conversation(
    state: &Arc<AppState>,
    account: &juggler_adapter::model::Account,
    cwd: &std::path::Path,
    after_ms: i64,
    header: &str,
    taken: &[PathBuf],
) -> Option<juggler_core::model::TranscriptRef> {
    let header = header.trim();
    if header.is_empty() {
        return None;
    }
    let adapter = state.registry.get(&account.harness)?;
    let since = std::time::UNIX_EPOCH
        + std::time::Duration::from_millis(u64::try_from(after_ms).unwrap_or(0));
    // Newest first, and only so many: a session started days ago can't
    // make this read a large history whole (review of #132).
    let mut recent: Vec<(std::time::SystemTime, PathBuf)> = adapter
        .conversation_files(&account.state_dir)
        .into_iter()
        .filter(|path| !taken.contains(path))
        .filter_map(|path| {
            let modified = std::fs::metadata(&path)
                .and_then(|meta| meta.modified())
                .ok()?;
            (modified >= since).then_some((modified, path))
        })
        .collect();
    // More than the cap could hide a second match: find nothing rather
    // than guess (re-review of #132).
    if recent.len() > PRIMED_SCAN_MAX {
        return None;
    }
    recent.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    let mut found = recent.into_iter().map(|(_, path)| path).filter_map(|path| {
        let summary = adapter
            .read_conversation_quick(&path)
            .or_else(|| adapter.read_conversation(&path))?;
        let opens = summary.title.as_deref().is_some_and(|title| {
            // A title is the first line, cut at a length with "…" added.
            let title = title.trim().trim_end_matches('…').trim_end();
            title.chars().count() >= OPENING_MIN.min(header.chars().count())
                && (header.starts_with(title) || title.starts_with(header))
        });
        let here = summary.cwd.as_deref() == Some(cwd);
        let after = summary.started_ms.is_some_and(|at| at >= after_ms);
        (opens && here && after).then_some(juggler_core::model::TranscriptRef {
            path,
            native_id: Some(summary.id),
        })
    });
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

/// The copyable line that resumes a conversation started outside AI
/// Juggler: its tool's adapter builds it, on its account's folder, from
/// its own folder.
///
/// # Errors
///
/// [`AppError::Refused`] when it's gone or its tool can't resume from a
/// terminal; storage errors.
pub fn resume_command_impl(state: &Arc<AppState>, account_id: &str, id: &str) -> AppResult<String> {
    let conversation = find(state, account_id, id)?;
    let account = crate::commands::find_account_arc(state, account_id)?;
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    let cwd = conversation.summary.cwd.clone().unwrap_or_default();
    juggler_adapter::adapter::shell_resume_snippet(adapter.as_ref(), &account.state_dir, id, &cwd)
        .ok_or_else(|| {
            AppError::Refused(format!(
                "{} can't resume a session from a terminal",
                adapter.spec().display_name
            ))
        })
}

/// "Continue in AI Juggler": resumes a conversation started outside AI
/// Juggler as a session of its own, on the same account, in the same
/// folder, with that account's first launch settings. Only on the user's
/// click, and refused while a session is on it elsewhere (two writers on
/// one conversation would tangle it).
///
/// # Errors
///
/// [`AppError::Refused`] in words: live elsewhere, its folder gone, no
/// launch settings; launch and storage errors.
pub fn continue_impl(
    state: &Arc<AppState>,
    account_id: &str,
    id: &str,
) -> AppResult<crate::commands::LaunchResult> {
    use juggler_core::model::{ConversationId, InstanceKind, SessionId};
    let conversation = find(state, account_id, id)?;
    if state
        .conversations
        .is_live(account_id, &conversation.tool, id)
    {
        return Err(AppError::Refused(
            "it's open in another session now: close it there first, then continue here".to_owned(),
        ));
    }
    let cwd = conversation
        .folder
        .clone()
        .filter(|folder| folder.is_dir())
        .ok_or_else(|| AppError::Refused("its folder isn't there any more".to_owned()))?;
    let account = crate::commands::find_account_arc(state, account_id)?;
    let (profile, project) = {
        let storage = lock_storage_arc(state)?;
        let profile = storage
            .profiles()
            .list_by_account(&account.id)?
            .into_iter()
            .next()
            .ok_or_else(|| {
                AppError::Refused(format!(
                    "{} has no launch settings yet: add them first",
                    account.label
                ))
            })?;
        let homes = storage.projects().worktree_homes()?;
        let project = storage.projects().touch(
            &juggler_core::storage::project_root_with(&cwd, &homes),
            None,
        )?;
        (profile, project)
    };
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.to_string()))?;
    let session = SessionId::new(format!("sess-{}", crate::commands::unique_suffix()));
    let spec = adapter.launch_spec(
        &account,
        &profile,
        &cwd,
        &juggler_adapter::LaunchMode::Resume {
            session_id: id.to_owned(),
            delta_prompt: None,
        },
    )?;
    // Intent before spawn (ADR-0010 §1): the row first, then the process.
    let spawned = crate::features::sessions::spawn::spawn_tracked(
        state,
        &crate::features::sessions::spawn::NewSession {
            id: &session,
            account: &account,
            profile: &profile.id,
            kind: InstanceKind::Terminal,
            project: Some(project.id.clone()),
            cwd,
            conversation: Some(ConversationId::new(id)),
            transcript_ref: None,
            origin: "resume",
        },
        || {
            let ctx = crate::commands::ledger_ctx(state, &account, Some(project.id.to_string()));
            Ok(state.supervisor.spawn(&session, &spec, &ctx)?)
        },
    )?;
    Ok(crate::commands::LaunchResult {
        session_id: session.to_string(),
        pid: spawned.pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, id: &str, started: i64, last: i64, input: u64) -> Conversation {
        let mut summary = ConversationSummary {
            id: id.to_owned(),
            cwd: None,
            git_branch: None,
            title: None,
            started_ms: Some(started),
            last_ms: Some(last),
            tokens: juggler_core::model::TokenCounts::default(),
        };
        summary.tokens.input = input;
        Conversation {
            summary,
            account_id: "a".to_owned(),
            tool: "t".to_owned(),
            path: PathBuf::from(path),
            folder: None,
        }
    }

    #[test]
    fn a_thread_written_to_several_files_is_one_conversation() {
        let all = merged(
            [
                file("first.jsonl", "thread", 100, 200, 50),
                file("resumed.jsonl", "thread", 300, 400, 80),
                file("other.jsonl", "other", 100, 150, 5),
            ]
            .into_iter(),
        );
        assert_eq!(all.len(), 2);
        let thread = all.iter().find(|c| c.summary.id == "thread").unwrap();
        assert_eq!(thread.path, PathBuf::from("resumed.jsonl"));
        assert_eq!(
            (thread.summary.started_ms, thread.summary.last_ms),
            (Some(100), Some(400))
        );
        // The resumed file repeats its parent's rows: the most, never the sum.
        assert_eq!(thread.summary.tokens.input, 80);
    }

    fn entry(conversation: Conversation, complete: bool) -> Entry {
        Entry {
            len: 1,
            modified: None,
            conversation: Some(conversation),
            complete,
        }
    }

    #[test]
    fn a_quick_read_keeps_the_last_count_until_the_whole_read() {
        let before = entry(file("live.jsonl", "live", 100, 200, 500), true);
        // A live session's file changed: its quick read counts nothing.
        let quick = carry_tokens(
            file("live.jsonl", "live", 100, 300, 0),
            false,
            Some(&before),
        );
        assert_eq!(
            quick.summary.tokens.input, 500,
            "never back to not measured"
        );
        assert_eq!(
            quick.summary.last_ms,
            Some(300),
            "the rest is the new read's"
        );
        // The whole read's count is the one kept, even a smaller one.
        let whole = carry_tokens(file("live.jsonl", "live", 100, 300, 0), true, Some(&before));
        assert_eq!(whole.summary.tokens.input, 0);
        // A new file has nothing to keep.
        let new = carry_tokens(file("new.jsonl", "new", 100, 300, 0), false, None);
        assert_eq!(new.summary.tokens.input, 0);
    }
}

//! AJ-E1529: every account's conversations, the ones AI Juggler started
//! and the ones it didn't, read from each tool's own files through its
//! adapter (`conversation_files`, `read_conversation`) and kept in memory.
//!
//! The index is keyed by file: a refresh lists each account's files, and
//! re-reads only those whose size or modification time changed. Nothing
//! is written: not into a setup, and not into AI Juggler's database
//! (there is no migration; a restart rebuilds it from the files).

pub mod archive;
pub mod asks;
pub mod by_relation;
pub mod commands;
pub mod dto;
pub mod feed;
mod folders;
pub mod gate;
pub mod goto;
pub mod last_messages;
pub mod list_build;
pub mod live;
mod pass;
pub mod places;
pub mod procs;
pub mod saved;
pub mod scan;
pub mod switch_source;

use folders::profiles;

/// Emitted when the conversations or which are live changed (the rail
/// reads them again). Its payload says which: [`CHANGED_INDEX`] or
/// [`CHANGED_LIVE`].
pub const EVENT_CHANGED: &str = AppEvent::ConversationsChanged.name();

/// The files changed (new conversations, titles, folders, counts): the
/// app's own lists (projects, sessions) are read again too.
pub const CHANGED_INDEX: &str = "index";

/// Only which are open, and their states, changed: just the
/// conversations are read again (with many open sessions this is every
/// few seconds).
pub const CHANGED_LIVE: &str = "live";

use crate::events::AppEvent;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use juggler_adapter::conversation::ConversationSummary;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// One conversation in the index: what its file says, and whose it is.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    // A lab using a real tool folder where it is (ADR-0027 condition 7)
    // indexes only its own conversations there: the rest is known, never
    // listed (and never read again until it changes).
    let hidden = summary
        .as_ref()
        .is_some_and(|summary| crate::lab_scope::hides(&account.state_dir, summary.cwd.as_deref()));
    let complete = complete || hidden;
    let summary = summary.filter(|_| !hidden);
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
    let before = files.get(path);
    note_placed(
        &state.conversations,
        before.and_then(|entry| entry.conversation.as_ref()),
        conversation.as_ref(),
    );
    let conversation =
        conversation.map(|conversation| carry_tokens(conversation, complete, before));
    files.insert(
        path.to_path_buf(),
        Entry {
            len,
            modified,
            conversation,
            complete,
        },
    );
    drop(files);
    crate::features::relations::changed(
        state,
        crate::features::relations::Change::ConversationsIndex,
    );
    Ok(())
}

/// AJ-E805 (round 3): what one read did to the folders ([`Placed`]): a
/// conversation new to the index or in another folder changes them; one
/// that only went on is touched.
fn note_placed(index: &ConversationIndex, was: Option<&Conversation>, now: Option<&Conversation>) {
    let (Some(was), Some(now)) = (was, now) else {
        if was.is_some() || now.is_some() {
            index.placed_changed();
        }
        return;
    };
    if was.folder != now.folder || was.summary.id != now.summary.id {
        index.placed_changed();
    } else if let (Some(folder), Some(at)) = (&now.folder, now.summary.last_ms)
        && was.summary.last_ms.is_none_or(|before| at > before)
        && let Ok(mut placed) = index.placed.lock()
    {
        placed.touched.push((folder.clone(), at));
    }
}

/// A quick read (a big file's start and end, before its reader has read
/// it: 0.4.5, AJ-E805) counts no tokens: until its reader has, it keeps
/// the count its last read had, so
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
    /// The profile folders it reads, refreshed with the accounts: when
    /// listed, the folders' stamp then, and the list.
    profiles: Mutex<Option<folders::Listed>>,
    /// Held while a pass reads the files: one pass at a time (QA batch 9:
    /// at a start, the background pass, the conversation list and the
    /// projects backfill each read every file at once).
    pass: Mutex<()>,
    /// Whether the saved index was restored (once, before the first pass),
    /// and when it was last saved.
    saved: Mutex<saved::Kept>,
    /// The list has been read whole once since the start: a pass's first
    /// round listed every file and read each new one (its second round
    /// only counts tokens).
    first_done: std::sync::atomic::AtomicBool,
    /// The app's background pass reads the files (after the first paint,
    /// at the lowest priority): a command never reads them itself then.
    background: std::sync::atomic::AtomicBool,
    /// How far the first build is, and whether its last pass failed.
    build: list_build::Progress,
    /// AJ-E805: which files the background pass reads (made at its first).
    gate: Mutex<Option<gate::Gate>>,
    /// AJ-E805 (round 3): what the reads since the projects were last
    /// brought up to date did to the conversations' folders.
    placed: Mutex<Placed>,
    /// AJ-E2311 item 6: the processes that ended since the live look now
    /// running began (or while none ran): its result leaves them out, and
    /// the next look's scanner forgets them.
    ended: Mutex<std::collections::BTreeSet<u32>>,
}

/// What reads did to the index's folders since [`ConversationIndex::take_placed`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Placed {
    /// A conversation came, went, or changed folder: the projects are
    /// worked out again (`projects_backfill_impl`).
    pub changed: bool,
    /// Known conversations that only went on (lines appended): each one's
    /// folder and its newest activity, for its project's last use.
    pub touched: Vec<(PathBuf, i64)>,
}

impl ConversationIndex {
    /// Forgets every file indexed under `folder`, so the next pass reads
    /// them again (a lab adopting a real folder, ADR-0027 condition 7:
    /// whatever was indexed before its scope held is read again with it).
    pub fn forget_under(&self, folder: &std::path::Path) {
        if let Ok(mut files) = self.files.lock() {
            files.retain(|path, _| !path.starts_with(folder));
        }
    }

    /// What reads did to the folders since the last call ([`Placed`]),
    /// and nothing since.
    pub fn take_placed(&self) -> Placed {
        self.placed
            .lock()
            .map(|mut placed| std::mem::take(&mut *placed))
            .unwrap_or_default()
    }

    /// A conversation came, went or moved folder (see [`Placed`]).
    fn placed_changed(&self) {
        if let Ok(mut placed) = self.placed.lock() {
            placed.changed = true;
        }
    }

    /// The app's background pass does the reading from now on.
    pub fn read_in_background(&self) {
        self.background
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// A pass is reading the tools' files now (AJ-E1538's builder waits).
    #[must_use]
    pub fn is_reading(&self) -> bool {
        matches!(
            self.pass.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        )
    }

    /// Holds the one pass. It guards no data, so a pass that panicked
    /// (its tick goes on) leaves nothing to distrust: before, its
    /// poisoned lock failed every later pass, and the first build never
    /// ended.
    fn hold_pass(&self) -> std::sync::MutexGuard<'_, ()> {
        self.pass
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Every conversation now in the index, one per account and id,
    /// newest activity first.
    ///
    /// # Errors
    ///
    /// [`AppError::StatePoisoned`] when the index's lock is poisoned.
    pub fn all(&self) -> AppResult<Vec<Conversation>> {
        let files = self.files.lock().map_err(|_| AppError::StatePoisoned)?;
        Ok(newest_first(merged(
            files
                .values()
                .filter_map(|entry| entry.conversation.clone()),
        )))
    }

    /// Whether the index holds any conversation (without copying them).
    ///
    /// # Errors
    ///
    /// [`AppError::StatePoisoned`] when the index's lock is poisoned.
    pub fn holds_any(&self) -> AppResult<bool> {
        let files = self.files.lock().map_err(|_| AppError::StatePoisoned)?;
        Ok(files.values().any(|entry| entry.conversation.is_some()))
    }

    /// AJ-E805: [`Self::all`]'s conversations for which `keep` holds of
    /// any of their files (a thread's other files with them), without
    /// copying the rest: the live refresh every 2 s and the working list
    /// copied, merged and sorted every conversation on the Mac.
    ///
    /// # Errors
    ///
    /// [`AppError::StatePoisoned`] when the index's lock is poisoned.
    pub fn all_where(&self, keep: impl Fn(&Conversation) -> bool) -> AppResult<Vec<Conversation>> {
        let files = self.files.lock().map_err(|_| AppError::StatePoisoned)?;
        let conversations = || {
            files
                .values()
                .filter_map(|entry| entry.conversation.as_ref())
        };
        let kept: std::collections::HashSet<(&str, &str, &str)> = conversations()
            .filter(|c| keep(c))
            .map(|c| {
                (
                    c.account_id.as_str(),
                    c.tool.as_str(),
                    c.summary.id.as_str(),
                )
            })
            .collect();
        Ok(newest_first(merged(
            conversations()
                .filter(|c| {
                    kept.contains(&(
                        c.account_id.as_str(),
                        c.tool.as_str(),
                        c.summary.id.as_str(),
                    ))
                })
                .cloned(),
        )))
    }

    /// 0.4.3: the list has been read whole once since the start (what a
    /// folder holds can be told from it).
    #[must_use]
    pub fn first_pass_done(&self) -> bool {
        self.first_done.load(std::sync::atomic::Ordering::Relaxed)
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

    /// AJ-E1520 (0.2): `account`'s conversation `id` as [`Self::path_of`]
    /// finds it, with its tool (the Subagents tab asks that tool's
    /// adapter).
    #[must_use]
    pub fn conversation_of(&self, account: &str, id: &str) -> Option<Conversation> {
        let files = self.files.lock().ok()?;
        files
            .values()
            .filter_map(|entry| entry.conversation.as_ref())
            .filter(|c| c.account_id == account && c.summary.id == id)
            .max_by(|a, b| (a.summary.last_ms, &a.path).cmp(&(b.summary.last_ms, &b.path)))
            .cloned()
    }

    /// 0.1.8: the title and folder of each `(account, id)` asked for, as
    /// the conversation list folds them (the newest file's folder, the
    /// newest title its files have), without copying the index: the cost
    /// table names one page of sessions at a time.
    #[must_use]
    pub fn facts_of(
        &self,
        wanted: &std::collections::HashSet<(String, String)>,
    ) -> HashMap<(String, String), (Option<String>, Option<PathBuf>)> {
        let Ok(files) = self.files.lock() else {
            return HashMap::new();
        };
        let mut matches: Vec<&Conversation> = files
            .values()
            .filter_map(|entry| entry.conversation.as_ref())
            .filter(|c| wanted.contains(&(c.account_id.clone(), c.summary.id.clone())))
            .collect();
        // Newest file first: it gives the folder, and the title unless
        // it has none.
        matches.sort_by_key(|c| std::cmp::Reverse(c.summary.last_ms));
        let mut found: HashMap<(String, String), (Option<String>, Option<PathBuf>)> =
            HashMap::new();
        for conversation in matches {
            let kept = found
                .entry((
                    conversation.account_id.clone(),
                    conversation.summary.id.clone(),
                ))
                .or_insert_with(|| (None, conversation.folder.clone()));
            if kept.0.is_none() {
                kept.0.clone_from(&conversation.summary.title);
            }
        }
        found
    }

    /// Whether a running session is on `account`'s `(tool, id)`.
    #[must_use]
    pub fn is_live(&self, account: &str, tool: &str, id: &str) -> bool {
        self.live.lock().is_ok_and(|live| {
            live.contains_key(&(account.to_owned(), tool.to_owned(), id.to_owned()))
        })
    }

    /// Every conversation a running session is on now, `(account, tool,
    /// id)` (the rail's subagent count follows them, AJ-E1520).
    #[must_use]
    pub fn live_keys(&self) -> Vec<live::Live> {
        self.live
            .lock()
            .map(|live| live.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Where `account`'s `(tool, id)` is open now, and its state.
    #[must_use]
    pub fn live_entry(&self, account: &str, tool: &str, id: &str) -> Option<scan::LiveEntry> {
        self.live.lock().ok().and_then(|live| {
            live.get(&(account.to_owned(), tool.to_owned(), id.to_owned()))
                .cloned()
        })
    }

    /// Every conversation in a running app's sidebar with no process on
    /// it, `(account, tool, id)`.
    #[must_use]
    pub fn inactive_keys(&self) -> Vec<live::Live> {
        self.inactive
            .lock()
            .map(|inactive| inactive.iter().cloned().collect())
            .unwrap_or_default()
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

    /// AJ-E2401: what is open now, as the live refresher last saw it (the
    /// Processes view's sessions other apps run).
    #[must_use]
    pub fn live_model(&self) -> scan::LiveModel {
        self.model
            .lock()
            .map(|model| model.clone())
            .unwrap_or_default()
    }

    /// F-rc3g-4: processes AI Juggler saw end hold nothing open any more:
    /// each place they held goes from what is open now, at once (the live
    /// refresher's next look is up to 15 s away; meanwhile an ended
    /// session's conversation stood as its own "Idle" row under its App).
    /// A conversation with no place left isn't open. Returns whether one
    /// was.
    pub fn processes_ended(&self, pids: &std::collections::BTreeSet<u32>) -> bool {
        let mut changed = false;
        if let Ok(mut model) = self.model.lock() {
            changed |= without_pids(&mut model, pids);
        }
        if let Ok(mut live) = self.live.lock() {
            live.retain(|_, entry| {
                let lost = lose_places(entry, pids);
                changed |= lost;
                !(lost && entry.places.is_empty())
            });
        }
        // AJ-E2311 item 6: a look already running (a look is short, but
        // it read them before they ended) leaves them out of what it
        // writes, and the next one's scanner forgets them.
        if let Ok(mut ended) = self.ended.lock() {
            ended.extend(pids);
        }
        if let Ok(mut scanner) = self.scanner.try_lock() {
            scanner.forget(pids);
        }
        changed
    }

    /// A live look's result without the processes that ended while it
    /// ran ([`Self::processes_ended`]), before it is written.
    fn without_ended(&self, model: &mut scan::LiveModel) {
        if let Ok(ended) = self.ended.lock() {
            without_pids(model, &ended);
        }
    }

    /// At a live look's start: the processes ended since the last one,
    /// taken (its scanner forgets them).
    fn take_ended(&self) -> std::collections::BTreeSet<u32> {
        self.ended
            .lock()
            .map(|mut ended| std::mem::take(&mut *ended))
            .unwrap_or_default()
    }

    /// Test seam: what the live refresher saw.
    #[cfg(test)]
    pub(crate) fn set_live_model(&self, model: scan::LiveModel) {
        if let Ok(mut now) = self.model.lock() {
            *now = model;
        }
    }

    /// AJ-E2401: the titles of these conversations, `(tool, id)` →
    /// `(title, folder)`, from one pass over the index (the newest file of
    /// each).
    #[must_use]
    pub fn titles_of(
        &self,
        wanted: &std::collections::HashSet<(String, String)>,
    ) -> HashMap<(String, String), (Option<String>, Option<PathBuf>)> {
        type Newest = (i64, Option<String>, Option<PathBuf>);
        let mut out: HashMap<(String, String), Newest> = HashMap::new();
        if wanted.is_empty() {
            return HashMap::new();
        }
        let Ok(files) = self.files.lock() else {
            return HashMap::new();
        };
        for conversation in files
            .values()
            .filter_map(|entry| entry.conversation.as_ref())
        {
            let key = (conversation.tool.clone(), conversation.summary.id.clone());
            if !wanted.contains(&key) {
                continue;
            }
            let last = conversation.summary.last_ms.unwrap_or_default();
            let newer = out.get(&key).is_none_or(|(known, _, _)| last >= *known);
            if newer {
                out.insert(
                    key,
                    (
                        last,
                        conversation.summary.title.clone(),
                        conversation
                            .folder
                            .clone()
                            .or_else(|| conversation.summary.cwd.clone()),
                    ),
                );
            }
        }
        out.into_iter()
            .map(|(key, (_, title, folder))| (key, (title, folder)))
            .collect()
    }
}

/// Newest activity first (then by file, so the order holds still).
fn newest_first(mut conversations: Vec<Conversation>) -> Vec<Conversation> {
    conversations.sort_by(|a, b| {
        b.summary
            .last_ms
            .cmp(&a.summary.last_ms)
            .then_with(|| a.path.cmp(&b.path))
    });
    conversations
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
    mut rows: Vec<crate::features::sessions::dto::InstanceInfo>,
) -> AppResult<Vec<crate::features::sessions::dto::InstanceInfo>> {
    // AJ-E805: only these rows' conversations (Needs you reads the rows
    // every 5 s; this copied every conversation on the Mac each time).
    let titles: HashMap<(String, String), String> = {
        let wanted: std::collections::HashSet<(&str, &str)> = rows
            .iter()
            .flat_map(|row| [own_conversation(row), carried_conversation(row)])
            .flatten()
            .collect();
        state
            .conversations
            .all_where(|c| wanted.contains(&(c.account_id.as_str(), c.summary.id.as_str())))?
            .into_iter()
            .filter_map(|c| Some(((c.account_id, c.summary.id), c.summary.title?)))
            .collect()
    };
    let title = |key: Option<(&str, &str)>| {
        key.and_then(|(account, id)| titles.get(&(account.to_owned(), id.to_owned())).cloned())
    };
    for row in &mut rows {
        let (own, carried) = (own_conversation(row), carried_conversation(row));
        if own.is_some() || carried.is_some() {
            row.title = title(own).or_else(|| title(carried));
        }
    }
    Ok(rows)
}

/// `entry` without its places in processes `pids`; whether it lost one.
fn lose_places(entry: &mut scan::LiveEntry, pids: &std::collections::BTreeSet<u32>) -> bool {
    let before = entry.places.len();
    entry.places.retain(|place| !pids.contains(&place.pid));
    entry.places.len() != before
}

/// `model` without the places of processes `pids`: an entry that lost its
/// last place isn't open (one open with none, an app's own idle row,
/// stays as it is). Whether anything went.
fn without_pids(model: &mut scan::LiveModel, pids: &std::collections::BTreeSet<u32>) -> bool {
    let mut changed = false;
    model.entries.retain_mut(|entry| {
        let lost = lose_places(entry, pids);
        changed |= lost;
        !(lost && entry.places.is_empty())
    });
    model.pid_dirs.retain(|pid, _| !pids.contains(pid));
    changed
}

/// A session row's own conversation: `(account, id)`.
fn own_conversation(row: &crate::features::sessions::dto::InstanceInfo) -> Option<(&str, &str)> {
    Some((row.account_id.as_deref()?, row.conversation_id.as_deref()?))
}

/// SK-11: the conversation a switch carried into a session row. Its copy
/// is the same conversation, so until the copy is read the row goes by
/// that one's title (never by its facts: "web · started 9:40 AM").
fn carried_conversation(
    row: &crate::features::sessions::dto::InstanceInfo,
) -> Option<(&str, &str)> {
    let from = row.continues.as_ref()?;
    Some((from.account_id.as_str(), from.conversation_id.as_deref()?))
}

/// F-rc3g-4 (the 0.4.6 rc3g walk; Open now means running): an AI
/// Juggler session's process ended (an `Exited` event: its `pid`, and the
/// processes under it at the last look): whatever they held open isn't
/// open any more, before the session's ending is stored, so no list reads
/// its conversation as open elsewhere in between. Any tool: by process,
/// not by its records. Any other event: nothing.
pub fn after_exit(state: &Arc<AppState>, event: &juggler_supervisor::LifecycleEvent) {
    let juggler_supervisor::LifecycleEvent::Exited { session, pid, .. } = event else {
        return;
    };
    let mut pids = std::collections::BTreeSet::from([*pid]);
    if let Ok(collector) = state.resources.lock() {
        pids.extend(
            collector
                .rows(session)
                .unwrap_or_default()
                .iter()
                .map(|row| row.pid),
        );
    }
    if state.conversations.processes_ended(&pids) {
        crate::features::relations::changed(
            state,
            crate::features::relations::Change::ConversationsLive,
        );
    }
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
        let _pass = state.conversations.hold_pass();
        let progress = || {
            if update_live(state).is_ok() {
                progress();
            }
        };
        match pass::plan(state, &accounts) {
            gate::Plan::Whole => refresh_with(state, &accounts, every, &progress)?,
            gate::Plan::Some(scope) if scope.is_empty() => false,
            gate::Plan::Some(scope) => {
                let changed =
                    pass::refresh_files(state, &accounts, every, &progress, Some(&scope))?;
                if changed {
                    saved::save_soon(state);
                }
                changed
            }
        }
    };
    // New folders become projects without the UI asking; lines appended
    // to known conversations move only their projects' last use (AJ-E805:
    // the whole backfill, a copy and a placing of every conversation and a
    // write of every project, ran on every pass while sessions worked).
    if files_changed {
        let placed = state.conversations.take_placed();
        if placed.changed {
            crate::features::projects::backfill::projects_backfill_impl(state)?;
        } else if !placed.touched.is_empty() {
            crate::features::projects::backfill::touch_last_used(state, &placed.touched)?;
        }
    }
    let live_changed = update_live(state)?;
    Ok(files_changed || live_changed)
}

/// The conversation list is being built for the first time: no saved
/// list to start from, and no whole pass done yet (a first start reads
/// every transcript on the Mac; about 40 s on a long history). The window
/// says so quietly where the lists would be.
#[must_use]
pub fn building(state: &AppState) -> bool {
    !state
        .conversations
        .first_done
        .load(std::sync::atomic::Ordering::Relaxed)
        && !saved::has_saved(state)
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
    // Only the conversations something is open on (or idle in an app).
    let open: std::collections::HashSet<(&str, &str)> = model
        .entries
        .iter()
        .map(|entry| (entry.tool.as_str(), entry.session_id.as_str()))
        .chain(
            model
                .inactive
                .iter()
                .map(|(tool, id)| (tool.as_str(), id.as_str())),
        )
        .collect();
    let all = state
        .conversations
        .all_where(|c| open.contains(&(c.tool.as_str(), c.summary.id.as_str())))?;
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
        // AJ-E3102: which are open moves counts; their states don't.
        if live.len() != now.len() || now.keys().any(|key| !live.contains_key(key)) {
            crate::features::relations::changed(
                state,
                crate::features::relations::Change::ConversationsLive,
            );
        }
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

/// How long after a start the live view reads every process each tick.
pub const AFTER_START: std::time::Duration = std::time::Duration::from_secs(30);

/// An app or profile was just started: the live view looks for it every
/// tick for [`AFTER_START`], so its row and page say it runs within a
/// couple of seconds.
pub fn look_for_started(state: &Arc<AppState>) {
    if let Ok(mut scanner) = state.conversations.scanner.lock() {
        scanner.full_for(std::time::Instant::now(), AFTER_START);
    }
}

/// AJ-E2101: one look at what is open now (the app's live thread calls
/// this every [`scan::TICK`]); `true` when what the UI shows changed.
///
/// # Errors
///
/// A poisoned lock.
pub fn live_tick(state: &Arc<AppState>) -> AppResult<bool> {
    let home = juggler_platform::user::home().unwrap_or_default();
    live_tick_on(state, &scan::RealOs, &home)
}

/// [`live_tick`] over `os`'s processes, with `home` the user's home (the
/// end-to-end tests script the process table; the files read are real).
///
/// # Errors
///
/// A poisoned lock.
pub fn live_tick_on(
    state: &Arc<AppState>,
    os: &dyn scan::Os,
    home: &std::path::Path,
) -> AppResult<bool> {
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
    // The user's own desktop profiles, running or not (D-84: never one
    // only seen): their records say who started whom while closed too.
    let remembered: Vec<(String, std::path::PathBuf)> =
        crate::features::places::with_book(state, |book| {
            book.places
                .values()
                .filter(|place| place.users)
                .filter_map(|place| Some((place.app_id.clone(), place.instance_dir.clone()?)))
                .collect()
        })
        .unwrap_or_default();
    let (model, apps) = {
        let mut scanner = state
            .conversations
            .scanner
            .lock()
            .map_err(|_| AppError::StatePoisoned)?;
        // AJ-E2101 R5: the app's scanner watches the session folders.
        scanner.use_file_events();
        scanner.remember(remembered);
        scanner.forget(&state.conversations.take_ended());
        let model = scanner.tick(os, &adapters, &profiles, home, std::time::Instant::now());
        (model, scanner.take_apps())
    };
    // AJ-E2311 item 6: what ended while it looked is never written back.
    let mut model = model;
    state.conversations.without_ended(&mut model);
    // 0.3.3: the apps and profiles running, remembered (a full scan's).
    if let Some(apps) = apps {
        crate::features::places::observe(state, &apps, &model.entries);
    }
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
    let _pass = state.conversations.hold_pass();
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
    saved::restore_once(state);
    if state.conversations.holds_any()? {
        return Ok(());
    }
    // The app's background pass reads them once the window has painted,
    // at the lowest priority, and tells the UI as it goes (the first
    // start's list builds while the window stays responsive).
    if state
        .conversations
        .background
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Ok(());
    }
    let accounts = lock_storage_arc(state)?.accounts().list()?;
    let pass = match state.conversations.pass.try_lock() {
        Ok(pass) => pass,
        Err(std::sync::TryLockError::WouldBlock) => return Ok(()),
        Err(std::sync::TryLockError::Poisoned(held)) => held.into_inner(),
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
    // The go sitting's startup minute: every start read every transcript
    // again. The last index, saved, is restored first: only what changed
    // since is read.
    saved::restore_once(state);
    let changed = pass::refresh_files(state, accounts, every, progress, None)?;
    state
        .conversations
        .first_done
        .store(true, std::sync::atomic::Ordering::Relaxed);
    if changed {
        saved::save_soon(state);
    }
    Ok(changed)
}

/// The title the conversation `id` on `account_id` shows under, when the
/// index has it: what a switch names before anything moves.
pub(crate) fn title_of(state: &Arc<AppState>, account_id: &str, id: &str) -> Option<String> {
    find(state, account_id, id)
        .ok()
        .and_then(|conversation| conversation.summary.title)
        .filter(|title| !title.trim().is_empty())
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
        // How it opens (AI Juggler's own words included): a title skips
        // them (the 0.2 walk, F9).
        let opens = summary
            .opening
            .as_deref()
            .or(summary.title.as_deref())
            .is_some_and(|title| {
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
    let cwd = conversation.summary.cwd.clone().unwrap_or_default();
    // rc21: no command handed out that its tool won't open.
    crate::features::launch::resume_gate::resume_command(state, &account, id, &cwd)
}

/// Where "Continue in AI Juggler" would continue a conversation: its own
/// folder, its account, that account's first launch settings, and the
/// project the folder belongs to (made if it's new).
pub struct ContinueTarget {
    /// The account the conversation is on.
    pub account: juggler_core::model::Account,
    /// The launch settings it continues with.
    pub profile: juggler_core::model::Profile,
    /// Its project.
    pub project: juggler_core::model::ProjectRecord,
    /// Its folder.
    pub cwd: PathBuf,
}

/// [`continue_impl`]'s checks and choices, without the launch: the one
/// place they're made (the L2 harness records sessions from it too).
///
/// # Errors
///
/// As [`continue_impl`]: live elsewhere, its folder gone, no launch
/// settings; storage errors.
pub fn continue_target(
    state: &Arc<AppState>,
    account_id: &str,
    id: &str,
) -> AppResult<ContinueTarget> {
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
    drop(storage);
    Ok(ContinueTarget {
        account,
        profile,
        project,
        cwd,
    })
}

/// [`continue_target`] and the launch that resumes the conversation there:
/// its App's own folder, the tool's own id for it, in its folder. Refused
/// in words when the tool wouldn't open that conversation by its id.
///
/// # Errors
///
/// As [`continue_target`]; the adapter's refusal (rc19: Claude Code shows
/// its Resume picker for an id it doesn't resume by); launch errors.
pub fn continue_launch(
    state: &Arc<AppState>,
    account_id: &str,
    id: &str,
) -> AppResult<(ContinueTarget, juggler_adapter::LaunchSpec)> {
    let target = continue_target(state, account_id, id)?;
    // rc21: the one check every resume asks, in the gate's launch.
    let spec = crate::features::launch::resume_gate::resume_spec(
        state,
        &target.account,
        &target.profile,
        &target.cwd,
        id,
    )?;
    Ok((target, spec))
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
/// launch settings, an id the tool doesn't resume by; launch and storage
/// errors.
pub fn continue_impl(
    state: &Arc<AppState>,
    account_id: &str,
    id: &str,
) -> AppResult<crate::features::launch::dto::LaunchResult> {
    use juggler_core::model::{ConversationId, SessionId};
    let (
        ContinueTarget {
            account,
            profile,
            project,
            cwd,
        },
        spec,
    ) = continue_launch(state, account_id, id)?;
    let session = SessionId::new(format!("sess-{}", crate::ids::unique_suffix()));
    // F-rc3g-3: AI Juggler ran it before (a run since ended): its name and
    // pin go on with it.
    let before = lock_storage_arc(state)?
        .sessions()
        .conversation_links()?
        .into_iter()
        .rfind(|link| link.account == account.id.as_str() && link.conversation == id)
        .map(|link| SessionId::new(link.session));
    // The one path: the row first (ADR-0010 §1), then the bridge (AJ-E1704,
    // D-30: continued in AI Juggler, it gets the agent tools) and the
    // account's key in the launch, under the supervisor.
    let started = crate::features::sessions::spawn::start_session(
        state,
        &crate::features::sessions::spawn::NewSession {
            id: &session,
            account: &account,
            profile: &profile.id,
            project: Some(project.id.clone()),
            cwd,
            conversation: Some(ConversationId::new(id)),
            transcript_ref: None,
            origin: "resume",
            continues: before
                .as_ref()
                .map(crate::features::sessions::spawn::GoesOn::From),
        },
        crate::features::sessions::spawn::Runs::Terminal(spec),
    )?;
    Ok(crate::features::launch::dto::LaunchResult {
        session_id: session.to_string(),
        pid: started.spawned.pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AJ-E2311 item 6 (rc3h item 13): a live look already running when a
    /// process ends had read it open, and wrote it back as open until the
    /// next look. Its result leaves it out, and the next look's scanner
    /// forgets it.
    #[test]
    fn a_look_running_when_a_process_ends_never_writes_it_back() {
        let index = ConversationIndex::default();
        let open = |pid: u32, id: &str| scan::LiveEntry {
            tool: "claude-code".into(),
            account_id: None,
            session_id: id.into(),
            places: vec![scan::Place {
                pid,
                start: None,
                host: None,
                profile: None,
                app_session: None,
            }],
            state: None,
            probably: false,
            viewed_ms: None,
            app_title: None,
            background: false,
        };
        // A look is running (it holds the scanner) when pid 42 ends.
        let looking = index.scanner.lock().unwrap();
        assert!(!index.processes_ended(&std::collections::BTreeSet::from([42])));
        drop(looking);
        // What that look read, before it ended.
        let mut look = scan::LiveModel {
            entries: vec![open(42, "ended"), open(7, "still-open")],
            pid_dirs: [(42, PathBuf::from("/p")), (7, PathBuf::from("/p"))].into(),
            ..scan::LiveModel::default()
        };
        index.without_ended(&mut look);
        let ids: Vec<&str> = look.entries.iter().map(|e| e.session_id.as_str()).collect();
        assert_eq!(ids, ["still-open"]);
        assert_eq!(look.pid_dirs.keys().copied().collect::<Vec<_>>(), [7]);
        // The next look takes them (its scanner forgets them), once.
        assert_eq!(index.take_ended(), std::collections::BTreeSet::from([42]));
        assert!(index.take_ended().is_empty());
    }

    /// ADR-0027 condition 7's review: what was indexed from the adopted
    /// folder before the lab's scope held is read again, with it.
    #[test]
    fn forgetting_a_folder_drops_only_its_files() {
        let index = ConversationIndex::default();
        let entry = || Entry {
            len: 1,
            modified: None,
            conversation: None,
            complete: true,
        };
        {
            let mut files = index.files.lock().unwrap();
            files.insert(PathBuf::from("/real/.codex/sessions/a.jsonl"), entry());
            files.insert(PathBuf::from("/tmp/aij-lab-x/.codex/b.jsonl"), entry());
        }
        index.forget_under(std::path::Path::new("/real/.codex"));
        let files = index.files.lock().unwrap();
        assert_eq!(
            files.keys().collect::<Vec<_>>(),
            [&PathBuf::from("/tmp/aij-lab-x/.codex/b.jsonl")]
        );
    }

    /// 0.3.5: storage busy at launch (its first migrations, a refresh)
    /// no longer gives the first scan an empty list: it waits that once.
    #[test]
    fn the_first_profile_list_waits_for_busy_storage() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState::init(dir.path().join("data")).unwrap());
        let expected = {
            let fresh = Arc::new(AppState::init(dir.path().join("other")).unwrap());
            profiles(&fresh)
        };
        assert!(!expected.is_empty(), "each tool's default folder is listed");
        let (held, release) = std::sync::mpsc::channel::<()>();
        let holder = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || {
                let _storage = state.storage.lock().unwrap();
                held.send(()).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(200));
            })
        };
        release.recv().unwrap();
        let first = profiles(&state);
        holder.join().unwrap();
        let dirs = |list: &[scan::Profile]| {
            list.iter()
                .map(|p| (p.tool.clone(), p.dir.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(dirs(&first), dirs(&expected));
    }

    fn file(path: &str, id: &str, started: i64, last: i64, input: u64) -> Conversation {
        let mut summary = ConversationSummary {
            id: id.to_owned(),
            cwd: None,
            git_branch: None,
            started_in: None,
            by_script: false,
            title: None,
            opening: None,
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

//! AJ-E1529: every account's conversations, the ones AI Juggler started
//! and the ones it didn't, read from each tool's own files through its
//! adapter (`conversation_files`, `read_conversation`) and kept in memory.
//!
//! The index is keyed by file: a refresh lists each account's files, and
//! re-reads only those whose size or modification time changed. Nothing
//! is written: not into a setup, and not into AI Juggler's database
//! (there is no migration; a restart rebuilds it from the files).

pub mod commands;
pub mod dto;
pub mod live;

/// Emitted when the conversations or which are live changed (the rail
/// reads them again).
pub const EVENT_CHANGED: &str = "conversations://changed";

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
    state
        .conversations
        .files
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .insert(
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

/// The in-memory index (one per app).
#[derive(Debug, Default)]
pub struct ConversationIndex {
    files: Mutex<HashMap<PathBuf, Entry>>,
    /// `(account, tool, id)` of the conversations a running session is on.
    live: Mutex<std::collections::HashSet<live::Live>>,
    /// Held while a pass reads the files: one pass at a time (QA batch 9:
    /// at a start, the background pass, the conversation list and the
    /// projects backfill each read every file at once).
    pass: Mutex<()>,
}

impl ConversationIndex {
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

    /// Whether a running session is on `account`'s `(tool, id)`.
    #[must_use]
    pub fn is_live(&self, account: &str, tool: &str, id: &str) -> bool {
        self.live
            .lock()
            .is_ok_and(|live| live.contains(&(account.to_owned(), tool.to_owned(), id.to_owned())))
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
            if update_live(state, &accounts).is_ok() {
                progress();
            }
        })?
    };
    if files_changed {
        // New folders become projects without the UI asking.
        crate::commands::projects_backfill_impl(state)?;
    }
    let live_changed = update_live(state, &accounts)?;
    Ok(files_changed || live_changed)
}

/// How often a long pass reports what it has read so far.
pub const PROGRESS_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// Which conversations are live now, from the process table and what's
/// indexed; `true` when that changed.
fn update_live(
    state: &Arc<AppState>,
    accounts: &[juggler_core::model::Account],
) -> AppResult<bool> {
    let running = live::running_sessions(state, accounts);
    let now = live::live_ids(&state.conversations.all()?, &running);
    let mut live = state
        .conversations
        .live
        .lock()
        .map_err(|_| AppError::StatePoisoned)?;
    let changed = *live != now;
    *live = now;
    Ok(changed)
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
    use juggler_core::model::{
        ConversationId, InstanceKind, InstanceRecord, SessionId, SessionStatus, Timestamp,
    };
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
        let project = storage
            .projects()
            .touch(&juggler_core::storage::project_root(&cwd), None)?;
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
    let spawned = state.supervisor.spawn(
        &session,
        &spec,
        &crate::commands::ledger_ctx(state, &account, Some(project.id.to_string())),
    )?;
    {
        let machine = crate::commands::local_machine_id(state);
        let storage = lock_storage_arc(state)?;
        let usage = crate::commands::engine_usage_snapshot(&storage, &profile.id);
        storage.sessions().insert(&InstanceRecord {
            id: session.clone(),
            conversation: Some(ConversationId::new(id)),
            profile: profile.id.clone(),
            machine,
            kind: InstanceKind::Terminal,
            project: Some(project.id.clone()),
            cwd,
            status: SessionStatus::Starting,
            started_at: Timestamp::now(),
            last_activity: None,
            usage,
            attention: None,
            running: None,
            window: None,
            transcript_ref: None,
        })?;
        crate::usage_stats::instance_started(account.harness.as_str(), false, "resume");
    }
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
}

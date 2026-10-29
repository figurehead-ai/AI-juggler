//! AJ-E1520 (0.2): the session page's Subagents tab — every subagent a
//! session started (and theirs), for any session the app shows, its own
//! or another app's, from the tool's own records.
//!
//! Nothing here knows which tool wrote them (ADR-0002): the session's
//! tool's adapter says whether it can list them
//! (`HarnessAdapter::subagent_support`) and reads them
//! (`HarnessAdapter::subagent_reader`); this module keeps a reader per
//! session whose page is open (a few, the latest used), redacts their
//! words (D-14, `clean.rs`), and says "running" only while the session
//! is open (D-06: once it has ended, a subagent its records never saw
//! end is unknown).
//!
//! 0.3: what each one was given, used and did (`view.rs`: the views,
//! groups and tree; `detail.rs`: one subagent's page), and a saved index
//! (`index.rs`): a reader's checkpoint, redacted, so a session with
//! hundreds of subagents opens at once and reads only what grew. A
//! project's subagents are its sessions' together (`project.rs`).
//!
//! The rail's count of a live session's running subagents (I-08,
//! `count.rs`) is read by the same readers: one source for the row and
//! the tab, for every tool whose adapter lists subagents, and for every
//! live session the rail shows, AI Juggler's own or another app's.
//!
//! A subagent's conversation is read like the session's own (the same
//! last-turns reader, paged, followed while it runs), from the transcript
//! its reader found — never a path the page names. Read only when the
//! page asks, kept nowhere (D-37), and off with the session's last
//! messages in Settings.

pub mod attribute;
pub mod clean;
pub mod commands;
pub mod count;
pub mod detail;
pub mod dto;
pub mod index;
pub mod project;
pub mod view;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_adapter::subagents::{
    Subagent, SubagentConversations, SubagentReader, SubagentScope, SubagentSupport,
    SubagentTokens, TokensCounted,
};
use juggler_core::model::HarnessId;

use crate::error::AppResult;
use crate::features::conversations::last_messages::{self, Ask, LastMessagesInfo};
use crate::paging::{Page, PageRequest};
use crate::state::AppState;

use dto::{SubagentsInfo, SubagentsQuery, SubagentsSummaryInfo};
pub use view::{Found, status_word};

/// How many sessions' readers the tab keeps (the pages opened last); a
/// live session's also stays while the count follows it.
const READERS_KEPT: usize = 8;

/// The most bytes of subagents' own transcripts one ask of the tab reads
/// (a first read of a long session is spread over a few asks).
pub const TAB_BUDGET: u64 = 96 * 1024 * 1024;

/// A reader's index is saved at most this often while it changes.
const SAVE_EVERY: Duration = Duration::from_secs(15);

/// What a reader is kept by: the account, the tool's id for the
/// conversation, and the record the conversations index found for it.
type Key = (String, String, PathBuf);

/// One command a subagent ran, for the Processes view's "started by".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentCommand {
    /// When it was asked for, epoch ms.
    pub at_ms: i64,
    /// The command, redacted.
    pub command: String,
    /// The subagent's id and its task (redacted).
    pub id: String,
    /// Its task (redacted), else its type.
    pub title: String,
}

/// What a reader's index last saved.
#[derive(Default)]
struct Saved {
    at: Option<Instant>,
    print: u64,
    /// Each text's redaction, by its raw words' print (a save redacts
    /// only what's new).
    memo: HashMap<u64, Option<String>>,
}

/// One session's reader. Its tab and the rail's count share it, each
/// taking its lock to read: another session never waits on it.
struct Entry {
    key: Key,
    reader: Mutex<Box<dyn SubagentReader>>,
    saved: Mutex<Saved>,
    clean: Mutex<clean::Cleaned>,
    commands: Mutex<Arc<Vec<AgentCommand>>>,
}

/// What one read of a reader found.
struct Listed {
    refreshed: std::io::Result<bool>,
    details: bool,
    agents: Vec<Subagent>,
    session_tokens: Option<SubagentTokens>,
    counted: TokensCounted,
}

impl Entry {
    fn new(key: Key, reader: Box<dyn SubagentReader>) -> Arc<Self> {
        Arc::new(Self {
            key,
            reader: Mutex::new(reader),
            saved: Mutex::new(Saved::default()),
            clean: Mutex::new(clean::Cleaned::default()),
            commands: Mutex::new(Arc::new(Vec::new())),
        })
    }

    /// A reader picked up from its saved index, when it has one.
    fn resumed(
        dir: Option<&Path>,
        messages: bool,
        key: Key,
        mut reader: Box<dyn SubagentReader>,
    ) -> Arc<Self> {
        if let Some(dir) = dir {
            let path = index::path_for(dir, &key.0, &key.1, &key.2);
            if let Some(bytes) = index::load(&path, messages) {
                let _ = reader.resume(&bytes);
            }
        }
        Self::new(key, reader)
    }

    /// Reads what its records gained (within the reader's budget) and how
    /// many run; `None` when a read panicked on this reader (the next ask
    /// makes a new one).
    fn count(&self) -> Option<(std::io::Result<bool>, u64)> {
        let mut reader = self.reader.lock().ok()?;
        let refreshed = reader.refresh();
        Some((refreshed, reader.running()))
    }

    /// Reads what its records and its subagents' own gained (the latter
    /// within `budget` bytes) and lists every subagent.
    fn list(&self, budget: u64) -> Option<Listed> {
        let mut reader = self.reader.lock().ok()?;
        let refreshed = reader.refresh();
        let details = reader.refresh_details(budget).unwrap_or(true);
        Some(Listed {
            refreshed,
            details,
            agents: reader.subagents(),
            session_tokens: reader.session_tokens(),
            counted: reader.tokens_counted(),
        })
    }

    /// Keeps each subagent's commands (redacted) for the Processes view.
    fn remember(&self, agents: &[Subagent], cleans: &HashMap<String, Arc<clean::Clean>>) {
        let mut out = Vec::new();
        for agent in agents {
            let Some(clean) = cleans.get(&agent.id) else {
                continue;
            };
            let title = clean
                .task
                .clone()
                .or_else(|| agent.kind.clone())
                .unwrap_or_else(|| "A subagent".to_owned());
            for (at, command) in &clean.commands {
                if let Some(at) = at {
                    out.push(AgentCommand {
                        at_ms: *at,
                        command: command.clone(),
                        id: agent.id.clone(),
                        title: title.clone(),
                    });
                }
            }
        }
        out.sort_by_key(|command| command.at_ms);
        if let Ok(mut kept) = self.commands.lock() {
            *kept = Arc::new(out);
        }
    }

    /// Saves its index when it's due and something changed.
    fn save(&self, dir: &Path, messages: bool) {
        let Ok(mut saved) = self.saved.lock() else {
            return;
        };
        if saved.at.is_some_and(|at| at.elapsed() < SAVE_EVERY) {
            return;
        }
        let Ok(reader) = self.reader.lock() else {
            return;
        };
        let memo = std::cell::RefCell::new(std::mem::take(&mut saved.memo));
        let filter = |kind, text: &str| {
            let print = print_of(kind, text);
            if let Some(kept) = memo.borrow().get(&print) {
                return kept.clone();
            }
            let kept = clean::kept(kind, text, messages);
            memo.borrow_mut().insert(print, kept.clone());
            kept
        };
        let checkpoint = reader.checkpoint(&filter);
        drop(reader);
        saved.memo = memo.into_inner();
        saved.at = Some(Instant::now());
        let Some(bytes) = checkpoint else {
            return;
        };
        let print = print_of(
            juggler_adapter::subagents::SubagentText::Task,
            &String::from_utf8_lossy(&bytes),
        );
        if print == saved.print {
            return;
        }
        let path = index::path_for(dir, &self.key.0, &self.key.1, &self.key.2);
        if index::save(&path, messages, &bytes).is_ok() {
            saved.print = print;
        }
    }
}

/// A text's print, with its kind.
fn print_of(kind: juggler_adapter::subagents::SubagentText, text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325 ^ (kind as u64);
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// The readers kept.
#[derive(Default)]
struct Kept {
    /// The sessions whose Subagents tab was opened last, the latest last.
    tab: Vec<Arc<Entry>>,
    /// Each live session's, by its rail row (`count.rs`).
    live: HashMap<count::Row, Arc<Entry>>,
}

/// Each session's subagent reader, for its tab and the rail's count, and
/// the count each live session last read.
#[derive(Default)]
pub struct Readers {
    kept: Mutex<Kept>,
    /// What the count last published: each live session's running
    /// subagents, by its rail row.
    counts: Mutex<HashMap<count::Row, u64>>,
    /// The count's passes so far.
    passes: std::sync::atomic::AtomicU64,
}

impl std::fmt::Debug for Readers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (tab, live) = self
            .kept
            .lock()
            .map_or((0, 0), |kept| (kept.tab.len(), kept.live.len()));
        let counted = self.counts.lock().map_or(0, |counts| counts.len());
        f.debug_struct("Readers")
            .field("tab", &tab)
            .field("live", &live)
            .field("counted", &counted)
            .finish_non_exhaustive()
    }
}

impl Readers {
    /// The reader kept for `key`, the tab's or the count's (one a read
    /// panicked on is passed over).
    fn find(&self, key: &Key) -> Option<Arc<Entry>> {
        let kept = self.kept.lock().ok()?;
        kept.tab
            .iter()
            .chain(kept.live.values())
            .find(|entry| &entry.key == key && !entry.reader.is_poisoned())
            .cloned()
    }

    /// The tab used `entry`: it is the latest, and the oldest beyond
    /// [`READERS_KEPT`] goes (a live session's stays with the count).
    fn used(&self, entry: &Arc<Entry>) {
        if let Ok(mut kept) = self.kept.lock() {
            kept.tab.retain(|other| other.key != entry.key);
            kept.tab.push(Arc::clone(entry));
            let extra = kept.tab.len().saturating_sub(READERS_KEPT);
            kept.tab.drain(..extra);
        }
    }

    /// AJ-E1520 (0.3): the commands the subagents of `account`'s
    /// conversation `id` ran, as its kept reader last listed them (no
    /// read: a session nobody follows has none).
    #[must_use]
    pub fn commands_of(&self, account: &str, id: &str) -> Arc<Vec<AgentCommand>> {
        let Ok(kept) = self.kept.lock() else {
            return Arc::default();
        };
        kept.tab
            .iter()
            .chain(kept.live.values())
            .find(|entry| entry.key.0 == account && entry.key.1 == id)
            .and_then(|entry| entry.commands.lock().ok().map(|c| Arc::clone(&c)))
            .unwrap_or_default()
    }
}

/// Why nothing is listed.
#[derive(Clone, Copy, Debug)]
pub enum Missing {
    /// No record of the session on this Mac.
    None,
    /// Its tool keeps none AI Juggler reads.
    Unsupported(&'static str),
    /// Its record couldn't be read.
    Unreadable,
}

/// The index folder, under AI Juggler's data folder.
fn index_dir(state: &AppState) -> PathBuf {
    state.data_dir.join(index::DIR)
}

/// Reads `account`'s conversation `id`'s subagents with its tool's
/// adapter (their own transcripts within `budget` bytes), keeping the
/// reader for the next ask and its index saved.
///
/// # Errors
///
/// Why nothing can be listed.
pub fn found(
    state: &Arc<AppState>,
    account: &str,
    id: &str,
    budget: u64,
) -> Result<Found, Missing> {
    let conversation = state
        .conversations
        .conversation_of(account, id)
        .ok_or(Missing::None)?;
    let adapter = state
        .registry
        .get(&HarnessId::new(conversation.tool.as_str()))
        .ok_or(Missing::Unsupported(juggler_adapter::subagents::UNANSWERED))?;
    let support = adapter.subagent_support();
    if let Some(reason) = support.reason() {
        return Err(Missing::Unsupported(reason));
    }
    let messages = last_messages::enabled(state);
    let dir = index_dir(state);
    let key = (account.to_owned(), id.to_owned(), conversation.path.clone());
    let entry = match state.subagents.find(&key) {
        Some(entry) => entry,
        None => Entry::resumed(
            Some(&dir),
            messages,
            key,
            adapter
                .subagent_reader(SubagentScope {
                    path: &conversation.path,
                    id,
                })
                .ok_or(Missing::None)?,
        ),
    };
    state.subagents.used(&entry);
    let listed = entry.list(budget).ok_or(Missing::Unreadable)?;
    let caught_up = listed.refreshed.map_err(|_| Missing::Unreadable)?;
    let cleans = entry
        .clean
        .lock()
        .map(|mut cleaned| cleaned.all(&listed.agents, messages))
        .map_err(|_| Missing::Unreadable)?;
    entry.remember(&listed.agents, &cleans);
    entry.save(&dir, messages);
    Ok(Found {
        support,
        live: state.conversations.is_live(account, &conversation.tool, id),
        reading: !(caught_up && listed.details),
        agents: listed.agents,
        cleans,
        session_tokens: listed.session_tokens,
        counted: listed.counted,
        tool: adapter.spec().display_name,
        now_ms: crate::commands::now_millis(),
        sessions: HashMap::new(),
    })
}

/// A session's subagents, a page of rows at a time.
///
/// # Errors
///
/// A cursor this app didn't make.
pub fn list_impl(
    state: &Arc<AppState>,
    account: &str,
    id: &str,
    query: &SubagentsQuery,
    request: &PageRequest,
) -> AppResult<SubagentsInfo> {
    match found(state, account, id, TAB_BUDGET) {
        Ok(found) => view::listed(&found, query, request),
        Err(missing) => Ok(without(missing)),
    }
}

/// The answer when nothing can be listed.
#[must_use]
pub fn without(missing: Missing) -> SubagentsInfo {
    let (state, reason) = match missing {
        Missing::None => ("none", None),
        Missing::Unsupported(reason) => ("unsupported", Some(reason.to_owned())),
        Missing::Unreadable => (
            "unreadable",
            Some("Its record couldn't be read just now.".to_owned()),
        ),
    };
    SubagentsInfo {
        state: state.to_owned(),
        reason,
        page: Page::empty(),
        summary: SubagentsSummaryInfo {
            counted: "none".to_owned(),
            ..SubagentsSummaryInfo::default()
        },
        kinds: Vec::new(),
        matched: 0,
        running: 0,
        reading: false,
        live: false,
        conversations_note: None,
        tool: String::new(),
    }
}

/// A subagent's conversation, read like the session's own.
///
/// # Errors
///
/// None today; the answer says what it couldn't read.
pub fn conversation_impl(
    state: &Arc<AppState>,
    account: &str,
    id: &str,
    subagent: &str,
    ask: &Ask,
) -> AppResult<LastMessagesInfo> {
    if !last_messages::enabled(state) {
        return Ok(LastMessagesInfo::without("off"));
    }
    let transcript = found(state, account, id, 0).ok().and_then(|found| {
        if !matches!(
            found.support,
            SubagentSupport::Supported {
                conversations: SubagentConversations::Read,
                ..
            }
        ) {
            return None;
        }
        found
            .agents
            .into_iter()
            .find(|agent| agent.id == subagent)?
            .transcript
    });
    Ok(last_messages::read_subagent(transcript.as_deref(), ask))
}

#[cfg(test)]
mod tests;

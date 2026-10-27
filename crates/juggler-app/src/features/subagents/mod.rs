//! AJ-E1520 (0.2): the session page's Subagents tab — every subagent a
//! session started (and theirs), for any session the app shows, its own
//! or another app's, from the tool's own records.
//!
//! Nothing here knows which tool wrote them (ADR-0002): the session's
//! tool's adapter says whether it can list them
//! (`HarnessAdapter::subagent_support`) and reads them
//! (`HarnessAdapter::subagent_reader`); this module keeps a reader per
//! session whose page is open (a few, the latest used), orders and pages
//! what it lists (`paging.rs`'s keyset idiom), redacts each task (D-14),
//! and says "running" only while the session is open (D-06: once it has
//! ended, a subagent its records never saw end is unknown).
//!
//! A subagent's conversation is read like the session's own (the same
//! last-turns reader, paged, followed while it runs), from the transcript
//! its reader found — never a path the page names. Read only when the
//! page asks, kept nowhere (D-37), and off with the session's last
//! messages in Settings.

pub mod commands;
pub mod dto;

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use juggler_adapter::subagents::{
    Subagent, SubagentConversations, SubagentReader, SubagentScope, SubagentStatus, SubagentSupport,
};
use juggler_core::model::HarnessId;

use crate::error::AppResult;
use crate::features::conversations::last_messages::{self, Ask, LastMessagesInfo};
use crate::paging::{Page, PageRequest, SortKey};
use crate::state::AppState;

use dto::{SubagentInfo, SubagentsInfo};

/// How many sessions' readers are kept (the pages opened last).
const READERS_KEPT: usize = 8;

/// A reader, and the session it reads.
struct Entry {
    key: (String, String, PathBuf),
    reader: Box<dyn SubagentReader>,
}

/// The readers of the sessions whose Subagents tab was opened last.
#[derive(Default)]
pub struct Readers {
    kept: Mutex<Vec<Entry>>,
}

impl std::fmt::Debug for Readers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kept = self.kept.lock().map_or(0, |kept| kept.len());
        f.debug_struct("Readers").field("kept", &kept).finish()
    }
}

impl Readers {
    /// Takes a session's reader out while it reads (another session's
    /// tab never waits on it).
    fn take(&self, key: &(String, String, PathBuf)) -> Option<Entry> {
        let mut kept = self.kept.lock().ok()?;
        let at = kept.iter().position(|entry| &entry.key == key)?;
        Some(kept.remove(at))
    }

    /// Puts it back as the latest used; the oldest beyond
    /// [`READERS_KEPT`] goes.
    fn put(&self, entry: Entry) {
        if let Ok(mut kept) = self.kept.lock() {
            kept.retain(|other| other.key != entry.key);
            kept.push(entry);
            let extra = kept.len().saturating_sub(READERS_KEPT);
            kept.drain(..extra);
        }
    }
}

/// What a listing found before paging.
struct Found {
    support: SubagentSupport,
    live: bool,
    reading: bool,
    agents: Vec<Subagent>,
}

/// Why nothing is listed.
#[derive(Clone, Copy)]
enum Missing {
    /// No record of the session on this Mac.
    None,
    /// Its tool keeps none AI Juggler reads.
    Unsupported(&'static str),
    /// Its record couldn't be read.
    Unreadable,
}

/// Reads `account`'s conversation `id`'s subagents with its tool's
/// adapter, keeping the reader for the next ask.
fn found(state: &Arc<AppState>, account: &str, id: &str) -> Result<Found, Missing> {
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
    let key = (account.to_owned(), id.to_owned(), conversation.path.clone());
    let mut entry = match state.subagents.take(&key) {
        Some(entry) => entry,
        None => Entry {
            reader: adapter
                .subagent_reader(SubagentScope {
                    path: &conversation.path,
                    id,
                })
                .ok_or(Missing::None)?,
            key,
        },
    };
    let refreshed = entry.reader.refresh();
    let agents = entry.reader.subagents();
    state.subagents.put(entry);
    let caught_up = refreshed.map_err(|_| Missing::Unreadable)?;
    Ok(Found {
        support,
        live: state.conversations.is_live(account, &conversation.tool, id),
        reading: !caught_up,
        agents,
    })
}

/// A session's subagents, a page at a time.
///
/// # Errors
///
/// A cursor this app didn't make.
pub fn list_impl(
    state: &Arc<AppState>,
    account: &str,
    id: &str,
    request: &PageRequest,
) -> AppResult<SubagentsInfo> {
    match found(state, account, id) {
        Ok(found) => listed(found, request),
        Err(missing) => Ok(without(missing)),
    }
}

fn without(missing: Missing) -> SubagentsInfo {
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
        running: 0,
        reading: false,
        live: false,
        conversations_note: None,
    }
}

/// Where a status stands on the page: running only while the session is
/// open.
fn status_word(status: SubagentStatus, live: bool) -> &'static str {
    match status {
        SubagentStatus::Running if live => "running",
        SubagentStatus::Running | SubagentStatus::Unknown => "unknown",
        SubagentStatus::Done => "done",
        SubagentStatus::Failed => "failed",
        SubagentStatus::Stopped => "stopped",
    }
}

/// The page of `found`'s subagents `request` asks for.
fn listed(found: Found, request: &PageRequest) -> AppResult<SubagentsInfo> {
    let note = match found.support {
        SubagentSupport::Supported {
            conversations: SubagentConversations::NotRead(why),
            ..
        } => Some(why.to_owned()),
        _ => None,
    };
    let conversations = note.is_none();
    let running = found
        .agents
        .iter()
        .filter(|agent| status_word(agent.status, found.live) == "running")
        .count();
    let keys = tree_keys(&found.agents);
    let mut page = crate::paging::page(
        found.agents,
        |agent| {
            keys.get(&agent.id)
                .cloned()
                .unwrap_or_else(|| SortKey::new([0], agent.id.clone()))
        },
        request,
    )?;
    // Only this page's tasks are redacted (D-14), together.
    let tasks: Vec<String> = page
        .items
        .iter()
        .map(|agent| agent.task.clone().unwrap_or_default())
        .collect();
    let (clean, _) = juggler_handover::redaction::strip_segments(&tasks, true, &BTreeSet::new());
    let items: Vec<SubagentInfo> = std::mem::take(&mut page.items)
        .into_iter()
        .zip(clean)
        .map(|(agent, task)| SubagentInfo {
            kind: agent.kind,
            task: agent.task.map(|_| task),
            status: status_word(agent.status, found.live).to_owned(),
            started_ms: agent.started_ms,
            ended_ms: agent.ended_ms.filter(|_| agent.status.is_final()),
            depth: agent.depth.max(1),
            background: agent.background,
            has_conversation: conversations && agent.transcript.is_some(),
            id: agent.id,
            parent: agent.parent,
        })
        .collect();
    Ok(SubagentsInfo {
        state: "listed".to_owned(),
        reason: None,
        page: Page {
            items,
            next_cursor: page.next_cursor,
            total: page.total,
        },
        running: u32::try_from(running).unwrap_or(u32::MAX),
        reading: found.reading,
        live: found.live,
        conversations_note: note,
    })
}

/// Each subagent's place in the tab's order: newest first, each followed
/// by the ones it started (newest first too). Its ranks are, for each of
/// its ancestors and then itself, the start negated (unknown starts sort
/// last) and a hash of the id, so a keyset cursor never shifts as more
/// start.
fn tree_keys(agents: &[Subagent]) -> HashMap<String, SortKey> {
    let by_id: HashMap<&str, &Subagent> = agents
        .iter()
        .map(|agent| (agent.id.as_str(), agent))
        .collect();
    let mut out = HashMap::with_capacity(agents.len());
    for agent in agents {
        let mut chain: Vec<&Subagent> = vec![agent];
        let mut seen: BTreeSet<&str> = BTreeSet::from([agent.id.as_str()]);
        let mut at = agent.parent.as_deref();
        while let Some(parent) = at.and_then(|id| by_id.get(id).copied()) {
            if !seen.insert(parent.id.as_str()) {
                break;
            }
            chain.push(parent);
            at = parent.parent.as_deref();
        }
        chain.reverse();
        let ranks: Vec<i64> = chain
            .iter()
            .flat_map(|link| [link.started_ms.map_or(0, |at| -at), id_rank(&link.id)])
            .collect();
        out.insert(agent.id.clone(), SortKey::new(ranks, agent.id.clone()));
    }
    out
}

/// A stable number for an id (FNV-1a), so siblings that started together
/// keep one order.
fn id_rank(id: &str) -> i64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    // Kept to 62 bits, so it never collides with the start's sign.
    i64::try_from(hash >> 2).unwrap_or(0)
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
    let transcript = found(state, account, id).ok().and_then(|found| {
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

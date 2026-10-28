//! 0.2: AI Juggler's own terminal session learns its conversation even
//! when its tool reports nothing (OpenCode, Grok Build and Kimi Code have
//! no hooks; Codex says it only when driven), so the rail shows the work
//! once: the session's row, carrying its "+N agents" count, never also an
//! unlinked conversation row for the same work.
//!
//! Exact or nothing, never a guess: a session is linked to a conversation
//! only when exactly one can be its own, and keeps today's behaviour
//! (no link) otherwise.
//! 1. **By its process.** The live view (the tool's own per-session
//!    files, through its adapter) sees a conversation of the session's
//!    account and tool open in a process of this session's own tree: that
//!    is its conversation. Two such: none.
//! 2. **By its launch.** The tool's adapter finds the one conversation
//!    its account started in the session's folder since the session
//!    started (`HarnessAdapter::locate_transcript`; two is `None`, and
//!    conversations linked to other sessions are never candidates). It
//!    is refused when another of AI Juggler's sessions of that account in
//!    that folder doesn't know its conversation either (either could own
//!    it), when it is linked to another session, or when the live view
//!    sees it open in a process that isn't this session's.
//!
//!    Never for an account used in place (shared with the user's own
//!    terminal) whose tool's records can't say which process holds a
//!    session (its adapter doesn't declare
//!    `Capabilities::SESSION_PROCESS_RECORDED`): a session the user
//!    started there by hand looks the same as AI Juggler's, so the launch
//!    would be a guess. Both rows stay.
//!
//! No tool is named here (ADR-0019): every tool's identification is its
//! adapter's, every process fact the supervisor's.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use juggler_adapter::Capabilities;
use juggler_core::model::{
    Account, ConversationId, InstanceKind, InstanceRecord, ProfileId, SessionId, SessionStatus,
};

use crate::state::AppState;

/// How often unlinked sessions are looked at (nothing is read while
/// every live session knows its conversation).
pub const EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// A file a tool creates for a session can predate the record by this
/// much (the record is written just after the start).
const SLACK_MS: i64 = 2_000;

/// Starts the linking's own tick; the rail hears when a session learned
/// its conversation (its row and the conversation list both change).
pub fn start(app: tauri::AppHandle, state: Arc<AppState>) {
    crate::ticks::every("conversation links", EVERY, move || {
        if tick(&state) > 0 {
            use tauri::Emitter;
            let _ = crate::commands::emit_sessions(&app, &state);
            let _ = app.emit(
                crate::features::conversations::EVENT_CHANGED,
                crate::features::conversations::CHANGED_LIVE,
            );
        }
    });
}

/// What one session's identification goes on.
#[derive(Debug, Default)]
pub struct Facts {
    /// Conversations of its account and tool the live view sees open in
    /// this session's own processes.
    pub in_own_process: Vec<String>,
    /// The conversation its launch created, as its tool's adapter found
    /// it (`None`: none, or more than one).
    pub by_launch: Option<String>,
    /// Another of AI Juggler's sessions of that account, in that folder,
    /// that doesn't know its conversation either, ran after this one
    /// started: it could own what the launch found.
    pub rival: bool,
    /// Conversations other sessions of the account are linked to.
    pub linked: HashSet<String>,
    /// Conversations the live view sees open in a process that isn't
    /// this session's.
    pub open_elsewhere: HashSet<String>,
}

/// The conversation `facts` say is the session's, when exactly one can
/// be; `None` otherwise (the session stays as it is).
#[must_use]
pub fn decide(facts: &Facts) -> Option<String> {
    let own: Vec<&String> = facts
        .in_own_process
        .iter()
        .filter(|id| !facts.linked.contains(*id))
        .collect();
    match own.as_slice() {
        [one] => return Some((*one).clone()),
        [] if facts.in_own_process.is_empty() => {}
        // Two open in its own processes, or the one is linked elsewhere.
        _ => return None,
    }
    let found = facts.by_launch.as_ref()?;
    (!facts.rival && !facts.linked.contains(found) && !facts.open_elsewhere.contains(found))
        .then(|| found.clone())
}

/// Whether a session's launch may name its conversation: always for an
/// account only AI Juggler uses; for one used in place, only when the
/// tool's records say which process holds each session (else a session
/// the user started in their own terminal can't be told from ours).
#[must_use]
pub fn launch_may_link(in_place: bool, tool: Capabilities) -> bool {
    !in_place || tool.contains(Capabilities::SESSION_PROCESS_RECORDED)
}

/// A session that ran since the earliest unlinked one started, and its
/// account's id.
type Other = (InstanceRecord, String);

/// One live session without its conversation.
struct Unlinked {
    record: InstanceRecord,
    account: Account,
}

/// Links every live own terminal session that doesn't know its
/// conversation and whose conversation can be told exactly. Returns how
/// many were linked.
pub fn tick(state: &Arc<AppState>) -> usize {
    let Some((wanted, others)) = gather(state) else {
        return 0;
    };
    let mut linked = 0;
    for one in &wanted {
        let facts = facts_of(state, one, &others);
        if let Some(id) = decide(&facts)
            && write(state, &one.record.id, &id)
        {
            linked += 1;
        }
    }
    linked
}

/// The sessions to link, and every session (with its account) that ran
/// since the earliest of them started: the only ones whose conversation
/// could have been born after it.
fn gather(state: &Arc<AppState>) -> Option<(Vec<Unlinked>, Vec<Other>)> {
    let storage = state.storage.lock().ok()?;
    let mut accounts: HashMap<ProfileId, Option<Account>> = HashMap::new();
    let mut account_of = |profile: &ProfileId| {
        accounts
            .entry(profile.clone())
            .or_insert_with(|| {
                let profile = storage.profiles().get(profile).ok()?;
                storage.accounts().get(&profile.account).ok()
            })
            .clone()
    };
    let wanted: Vec<Unlinked> = storage
        .sessions()
        .live()
        .ok()?
        .into_iter()
        .filter(|record| record.conversation.is_none() && record.kind == InstanceKind::Terminal)
        // A driven session's driver links it (the tool names it).
        .filter(|record| !state.runtime.is_driven(&record.id))
        .filter_map(|record| {
            let account = account_of(&record.profile)?;
            Some(Unlinked { record, account })
        })
        .collect();
    let since = wanted
        .iter()
        .map(|one| one.record.started_at.millis())
        .min()?;
    let others = storage
        .sessions()
        .working_set(since.saturating_sub(SLACK_MS), i64::MAX, 0)
        .ok()?
        .into_iter()
        .filter_map(|record| {
            let account = account_of(&record.profile)?;
            Some((record, account.id.to_string()))
        })
        .collect();
    Some((wanted, others))
}

/// What `one`'s identification goes on, read now.
fn facts_of(state: &Arc<AppState>, one: &Unlinked, others: &[Other]) -> Facts {
    let account = one.account.id.as_str();
    let tool = one.account.harness.as_str();
    let started = one.record.started_at.millis();
    let mut facts = Facts::default();
    let mut taken: Vec<PathBuf> = Vec::new();
    for (other, of) in others {
        if other.id == one.record.id || of != account {
            continue;
        }
        if let Some(conversation) = &other.conversation {
            facts.linked.insert(conversation.as_str().to_owned());
            if let Some(found) = state
                .conversations
                .conversation_of(account, conversation.as_str())
            {
                taken.push(found.path);
            }
        }
        if let Some(reference) = &other.transcript_ref {
            taken.push(reference.path.clone());
            if let Some(id) = &reference.native_id {
                facts.linked.insert(id.clone());
            }
        }
        facts.rival |= other.conversation.is_none()
            && other.transcript_ref.is_none()
            && other.cwd == one.record.cwd
            && !ended_before(other, started);
    }
    let own = own_processes(state, &one.record.id);
    for (of, of_tool, id) in state.conversations.live_keys() {
        if of != account || of_tool != tool {
            continue;
        }
        let Some(entry) = state.conversations.live_entry(&of, &of_tool, &id) else {
            continue;
        };
        if entry.places.iter().any(|place| own.contains(&place.pid)) {
            facts.in_own_process.push(id);
        } else if !entry.places.is_empty() {
            facts.open_elsewhere.insert(id);
        }
    }
    // The launch is read only when the processes didn't tell.
    if facts.in_own_process.is_empty() {
        facts.by_launch = match &one.record.transcript_ref {
            // Found exactly before (a switch resolved it): its id.
            Some(reference) => reference.native_id.clone(),
            None if facts.rival => None,
            None => state
                .registry
                .get(&one.account.harness)
                .filter(|adapter| launch_may_link(one.account.in_place, adapter.capabilities()))
                .and_then(|adapter| {
                    adapter.locate_transcript(
                        &one.account.state_dir,
                        &one.record.cwd,
                        started,
                        &taken,
                    )
                })
                .and_then(|found| found.native_id),
        };
    }
    facts
}

/// The pids of the session's own process tree (none when it isn't
/// supervised here).
fn own_processes(state: &Arc<AppState>, session: &SessionId) -> HashSet<u32> {
    let mut own: HashSet<u32> = state
        .supervisor
        .live_roots()
        .into_iter()
        .filter(|(id, _, finished)| id == session && !finished)
        .map(|(_, pid, _)| pid)
        .collect();
    if !own.is_empty() {
        own.extend(state.supervisor.descendants(session).unwrap_or_default());
    }
    own
}

/// Whether `session` had ended before `at_ms` (a session that ended
/// earlier could not have started a conversation after it).
fn ended_before(session: &InstanceRecord, at_ms: i64) -> bool {
    matches!(
        session.status,
        SessionStatus::Finished
            | SessionStatus::Stopped
            | SessionStatus::Failed
            | SessionStatus::HandedOver
    ) && session
        .last_activity
        .is_some_and(|last| last.millis() < at_ms.saturating_sub(SLACK_MS))
}

/// Writes the link, only if nothing linked the session meanwhile.
fn write(state: &Arc<AppState>, session: &SessionId, id: &str) -> bool {
    let Ok(storage) = state.storage.lock() else {
        return false;
    };
    let Ok(mut record) = storage.sessions().get(session) else {
        return false;
    };
    if record.conversation.is_some() {
        return false;
    }
    record.conversation = Some(ConversationId::new(id));
    storage.sessions().update(&record).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    #[test]
    fn its_own_process_tells_it_exactly() {
        let facts = Facts {
            in_own_process: vec!["mine".into()],
            by_launch: Some("other".into()),
            rival: true,
            ..Facts::default()
        };
        assert_eq!(decide(&facts).as_deref(), Some("mine"));
    }

    #[test]
    fn two_open_in_its_own_processes_is_none() {
        let facts = Facts {
            in_own_process: vec!["a".into(), "b".into()],
            by_launch: Some("a".into()),
            ..Facts::default()
        };
        assert_eq!(decide(&facts), None);
    }

    #[test]
    fn one_open_in_its_processes_but_linked_elsewhere_is_none() {
        let facts = Facts {
            in_own_process: vec!["a".into()],
            by_launch: Some("a".into()),
            linked: set(&["a"]),
            ..Facts::default()
        };
        assert_eq!(decide(&facts), None);
    }

    #[test]
    fn the_launch_tells_it_when_nothing_contradicts() {
        let facts = Facts {
            by_launch: Some("found".into()),
            ..Facts::default()
        };
        assert_eq!(decide(&facts).as_deref(), Some("found"));
    }

    #[test]
    fn an_account_used_in_place_links_by_launch_only_when_records_name_the_process() {
        let recorded = Capabilities::RESUME | Capabilities::SESSION_PROCESS_RECORDED;
        let unrecorded = Capabilities::RESUME;
        // Only AI Juggler uses it: its launch tells.
        assert!(launch_may_link(false, unrecorded));
        assert!(launch_may_link(false, recorded));
        // Shared with the user's own terminal: only with the process named.
        assert!(launch_may_link(true, recorded));
        assert!(!launch_may_link(true, unrecorded));
    }

    #[test]
    fn the_launch_is_refused_when_anything_could_contradict_it() {
        let found = || Some("found".to_owned());
        // Another unlinked session of the account in the folder.
        assert_eq!(
            decide(&Facts {
                by_launch: found(),
                rival: true,
                ..Facts::default()
            }),
            None
        );
        // Linked to another session.
        assert_eq!(
            decide(&Facts {
                by_launch: found(),
                linked: set(&["found"]),
                ..Facts::default()
            }),
            None
        );
        // Open in a process that isn't this session's.
        assert_eq!(
            decide(&Facts {
                by_launch: found(),
                open_elsewhere: set(&["found"]),
                ..Facts::default()
            }),
            None
        );
        // The adapter found none, or two.
        assert_eq!(decide(&Facts::default()), None);
    }
}

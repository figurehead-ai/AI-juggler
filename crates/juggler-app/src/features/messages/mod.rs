//! E18 (D-28, D-29): messages between the agents of a pool, and the
//! user's messages to them.
//!
//! - **Sending** ([`send`]): one pipeline for an agent's `send_message`
//!   and the user's compose. Size caps, the pool's pause, the thread's
//!   state and hop cap, the sender's rate and duplicates, then
//!   `allowed(pool, post_message)` (an ask holds the message for the
//!   user's OK in Needs you), redaction as the user chose (D-14), the
//!   store, and the loop checks (ping-pong, the breaker).
//! - **Delivery** ([`delivery`]): a message reaches a session's input only
//!   through its driven runtime's `send_turn`, between turns, never over
//!   the user's typing or at a tool's permission prompt, and only with the
//!   user's permission: a Deliver click (or the user's own compose), or
//!   `allowed(pool, deliver_to_session)` at Full or by a rule, within the
//!   hop limit. A session in a terminal is never typed into: its agent
//!   reads its inbox when it chooses, and Claude Code gets a facts-only
//!   note at turn boundaries through its hooks.
//! - **The agent's tools** ([`agent`]), **shared docs** ([`docs`]),
//!   **what waits on the user** ([`needs`]) and the **commands**
//!   ([`commands`]) the app's window calls.
//!
//! Hard lines (D-28): nothing here changes a pool, a level or a rule for
//! an agent, starts a session, picks an account, reads a credential or
//! quota, or answers a tool's own prompt. Everything is AI Juggler's own
//! data on this Mac.

pub mod agent;
pub mod commands;
pub mod delivery;
pub mod docs;
pub mod dto;
pub mod needs;
pub mod send;
pub mod views;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use juggler_core::model::{SessionId, SessionStatus};
use juggler_core::storage::{MemberRef, SENDER_USER, Storage};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Event: messages changed (payload: the pools' ids).
pub const EVENT_CHANGED: &str = "messages://changed";

/// Tells the window which pools' messages changed.
pub type Notifier = Arc<dyn Fn(&str) + Send + Sync>;

/// The messages' state in the app: the delivery worker, the sessions
/// whose composer the user is typing in, and the last note each session
/// got. Everything else is in storage.
#[derive(Default)]
pub struct Messages {
    pub(crate) worker: Mutex<Option<std::sync::mpsc::Sender<String>>>,
    pub(crate) typing: Mutex<HashMap<String, Instant>>,
    pub(crate) noted: Mutex<HashMap<String, usize>>,
    notifier: Mutex<Option<Notifier>>,
}

impl std::fmt::Debug for Messages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Messages").finish_non_exhaustive()
    }
}

impl Messages {
    /// Sets what tells the window.
    pub fn set_notifier(&self, notifier: Notifier) {
        if let Ok(mut slot) = self.notifier.lock() {
            *slot = Some(notifier);
        }
    }

    /// Tells the window `pool`'s messages changed.
    pub fn notify(&self, pool: &str) {
        let notifier = self.notifier.lock().ok().and_then(|slot| slot.clone());
        if let Some(notifier) = notifier {
            notifier(pool);
        }
    }
}

/// Starts telling the window about changes (at most every 250 ms, one
/// event a burst).
pub fn start_emitter(state: &Arc<AppState>, app: tauri::AppHandle) {
    use tauri::Emitter;
    let dirty: Arc<Mutex<std::collections::BTreeSet<String>>> = Arc::default();
    let marks = Arc::clone(&dirty);
    state.messages.set_notifier(Arc::new(move |pool| {
        if let Ok(mut dirty) = marks.lock() {
            dirty.insert(pool.to_owned());
        }
    }));
    let weak = Arc::downgrade(state);
    let _ = std::thread::Builder::new()
        .name("messages-emitter".into())
        .spawn(move || {
            loop {
                // poll: coalesces a burst of messages into one event for
                // the window; ends with the app.
                std::thread::sleep(std::time::Duration::from_millis(250));
                if weak.upgrade().is_none() {
                    return;
                }
                let pools: Vec<String> = dirty
                    .lock()
                    .map(|mut d| std::mem::take(&mut *d).into_iter().collect())
                    .unwrap_or_default();
                if !pools.is_empty() {
                    let _ = app.emit(EVENT_CHANGED, pools);
                }
            }
        });
}

/// A new id: `prefix` and 16 random hex digits.
pub(crate) fn new_id(prefix: &str) -> String {
    use std::fmt::Write as _;
    let mut bytes = [0_u8; 8];
    // A failed draw still gives a unique id with the clock.
    if getrandom::fill(&mut bytes).is_err() {
        bytes = juggler_core::model::Timestamp::now().millis().to_le_bytes();
    }
    bytes.iter().fold(format!("{prefix}-"), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// SHA-256, hex.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// One member of a pool, as messages see it.
#[derive(Clone, Debug)]
pub(crate) struct Member {
    /// Its session (the card's).
    pub session_id: String,
    /// Its name.
    pub name: String,
    /// Its tool, in words.
    pub tool: String,
    /// Its member ref (its Work item, else its session).
    pub member_ref: String,
}

/// A pool's members, by their cards (names, tools, roles).
pub(crate) fn roster(state: &Arc<AppState>, pool_id: &str) -> AppResult<Vec<Member>> {
    let cards = crate::features::pools::cards::cards_impl(state, pool_id)?.cards;
    let works = {
        let storage = crate::commands::lock_storage_arc(state)?;
        storage.switch_graph().works_by_session()?
    };
    Ok(cards
        .into_iter()
        .map(|card| {
            let session = SessionId::new(card.session_id.as_str());
            Member {
                member_ref: MemberRef::of_session(&session, works.get(&session)).to_wire(),
                session_id: card.session_id,
                name: card.name,
                tool: card.tool,
            }
        })
        .collect())
}

/// Every ref a session's messages may be addressed to: its member ref
/// (its Work item's, so a switch keeps its inbox) and each session of
/// that Work item (a message sent before the switch).
pub(crate) fn refs_of(storage: &Storage, session: &SessionId) -> Vec<String> {
    let works = storage
        .switch_graph()
        .works_by_session()
        .unwrap_or_default();
    let mut refs = vec![format!("session:{}", session.as_str())];
    if let Some(work) = works.get(session) {
        refs.push(MemberRef::Work(work.clone()).to_wire());
        refs.extend(
            works
                .iter()
                .filter(|(other, w)| *w == work && *other != session)
                .map(|(other, _)| format!("session:{}", other.as_str())),
        );
    }
    refs
}

/// Whether a session runs now.
pub(crate) fn is_live(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Starting
            | SessionStatus::Running
            | SessionStatus::WaitingUser
            | SessionStatus::Stuck
    )
}

/// The live session a member ref stands for now (its Work item's newest
/// running session), when there is one.
pub(crate) fn live_session_of(storage: &Storage, member_ref: &str) -> Option<SessionId> {
    if member_ref == SENDER_USER {
        return None;
    }
    let works = storage
        .switch_graph()
        .works_by_session()
        .unwrap_or_default();
    let anchor = MemberRef::parse(member_ref)?.anchor(&works);
    let mut live: Vec<_> = storage
        .sessions()
        .live()
        .unwrap_or_default()
        .into_iter()
        .filter(|record| is_live(record.status))
        .filter(|record| MemberRef::of_session(&record.id, works.get(&record.id)) == anchor)
        .collect();
    live.sort_by_key(|record| std::cmp::Reverse(record.started_at.millis()));
    live.into_iter().next().map(|record| record.id)
}

/// A pool's name as the app shows it.
pub(crate) fn pool_name(storage: &Storage, pool_id: &str) -> AppResult<String> {
    crate::features::pools::pools_info(storage)?
        .pools
        .into_iter()
        .find(|pool| pool.id == pool_id)
        .map(|pool| pool.name)
        .ok_or_else(|| AppError::Refused("That pool isn't there any more".to_owned()))
}

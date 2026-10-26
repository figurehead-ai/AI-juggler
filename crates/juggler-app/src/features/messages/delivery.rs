//! AJ-E1803 (D-29): a message into a session, and the turn note.
//!
//! - **Only into a driven session, only through its runtime.** A queued
//!   message becomes the session's next turn through `send_turn` with the
//!   message's origin (`TurnOrigin::Delivered`), never by typing into a
//!   terminal. A session in a terminal keeps its messages in its inbox.
//! - **Only between turns** ([`pump`]): not while a turn runs, not while
//!   the tool asks the user something (a permission prompt or a question),
//!   not while the user is typing in the session's message box, and not
//!   once the session ended. The tool itself also refuses a turn while
//!   one runs, so a race loses nothing: the message stays queued.
//! - **Only with the user's permission.** A message is queued by the
//!   user's Deliver (or their compose with Deliver), or by
//!   `allowed(pool, deliver_to_session)` (a pool at Full, or a rule) within
//!   the hop limit (`decide`). At the limit the thread pauses and the
//!   user is asked in Needs you. A message queued by the pool's
//!   permission waits while the pool or the thread is paused, or the
//!   pool's delivery is stopped; the user's own click doesn't.
//! - **The note** ([`note_for`]): a Claude Code session in a terminal
//!   hears "N unread messages from …" from its hooks at turn boundaries,
//!   once per change, facts only.

use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::runtime::{LiveStatusWord, TurnInput, TurnOrigin};
use juggler_core::agent_access::{AgentAction, Decision};
use juggler_core::messaging::{self, Delivered, MessageKind, PER_TURN, Sender};
use juggler_core::model::{SessionId, Timestamp};
use juggler_core::storage::{HeldAsk, Message, RecipientState, SENDER_USER, ThreadState};

use super::{new_id, refs_of};
use crate::state::AppState;

/// How long the user's typing holds a delivery back (they may have
/// walked away from a half-written message).
pub const TYPING_FRESH: Duration = Duration::from_secs(120);

/// `via` for a message the user queued (their click).
pub const BY_USER: &str = "user";
/// `via` for a message the pool's level or a rule queued.
pub const BY_POOL: &str = "pool";

/// Queues a message for `session`'s next turn boundary (the user's
/// click: it goes even when the pool's own delivery is stopped).
pub(crate) fn queue(state: &Arc<AppState>, message_id: &str, session: &SessionId) {
    queue_as(state, message_id, session, BY_USER);
}

fn queue_as(state: &Arc<AppState>, message_id: &str, session: &SessionId, by: &str) {
    if let Ok(storage) = state.storage.lock() {
        let refs = refs_of(&storage, session);
        let _ = storage.messages().set_state(
            &[message_id.to_owned()],
            &refs,
            &[RecipientState::Unread, RecipientState::Read],
            RecipientState::Queued,
            Some(by),
            if by == BY_USER { SENDER_USER } else { "system" },
            "queued",
        );
    }
    kick(state, session);
}

/// Whether the pool's level or a rule would deliver (so an ask is the
/// hop limit's).
fn pool_delivers(state: &Arc<AppState>, pool_id: &str) -> bool {
    let Ok(storage) = state.storage.lock() else {
        return false;
    };
    storage.agent_access().access(pool_id).is_ok_and(|access| {
        access.level.is_some_and(|level| {
            level
                .actions(&access.custom)
                .contains(&AgentAction::DeliverToSession)
        }) || access.rules.contains(&AgentAction::DeliverToSession)
    })
}

/// An agent's message to a driven session: delivered at its next turn
/// boundary when the pool allows it (D-29), within the hop limit; at the
/// limit, the thread pauses and the user is asked. How, in words.
pub(crate) fn decide(
    state: &Arc<AppState>,
    message: &Message,
    sender: &SessionId,
    session: &SessionId,
) -> &'static str {
    // The hops already taken before this one (a person's word resets them).
    let taken = message.hop.saturating_sub(1);
    match crate::features::pools::access::allowed(
        state,
        sender.as_str(),
        &message.pool_id,
        AgentAction::DeliverToSession,
        Some(taken),
    ) {
        Decision::Allow => {
            queue_as(state, &message.id, session, BY_POOL);
            "at its next turn"
        }
        Decision::Ask { .. } if pool_delivers(state, &message.pool_id) => {
            if let Ok(storage) = state.storage.lock() {
                let repo = storage.messages();
                let to = repo
                    .recipients(&message.id)
                    .unwrap_or_default()
                    .into_iter()
                    .find(|r| {
                        super::live_session_of(&storage, &r.recipient_ref).as_ref() == Some(session)
                    })
                    .map_or_else(
                        || "another session".to_owned(),
                        |r| format!("«{}»", r.recipient_name),
                    );
                let _ = repo.set_thread_state(
                    &message.thread_id,
                    ThreadState::Paused,
                    Some("it reached this pool's hop limit"),
                    "system",
                );
                let _ = repo.hold(&HeldAsk {
                    id: new_id("ask"),
                    pool_id: message.pool_id.clone(),
                    actor_ref: message.sender_ref.clone(),
                    actor_name: message.sender_name.clone(),
                    action: AgentAction::DeliverToSession.as_str().to_owned(),
                    object_ref: message.id.clone(),
                    summary: format!(
                        "«{}»'s message would start {to}'s next turn",
                        message.sender_name
                    ),
                    created_at: Timestamp::now().millis(),
                });
            }
            "when the user says (the chain reached the hop limit)"
        }
        Decision::Ask { .. } => "in its inbox (the user can deliver it)",
        Decision::Deny => "in its inbox (delivery is stopped in this pool)",
    }
}

/// Asks the delivery worker to look at `session` (a turn ended, an ask
/// closed, a message was queued, the user stopped typing).
pub fn kick(state: &Arc<AppState>, session: &SessionId) {
    let Ok(mut worker) = state.messages.worker.lock() else {
        return;
    };
    if let Some(tx) = worker.as_ref()
        && tx.send(session.to_string()).is_ok()
    {
        return;
    }
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let weak = Arc::downgrade(state);
    let started = std::thread::Builder::new()
        .name("messages-delivery".into())
        .spawn(move || {
            while let Ok(session) = rx.recv() {
                let Some(state) = weak.upgrade() else { return };
                pump(&state, &SessionId::new(session));
            }
        });
    if started.is_ok() && tx.send(session.to_string()).is_ok() {
        *worker = Some(tx);
    }
}

/// The user is typing in `session`'s message box (or stopped).
pub fn set_typing(state: &Arc<AppState>, session: &SessionId, typing: bool) {
    if let Ok(mut map) = state.messages.typing.lock() {
        if typing {
            map.insert(session.to_string(), Instant::now());
        } else {
            map.remove(session.as_str());
        }
    }
    if !typing {
        kick(state, session);
    }
}

fn typing(state: &Arc<AppState>, session: &SessionId) -> bool {
    state.messages.typing.lock().is_ok_and(|map| {
        map.get(session.as_str())
            .is_some_and(|at| at.elapsed() < TYPING_FRESH)
    })
}

/// Whether `session`'s tool is idle now: driven, not ended, no turn
/// running, nothing asked of the user.
#[must_use]
pub fn idle(state: &Arc<AppState>, session: &SessionId) -> bool {
    let Some(driver) = state.runtime.driver(session.as_str()) else {
        return false;
    };
    let snap = driver.snapshot();
    !snap.ended
        && !snap.turn_running
        && snap.asks.is_empty()
        && !matches!(
            snap.status,
            Some(LiveStatusWord::Busy | LiveStatusWord::Waiting)
        )
}

/// Whether a message may go into `session` now: its tool is idle and the
/// user isn't typing to it.
#[must_use]
pub fn between_turns(state: &Arc<AppState>, session: &SessionId) -> bool {
    idle(state, session) && !typing(state, session)
}

/// What may go into `session` now (oldest first, one pool, at most
/// [`PER_TURN`]): queued by the user's click, or by the pool's level or a
/// rule while the pool, the thread and the pool's delivery are open.
/// With the refs, the pool's name and each member's tool.
type Due = (
    Vec<String>,
    Vec<juggler_core::storage::InboxRow>,
    String,
    std::collections::HashMap<String, String>,
);

fn due(state: &Arc<AppState>, session: &SessionId) -> Option<Due> {
    let Ok(storage) = state.storage.lock() else {
        return None;
    };
    let refs = refs_of(&storage, session);
    let repo = storage.messages();
    let rows: Vec<_> = repo
        .inbox(&refs, &[RecipientState::Queued], None, None, 50)
        .unwrap_or_default()
        .into_iter()
        .rev() // oldest first
        .filter(|row| {
            row.recipient.via.as_deref() == Some(BY_USER) || {
                let paused = repo
                    .pool_pause(&row.message.pool_id)
                    .ok()
                    .flatten()
                    .is_some();
                let thread_open = repo
                    .thread(&row.message.thread_id)
                    .ok()
                    .flatten()
                    .is_some_and(|thread| thread.state == ThreadState::Open);
                let stopped = storage
                    .agent_access()
                    .access(&row.message.pool_id)
                    .is_ok_and(|access| access.delivery_stopped);
                !paused && thread_open && !stopped
            }
        })
        .collect();
    let first = rows.first()?;
    let pool_id = first.message.pool_id.clone();
    let rows: Vec<_> = rows
        .into_iter()
        .filter(|row| row.message.pool_id == pool_id)
        .take(PER_TURN)
        .collect();
    let pool_name = super::pool_name(&storage, &pool_id).unwrap_or_default();
    drop(storage);
    let tools: std::collections::HashMap<String, String> = super::roster(state, &pool_id)
        .unwrap_or_default()
        .into_iter()
        .map(|member| (member.anchor, member.tool))
        .collect();
    Some((refs, rows, pool_name, tools))
}

/// Delivers what is queued for `session` as one turn, when it is between
/// turns; otherwise leaves it queued for the next boundary.
pub fn pump(state: &Arc<AppState>, session: &SessionId) {
    if !between_turns(state, session) {
        return;
    }
    let Some(driver) = state.runtime.driver(session.as_str()) else {
        return;
    };
    let Some((refs, rows, pool_name, tools)) = due(state, session) else {
        return;
    };
    let unknown = "a tool".to_owned();
    let delivered: Vec<Delivered<'_>> = rows
        .iter()
        .map(|row| Delivered {
            from: if row.message.sender_kind == SENDER_USER {
                Sender::User
            } else {
                Sender::Agent {
                    name: &row.message.sender_name,
                    tool: tools.get(&row.message.sender_ref).unwrap_or(&unknown),
                }
            },
            kind: MessageKind::parse(&row.message.kind).unwrap_or(MessageKind::Note),
            subject: &row.message.subject,
            body: &row.message.body,
            thread: &row.message.thread_id,
            hop: row.message.hop,
        })
        .collect();
    let text = messaging::delivered_turn(&pool_name, &delivered);
    let mut senders: Vec<&str> = rows
        .iter()
        .map(|row| row.message.sender_name.as_str())
        .collect();
    senders.dedup();
    let sender = match senders.as_slice() {
        [one] => (*one).to_owned(),
        _ => format!("{} messages", rows.len()),
    };
    let input = TurnInput {
        text,
        origin: TurnOrigin::Delivered { sender },
    };
    if driver.turn(input).is_err() {
        return; // a turn started meanwhile: it stays queued
    }
    let ids: Vec<String> = rows.iter().map(|row| row.message.id.clone()).collect();
    if let Ok(storage) = state.storage.lock() {
        let _ = storage.messages().set_state(
            &ids,
            &refs,
            &[RecipientState::Queued],
            RecipientState::Delivered,
            Some("turn"),
            "system",
            "delivered",
        );
    }
    if let Some(row) = rows.first() {
        state.messages.notify(&row.message.pool_id);
    }
}

/// The note a Claude Code session in a terminal gets at a turn boundary
/// (its `UserPromptSubmit` and `PostToolUse` hooks): how many messages
/// wait and from whom, once per change; `None` when nothing changed, or
/// the session hasn't the agent tools to read them.
#[must_use]
pub fn note_for(state: &Arc<AppState>, session: &SessionId) -> Option<String> {
    if !state
        .agent_bridge
        .has_tools_key(&state.data_dir, session.as_str())
    {
        return None;
    }
    let (count, senders) = {
        let storage = state.storage.lock().ok()?;
        let refs = refs_of(&storage, session);
        let repo = storage.messages();
        let (unread, mut senders) = repo.waiting(&refs, RecipientState::Unread).ok()?;
        let (queued, more) = repo.waiting(&refs, RecipientState::Queued).ok()?;
        for name in more {
            if !senders.contains(&name) {
                senders.push(name);
            }
        }
        (unread + queued, senders)
    };
    let mut noted = state.messages.noted.lock().ok()?;
    let before = noted.insert(session.to_string(), count).unwrap_or(0);
    if count == before {
        return None;
    }
    messaging::unread_note(count, &senders)
}

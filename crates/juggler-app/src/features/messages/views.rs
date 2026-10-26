//! AJ-E1802: what the user sees and does with messages in AI Juggler.
//!
//! - A pool's threads, or a session's, one page at a time; a thread's
//!   messages, newest page first (shown oldest first), with where each is
//!   for each recipient and whether the user can Deliver it.
//! - The user writes to a member or the whole pool (with Deliver, or
//!   not), delivers one message, pauses, stops or resumes a thread, pauses
//!   or resumes a pool's messages, searches them, and links pools.
//!
//! Reading a thread marks the messages to the user as read. Nothing here
//! types into a terminal: Deliver queues for a driven session's next turn
//! boundary ([`super::delivery`]).

use std::collections::HashSet;
use std::sync::Arc;

use juggler_core::messaging::MAX_THREAD_HOPS;
use juggler_core::model::SessionId;
use juggler_core::storage::{Message, RecipientState, SENDER_USER, Storage, Thread, ThreadState};

use super::dto::{
    ComposeInput, MessageInfo, PauseInfo, RecipientInfo, ThreadInfo, ThreadPage, ThreadsPage,
};
use super::send::{Author, Outgoing, send};
use super::{live_session_of, refs_of};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// A page of threads.
pub const THREADS_PAGE: u32 = 30;
/// A page of a thread's messages.
pub const MESSAGES_PAGE: u32 = 30;

fn cursor(text: Option<&str>, prefix: char) -> Option<i64> {
    text.and_then(|c| c.strip_prefix(prefix))
        .and_then(|seq| seq.parse().ok())
}

fn first_line(text: &str) -> String {
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    juggler_core::messaging::one_line(line, 140)
}

/// A thread as a row.
fn thread_info(storage: &Storage, thread: Thread, pool_name: &str) -> ThreadInfo {
    let repo = storage.messages();
    let recent = repo
        .thread_messages(&thread.id, None, 20)
        .unwrap_or_default();
    let mut people: Vec<String> = Vec::new();
    for message in recent.iter().rev() {
        let mut names = vec![message.sender_name.clone()];
        names.extend(
            repo.recipients(&message.id)
                .unwrap_or_default()
                .into_iter()
                .map(|r| r.recipient_name),
        );
        for name in names {
            if !people.contains(&name) {
                people.push(name);
            }
        }
    }
    let unread = repo
        .inbox(
            &[SENDER_USER.to_owned()],
            &[RecipientState::Unread],
            Some(&thread.id),
            None,
            200,
        )
        .map_or(0, |rows| u32::try_from(rows.len()).unwrap_or(u32::MAX));
    let last = recent.first();
    ThreadInfo {
        pool_name: pool_name.to_owned(),
        people,
        hop_cap: MAX_THREAD_HOPS,
        state: thread.state.as_str().to_owned(),
        last_from: last.map(|m| m.sender_name.clone()).unwrap_or_default(),
        last_line: last.map(|m| first_line(&m.body)).unwrap_or_default(),
        unread,
        id: thread.id,
        pool_id: thread.pool_id,
        subject: thread.subject,
        count: thread.count,
        hop: thread.hop,
        state_reason: thread.state_reason,
        updated_at_ms: thread.updated_at,
    }
}

/// Where a message is for one recipient, and what Deliver does.
fn recipient_info(
    state: &Arc<AppState>,
    storage: &Storage,
    thread_open: bool,
    recipient: juggler_core::storage::Recipient,
) -> RecipientInfo {
    let session = live_session_of(storage, &recipient.recipient_ref);
    let driven = session
        .as_ref()
        .is_some_and(|session| state.runtime.is_driven(session));
    let waiting = matches!(
        recipient.state,
        RecipientState::Unread | RecipientState::Read
    );
    let user = recipient.recipient_ref == SENDER_USER;
    let (deliverable, how) = match (user, recipient.state, &session) {
        (true, _, _) => (false, "To you".to_owned()),
        (_, RecipientState::Queued, _) => (false, "Goes in at its next turn".to_owned()),
        (_, RecipientState::Delivered, _) => (false, "Started its turn".to_owned()),
        (_, RecipientState::Held, _) => (false, "Waits on your OK".to_owned()),
        (_, RecipientState::Refused, _) => (false, "Not sent".to_owned()),
        (_, RecipientState::Acked, _) => (false, "It handled it".to_owned()),
        _ if !thread_open => (false, "The thread is paused or stopped".to_owned()),
        (_, _, Some(_)) if driven => (
            waiting,
            "Deliver puts it in as its next prompt, between turns".to_owned(),
        ),
        (_, _, Some(_)) => (
            false,
            "Runs in a terminal: it reads its messages itself; AI Juggler never types into a terminal".to_owned(),
        ),
        (_, _, None) => (
            waiting,
            "Not running: Deliver gives it this as its next prompt when you resume it".to_owned(),
        ),
    };
    RecipientInfo {
        id: recipient.recipient_ref,
        name: recipient.recipient_name,
        session_id: session.map(|s| s.to_string()),
        state: recipient.state.as_str().to_owned(),
        via: recipient.via,
        deliverable,
        how,
    }
}

fn message_info(
    state: &Arc<AppState>,
    storage: &Storage,
    thread_open: bool,
    message: Message,
) -> MessageInfo {
    let recipients = storage
        .messages()
        .recipients(&message.id)
        .unwrap_or_default()
        .into_iter()
        .map(|r| recipient_info(state, storage, thread_open, r))
        .collect();
    let from_session = message
        .sender_ref
        .strip_prefix("session:")
        .map(str::to_owned)
        .or_else(|| live_session_of(storage, &message.sender_ref).map(|s| s.to_string()));
    MessageInfo {
        id: message.id,
        thread_id: message.thread_id,
        from_name: message.sender_name,
        from_kind: if message.sender_kind == SENDER_USER {
            "user"
        } else {
            "agent"
        }
        .to_owned(),
        from_session_id: from_session,
        kind: message.kind,
        subject: message.subject,
        body: message.body,
        hop: message.hop,
        redacted: message.redacted,
        to_all: message.to_all,
        at_ms: message.created_at,
        recipients,
    }
}

/// One page of threads: a pool's, or a session's (any pool), newest
/// first; a pool's page says when its messages are paused.
///
/// # Errors
///
/// Storage failures.
pub fn threads(
    state: &Arc<AppState>,
    pool_id: Option<&str>,
    session_id: Option<&str>,
    cursor_text: Option<&str>,
) -> AppResult<ThreadsPage> {
    let storage = lock_storage_arc(state)?;
    let refs = session_id.map(|session| refs_of(&storage, &SessionId::new(session)));
    let mut rows = storage.messages().threads(
        pool_id,
        refs.as_deref(),
        cursor(cursor_text, 't'),
        THREADS_PAGE + 1,
    )?;
    let next = (rows.len() > THREADS_PAGE as usize).then(|| {
        rows.truncate(THREADS_PAGE as usize);
        rows.last().map(|thread| format!("t{}", thread.last_seq))
    });
    let names: std::collections::HashMap<String, String> =
        crate::features::pools::pools_info(&storage)?
            .pools
            .into_iter()
            .map(|pool| (pool.id, pool.name))
            .collect();
    let paused = pool_id.and_then(|pool| {
        storage
            .messages()
            .pool_pause(pool)
            .ok()
            .flatten()
            .map(|(at, reason)| PauseInfo {
                pool_id: pool.to_owned(),
                pool_name: names.get(pool).cloned().unwrap_or_default(),
                at_ms: at,
                reason,
            })
    });
    let threads = rows
        .into_iter()
        .map(|thread| {
            let name = names.get(&thread.pool_id).cloned().unwrap_or_default();
            thread_info(&storage, thread, &name)
        })
        .collect();
    Ok(ThreadsPage {
        threads,
        next: next.flatten(),
        paused,
    })
}

/// One page of a thread (the newest, or before `cursor`), oldest first;
/// the messages to the user on it are marked read.
///
/// # Errors
///
/// [`AppError::Refused`] for a thread that isn't there.
pub fn thread(
    state: &Arc<AppState>,
    thread_id: &str,
    cursor_text: Option<&str>,
) -> AppResult<ThreadPage> {
    let storage = lock_storage_arc(state)?;
    let repo = storage.messages();
    let thread = repo
        .thread(thread_id)?
        .ok_or_else(|| AppError::Refused("That thread isn't there any more".to_owned()))?;
    let mut rows = repo.thread_messages(thread_id, cursor(cursor_text, 'm'), MESSAGES_PAGE + 1)?;
    let earlier = (rows.len() > MESSAGES_PAGE as usize).then(|| {
        rows.truncate(MESSAGES_PAGE as usize);
        rows.last().map(|m| format!("m{}", m.seq))
    });
    let ids: Vec<String> = rows.iter().map(|m| m.id.clone()).collect();
    let read = repo.set_state(
        &ids,
        &[SENDER_USER.to_owned()],
        &[RecipientState::Unread],
        RecipientState::Read,
        Some("user"),
        SENDER_USER,
        "read",
    )?;
    let open = thread.state == ThreadState::Open;
    let pool_name = super::pool_name(&storage, &thread.pool_id).unwrap_or_default();
    let pool = thread.pool_id.clone();
    let info = thread_info(&storage, thread, &pool_name);
    let messages = rows
        .into_iter()
        .rev()
        .map(|message| message_info(state, &storage, open, message))
        .collect();
    drop(storage);
    if read > 0 {
        state.messages.notify(&pool);
    }
    Ok(ThreadPage {
        thread: info,
        messages,
        earlier: earlier.flatten(),
    })
}

/// The user writes to a member or the pool: a person's word (it resets
/// the thread's hops and reopens it), delivered with Deliver.
///
/// # Errors
///
/// [`AppError::Refused`] with why not.
pub fn compose(state: &Arc<AppState>, input: ComposeInput) -> AppResult<ThreadPage> {
    let sent = send(
        state,
        &Outgoing {
            pool_id: input.pool_id,
            from: Author::User,
            to: input.to,
            kind: input.kind,
            subject: input.subject,
            body: input.body,
            thread: input.thread_id,
            reply_to: None,
            deliver: input.deliver,
        },
    )
    .map_err(|refused| AppError::Refused(refused.reason.unwrap_or_default()))?;
    thread(state, &sent.message.thread_id, None)
}

/// The user's Deliver for one recipient of a message: queued for its
/// session's next turn boundary (or, for a member not running, its next
/// session's). A person's click: the thread's hops go back to 0.
///
/// # Errors
///
/// [`AppError::Refused`] when it can't be delivered.
pub fn deliver(state: &Arc<AppState>, message_id: &str, recipient: &str) -> AppResult<ThreadPage> {
    let (message, session) = {
        let storage = lock_storage_arc(state)?;
        let repo = storage.messages();
        let message = repo
            .message(message_id)?
            .ok_or_else(|| AppError::Refused("That message isn't there any more".to_owned()))?;
        let row = repo
            .recipients(message_id)?
            .into_iter()
            .find(|r| r.recipient_ref == recipient && r.recipient_ref != SENDER_USER)
            .ok_or_else(|| AppError::Refused("It wasn't sent to them".to_owned()))?;
        let session = live_session_of(&storage, &row.recipient_ref);
        if let Some(session) = &session
            && !state.runtime.is_driven(session)
        {
            return Err(AppError::Refused(
                "That session runs in a terminal: it reads its messages itself, and AI Juggler never types into a terminal".to_owned(),
            ));
        }
        repo.reset_hop(&message.thread_id)?;
        repo.set_state(
            &[message_id.to_owned()],
            std::slice::from_ref(&row.recipient_ref),
            &[RecipientState::Unread, RecipientState::Read],
            RecipientState::Queued,
            Some(super::delivery::BY_USER),
            SENDER_USER,
            "queued",
        )?;
        (message, session)
    };
    if let Some(session) = session {
        super::delivery::kick(state, &session);
    }
    state.messages.notify(&message.pool_id);
    thread(state, &message.thread_id, None)
}

/// The user's Deliver for every message waiting for a session run in AI
/// Juggler (Needs you's one click): queued for its next turn boundary,
/// at most a page; each thread's hops reset (a person's click). How many.
///
/// # Errors
///
/// [`AppError::Refused`] for a session that isn't run from AI Juggler.
pub fn deliver_waiting(state: &Arc<AppState>, session_id: &str) -> AppResult<u32> {
    let session = SessionId::new(session_id);
    if !state.runtime.is_driven(&session) {
        return Err(AppError::Refused(
            "That session isn't run from AI Juggler: it reads its messages itself".to_owned(),
        ));
    }
    let (count, pools) = {
        let storage = lock_storage_arc(state)?;
        let refs = refs_of(&storage, &session);
        let repo = storage.messages();
        let rows = repo.inbox(&refs, &[RecipientState::Unread], None, None, MESSAGES_PAGE)?;
        let mut pools = HashSet::new();
        for row in &rows {
            repo.reset_hop(&row.message.thread_id)?;
            pools.insert(row.message.pool_id.clone());
        }
        let ids: Vec<String> = rows.iter().map(|row| row.message.id.clone()).collect();
        let moved = repo.set_state(
            &ids,
            &refs,
            &[RecipientState::Unread],
            RecipientState::Queued,
            Some(super::delivery::BY_USER),
            SENDER_USER,
            "queued",
        )?;
        (u32::try_from(moved).unwrap_or(u32::MAX), pools)
    };
    super::delivery::kick(state, &session);
    for pool in pools {
        state.messages.notify(&pool);
    }
    Ok(count)
}

/// The user pauses, stops or resumes a thread (resuming resets its hops).
///
/// # Errors
///
/// [`AppError::Validation`] for an unknown state; storage failures.
pub fn set_thread(state: &Arc<AppState>, thread_id: &str, word: &str) -> AppResult<ThreadPage> {
    let next = match word {
        "open" => ThreadState::Open,
        "paused" => ThreadState::Paused,
        "stopped" => ThreadState::Stopped,
        other => {
            return Err(AppError::Validation {
                subject: "thread state",
                message: format!("{other} isn't open, paused or stopped"),
            });
        }
    };
    let pool = {
        let storage = lock_storage_arc(state)?;
        let repo = storage.messages();
        let reason = match next {
            ThreadState::Open => None,
            ThreadState::Paused => Some("you paused it"),
            ThreadState::Stopped => Some("you stopped it"),
        };
        repo.set_thread_state(thread_id, next, reason, SENDER_USER)?;
        if next == ThreadState::Open {
            repo.reset_hop(thread_id)?;
        }
        repo.thread(thread_id)?
            .map(|t| t.pool_id)
            .unwrap_or_default()
    };
    state.messages.notify(&pool);
    thread(state, thread_id, None)
}

/// The user pauses or resumes a pool's messages (the breaker's pause
/// too).
///
/// # Errors
///
/// Storage failures.
pub fn set_pool(state: &Arc<AppState>, pool_id: &str, paused: bool) -> AppResult<ThreadsPage> {
    {
        let storage = lock_storage_arc(state)?;
        super::pool_name(&storage, pool_id)?;
        if paused {
            storage
                .messages()
                .pause_pool(pool_id, "you paused them", SENDER_USER)?;
        } else {
            storage.messages().resume_pool(pool_id, SENDER_USER)?;
        }
    }
    state.messages.notify(pool_id);
    threads(state, Some(pool_id), None, None)
}

/// A pool's messages with every word of `query`, newest first.
///
/// # Errors
///
/// Storage failures.
pub fn search(state: &Arc<AppState>, pool_id: &str, query: &str) -> AppResult<Vec<MessageInfo>> {
    let storage = lock_storage_arc(state)?;
    let found = storage.messages().search(pool_id, query, 30)?;
    let open: HashSet<String> = found
        .iter()
        .filter_map(|m| storage.messages().thread(&m.thread_id).ok().flatten())
        .filter(|t| t.state == ThreadState::Open)
        .map(|t| t.id)
        .collect();
    Ok(found
        .into_iter()
        .map(|m| {
            let is_open = open.contains(&m.thread_id);
            message_info(state, &storage, is_open, m)
        })
        .collect())
}

/// The pools linked with `pool_id` (ids).
///
/// # Errors
///
/// Storage failures.
pub fn links(state: &Arc<AppState>, pool_id: &str) -> AppResult<Vec<String>> {
    Ok(lock_storage_arc(state)?.messages().links(pool_id)?)
}

/// The user links two pools, or takes the link back.
///
/// # Errors
///
/// [`AppError::Refused`] for a pool that isn't there or itself.
pub fn link(state: &Arc<AppState>, pool_id: &str, other: &str, on: bool) -> AppResult<Vec<String>> {
    let storage = lock_storage_arc(state)?;
    super::pool_name(&storage, pool_id)?;
    super::pool_name(&storage, other)?;
    if on {
        storage
            .messages()
            .link(pool_id, other)
            .map_err(|_| AppError::Refused("Pick another pool".to_owned()))?;
    } else {
        storage.messages().unlink(pool_id, other)?;
    }
    Ok(storage.messages().links(pool_id)?)
}

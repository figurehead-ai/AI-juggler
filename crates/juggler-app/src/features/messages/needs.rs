//! AJ-E1802 Req.2: messages that wait on the user, for Home's Needs you.
//!
//! - **An agent's message above its pool's level** (`ask`): held until
//!   the user answers with the same three choices as every agent ask
//!   (D-28): Allow once, Always allow in this pool (offered from the
//!   second approval), Deny.
//! - **A chain at the hop limit** (`deliver`): an agent's message would
//!   start another session's turn past the pool's hop limit (D-29); the
//!   user's Deliver sends it and resets the count, or Deny leaves it in
//!   the inbox.
//! - **A question to the user** (`question`), **a paused pool or thread**
//!   (`pool_paused`, `thread_paused`), and **messages waiting for a
//!   session that runs in AI Juggler** (`waiting`, one row a session: the
//!   user's Deliver is one click).
//!
//! The answers are the user's only: [`answer`] runs from the app's own
//! command.

use std::sync::Arc;

use juggler_core::agent_access::AgentAction;
use juggler_core::storage::{RecipientState, SENDER_USER, ThreadState};

use super::dto::MessageNeedInfo;
use super::send::{Author, route};
use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The most of each kind listed.
const EACH: u32 = 20;

fn preview(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(280)
        .collect()
}

/// Everything about messages that waits on the user, oldest first within
/// each kind.
///
/// # Errors
///
/// Storage failures.
#[allow(clippy::too_many_lines)]
pub fn needs(state: &Arc<AppState>) -> AppResult<Vec<MessageNeedInfo>> {
    let storage = lock_storage_arc(state)?;
    let repo = storage.messages();
    let names: std::collections::HashMap<String, String> =
        crate::features::pools::pools_info(&storage)?
            .pools
            .into_iter()
            .map(|pool| (pool.id, pool.name))
            .collect();
    let name_of = |pool: &str| names.get(pool).cloned().unwrap_or_default();
    let mut out = Vec::new();
    let mut asked_threads = Vec::new();
    for ask in repo.open_asks(0, EACH)? {
        let Some(message) = repo.message(&ask.object_ref)? else {
            continue;
        };
        let deliver = ask.action == AgentAction::DeliverToSession.as_str();
        let offer_always = !deliver
            && AgentAction::parse(&ask.action).is_some_and(|action| {
                storage
                    .agent_access()
                    .approvals(&ask.pool_id, action)
                    .unwrap_or(0)
                    >= 1
            });
        asked_threads.push(message.thread_id.clone());
        out.push(MessageNeedInfo {
            key: format!("ask:{}", ask.id),
            kind: if deliver { "deliver" } else { "ask" }.to_owned(),
            pool_name: name_of(&ask.pool_id),
            pool_id: ask.pool_id,
            thread_id: Some(message.thread_id.clone()),
            message_id: Some(message.id.clone()),
            session_id: None,
            ask_id: Some(ask.id),
            offer_always,
            title: ask.summary,
            detail: if deliver {
                format!(
                    "«{}» · {} agent messages in a row without you",
                    message.subject, message.hop
                )
            } else {
                format!("«{}» · above what the pool lets agents do", message.subject)
            },
            preview: Some(preview(&message.body)),
            at_ms: ask.created_at,
        });
    }
    for (pool, at, reason) in repo.paused_pools()? {
        out.push(MessageNeedInfo {
            key: format!("pool:{pool}"),
            kind: "pool_paused".to_owned(),
            pool_name: name_of(&pool),
            title: format!("Messages in «{}» are paused", name_of(&pool)),
            pool_id: pool,
            thread_id: None,
            message_id: None,
            session_id: None,
            ask_id: None,
            offer_always: false,
            detail: reason,
            preview: None,
            at_ms: at,
        });
    }
    for thread in repo.paused_threads(EACH)? {
        if asked_threads.contains(&thread.id) || thread.state != ThreadState::Paused {
            continue;
        }
        out.push(MessageNeedInfo {
            key: format!("thread:{}", thread.id),
            kind: "thread_paused".to_owned(),
            pool_name: name_of(&thread.pool_id),
            title: format!("«{}» is paused", thread.subject),
            pool_id: thread.pool_id,
            thread_id: Some(thread.id),
            message_id: None,
            session_id: None,
            ask_id: None,
            offer_always: false,
            detail: thread.state_reason.unwrap_or_else(|| "paused".to_owned()),
            preview: None,
            at_ms: thread.updated_at,
        });
    }
    for row in repo.inbox(
        &[SENDER_USER.to_owned()],
        &[RecipientState::Unread],
        None,
        None,
        EACH,
    )? {
        if row.message.kind != "question" {
            continue;
        }
        out.push(MessageNeedInfo {
            key: format!("question:{}", row.message.id),
            kind: "question".to_owned(),
            pool_name: name_of(&row.message.pool_id),
            title: format!("«{}» asks you", row.message.sender_name),
            detail: format!(
                "«{}» · {}",
                row.message.subject,
                name_of(&row.message.pool_id)
            ),
            pool_id: row.message.pool_id,
            thread_id: Some(row.message.thread_id),
            preview: Some(preview(&row.message.body)),
            message_id: Some(row.message.id),
            session_id: None,
            ask_id: None,
            offer_always: false,
            at_ms: row.message.created_at,
        });
    }
    let live = storage.sessions().live().unwrap_or_default();
    for record in live.into_iter().filter(|r| super::is_live(r.status)) {
        if !state.runtime.is_driven(&record.id) {
            continue;
        }
        let refs = super::refs_of(&storage, &record.id);
        let rows = repo.inbox(&refs, &[RecipientState::Unread], None, None, 50)?;
        let Some(newest) = rows.first() else {
            continue;
        };
        let name = newest.recipient.recipient_name.clone();
        let count = rows.len();
        out.push(MessageNeedInfo {
            key: format!("waiting:{}", record.id.as_str()),
            kind: "waiting".to_owned(),
            pool_name: name_of(&newest.message.pool_id),
            title: if count == 1 {
                format!("{name} has a message waiting")
            } else {
                format!("{name} has {count} messages waiting")
            },
            detail: format!(
                "from {} · {}",
                newest.message.sender_name,
                name_of(&newest.message.pool_id)
            ),
            pool_id: newest.message.pool_id.clone(),
            thread_id: Some(newest.message.thread_id.clone()),
            message_id: Some(newest.message.id.clone()),
            session_id: Some(record.id.to_string()),
            ask_id: None,
            offer_always: false,
            preview: Some(preview(&newest.message.body)),
            at_ms: newest.message.created_at,
        });
    }
    Ok(out)
}

/// The user's answer to a held ask: `once`, `always` (a rule for this
/// action in this pool, D-28) or `deny`.
///
/// # Errors
///
/// [`AppError::Refused`] for an ask that isn't open; validation for an
/// unknown answer.
pub fn answer(state: &Arc<AppState>, ask_id: &str, answer: &str) -> AppResult<()> {
    if !matches!(answer, "once" | "always" | "deny") {
        return Err(AppError::Validation {
            subject: "answer",
            message: format!("{answer} isn't an answer (once, always or deny)"),
        });
    }
    let (ask, message) = {
        let storage = lock_storage_arc(state)?;
        let repo = storage.messages();
        let ask = repo
            .open_asks(0, 200)?
            .into_iter()
            .find(|ask| ask.id == ask_id)
            .ok_or_else(|| AppError::Refused("That isn't waiting any more".to_owned()))?;
        let message = repo
            .message(&ask.object_ref)?
            .ok_or_else(|| AppError::Refused("That message isn't there any more".to_owned()))?;
        (ask, message)
    };
    let deliver = ask.action == AgentAction::DeliverToSession.as_str();
    if deliver && answer == "always" {
        return Err(AppError::Refused(
            "A hop limit is never lifted by a rule: Deliver once, or change the pool's hop limit"
                .to_owned(),
        ));
    }
    if !deliver {
        // The same answer as every agent ask (counted, or a rule).
        crate::features::pools::access::answer_impl(state, &ask.pool_id, &ask.action, answer)?;
    }
    {
        let storage = lock_storage_arc(state)?;
        let repo = storage.messages();
        repo.close_ask(&ask.id, answer)?;
        let refs: Vec<String> = repo
            .recipients(&message.id)?
            .into_iter()
            .map(|r| r.recipient_ref)
            .collect();
        match (deliver, answer) {
            (false, "deny") => {
                repo.set_state(
                    std::slice::from_ref(&message.id),
                    &refs,
                    &[RecipientState::Held],
                    RecipientState::Refused,
                    Some("user"),
                    SENDER_USER,
                    "refused",
                )?;
            }
            (false, _) => {
                repo.set_state(
                    std::slice::from_ref(&message.id),
                    &refs,
                    &[RecipientState::Held],
                    RecipientState::Unread,
                    None,
                    SENDER_USER,
                    "allowed",
                )?;
            }
            (true, "deny") => {
                repo.log(
                    &message.pool_id,
                    SENDER_USER,
                    "not_delivered",
                    &message.id,
                    None,
                )?;
            }
            (true, _) => {
                // A person's click: the chain starts again from 0.
                repo.reset_hop(&message.thread_id)?;
                repo.set_thread_state(&message.thread_id, ThreadState::Open, None, SENDER_USER)?;
            }
        }
    }
    match (deliver, answer) {
        (false, "deny") | (true, "deny") => {}
        (false, _) => {
            let sender = super::live_session_of(&*lock_storage_arc(state)?, &message.sender_ref);
            if let Some(session) = sender {
                let from = Author::Agent {
                    session,
                    member_ref: message.sender_ref.clone(),
                    name: message.sender_name.clone(),
                    tool: String::new(),
                };
                route(state, &message, &from, false);
            }
        }
        (true, _) => {
            let recipients = lock_storage_arc(state)?
                .messages()
                .recipients(&message.id)?;
            for recipient in recipients {
                let session =
                    super::live_session_of(&*lock_storage_arc(state)?, &recipient.recipient_ref);
                if let Some(session) = session.filter(|s| state.runtime.is_driven(s)) {
                    super::delivery::queue(state, &message.id, &session);
                }
            }
        }
    }
    state.messages.notify(&message.pool_id);
    Ok(())
}

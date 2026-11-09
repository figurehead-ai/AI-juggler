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
use juggler_core::model::Timestamp;
use juggler_core::storage::{
    HeldAsk, Message, MessagesRepo, RecipientState, SENDER_USER, Storage, ThreadState,
};

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

/// The messages an open ask is about. Each has its own card in Needs you,
/// which says what a yes does (a hop-limit delivery says so), so no other
/// card lists it and no other Deliver sends it past the ask.
///
/// # Errors
///
/// Storage failures.
pub(crate) fn asked_messages(
    repo: &juggler_core::storage::MessagesRepo<'_>,
) -> AppResult<std::collections::HashSet<String>> {
    Ok(repo
        .open_asks(0, 200)?
        .into_iter()
        .map(|ask| ask.object_ref)
        .collect())
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
    let names = crate::features::pools::pool_names(&storage)?;
    let name_of = |pool: &str| names.get(pool).cloned().unwrap_or_default();
    let mut out = Vec::new();
    let mut asked_threads = Vec::new();
    for ask in repo.open_asks(0, EACH)? {
        let Some(message) = repo.message(&ask.object_ref)? else {
            continue;
        };
        let deliver = ask.action == AgentAction::DeliverToSession.as_str();
        let talk = ask.action == super::talk::TALK_ASK;
        let watched = ask.action == super::talk::WATCHED_ASK;
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
                    crate::features::tasks::words::name_tasks(&storage, &message.subject),
                    message.hop
                )
            } else if watched {
                format!(
                    "«{}» · it runs in another app and can't read messages until it's continued in AI Juggler",
                    crate::features::tasks::words::name_tasks(&storage, &message.subject)
                )
            } else if talk {
                format!(
                    "«{}» · this pool asks you first between these members",
                    crate::features::tasks::words::name_tasks(&storage, &message.subject)
                )
            } else {
                format!(
                    "«{}» · above what the pool lets agents do",
                    crate::features::tasks::words::name_tasks(&storage, &message.subject)
                )
            },
            preview: Some(preview(&crate::features::tasks::words::name_tasks(
                &storage,
                &message.body,
            ))),
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
            title: format!(
                "«{}» is paused",
                crate::features::tasks::words::name_tasks(&storage, &thread.subject)
            ),
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
                crate::features::tasks::words::name_tasks(&storage, &row.message.subject),
                name_of(&row.message.pool_id)
            ),
            pool_id: row.message.pool_id,
            thread_id: Some(row.message.thread_id),
            preview: Some(preview(&crate::features::tasks::words::name_tasks(
                &storage,
                &row.message.body,
            ))),
            message_id: Some(row.message.id),
            session_id: None,
            ask_id: None,
            offer_always: false,
            at_ms: row.message.created_at,
        });
    }
    let asked = asked_messages(&repo)?;
    let live = storage.sessions().live().unwrap_or_default();
    for record in live.into_iter().filter(|r| super::is_live(r.status)) {
        if !state.runtime.is_driven(&record.id) {
            continue;
        }
        let refs = super::refs_of(&storage, &record.id);
        let mut rows = repo.inbox(&refs, &[RecipientState::Unread], None, None, 50)?;
        // A message with its own ask (a paused chain) is that card's, once.
        rows.retain(|row| !asked.contains(&row.message.id));
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
            preview: Some(preview(&crate::features::tasks::words::name_tasks(
                &storage,
                &newest.message.body,
            ))),
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
    let talk = ask.action == super::talk::TALK_ASK || ask.action == super::talk::WATCHED_ASK;
    if talk && answer == "always" {
        return Err(AppError::Refused(
            "Ask first is the pool's rule between these members: Deliver once, or change it on the pool's canvas"
                .to_owned(),
        ));
    }
    if !deliver && !talk {
        // The same answer as every agent ask (counted, or a rule).
        crate::features::pools::access::answer_impl(state, &ask.pool_id, &ask.action, answer)?;
    }
    lock_storage_arc(state)?
        .messages()
        .close_ask(&ask.id, answer)?;
    let waiting = settle(state, &ask, deliver, answer, &message)?;
    if answer != "deny" {
        go_on(state, deliver, &message)?;
        if !waiting.is_empty() {
            // The words a direct send to a Watched member gets.
            tell_sender(
                state,
                &message,
                &format!(
                    "Allowed in AI Juggler. {} gets it {}.",
                    waiting.join(", "),
                    super::talk::WATCHED_GETS_IT
                ),
            );
        }
    } else if !deliver {
        tell_sender(
            state,
            &message,
            "Not sent: declined in AI Juggler. This pool asks the person first between these members; don't send it again unless they ask.",
        );
    }
    state.messages.notify(&message.pool_id);
    Ok(())
}

/// What the answer changes in storage: a held message released or
/// refused; a hop-limit delivery's chain reopened from 0 (a person's
/// click), or logged as not delivered.
///
/// D-62 decision 15 (0.4.1): a yes to any other hold doesn't let a
/// Watched member read. Its copy goes on waiting, with the Watched
/// reason's own ask, exactly as a direct send to it does. Returns those
/// members' names, for the sender.
fn settle(
    state: &Arc<AppState>,
    ask: &HeldAsk,
    deliver: bool,
    answer: &str,
    message: &Message,
) -> AppResult<Vec<String>> {
    let storage = lock_storage_arc(state)?;
    let repo = storage.messages();
    let recipients = repo.recipients(&message.id)?;
    let waits: Vec<(String, String)> =
        if deliver || answer == "deny" || ask.action == super::talk::WATCHED_ASK {
            Vec::new()
        } else {
            let continued = continued(&storage);
            recipients
                .iter()
                .filter(|r| {
                    r.state == RecipientState::Held
                        && r.recipient_ref.starts_with("conversation:")
                        && !continued.contains(&r.recipient_ref)
                })
                .map(|r| (r.recipient_ref.clone(), r.recipient_name.clone()))
                .collect()
        };
    let refs: Vec<String> = recipients
        .into_iter()
        .map(|r| r.recipient_ref)
        .filter(|r| !waits.iter().any(|(waiting, _)| waiting == r))
        .collect();
    let id = std::slice::from_ref(&message.id);
    match (deliver, answer) {
        (false, "deny") => {
            repo.set_state(
                id,
                &refs,
                &[RecipientState::Held],
                RecipientState::Refused,
                Some("user"),
                SENDER_USER,
                "refused",
            )?;
        }
        (false, _) => {
            if !refs.is_empty() {
                repo.set_state(
                    id,
                    &refs,
                    &[RecipientState::Held],
                    RecipientState::Unread,
                    None,
                    SENDER_USER,
                    "allowed",
                )?;
            }
            hold_watched(&repo, ask, message, &waits)?;
        }
        (true, "deny") => repo.log(
            &message.pool_id,
            SENDER_USER,
            "not_delivered",
            &message.id,
            None,
        )?,
        (true, _) => {
            repo.reset_hop(&message.thread_id)?;
            repo.set_thread_state(&message.thread_id, ThreadState::Open, None, SENDER_USER)?;
        }
    }
    Ok(waits.into_iter().map(|(_, name)| name).collect())
}

/// The Watched refs (`conversation:folder/id`) AI Juggler's own sessions
/// have continued: those read their messages now.
fn continued(storage: &Storage) -> std::collections::HashSet<String> {
    storage
        .sessions()
        .pool_facts()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|facts| {
            let (folder, id) = facts.conversation?;
            Some(format!("conversation:{folder}/{id}"))
        })
        .collect()
}

/// The Watched wait for `waits`, as a direct send opens it (one a
/// message; the one already open stands).
fn hold_watched(
    repo: &MessagesRepo<'_>,
    ask: &HeldAsk,
    message: &Message,
    waits: &[(String, String)],
) -> AppResult<()> {
    if waits.is_empty()
        || repo
            .open_asks(0, 200)?
            .iter()
            .any(|open| open.object_ref == message.id && open.action == super::talk::WATCHED_ASK)
    {
        return Ok(());
    }
    let names: Vec<&str> = waits.iter().map(|(_, name)| name.as_str()).collect();
    repo.hold(&HeldAsk {
        id: super::new_id("ask"),
        pool_id: message.pool_id.clone(),
        actor_ref: ask.actor_ref.clone(),
        actor_name: ask.actor_name.clone(),
        action: super::talk::WATCHED_ASK.to_owned(),
        object_ref: message.id.clone(),
        summary: format!("«{}» wants to message {}", ask.actor_name, names.join(", ")),
        created_at: Timestamp::now().millis(),
    })?;
    Ok(())
}

/// P2 step 2: a refused message's sender is told, in words, in its own
/// inbox (the thread it wrote in): a refusal it never hears of looks
/// like a recipient that didn't answer. So is a released one that still
/// waits on a Watched member. Best effort: a sender no longer
/// running reads it in the thread.
fn tell_sender(state: &Arc<AppState>, message: &Message, body: &str) {
    let Ok(Some(session)) = lock_storage_arc(state)
        .map(|storage| super::live_session_of(&storage, &message.sender_ref))
    else {
        return;
    };
    let note = super::send::Outgoing {
        pool_id: message.pool_id.clone(),
        from: Author::User,
        to: session.as_str().to_owned(),
        kind: None,
        subject: None,
        body: body.to_owned(),
        thread: Some(message.thread_id.clone()),
        reply_to: Some(message.id.clone()),
        deliver: false,
    };
    if let Err(refused) = super::send::send(state, &note) {
        tracing::warn!(
            "the sender wasn't told of the refusal: {:?}",
            refused.reason
        );
    }
}

/// After a yes: a released message goes where it would have (the pool's
/// level and the hop limit decide its delivery); a hop-limit delivery is
/// queued as the user's click.
fn go_on(state: &Arc<AppState>, deliver: bool, message: &Message) -> AppResult<()> {
    if !deliver {
        let sender = super::live_session_of(&*lock_storage_arc(state)?, &message.sender_ref);
        if let Some(session) = sender {
            let from = Author::Agent {
                session,
                member_ref: message.sender_ref.clone(),
                name: message.sender_name.clone(),
                tool: String::new(),
            };
            route(state, message, &from, false);
        }
        return Ok(());
    }
    let recipients = lock_storage_arc(state)?
        .messages()
        .recipients(&message.id)?;
    for recipient in recipients {
        let session = super::live_session_of(&*lock_storage_arc(state)?, &recipient.recipient_ref);
        if let Some(session) = session.filter(|s| state.runtime.is_driven(s)) {
            super::delivery::queue(state, &message.id, &session);
        }
    }
    Ok(())
}

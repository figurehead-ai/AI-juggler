//! One pipeline for every message, an agent's or the user's (E18,
//! AJ-E1801). In order:
//!
//! 1. its shape: kind, a one-line subject, a body within the size cap;
//! 2. the pool: paused (the breaker, or the user) refuses an agent;
//! 3. the thread: stopped or paused refuses an agent; past the hop cap
//!    an agent's message is refused and the thread pauses for the user;
//! 4. the recipients: members of the pool, the whole pool, the user, or a
//!    member of a pool the user linked (never otherwise);
//! 5. the sender's rate (10 a minute, 100 a day in the pool) and
//!    duplicates (10 minutes); a sender over its rate three times in ten
//!    minutes pauses the pool;
//! 6. `allowed(pool, post_message)` for an agent (D-28): an ask holds
//!    the message for the user's OK in Needs you;
//! 7. secrets handled as the user chose (D-14: Keep keeps them, anything
//!    else replaces them with •••);
//! 8. the store, and the loop check: two agents answering each other
//!    without a person pause the pool;
//! 9. routing: where it goes for each recipient ([`route`]).

use std::collections::BTreeSet;
use std::sync::Arc;

use juggler_agentbridge::wire::{Refusal, Response};
use juggler_core::agent_access::{AgentAction, Decision};
use juggler_core::messaging::{self, BodyError, DUPLICATE_MS, Limit, MessageKind, SUBJECT_MAX};
use juggler_core::model::{SessionId, Timestamp};
use juggler_core::storage::{
    HeldAsk, Message, NewMessage, RecipientState, SENDER_USER, Storage, ThreadState,
};

use super::{Member, new_id, roster, sha256_hex};
use crate::commands::lock_storage_arc;
use crate::state::AppState;

/// Who sends.
#[derive(Clone, Debug)]
pub enum Author {
    /// An agent, through the bridge: its session, member ref, name, tool.
    Agent {
        /// Its session.
        session: SessionId,
        /// Its member ref.
        member_ref: String,
        /// Its name.
        name: String,
        /// Its tool, in words.
        tool: String,
    },
    /// The user, in AI Juggler.
    User,
}

impl Author {
    fn sender_ref(&self) -> &str {
        match self {
            Self::Agent { member_ref, .. } => member_ref,
            Self::User => SENDER_USER,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Agent { name, .. } => name,
            Self::User => "You",
        }
    }
}

/// One message to send.
#[derive(Clone, Debug)]
pub struct Outgoing {
    /// The sender's pool.
    pub pool_id: String,
    /// Who.
    pub from: Author,
    /// A member's session id, `pool`, or `user`.
    pub to: String,
    /// Its kind's word (a note when `None`).
    pub kind: Option<String>,
    /// A new thread's subject.
    pub subject: Option<String>,
    /// What it says.
    pub body: String,
    /// The thread it answers in.
    pub thread: Option<String>,
    /// The message it answers.
    pub reply_to: Option<String>,
    /// The user's Deliver with it (their click is the permission).
    pub deliver: bool,
}

/// What became of a message.
#[derive(Clone, Debug)]
pub struct Sent {
    /// The stored message.
    pub message: Message,
    /// Held for the user's OK (D-28's ask).
    pub held: bool,
    /// For each recipient: their name, and how it reaches them, in words.
    pub routes: Vec<(String, &'static str)>,
}

fn refuse(refusal: Refusal, words: impl Into<String>) -> Response {
    Response::refused(refusal, words)
}

/// The recipients `to` names, from the sender's pool and the pools the
/// user linked with it; `(ref, name, session id)`.
fn recipients(
    state: &Arc<AppState>,
    storage_links: &[String],
    pool_id: &str,
    from: &Author,
    to: &str,
) -> Result<Vec<(String, String, Option<String>)>, Response> {
    let mine = roster(state, pool_id)
        .map_err(|_| refuse(Refusal::Bad, "AI Juggler couldn't read the pool."))?;
    let me = from.sender_ref();
    let named = |member: &Member| {
        (
            member.anchor.clone(),
            member.name.clone(),
            Some(member.session_id.clone()),
        )
    };
    match to.trim() {
        "user" if matches!(from, Author::User) => {
            Err(refuse(Refusal::Bad, "Pick a member or the whole pool."))
        }
        "user" => Ok(vec![(SENDER_USER.to_owned(), "You".to_owned(), None)]),
        "pool" => {
            let all: Vec<_> = mine
                .iter()
                .filter(|member| member.anchor != me)
                .map(named)
                .collect();
            if all.is_empty() {
                Err(refuse(Refusal::Deny, "There's no one else in this pool."))
            } else {
                Ok(all)
            }
        }
        who => {
            if let Some(member) = mine.iter().find(|m| m.session_id == who) {
                if member.anchor == me {
                    return Err(refuse(Refusal::Bad, "That's this session itself."));
                }
                return Ok(vec![named(member)]);
            }
            for linked in storage_links {
                if let Ok(theirs) = roster(state, linked)
                    && let Some(member) = theirs.iter().find(|m| m.session_id == who)
                {
                    return Ok(vec![named(member)]);
                }
            }
            Err(refuse(
                Refusal::Deny,
                "No one with that session_id is in this pool (see pool_roster). A member of another pool can be written to only when the user links the two pools, in AI Juggler.",
            ))
        }
    }
}

/// D-14: the user's choice for secrets: keep them only when they chose
/// Keep; otherwise replace them with •••, since no one is there to ask
/// when an agent writes. `(keep, the choice in words)`.
pub(crate) fn secrets_policy(storage: &Storage) -> (bool, &'static str) {
    use juggler_handover::redaction::RedactionPolicy;
    match crate::commands::redaction_global_of(storage) {
        Some(RedactionPolicy::Keep) => (true, "Keep — your choice for secrets"),
        Some(RedactionPolicy::Strip) => (false, "Strip — your choice for secrets"),
        Some(RedactionPolicy::AskEachTime) => (
            false,
            "Stripped — your choice is Ask each time, and no one is asked when an agent writes",
        ),
        None => (
            false,
            "Stripped — you haven't chosen yet (Settings › Privacy)",
        ),
    }
}

/// Texts with their secrets handled; and how many were found.
pub(crate) fn redacted(
    storage: &Storage,
    texts: &[String],
) -> (Vec<String>, u32, Vec<(String, u32)>) {
    let (keep, _) = secrets_policy(storage);
    let (out, counts) = juggler_handover::redaction::strip_segments(texts, !keep, &BTreeSet::new());
    let total = counts.iter().map(|count| count.count).sum();
    (
        out,
        if keep { 0 } else { total },
        counts.into_iter().map(|c| (c.kind, c.count)).collect(),
    )
}

/// The shape of a message: its kind, subject and body, or why not.
fn shaped(
    out: &Outgoing,
    thread_subject: Option<&str>,
) -> Result<(MessageKind, String, String), Response> {
    let kind = match out.kind.as_deref() {
        None | Some("") => MessageKind::Note,
        Some(word) => MessageKind::parse(word).ok_or_else(|| {
            refuse(
                Refusal::Bad,
                "A kind is note, question, answer, handoff, review_request or status.",
            )
        })?,
    };
    let body = messaging::clean_body(&out.body).map_err(|error| match error {
        BodyError::Empty => refuse(Refusal::Bad, "A message needs a body."),
        BodyError::TooLong => refuse(
            Refusal::Limit,
            format!(
                "A message is at most {} characters: share a longer text with share_doc.",
                messaging::BODY_MAX
            ),
        ),
    })?;
    let subject = out
        .subject
        .as_deref()
        .map(|subject| messaging::one_line(subject, SUBJECT_MAX))
        .filter(|subject| !subject.is_empty())
        .or_else(|| thread_subject.map(str::to_owned))
        .unwrap_or_else(|| messaging::one_line(body.lines().next().unwrap_or("Message"), 60));
    Ok((kind, subject, body))
}

/// Checks an agent's rate and duplicates; logs a hit, and pauses the
/// pool when it hits too often (the breaker).
fn within_rate(
    storage: &Storage,
    pool_id: &str,
    pool_name: &str,
    from: &Author,
    hash: &str,
) -> Result<(), Response> {
    let Author::Agent {
        member_ref, name, ..
    } = from
    else {
        return Ok(());
    };
    let now = Timestamp::now().millis();
    let repo = storage.messages();
    let bad = |_| refuse(Refusal::Bad, "AI Juggler couldn't read its messages.");
    if let Some(earlier) = repo
        .duplicate_of(pool_id, member_ref, hash, now - DUPLICATE_MS)
        .map_err(bad)?
    {
        let _ = repo.log(pool_id, member_ref, "dropped_duplicate", &earlier.id, None);
        return Err(refuse(
            Refusal::Limit,
            format!(
                "The same message went to the same people {} min ago ({}): it isn't sent again.",
                ((now - earlier.created_at) / 60_000).max(1),
                earlier.id
            ),
        ));
    }
    let sent = repo
        .sent_since(pool_id, member_ref, now - messaging::DAY_MS)
        .map_err(bad)?;
    let Some(limit) = messaging::rate_limit(&sent, now) else {
        return Ok(());
    };
    let _ = repo.log(pool_id, member_ref, "rate_limited", pool_id, None);
    let hits = repo
        .verbs_since(
            pool_id,
            member_ref,
            "rate_limited",
            now - messaging::BREAKER_MS,
        )
        .unwrap_or_default();
    if messaging::breaker_trips(&hits, now) {
        let _ = repo.pause_pool(
            pool_id,
            &format!("«{name}» kept sending past its limit"),
            "system",
        );
    }
    Err(refuse(
        Refusal::Limit,
        match limit {
            Limit::PerMinute { retry_after_ms } => format!(
                "At most {} messages a minute in «{pool_name}»: wait {} s.",
                messaging::PER_MINUTE,
                (retry_after_ms / 1000).max(1)
            ),
            Limit::PerDay => format!(
                "At most {} messages a day in «{pool_name}».",
                messaging::PER_DAY
            ),
        },
    ))
}

/// Sends one message. An `Err` is the refusal, in words, for the agent
/// or the user; nothing was stored (except a held message, which is `Ok`
/// with `held`).
///
/// # Errors
///
/// The refusal, as the bridge answers it.
#[allow(clippy::too_many_lines)]
pub fn send(state: &Arc<AppState>, out: &Outgoing) -> Result<Sent, Response> {
    let agent = matches!(out.from, Author::Agent { .. });
    let busy = || refuse(Refusal::Bad, "AI Juggler is busy: try again.");
    let (pool_name, thread, links) = {
        let storage = lock_storage_arc(state).map_err(|_| busy())?;
        let pool_name = super::pool_name(&storage, &out.pool_id)
            .map_err(|_| refuse(Refusal::Deny, "This pool isn't there any more."))?;
        if agent && let Ok(Some((_, why))) = storage.messages().pool_pause(&out.pool_id) {
            return Err(refuse(
                Refusal::Limit,
                format!(
                    "Messages in «{pool_name}» are paused ({why}). Only the user can resume them, in AI Juggler."
                ),
            ));
        }
        let thread = match out.thread.as_deref() {
            Some(id) => Some(
                storage
                    .messages()
                    .thread(id)
                    .ok()
                    .flatten()
                    .filter(|thread| thread.pool_id == out.pool_id)
                    .ok_or_else(|| refuse(Refusal::Deny, "That thread isn't in this pool."))?,
            ),
            None => None,
        };
        let links = storage.messages().links(&out.pool_id).unwrap_or_default();
        (pool_name, thread, links)
    };
    if agent && let Some(thread) = &thread {
        match thread.state {
            ThreadState::Open => {}
            ThreadState::Paused => {
                return Err(refuse(
                    Refusal::Limit,
                    format!(
                        "This thread is paused ({}). Only the user can resume it.",
                        thread.state_reason.as_deref().unwrap_or("by the user")
                    ),
                ));
            }
            ThreadState::Stopped => {
                return Err(refuse(Refusal::Deny, "The user stopped this thread."));
            }
        }
    }
    let (kind, subject, body) = shaped(out, thread.as_ref().map(|t| t.subject.as_str()))?;
    let hop = messaging::next_hop(thread.as_ref().map_or(0, |t| t.hop), !agent);
    if agent && messaging::over_hops(hop) {
        if let (Some(thread), Ok(storage)) = (&thread, lock_storage_arc(state)) {
            let _ = storage.messages().set_thread_state(
                &thread.id,
                ThreadState::Paused,
                Some(&format!(
                    "{} agent messages in a row without you",
                    messaging::MAX_THREAD_HOPS
                )),
                "system",
            );
        }
        state.messages.notify(&out.pool_id);
        return Err(refuse(
            Refusal::Limit,
            format!(
                "This thread had {} agent messages in a row without the user: it's paused until the user resumes it.",
                messaging::MAX_THREAD_HOPS
            ),
        ));
    }
    let to = recipients(state, &links, &out.pool_id, &out.from, &out.to)?;
    let mut names: Vec<&str> = to.iter().map(|(r, _, _)| r.as_str()).collect();
    names.sort_unstable();
    let hash = sha256_hex(
        format!(
            "{}\n{}\n{}\n{}\n{}",
            out.from.sender_ref(),
            names.join(","),
            kind.as_str(),
            subject,
            body
        )
        .as_bytes(),
    );
    let decision = match &out.from {
        Author::Agent { session, .. } => crate::features::pools::access::allowed(
            state,
            session.as_str(),
            &out.pool_id,
            AgentAction::PostMessage,
            None,
        ),
        Author::User => Decision::Allow,
    };
    let storage = lock_storage_arc(state).map_err(|_| busy())?;
    match decision {
        Decision::Deny => {
            return Err(refuse(Refusal::Deny, "AI Juggler doesn't allow that here."));
        }
        Decision::Allow | Decision::Ask { .. } => {}
    }
    within_rate(&storage, &out.pool_id, &pool_name, &out.from, &hash)?;
    let held = matches!(decision, Decision::Ask { .. });
    let (texts, withheld, _) = redacted(&storage, &[subject, body]);
    let repo = storage.messages();
    let thread_id = if let Some(thread) = &thread {
        thread.id.clone()
    } else {
        let id = new_id("thr");
        repo.create_thread(&id, &out.pool_id, &texts[0], out.from.sender_ref())
            .map_err(|_| busy())?;
        id
    };
    let start = if held {
        RecipientState::Held
    } else {
        RecipientState::Unread
    };
    let rows: Vec<(String, String, RecipientState)> = to
        .iter()
        .map(|(r, name, _)| (r.clone(), name.clone(), start))
        .collect();
    let id = new_id("msg");
    let message = repo
        .insert_message(&NewMessage {
            id: &id,
            thread_id: &thread_id,
            pool_id: &out.pool_id,
            sender_ref: out.from.sender_ref(),
            sender_kind: if agent { "agent" } else { SENDER_USER },
            sender_name: out.from.name(),
            kind: kind.as_str(),
            subject: &texts[0],
            body: &texts[1],
            body_sha256: &hash,
            hop,
            reply_to: out.reply_to.as_deref(),
            to_all: out.to.trim() == "pool",
            redacted: withheld,
            recipients: &rows,
        })
        .map_err(|_| busy())?;
    if !agent
        || thread
            .as_ref()
            .is_some_and(|t| t.state != ThreadState::Open)
    {
        // A person's word reopens the thread (and has reset its hop).
        let _ = repo.set_thread_state(&thread_id, ThreadState::Open, None, SENDER_USER);
    }
    if held
        && let Author::Agent {
            member_ref, name, ..
        } = &out.from
    {
        let ask = HeldAsk {
            id: new_id("ask"),
            pool_id: out.pool_id.clone(),
            actor_ref: member_ref.clone(),
            actor_name: name.clone(),
            action: AgentAction::PostMessage.as_str().to_owned(),
            object_ref: message.id.clone(),
            summary: format!(
                "«{name}» wants to message {}",
                to.iter()
                    .map(|(_, n, _)| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            created_at: Timestamp::now().millis(),
        };
        let _ = repo.hold(&ask);
    }
    if agent && !held {
        loop_check(&storage, &out.pool_id);
    }
    drop(storage);
    let routes = if held {
        to.iter()
            .map(|(_, name, _)| (name.clone(), "when the user allows it"))
            .collect()
    } else {
        route(state, &message, &out.from, out.deliver)
    };
    state.messages.notify(&out.pool_id);
    Ok(Sent {
        message,
        held,
        routes,
    })
}

/// Two agents answering each other without a person pause the pool.
fn loop_check(storage: &Storage, pool_id: &str) {
    let now = Timestamp::now().millis();
    let repo = storage.messages();
    let traffic = repo
        .traffic(pool_id, now - messaging::PING_PONG_MS)
        .unwrap_or_default();
    if let Some((a, b)) = messaging::ping_pong(&traffic, now) {
        let names = |member: &str| {
            repo.sender_name(pool_id, member)
                .ok()
                .flatten()
                .unwrap_or_else(|| "an agent".to_owned())
        };
        let mut pair = [names(&a), names(&b)];
        pair.sort();
        let _ = repo.pause_pool(
            pool_id,
            &format!(
                "«{}» and «{}» kept answering each other ({} messages without you)",
                pair[0],
                pair[1],
                messaging::PING_PONG_RUN
            ),
            "system",
        );
    }
}

/// Where a stored (not held) message goes for each recipient, and how,
/// in words. A recipient whose session runs in AI Juggler gets it at its
/// next turn boundary when the user said so (their compose with Deliver)
/// or the pool allows it (Full, or a rule, within the hop limit); at the
/// hop limit the thread pauses and the user is asked. Otherwise it waits
/// in the inbox (the agent reads it when it chooses).
pub(crate) fn route(
    state: &Arc<AppState>,
    message: &Message,
    from: &Author,
    deliver: bool,
) -> Vec<(String, &'static str)> {
    let Ok(storage) = lock_storage_arc(state) else {
        return Vec::new();
    };
    let recipients = storage
        .messages()
        .recipients(&message.id)
        .unwrap_or_default();
    let mut targets = Vec::new();
    for recipient in &recipients {
        if recipient.recipient_ref == SENDER_USER {
            targets.push((recipient.recipient_name.clone(), "in your messages", None));
            continue;
        }
        let session = super::live_session_of(&storage, &recipient.recipient_ref)
            .filter(|session| state.runtime.is_driven(session));
        targets.push((recipient.recipient_name.clone(), "in its inbox", session));
    }
    drop(storage);
    let mut routes = Vec::new();
    for (name, inbox, session) in targets {
        let Some(session) = session else {
            routes.push((name, inbox));
            continue;
        };
        let how = match from {
            Author::User if deliver => {
                super::delivery::queue(state, &message.id, &session);
                "at its next turn"
            }
            Author::User => "in its inbox",
            Author::Agent {
                session: sender, ..
            } => super::delivery::decide(state, message, sender, &session),
        };
        routes.push((name, how));
    }
    routes
}

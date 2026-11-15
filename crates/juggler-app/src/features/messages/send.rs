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
//! 9. routing: where it goes for each recipient (`route`).

use std::collections::BTreeSet;
use std::sync::Arc;

use juggler_agentbridge::wire::{Refusal, Response};
use juggler_core::agent_access::{AgentAction, Decision};
use juggler_core::messaging::{self, BodyError, DUPLICATE_MS, Limit, MessageKind, SUBJECT_MAX};
use juggler_core::model::{SessionId, Timestamp};
use juggler_core::storage::{
    HeldAsk, Message, NewMessage, RecipientState, SENDER_USER, Storage, ThreadState,
};

use juggler_core::pools::talk::HoldReason;

use super::{Member, new_id, roster, sha256_hex};
use crate::commands::lock_storage_arc;
use crate::features::connector::reach::DesktopConversation;
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

/// One recipient: `(ref, name, session id)`, its role where it's reached
/// and the linked pool it's reached in (`None`: the sender's own).
type To = (Addressee, String, Option<String>);

/// A recipient as a message's rows name it: `(ref, name, session id)`.
type Addressee = (String, String, Option<String>);

/// What the talk rules leave: the kept recipients, why each one's copy
/// waits on the person when it does, and the words for those left out.
type Ruled = (Vec<Addressee>, Vec<Option<HoldReason>>, Vec<&'static str>);

/// The recipients `to` names, from the sender's pool and the pools the
/// user linked with it, and the sender's own role in its pool. `desktop`
/// is the pool's desktop conversations (AJ-E3013): a connected one is a
/// member like any other (the whole pool reaches it too), a Watched one
/// only by its own ref.
fn recipients(
    state: &Arc<AppState>,
    storage_links: &[String],
    pool_id: &str,
    from: &Author,
    to: &str,
    desktop: &[DesktopConversation],
) -> Result<(Vec<To>, Option<String>), Response> {
    let mine = roster(state, pool_id)
        .map_err(|_| refuse(Refusal::Bad, "AI Juggler couldn't read the pool."))?;
    let me = from.sender_ref();
    let my_role = mine
        .iter()
        .find(|member| member.anchor == me)
        .map(|member| member.role.clone())
        .or_else(|| {
            desktop
                .iter()
                .find(|one| one.member == me)
                .map(|one| one.role.clone())
        });
    let as_to = |one: &DesktopConversation| -> To {
        (
            (one.member.clone(), one.name.clone(), one.session.clone()),
            one.role.clone(),
            None,
        )
    };
    let named = |member: &Member, linked: Option<&String>| -> To {
        (
            (
                member.anchor.clone(),
                member.name.clone(),
                Some(member.session_id.clone()),
            ),
            member.role.clone(),
            linked.cloned(),
        )
    };
    let found = match to.trim() {
        "user" if matches!(from, Author::User) => {
            Err(refuse(Refusal::Bad, "Pick a member or the whole pool."))
        }
        "user" => Ok(vec![(
            (SENDER_USER.to_owned(), "You".to_owned(), None),
            String::new(),
            None,
        )]),
        "pool" => {
            let all: Vec<_> = mine
                .iter()
                .filter(|member| member.anchor != me)
                .map(|member| named(member, None))
                .chain(
                    desktop
                        .iter()
                        .filter(|one| one.connected && one.member != me)
                        .map(as_to),
                )
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
                return Ok((vec![named(member, None)], my_role));
            }
            // AJ-E3013: a desktop conversation, by the ref pool_roster
            // gives it (or its attested session's id). Connected, it reads
            // it through AI Juggler's connector; Watched (D-62 decision
            // 15), its copy waits until it's continued in AI Juggler.
            if let Some(one) = desktop
                .iter()
                .find(|one| one.member == who || one.session.as_deref() == Some(who))
            {
                if one.member == me {
                    return Err(refuse(Refusal::Bad, "That's this session itself."));
                }
                return Ok((vec![as_to(one)], my_role));
            }
            for linked in storage_links {
                if let Ok(theirs) = roster(state, linked)
                    && let Some(member) = theirs.iter().find(|m| m.session_id == who)
                {
                    return Ok((vec![named(member, Some(linked))], my_role));
                }
            }
            Err(refuse(Refusal::Deny, not_here(state, pool_id, who)))
        }
    }?;
    Ok((found, my_role))
}

/// Why `who` can't be written to from `pool_id`, in true words: a link
/// lets a member of another pool in, but a sealed pool takes no links
/// (D-62 decision 14; the rc27 P7 walk's refusal offered one).
fn not_here(state: &Arc<AppState>, pool_id: &str, who: &str) -> String {
    const NOT_IN: &str = "No one with that session_id is in this pool (see pool_roster).";
    let Ok(storage) = lock_storage_arc(state) else {
        return NOT_IN.to_owned();
    };
    let sealed = |id: &str| storage.pools().get(id).ok().filter(|pool| pool.sealed);
    if let Some(pool) = sealed(pool_id) {
        return format!(
            "{NOT_IN} «{}» is sealed: nothing goes out of it, and it can't be linked to another pool.",
            pool.name
        );
    }
    let theirs = crate::features::pools::members::resolution_of(&storage, &SessionId::new(who))
        .ok()
        .flatten()
        .and_then(|resolution| resolution.primary().map(|place| place.pool_id.clone()))
        .and_then(|id| sealed(&id));
    if let Some(pool) = theirs {
        return format!(
            "{NOT_IN} That session is in «{}», which is sealed: nothing comes into it from another pool.",
            pool.name
        );
    }
    format!(
        "{NOT_IN} A member of another pool can be written to only when the user links the two pools, in AI Juggler."
    )
}

/// D-62: the pool's talk rules on an agent's recipients: the refused are
/// left out (`Err` when none is left), and each kept one says whether its
/// copy waits on the person.
fn ruled(
    storage: &Storage,
    pool_id: &str,
    sender: (&str, Option<&str>),
    found: &[To],
    desktop: &[DesktopConversation],
) -> Result<Ruled, Response> {
    use juggler_core::storage::PoolRole;
    let pools = storage.pools();
    let sealed_of = |id: &str| pools.get(id).is_ok_and(|pool| pool.sealed);
    let role = |word: &str| PoolRole::parse(word).unwrap_or(PoolRole::Member);
    let receivers: Vec<super::talk::Receiver<'_>> = found
        .iter()
        .map(|((anchor, ..), word, linked)| super::talk::Receiver {
            anchor,
            role: role(word),
            linked: linked.is_some(),
            sealed: linked.as_deref().is_some_and(sealed_of),
            // AJ-E3013: only a desktop conversation with no connector is
            // Watched; a connected one is reached through its tools.
            watched: anchor.starts_with("conversation:")
                && !desktop
                    .iter()
                    .any(|one| one.connected && one.member == *anchor),
        })
        .collect();
    let decided = super::talk::rule(
        storage,
        pool_id,
        (sender.0, role(sender.1.unwrap_or("member"))),
        sealed_of(pool_id),
        &receivers,
    )?;
    let mut kept = Vec::new();
    let mut held = Vec::new();
    for (at, hold) in decided.kept {
        kept.push(found[at].0.clone());
        held.push(hold);
    }
    Ok((kept, held, decided.refused))
}

/// D-14: the user's choice for secrets: keep them only when they chose
/// Keep; otherwise replace them with •••, since no one is there to ask
/// when an agent writes. `(keep, the choice in words)`.
pub(crate) fn secrets_policy(storage: &Storage) -> (bool, &'static str) {
    use juggler_handover::redaction::RedactionPolicy;
    match crate::features::redaction::redaction_global_of(storage) {
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
    // One limit per session, across every pool it's in (P11 step 1: two
    // pools never double what it may send); the breaker stays per pool.
    let sent = repo
        .sent_since(member_ref, now - messaging::DAY_MS)
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
                "This session sends at most {} messages a minute, in all its pools together: wait {} s.",
                messaging::PER_MINUTE,
                (retry_after_ms / 1000).max(1)
            ),
            Limit::PerDay => format!(
                "This session sends at most {} messages a day, in all its pools together.",
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
    let (thread, links) = {
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
        (thread, links)
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
    let desktop =
        crate::features::connector::reach::of_pool(state, &out.pool_id).unwrap_or_default();
    let (found, my_role) = recipients(state, &links, &out.pool_id, &out.from, &out.to, &desktop)?;
    let (to, talk_held, left_out) = if agent {
        let storage = lock_storage_arc(state).map_err(|_| busy())?;
        ruled(
            &storage,
            &out.pool_id,
            (out.from.sender_ref(), my_role.as_deref()),
            &found,
            &desktop,
        )?
    } else {
        let n = found.len();
        (
            found.into_iter().map(|(to, ..)| to).collect(),
            vec![None; n],
            Vec::new(),
        )
    };
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
    within_rate(&storage, &out.pool_id, &out.from, &hash)?;
    let held = matches!(decision, Decision::Ask { .. });
    // D-62: the copies a pool's ask-first rule holds, when the level
    // doesn't hold the whole message already.
    let talk_holds = !held && talk_held.iter().any(Option::is_some);
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
        .zip(&talk_held)
        .map(|((r, name, _), hold)| {
            let state = if hold.is_some() {
                RecipientState::Held
            } else {
                start
            };
            (r.clone(), name.clone(), state)
        })
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
    if talk_holds
        && let Author::Agent {
            member_ref, name, ..
        } = &out.from
    {
        // One held ask per reason: ask first, and a Watched member's wait.
        for (reason, action) in [
            (HoldReason::AskFirst, super::talk::TALK_ASK),
            (HoldReason::Watched, super::talk::WATCHED_ASK),
        ] {
            let waiting: Vec<&str> = to
                .iter()
                .zip(&talk_held)
                .filter(|(_, hold)| **hold == Some(reason))
                .map(|((_, n, _), _)| n.as_str())
                .collect();
            if waiting.is_empty() {
                continue;
            }
            let _ = repo.hold(&HeldAsk {
                id: new_id("ask"),
                pool_id: out.pool_id.clone(),
                actor_ref: member_ref.clone(),
                actor_name: name.clone(),
                action: action.to_owned(),
                object_ref: message.id.clone(),
                summary: format!("«{name}» wants to message {}", waiting.join(", ")),
                created_at: Timestamp::now().millis(),
            });
        }
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
        let mut routes = route_with(state, &message, &out.from, out.deliver, &desktop);
        for ((_, name, _), hold) in to.iter().zip(&talk_held) {
            let Some(why) = hold else { continue };
            routes.retain(|(routed, _)| routed != name);
            routes.push((
                name.clone(),
                match why {
                    HoldReason::AskFirst => "when the user delivers it (this pool asks first)",
                    HoldReason::Watched => super::talk::WATCHED_GETS_IT,
                },
            ));
        }
        routes.extend(left_out.iter().map(|why| (String::new(), *why)));
        routes.retain(|(name, why)| !name.is_empty() || !why.is_empty());
        routes
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
/// in words. A recipient whose session runs in AI Juggler (driven, or in
/// its terminal by a tool that takes a typed prompt, D-29) gets it at its
/// next turn boundary when the user said so (their compose with Deliver)
/// or the pool allows it (Full, or a rule, within the hop limit); at the
/// hop limit the thread pauses and the user is asked. A connected desktop
/// member (AJ-E3013) reads it through AI Juggler's connector: woken for an
/// agent's message when the pool's waking rule allows (D-119), else at
/// its next turn. Otherwise it waits in the inbox (the agent reads it when
/// it chooses).
pub(crate) fn route(
    state: &Arc<AppState>,
    message: &Message,
    from: &Author,
    deliver: bool,
) -> Vec<(String, &'static str)> {
    let desktop =
        crate::features::connector::reach::of_pool(state, &message.pool_id).unwrap_or_default();
    route_with(state, message, from, deliver, &desktop)
}

/// Where one recipient's copy goes.
enum Target {
    /// Into its inbox, or (a session AI Juggler runs) its next turn.
    Inbox(&'static str, Option<SessionId>),
    /// A connected desktop member: whether its App can be woken now.
    Desktop(bool),
}

/// [`route`] with the pool's desktop conversations already read.
fn route_with(
    state: &Arc<AppState>,
    message: &Message,
    from: &Author,
    deliver: bool,
    desktop: &[DesktopConversation],
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
        let name = recipient.recipient_name.clone();
        if recipient.recipient_ref == SENDER_USER {
            targets.push((name, Target::Inbox("in your messages", None)));
            continue;
        }
        if let Some(one) = desktop
            .iter()
            .find(|one| one.connected && one.member == recipient.recipient_ref)
        {
            targets.push((name, Target::Desktop(one.wakes)));
            continue;
        }
        let session = super::live_session_of(&storage, &recipient.recipient_ref)
            .filter(|session| super::reachable(state, &storage, session));
        targets.push((name, Target::Inbox("in its inbox", session)));
    }
    drop(storage);
    let mut routes = Vec::new();
    for (name, target) in targets {
        let (inbox, session) = match target {
            Target::Desktop(wakes) => {
                routes.push((name, desktop_how(state, message, from, wakes)));
                continue;
            }
            Target::Inbox(inbox, session) => (inbox, session),
        };
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

/// What a connected desktop member's copy becomes, in words (D-119): an
/// agent's message wakes it when its App can be woken and
/// `allowed(deliver_to_session)` says so within the hop limit (the same
/// gate the connector's wait asks); else, and for a person's message, it
/// reads it at its next turn.
fn desktop_how(
    state: &Arc<AppState>,
    message: &Message,
    from: &Author,
    wakes: bool,
) -> &'static str {
    let woken = wakes
        && match from {
            Author::Agent {
                session: sender, ..
            } => {
                crate::features::pools::access::allowed(
                    state,
                    sender.as_str(),
                    &message.pool_id,
                    AgentAction::DeliverToSession,
                    Some(message.hop.saturating_sub(1)),
                ) == Decision::Allow
            }
            Author::User => false,
        };
    if woken {
        DESKTOP_WOKEN
    } else {
        DESKTOP_NEXT_TURN
    }
}

/// How a message reaches a connected desktop member that it wakes.
pub(crate) const DESKTOP_WOKEN: &str =
    "now: AI Juggler wakes it for this through its app's connector";
/// How a message reaches a connected desktop member at its next turn.
pub(crate) const DESKTOP_NEXT_TURN: &str =
    "at its next turn (it reads its messages through AI Juggler's connector)";

//! D-119 item 6 (2026-11-16): a pool-wide message is copied into the
//! inboxes of the members open when it's sent ([`super::send`]); an ended
//! member gets none. When one is resumed, its first inbox read includes
//! the pool-wide messages sent while it was gone, from the pool's own
//! log: at most [`MISSED_MAX`], newest first, and how many earlier ones
//! there were is said once, on that read. After it, everything is as
//! before.
//!
//! - **While it was gone** starts where it ended, as stored: for a session
//!   AI Juggler resumed (Resume on an ended session or an epoch), the
//!   status journal's last change of the session it resumes into an ended
//!   status (`instance_transitions`); for a desktop conversation, whose
//!   end nothing stores (its app records none; its attestation is kept in
//!   memory), its last inbox read in that pool (the pool's log), else when
//!   it joined the pool.
//! - **When:** at the Resume itself for a session AI Juggler runs (its
//!   copies are in its inbox at once, so its turn note counts them); for a
//!   desktop conversation, from a new process of its tool, at its hooks'
//!   first note or its first inbox read, whichever comes first.
//! - **The same rules as a send:** a copy the pool's talk rules refuse
//!   isn't put in; one they hold for the person waits with an ask, and a
//!   message still waiting on the person's OK as a whole waits with the
//!   others' copies. Never a second copy of a message it has, never its
//!   own message, never a message the person refused. Nothing is
//!   delivered or woken: the copies are unread in its inbox.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use juggler_core::model::{SessionId, Timestamp};
use juggler_core::pools::talk::HoldReason;
use juggler_core::storage::{HeldAsk, MemberRef, PoolRole, RecipientState, SENDER_USER, Storage};
use serde_json::{Value, json};

use super::talk::{self, Receiver};
use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::state::AppState;

/// The most missed pool-wide messages a resumed member is given.
pub const MISSED_MAX: u32 = 50;
/// The pool log's verb for a member's catch-up (detail: its counts).
const CAUGHT_UP: &str = "caught_up";
/// The pool log's verb for its counts said, on its first read.
const TOLD: &str = "caught_up_told";

/// The member a catch-up is for.
struct Member<'a> {
    /// Its session, as the pool's log names it (`session:<id>`).
    actor: String,
    /// The ref its copies go to, as a send addresses it.
    to: &'a str,
    /// Its name.
    name: String,
    /// Its role in the pool.
    role: PoolRole,
    /// Every ref its messages may be addressed to: what it already has,
    /// and what it sent.
    refs: &'a [String],
}

/// After `to` resumed `from`'s conversation (Resume): in each pool `to`
/// is in, the pool-wide messages sent since `from` ended go into its
/// inbox. How many went in.
///
/// # Errors
///
/// Storage failures.
pub fn after_resume(state: &Arc<AppState>, from: &SessionId, to: &SessionId) -> AppResult<usize> {
    let (since, placements, refs, member) = {
        let storage = lock_storage_arc(state)?;
        let Some(resolution) = crate::features::pools::members::resolution_of(&storage, to)? else {
            return Ok(0);
        };
        let works = storage.switch_graph().works_by_session()?;
        let mut refs = super::refs_of(&storage, to);
        for one in super::refs_of(&storage, from) {
            if !refs.contains(&one) {
                refs.push(one);
            }
        }
        (
            gone_since(&storage, from)?,
            resolution.placements,
            refs,
            MemberRef::of_session(to, works.get(to)).to_wire(),
        )
    };
    let actor = format!("session:{}", to.as_str());
    let object = format!("session:{}", from.as_str());
    let mut copied = 0;
    for place in placements {
        // Once a resume: a second call puts nothing in.
        let done = lock_storage_arc(state)?
            .messages()
            .last_event(&place.pool_id, &actor, CAUGHT_UP)?
            .is_some_and(|done| done.object_ref == object);
        if done {
            continue;
        }
        let name = super::roster(state, &place.pool_id)
            .ok()
            .and_then(|all| all.into_iter().find(|m| m.session_id == to.as_str()))
            .map_or_else(|| "This session".to_owned(), |m| m.name);
        let who = Member {
            actor: actor.clone(),
            to: &member,
            name,
            role: place.role,
            refs: &refs,
        };
        copied += catch_up(state, &place.pool_id, &who, since, &object)?.0;
    }
    Ok(copied)
}

/// [`after_resume`] for a resume that already started: a failure is
/// logged, never the resume's.
pub fn after_resume_logged(state: &Arc<AppState>, from: &SessionId, to: &SessionId) {
    if let Err(error) = after_resume(state, from, to) {
        tracing::warn!(target: "aij::flow", flow = "resume", step = "messages_not_caught_up", %error);
    }
}

/// When `from` ended: its status journal's last change into an ended
/// status; for a session with no such change, its last activity, else
/// its start.
fn gone_since(storage: &Storage, from: &SessionId) -> AppResult<i64> {
    let ended = storage
        .sessions()
        .transitions(from)?
        .into_iter()
        .rev()
        .find(|(_, to, _)| {
            juggler_core::model::SessionStatus::parse(to).is_some_and(|s| !super::is_live(s))
        })
        .map(|(_, _, at)| at);
    if let Some(at) = ended {
        return Ok(at);
    }
    let record = storage.sessions().get(from)?;
    Ok(record.last_activity.unwrap_or(record.started_at).millis())
}

/// A desktop conversation's inbox read or hook note: from a new process
/// of its tool, first the pool-wide messages it missed while it was gone
/// go into its inbox (once a process; never for a session AI Juggler
/// runs, whose catch-up is its Resume).
pub(crate) fn on_desktop_back(state: &Arc<AppState>, session: &SessionId, member_ref: &str) {
    if !member_ref.starts_with("conversation:") {
        return;
    }
    let Some(start) = state
        .agent_bridge
        .desktop
        .lock()
        .ok()
        .and_then(|kept| kept.get(session.as_str()).map(|(_, s)| s.tool_start))
    else {
        return;
    };
    let process = format!("process:{start}");
    let actor = format!("session:{}", session.as_str());
    let todo: Vec<(String, PoolRole, i64, Vec<String>)> = {
        let Ok(storage) = lock_storage_arc(state) else {
            return;
        };
        let Ok(Some(resolution)) =
            crate::features::pools::members::resolution_of(&storage, session)
        else {
            return;
        };
        let refs = super::refs_of(&storage, session);
        let repo = storage.messages();
        resolution
            .placements
            .into_iter()
            .filter(|place| {
                repo.last_event(&place.pool_id, &actor, CAUGHT_UP)
                    .ok()
                    .flatten()
                    .is_none_or(|done| done.object_ref != process)
            })
            .map(|place| {
                // Its last read there, else when it joined.
                let since = repo
                    .last_event(&place.pool_id, member_ref, "read")
                    .ok()
                    .flatten()
                    .map_or(place.joined_at, |read| read.at);
                (place.pool_id, place.role, since, refs.clone())
            })
            .collect()
    };
    for (pool, role, since, mut refs) in todo {
        if !refs.iter().any(|one| one == member_ref) {
            refs.push(member_ref.to_owned());
        }
        let name = crate::features::connector::reach::of_pool(state, &pool)
            .ok()
            .and_then(|all| all.into_iter().find(|one| one.member == member_ref))
            .map_or_else(|| "This session".to_owned(), |one| one.name);
        let who = Member {
            actor: actor.clone(),
            to: member_ref,
            name,
            role,
            refs: &refs,
        };
        if let Err(error) = catch_up(state, &pool, &who, since, &process) {
            tracing::warn!(target: "aij::flow", flow = "messages", step = "not_caught_up", %error);
        }
    }
}

/// Puts the pool-wide messages of `pool_id` that `who` missed since
/// `since` into its inbox, as the talk rules and the person's OKs let
/// them; logs the catch-up (`object`: what it came back from) with its
/// counts. `(copied, earlier)`.
fn catch_up(
    state: &Arc<AppState>,
    pool_id: &str,
    who: &Member<'_>,
    since: i64,
    object: &str,
) -> AppResult<(usize, usize)> {
    let roles = roles_of(state, pool_id);
    let storage = lock_storage_arc(state)?;
    let repo = storage.messages();
    let (missed, total) = repo.missed_broadcasts(pool_id, since, who.refs, MISSED_MAX)?;
    let earlier = total.saturating_sub(missed.len());
    let sealed = storage.pools().get(pool_id).is_ok_and(|pool| pool.sealed);
    let asked: std::collections::HashSet<String> = repo
        .open_asks(0, 200)?
        .into_iter()
        .map(|ask| ask.object_ref)
        .collect();
    let mut rows = Vec::new();
    let mut asks = Vec::new();
    for one in missed {
        let message = one.message;
        let hold = if message.sender_kind == SENDER_USER {
            None
        } else {
            let receiver = Receiver {
                anchor: who.to,
                role: who.role,
                linked: false,
                sealed: false,
                watched: false,
            };
            let role = roles
                .get(&message.sender_ref)
                .copied()
                .unwrap_or(PoolRole::Member);
            match talk::rule(
                &storage,
                pool_id,
                (&message.sender_ref, role),
                sealed,
                &[receiver],
            ) {
                Ok(ruled) => ruled.kept.first().and_then(|(_, hold)| *hold),
                // Its talk rules refuse it: no copy.
                Err(_) => continue,
            }
        };
        let state_now = if one.awaits_ok || hold.is_some() {
            RecipientState::Held
        } else {
            RecipientState::Unread
        };
        if hold == Some(HoldReason::AskFirst) && !one.awaits_ok && !asked.contains(&message.id) {
            asks.push(HeldAsk {
                id: super::new_id("ask"),
                pool_id: pool_id.to_owned(),
                actor_ref: message.sender_ref.clone(),
                actor_name: message.sender_name.clone(),
                action: talk::TALK_ASK.to_owned(),
                object_ref: message.id.clone(),
                summary: format!("“{}” wants to message {}", message.sender_name, who.name),
                created_at: Timestamp::now().millis(),
            });
        }
        rows.push((message.id, state_now));
    }
    let copied = repo.add_copies((who.to, &who.name), &rows)?;
    for ask in &asks {
        repo.hold(ask)?;
    }
    repo.log(
        pool_id,
        &who.actor,
        CAUGHT_UP,
        object,
        Some(&json!({"copied": copied, "earlier": earlier}).to_string()),
    )?;
    drop(storage);
    if copied > 0 {
        state.messages.notify(pool_id);
    }
    Ok((copied, earlier))
}

/// Each member's role in `pool_id` by its ref (the senders' roles, for
/// the talk rules): its sessions' and its desktop conversations'.
fn roles_of(state: &Arc<AppState>, pool_id: &str) -> HashMap<String, PoolRole> {
    let parse = |word: &str| PoolRole::parse(word).unwrap_or(PoolRole::Member);
    let mut roles: HashMap<String, PoolRole> = super::roster(state, pool_id)
        .unwrap_or_default()
        .into_iter()
        .map(|member| (member.anchor, parse(&member.role)))
        .collect();
    for one in crate::features::connector::reach::of_pool(state, pool_id).unwrap_or_default() {
        roles.entry(one.member).or_insert_with(|| parse(&one.role));
    }
    roles
}

/// What the first inbox read after a catch-up says, per pool: how many
/// pool-wide messages sent while the session was away are in its inbox
/// now, and how many earlier ones aren't. Said once: it's logged as told.
pub(crate) fn first_read_note(storage: &Storage, session: &SessionId) -> Vec<Value> {
    let Ok(Some(resolution)) = crate::features::pools::members::resolution_of(storage, session)
    else {
        return Vec::new();
    };
    let actor = format!("session:{}", session.as_str());
    let repo = storage.messages();
    let mut out = Vec::new();
    for place in resolution.placements {
        let Ok(Some(caught)) = repo.last_event(&place.pool_id, &actor, CAUGHT_UP) else {
            continue;
        };
        let told = repo
            .last_event(&place.pool_id, &actor, TOLD)
            .ok()
            .flatten()
            .is_some_and(|told| told.id > caught.id);
        let counts: Value = caught
            .detail
            .as_deref()
            .and_then(|detail| serde_json::from_str(detail).ok())
            .unwrap_or_default();
        let copied = counts["copied"].as_u64().unwrap_or(0);
        let earlier = counts["earlier"].as_u64().unwrap_or(0);
        if told || (copied == 0 && earlier == 0) {
            continue;
        }
        let pool = super::pool_name(storage, &place.pool_id).unwrap_or_default();
        let mut says = format!(
            "{copied} pool-wide messages from “{pool}”, sent while this session was away, are in its inbox now (newest first)."
        );
        if earlier > 0 {
            let _ = write!(
                says,
                " {earlier} earlier ones aren't: only the newest {MISSED_MAX} are given."
            );
        }
        out.push(json!({"pool": pool, "copied": copied, "earlier": earlier, "says": says}));
        let _ = repo.log(&place.pool_id, &actor, TOLD, &caught.id.to_string(), None);
    }
    out
}

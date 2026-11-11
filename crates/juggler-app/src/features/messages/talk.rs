//! D-62 decision 13 (AJ-E3005 req. 1): who may tell whom, applied to an
//! agent's message as it's sent.
//!
//! - Each recipient's rule is its connection's own, else its roles' pair,
//!   else allowed (`juggler_core::pools::talk`).
//! - Blocked: that recipient is left out, and the sender is told why; a
//!   message to it alone is refused.
//! - Ask first: that recipient's copy waits in Needs you (a held ask of
//!   its own kind, [`TALK_ASK`]) until the person delivers it.
//! - Across a pool link, nothing leaves or enters a sealed pool.
//!
//! The person's own messages aren't governed by talk rules: the pools
//! are theirs.

use juggler_agentbridge::wire::{Refusal, Response};
use juggler_core::pools::talk::{self, HoldReason, PoolTalk, Reach, RefuseReason, Route, Talk};
use juggler_core::storage::{MemberRef, PoolRole, Storage};

/// The held ask's action word for a message a pool's talk rule holds
/// (beside the D-28 actions; it never offers "always": the rule is on the
/// pool's canvas).
pub const TALK_ASK: &str = "talk";

/// The held ask's action word for a message to a Watched member (D-62
/// decision 15): it waits until the session is continued in AI Juggler.
pub const WATCHED_ASK: &str = "watched";

/// Where a message to a Watched member goes, in the words its sender is
/// told: at the send, and again when the person's Allow once on another
/// hold leaves it waiting for that reason.
pub const WATCHED_GETS_IT: &str =
    "when it's continued in AI Juggler (it runs in another app and can't read messages)";

/// One recipient as the talk rules see it: its stored ref, its role in
/// the pool it's reached in, and whether that's across a link.
pub struct Receiver<'a> {
    /// Its member ref (its Work item's, else its session's).
    pub anchor: &'a str,
    /// Its role there.
    pub role: PoolRole,
    /// Reached across a pool link the user made.
    pub linked: bool,
    /// The pool it's reached in is sealed.
    pub sealed: bool,
    /// A session another app runs (a Watched member).
    pub watched: bool,
}

/// What the rules leave of a send: for each recipient, whether its copy
/// waits on the person; the refused ones are gone.
pub struct Ruled {
    /// Each kept recipient's index in the input, and why it's held, when
    /// it is.
    pub kept: Vec<(usize, Option<HoldReason>)>,
    /// The words for each recipient left out.
    pub refused: Vec<&'static str>,
}

/// A pool's talk settings, as stored.
fn pool_talk(storage: &Storage, pool_id: &str) -> PoolTalk {
    let rows = storage.pools().talk().unwrap_or_default();
    PoolTalk::new(
        rows.iter()
            .filter(|(pool, ..)| pool == pool_id)
            .map(|(_, key, rule)| (key, *rule)),
    )
}

/// Applies `pool_id`'s talk rules to a message from `from` (its anchor
/// and role) to `to`.
///
/// # Errors
///
/// A refusal in words when no recipient is left.
pub fn rule(
    storage: &Storage,
    pool_id: &str,
    from: (&str, PoolRole),
    sealed: bool,
    to: &[Receiver<'_>],
) -> Result<Ruled, Response> {
    let rules = pool_talk(storage, pool_id);
    let sender = MemberRef::parse(from.0);
    let mut kept = Vec::new();
    let mut refused = Vec::new();
    for (at, receiver) in to.iter().enumerate() {
        let (Some(sender), Some(anchor)) = (sender.as_ref(), MemberRef::parse(receiver.anchor))
        else {
            // The user, or a ref the rules don't name: as before.
            kept.push((at, None));
            continue;
        };
        let decided = talk::decide(
            &rules,
            (sender, from.1),
            (&anchor, receiver.role),
            if receiver.watched {
                Reach::Watched
            } else {
                Reach::Tools
            },
            false,
            Route {
                linked: receiver.linked,
                sealed: sealed || receiver.sealed,
            },
        );
        match decided {
            Talk::Deliver { .. } => kept.push((at, None)),
            Talk::Hold(why) => kept.push((at, Some(why))),
            Talk::Refuse(why) => refused.push(why.words()),
        }
    }
    if kept.is_empty() {
        let why = refused
            .first()
            .copied()
            .unwrap_or(RefuseReason::Blocked.words());
        return Err(Response::refused(Refusal::Deny, why));
    }
    Ok(Ruled { kept, refused })
}

//! D-62 decisions 13–14 (AJ-E3005): who may tell whom, inside one pool.
//!
//! - Set per role pair (Lead → Member, Member → Reviewer, …), any one
//!   connection between two members overriding its pair. Nothing set:
//!   allowed (0.3's behaviour).
//! - **Allowed** delivers to the inbox, waking the receiver only where it
//!   works at Full (D-29); **ask first** holds the message in Needs you;
//!   **blocked** refuses it and tells the sender why.
//! - A receiver that can only be watched (a session another app runs)
//!   can't read messages until it's continued in AI Juggler: held, with
//!   that reason.
//! - A sealed pool lets nothing out: a message across a pool link from or
//!   to it is refused.
//! - Claude's own cross-session channel can only be set per receiving
//!   session AI Juggler launches ([`claude_inbound`]): refuse in a sealed
//!   pool, hold where any pair into it holds or blocks, else accept.

use std::collections::HashMap;

use crate::storage::{MemberRef, PoolRole, TalkKey, TalkRule};

/// One pool's talk settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PoolTalk {
    pairs: HashMap<(PoolRole, PoolRole), TalkRule>,
    connections: HashMap<(MemberRef, MemberRef), TalkRule>,
}

impl PoolTalk {
    /// From one pool's stored settings.
    #[must_use]
    pub fn new<'a>(rows: impl IntoIterator<Item = (&'a TalkKey, TalkRule)>) -> Self {
        let mut talk = Self::default();
        for (key, rule) in rows {
            match key {
                TalkKey::Roles(from, to) => {
                    talk.pairs.insert((*from, *to), rule);
                }
                TalkKey::Connection(from, to) => {
                    talk.connections.insert((from.clone(), to.clone()), rule);
                }
            }
        }
        talk
    }

    /// The rule for `from` (with its role) telling `to`: the connection's
    /// own, else the role pair's, else allowed.
    #[must_use]
    pub fn rule(&self, from: (&MemberRef, PoolRole), to: (&MemberRef, PoolRole)) -> TalkRule {
        self.connections
            .get(&(from.0.clone(), to.0.clone()))
            .or_else(|| self.pairs.get(&(from.1, to.1)))
            .copied()
            .unwrap_or(TalkRule::Allowed)
    }

    /// The role pair's rule alone.
    #[must_use]
    pub fn pair(&self, from: PoolRole, to: PoolRole) -> TalkRule {
        self.pairs
            .get(&(from, to))
            .copied()
            .unwrap_or(TalkRule::Allowed)
    }

    /// Whether any pair or connection into `to` (by role or member) holds
    /// or blocks: Claude's own channel into it is then held.
    #[must_use]
    pub fn holds_into(&self, to: (&MemberRef, PoolRole)) -> bool {
        self.pairs
            .iter()
            .any(|((_, into), rule)| *into == to.1 && *rule != TalkRule::Allowed)
            || self
                .connections
                .iter()
                .any(|((_, into), rule)| into == to.0 && *rule != TalkRule::Allowed)
    }
}

/// What the receiver can do with a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    /// A session AI Juggler started with its agent tools (MCP).
    Tools,
    /// A session AI Juggler started that reaches the tools through its
    /// shell only (OpenCode, Grok Build, Kimi Code until their lab).
    Shell,
    /// A session another app runs: watched, no tools.
    Watched,
}

/// Where the message would go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Route {
    /// Across a pool link the user made (else inside one pool).
    pub linked: bool,
    /// The sending pool, or the receiving one across a link, is sealed.
    pub sealed: bool,
}

/// What happens to one message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Talk {
    /// Delivered to the inbox; `wake` when the receiver works at Full.
    Deliver {
        /// It's woken (D-29) rather than left to read it next.
        wake: bool,
    },
    /// Held in Needs you until the person delivers it.
    Hold(HoldReason),
    /// Refused; the sender is told why.
    Refuse(RefuseReason),
}

/// Why a message waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldReason {
    /// The pool says ask first for this pair or connection.
    AskFirst,
    /// The receiver can't read messages until continued in AI Juggler.
    Watched,
}

/// Why a message is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefuseReason {
    /// The pool blocks this pair or connection.
    Blocked,
    /// Nothing leaves a sealed pool, and nothing comes in across a link.
    Sealed,
}

impl RefuseReason {
    /// What the sending agent is told.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::Blocked => "This pool doesn't let you message that member.",
            Self::Sealed => "This pool is sealed: nothing leaves it and nothing comes in.",
        }
    }
}

impl HoldReason {
    /// What the sending agent is told.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::AskFirst => {
                "This pool asks the person first; the message waits in their Needs you."
            }
            Self::Watched => {
                "It can't read messages until it's continued in AI Juggler; the message waits."
            }
        }
    }
}

/// Decides one message: `from` and `to` with their roles in the pool,
/// the receiver's reach, whether it works at Full there, and the route.
#[must_use]
pub fn decide(
    talk: &PoolTalk,
    from: (&MemberRef, PoolRole),
    to: (&MemberRef, PoolRole),
    reach: Reach,
    full: bool,
    route: Route,
) -> Talk {
    if route.linked && route.sealed {
        return Talk::Refuse(RefuseReason::Sealed);
    }
    match talk.rule(from, to) {
        TalkRule::Blocked => Talk::Refuse(RefuseReason::Blocked),
        TalkRule::AskFirst => Talk::Hold(HoldReason::AskFirst),
        TalkRule::Allowed if reach == Reach::Watched => Talk::Hold(HoldReason::Watched),
        TalkRule::Allowed => Talk::Deliver { wake: full },
    }
}

/// Claude's own cross-session inbound (`crossSessionInbound`) for one
/// receiving Claude session AI Juggler launches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Inbound {
    /// Untouched.
    Accept,
    /// Held for the person.
    Hold,
    /// Refused.
    Refuse,
}

impl Inbound {
    /// The settings word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Hold => "hold",
            Self::Refuse => "refuse",
        }
    }
}

/// One of the receiver's pools, as Claude's channel sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InboundPool {
    /// The pool is sealed.
    pub sealed: bool,
    /// Some pair or connection into the receiver holds or blocks.
    pub holds: bool,
}

/// The strictest of the receiver's pools: refuse in a sealed pool, hold
/// where any pair into it holds or blocks, else accept.
#[must_use]
pub fn claude_inbound(pools: &[InboundPool]) -> Inbound {
    pools
        .iter()
        .map(|pool| {
            if pool.sealed {
                Inbound::Refuse
            } else if pool.holds {
                Inbound::Hold
            } else {
                Inbound::Accept
            }
        })
        .max()
        .unwrap_or(Inbound::Accept)
}

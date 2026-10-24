//! D-28 (ruled 2026-10-24): what agents in a pool may do, as data.
//!
//! - A typed catalog of the actions an agent can ask AI Juggler for
//!   ([`AgentAction`]). It names agent work only: nothing in it changes a
//!   pool, a level or a rule, starts a session, picks an account or hands
//!   out a credential, so no level or rule can grant those (the hard lines;
//!   `tests/agent_access.rs` holds them).
//! - Each pool has an access level ([`AccessLevel`]), set by the user in
//!   the app only. A level is a set of allowed actions; Custom is an
//!   explicit set.
//! - Above the level is an ask, never a block: the user answers Allow once,
//!   Always allow in this pool, or Deny. From the second identical
//!   approval (same action, same pool) the ask offers "Always allow".
//! - An "always allow" rule records its action, pool, when it was made and
//!   that the user made it; it is revocable at any time.
//!
//! [`decide`] is the one place a decision is made; storage
//! (`storage::AgentAccessRepo::allowed`) feeds it and writes the audit.

use std::collections::BTreeSet;

/// Something an agent can ask AI Juggler for. Extensible: a new action
/// is a new variant, its word, and the levels that include it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AgentAction {
    /// See who's in the pool.
    ReadRoster,
    /// See the members' context cards.
    ReadCards,
    /// See the pool's tasks.
    ReadTasks,
    /// Say what it's doing (its card's status line).
    SetStatus,
    /// Post a message to a member or the pool.
    PostMessage,
    /// Share a document with the pool.
    ShareDoc,
    /// Create a task.
    CreateTask,
    /// Take an unassigned task.
    ClaimTask,
    /// Update a task.
    UpdateTask,
    /// Give a task to someone else.
    AssignTask,
    /// Deliver a message straight into another member's session (it
    /// starts that session's turn), within the loop caps.
    DeliverToSession,
    /// Stage a launch for the user to start with a click (an agent never
    /// starts one, and never picks the account).
    PrepareLaunch,
}

impl AgentAction {
    /// Every action, in the order the app lists them.
    pub const ALL: [Self; 12] = [
        Self::ReadRoster,
        Self::ReadCards,
        Self::ReadTasks,
        Self::SetStatus,
        Self::PostMessage,
        Self::ShareDoc,
        Self::CreateTask,
        Self::ClaimTask,
        Self::UpdateTask,
        Self::AssignTask,
        Self::DeliverToSession,
        Self::PrepareLaunch,
    ];

    /// The stored and wire word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadRoster => "read_roster",
            Self::ReadCards => "read_cards",
            Self::ReadTasks => "read_tasks",
            Self::SetStatus => "set_status",
            Self::PostMessage => "post_message",
            Self::ShareDoc => "share_doc",
            Self::CreateTask => "create_task",
            Self::ClaimTask => "claim_task",
            Self::UpdateTask => "update_task",
            Self::AssignTask => "assign_task",
            Self::DeliverToSession => "deliver_to_session",
            Self::PrepareLaunch => "prepare_launch",
        }
    }

    /// The action a word names, when it names one.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.as_str() == word)
    }

    /// Only looks: what an observer may do without asking.
    #[must_use]
    pub const fn reads_only(self) -> bool {
        matches!(self, Self::ReadRoster | Self::ReadCards | Self::ReadTasks)
    }
}

/// A pool's access level, set by the user in the app only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessLevel {
    /// See the roster, the cards and the tasks.
    Read,
    /// Read, plus set its own status, post messages, share documents and
    /// create, claim and update tasks. Delivering into an idle session
    /// needs the user's click. The default pool starts here.
    WorkTogether,
    /// Work together, plus deliver messages straight into other members'
    /// sessions, within the loop caps.
    Full,
    /// The user's own set of actions.
    Custom,
}

impl AccessLevel {
    /// Every level, in the order the app lists them.
    pub const ALL: [Self; 4] = [Self::Read, Self::WorkTogether, Self::Full, Self::Custom];

    /// Where a new pool starts (the default pool too).
    pub const STARTING: Self = Self::WorkTogether;

    /// The stored and wire word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::WorkTogether => "work_together",
            Self::Full => "full",
            Self::Custom => "custom",
        }
    }

    /// The level a word names, when it names one.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.as_str() == word)
    }

    /// What a named level allows; Custom allows `custom`.
    #[must_use]
    pub fn actions(self, custom: &BTreeSet<AgentAction>) -> BTreeSet<AgentAction> {
        use AgentAction as A;
        let read = [A::ReadRoster, A::ReadCards, A::ReadTasks];
        let work = [
            A::SetStatus,
            A::PostMessage,
            A::ShareDoc,
            A::CreateTask,
            A::ClaimTask,
            A::UpdateTask,
        ];
        match self {
            Self::Read => read.into_iter().collect(),
            Self::WorkTogether => read.into_iter().chain(work).collect(),
            Self::Full => read
                .into_iter()
                .chain(work)
                .chain([A::DeliverToSession])
                .collect(),
            Self::Custom => custom.clone(),
        }
    }
}

/// The answer to "may this agent do this here?".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Within the pool's level, or a rule the user made.
    Allow,
    /// Above the level: ask the user (Needs you, grouped, never a modal).
    /// `offer_always` — the user has allowed this action in this pool
    /// once already, so the ask offers "Always allow".
    Ask {
        /// Offer "Always allow <action> in <pool>".
        offer_always: bool,
    },
    /// Not at all: the agent isn't in the pool, or the pool is gone.
    Deny,
}

impl Decision {
    /// The audit's word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Ask { .. } => "ask",
            Self::Deny => "deny",
        }
    }
}

/// What the decision is made from: the pool's level (and its Custom
/// set), the user's rules for the pool, how often the user allowed the
/// action once, and whether the agent only observes.
#[derive(Clone, Debug, Default)]
pub struct PoolAccess {
    /// The pool's level; `None` when the pool isn't there.
    pub level: Option<AccessLevel>,
    /// The Custom set (used only at Custom).
    pub custom: BTreeSet<AgentAction>,
    /// Actions the user always allows in this pool.
    pub rules: BTreeSet<AgentAction>,
}

/// The one decision: `action` by a member of a pool with `access`.
/// `member` — the agent is in the pool; `observer` — its role there only
/// observes, so only reading needs no ask; `approvals` — how often the
/// user has allowed this action in this pool once.
#[must_use]
pub fn decide(
    access: &PoolAccess,
    member: bool,
    observer: bool,
    action: AgentAction,
    approvals: u32,
) -> Decision {
    let Some(level) = access.level else {
        return Decision::Deny;
    };
    if !member {
        return Decision::Deny;
    }
    let within = level.actions(&access.custom).contains(&action) || access.rules.contains(&action);
    if within && (!observer || action.reads_only()) {
        Decision::Allow
    } else {
        Decision::Ask {
            offer_always: approvals >= 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AccessLevel, AgentAction, Decision, PoolAccess, decide};
    use std::collections::BTreeSet;

    fn at(level: AccessLevel) -> PoolAccess {
        PoolAccess {
            level: Some(level),
            ..PoolAccess::default()
        }
    }

    #[test]
    fn levels_nest_and_custom_is_its_own_set() {
        let none = BTreeSet::new();
        let read = AccessLevel::Read.actions(&none);
        let work = AccessLevel::WorkTogether.actions(&none);
        let full = AccessLevel::Full.actions(&none);
        assert!(read.is_subset(&work) && work.is_subset(&full));
        assert!(!work.contains(&AgentAction::DeliverToSession));
        assert!(full.contains(&AgentAction::DeliverToSession));
        for above in [AgentAction::AssignTask, AgentAction::PrepareLaunch] {
            assert!(
                !full.contains(&above),
                "{above:?} always asks unless a rule allows it"
            );
        }
        let custom: BTreeSet<_> = [AgentAction::ReadRoster, AgentAction::AssignTask].into();
        assert_eq!(AccessLevel::Custom.actions(&custom), custom);
    }

    #[test]
    fn above_the_level_asks_and_offers_always_from_the_second_approval() {
        let access = at(AccessLevel::Read);
        assert_eq!(
            decide(&access, true, false, AgentAction::ReadCards, 0),
            Decision::Allow
        );
        assert_eq!(
            decide(&access, true, false, AgentAction::PostMessage, 0),
            Decision::Ask {
                offer_always: false
            }
        );
        assert_eq!(
            decide(&access, true, false, AgentAction::PostMessage, 1),
            Decision::Ask { offer_always: true }
        );
    }

    #[test]
    fn a_rule_allows_and_an_observer_asks_beyond_reading() {
        let mut access = at(AccessLevel::WorkTogether);
        access.rules.insert(AgentAction::AssignTask);
        assert_eq!(
            decide(&access, true, false, AgentAction::AssignTask, 0),
            Decision::Allow
        );
        assert_eq!(
            decide(&access, true, true, AgentAction::ReadTasks, 0),
            Decision::Allow
        );
        assert!(matches!(
            decide(&access, true, true, AgentAction::PostMessage, 0),
            Decision::Ask { .. }
        ));
    }

    #[test]
    fn no_pool_or_no_membership_is_a_deny() {
        assert_eq!(
            decide(
                &PoolAccess::default(),
                true,
                false,
                AgentAction::ReadRoster,
                0
            ),
            Decision::Deny
        );
        assert_eq!(
            decide(
                &at(AccessLevel::Full),
                false,
                false,
                AgentAction::ReadRoster,
                0
            ),
            Decision::Deny
        );
    }

    #[test]
    fn words_round_trip() {
        for action in AgentAction::ALL {
            assert_eq!(AgentAction::parse(action.as_str()), Some(action));
        }
        for level in AccessLevel::ALL {
            assert_eq!(AccessLevel::parse(level.as_str()), Some(level));
        }
        assert_eq!(AgentAction::parse("change_pool"), None);
    }
}

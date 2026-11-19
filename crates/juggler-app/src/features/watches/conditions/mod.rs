//! The closed set of watch conditions (ADR-0036 §2.2), as a registry of
//! rules. Each condition is one module here registering one
//! [`ConditionRule`]: its code, the targets it is offered on, the
//! observations it can be met from ([`Observes`], for D-127), its typed
//! parameter with compiled bounds, its words, and how one observation
//! moves it. The engine asks the rule and never branches on a condition,
//! so a new condition is a new module and one line in [`SHIPPED`]; the
//! engine doesn't change (the registry extension test proves it).

use std::sync::{Arc, OnceLock};

use juggler_adapter::Observes;

use super::model::{Param, TargetKind};
use super::words::CondWords;
use crate::features::observe::Observation;

mod asks_you;
mod commits;
mod ends;
mod fails;
mod finishes;
mod hits_limit;
mod idle_for;
mod leaves_processes;
mod pool_message;
mod restarted;
mod runway_below;
mod runway_resets;
mod starts;
mod stuck;
mod task_changes;

/// A parameter's type and compiled bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamKind {
    /// Whole minutes in `min..=max`.
    Minutes {
        /// Least.
        min: u32,
        /// Most.
        max: u32,
    },
    /// A share in percent, in `min..=max`.
    Percent {
        /// Least.
        min: u8,
        /// Most.
        max: u8,
    },
    /// A task state.
    TaskState,
    /// An agent, by id.
    Agent,
}

/// A condition's parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParamSpec {
    /// Its name (`for_min`, `percent`, …).
    pub name: &'static str,
    /// Its type and bounds.
    pub kind: ParamKind,
    /// It must be given.
    pub required: bool,
}

/// What one observation does to a condition, for one member of a target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing.
    Ignore,
    /// It is met now.
    Met,
    /// It will be met at this time unless something disarms it first
    /// (stuck for 10 minutes, idle for 30).
    Arm {
        /// Epoch ms.
        at_ms: i64,
    },
    /// What was armed no longer holds.
    Disarm,
}

/// A rule's own memory for one member (the last share of room seen, so a
/// crossing is told once). The engine keeps it and never reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Memo {
    /// The rule's value.
    pub value: Option<i64>,
}

/// One condition of the closed set.
pub trait ConditionRule: Send + Sync {
    /// Its code (`finishes`), stored in a watch.
    fn code(&self) -> &'static str;
    /// The target kinds it is offered on.
    fn targets(&self) -> &'static [TargetKind];
    /// The observations it can be met from: offered for a target only
    /// when the target's Apps report one of them (D-127).
    fn needs(&self) -> Observes;
    /// Its parameter, when it takes one.
    fn param(&self) -> Option<ParamSpec> {
        None
    }
    /// Its words.
    fn words(&self) -> &'static CondWords;
    /// What `obs` does to it, for one member of a target.
    fn on(&self, param: Option<&Param>, memo: &mut Memo, obs: &Observation) -> Verdict;
    /// AJ-E3817: the need or state its event is also told as, by its
    /// notice kind (`asks`, `failed`, `stuck`, `limit`, `finished`,
    /// `leftovers`, `restarted`, `reset`), so a watch on something a need
    /// already tells is one notification with it, and takes its level.
    /// `None`: only the watch tells it.
    fn told_as(&self) -> Option<&'static str> {
        None
    }
    /// What the watch's start does, for a session target known at once
    /// (idle for N minutes counts from the start).
    fn on_start(&self, _param: Option<&Param>, _now_ms: i64) -> Verdict {
        Verdict::Ignore
    }
}

/// The minutes a parameter holds, if any.
pub(super) fn minutes(param: Option<&Param>) -> Option<u32> {
    match param {
        Some(Param::Minutes(n)) => Some(*n),
        _ => None,
    }
}

/// Met at once for 0 or no minutes; else armed for that long after `at`.
pub(super) fn after(param: Option<&Param>, at_ms: i64) -> Verdict {
    match minutes(param) {
        None | Some(0) => Verdict::Met,
        Some(n) => Verdict::Arm {
            at_ms: at_ms.saturating_add(i64::from(n) * 60_000),
        },
    }
}

/// The conditions AI Juggler ships, in the order the catalogue offers them.
pub const SHIPPED: &[&dyn ConditionRule] = &[
    &finishes::RULE,
    &asks_you::RULE,
    &fails::RULE,
    &stuck::RULE,
    &ends::RULE,
    &idle_for::RULE,
    &hits_limit::RULE,
    &runway_below::RULE,
    &runway_resets::RULE,
    &leaves_processes::RULE,
    &restarted::RULE,
    &starts::RULE,
    &pool_message::RULE,
    &task_changes::RULE,
    &commits::RULE,
];

/// The rules, by code.
#[derive(Clone)]
pub struct Registry {
    rules: Vec<&'static dyn ConditionRule>,
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.rules.iter().map(|r| r.code()))
            .finish()
    }
}

impl Registry {
    /// The shipped closed set, built once.
    #[must_use]
    pub fn shipped() -> Arc<Self> {
        static SHIPPED_SET: OnceLock<Arc<Registry>> = OnceLock::new();
        SHIPPED_SET
            .get_or_init(|| Arc::new(Self::of(SHIPPED.iter().copied())))
            .clone()
    }

    /// A registry of these rules (the first of a code wins).
    #[must_use]
    pub fn of(rules: impl IntoIterator<Item = &'static dyn ConditionRule>) -> Self {
        let mut out: Vec<&'static dyn ConditionRule> = Vec::new();
        for rule in rules {
            if !out.iter().any(|r| r.code() == rule.code()) {
                out.push(rule);
            }
        }
        Self { rules: out }
    }

    /// This set with one more rule.
    #[must_use]
    pub fn with(&self, rule: &'static dyn ConditionRule) -> Self {
        Self::of(self.rules.iter().copied().chain(std::iter::once(rule)))
    }

    /// The rule of a code.
    #[must_use]
    pub fn get(&self, code: &str) -> Option<&'static dyn ConditionRule> {
        self.rules.iter().copied().find(|rule| rule.code() == code)
    }

    /// Every rule, in order.
    pub fn iter(&self) -> impl Iterator<Item = &'static dyn ConditionRule> + '_ {
        self.rules.iter().copied()
    }
}

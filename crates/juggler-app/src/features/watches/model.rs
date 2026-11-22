//! The typed watch (ADR-0036 §2.1): targets, conditions from the closed
//! set with typed parameters, matching, a lifetime and the user's own
//! quiet-hours switch. No action field, and no way to hold one: a watch's
//! only output is a notification.

use serde::{Deserialize, Serialize};

use super::conditions::{ParamKind, Registry};
use crate::features::observe::TaskState;

/// The most active watches (a new one past it asks the user to end one).
pub const MAX_ACTIVE: usize = 50;
/// The most targets a watch has.
pub const MAX_TARGETS: usize = 10;
/// The most conditions a watch has (any of them fires).
pub const MAX_CONDITIONS: usize = 4;
/// The longest id a target or a parameter names.
const ID_MAX: usize = 128;

/// A watch's unique id, end to end (actions target by id).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct WatchId(pub String);

impl WatchId {
    /// A fresh id: `w-` and 20 hex digits from the time and a counter.
    #[must_use]
    pub fn fresh(now_ms: i64) -> Self {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let salt = std::process::id();
        Self(format!(
            "w-{:012x}{:04x}{:04x}",
            now_ms.max(0),
            salt & 0xffff,
            n & 0xffff
        ))
    }

    /// A well-formed id: `w-` and 1–40 hex digits.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.0.strip_prefix("w-").is_some_and(|hex| {
            (1..=40).contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit())
        })
    }
}

/// What a target is.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum TargetKind {
    /// One session (AI Juggler's, or another app's conversation).
    Session,
    /// Every session of an App or profile.
    App,
    /// A project.
    Project,
    /// A pool.
    Pool,
    /// An account.
    Account,
    /// Every session on this Mac.
    Any,
}

/// What a watch watches, by id.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case", tag = "kind", content = "id")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum WatchTarget {
    /// One session.
    Session(String),
    /// An App or profile.
    App(String),
    /// A project.
    Project(String),
    /// A pool.
    Pool(String),
    /// An account.
    Account(String),
    /// Every session on this Mac.
    Any,
}

impl WatchTarget {
    /// Its kind.
    #[must_use]
    pub const fn kind(&self) -> TargetKind {
        match self {
            Self::Session(_) => TargetKind::Session,
            Self::App(_) => TargetKind::App,
            Self::Project(_) => TargetKind::Project,
            Self::Pool(_) => TargetKind::Pool,
            Self::Account(_) => TargetKind::Account,
            Self::Any => TargetKind::Any,
        }
    }

    /// Its id (`None` for any session).
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Session(id)
            | Self::App(id)
            | Self::Project(id)
            | Self::Pool(id)
            | Self::Account(id) => Some(id),
            Self::Any => None,
        }
    }
}

/// How several targets fire.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum Matching {
    /// Each target fires on its own.
    #[default]
    Each,
    /// Once, when every target has met a condition since the watch began.
    All,
}

/// A condition's parameter, typed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum Param {
    /// Minutes.
    Minutes(u32),
    /// A share, in percent.
    Percent(u8),
    /// A task's state.
    TaskState(TaskState),
    /// An agent, by id.
    Agent(String),
}

/// One condition: a code from the closed set (the registry) and its
/// parameter.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct Condition {
    /// The condition's code (`finishes`, `stuck`, …).
    pub code: String,
    /// Its parameter, when it takes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub param: Option<Param>,
}

/// How long a watch lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum Lifetime {
    /// The first fire ends it.
    Once,
    /// Until every target session has ended (session targets only).
    UntilEnds,
    /// Until a time ("today" is local midnight).
    Until {
        /// Epoch ms.
        #[serde(rename = "atMs")]
        #[ts(type = "number")]
        at_ms: i64,
    },
    /// Until the user removes it.
    Always,
}

/// The lifetime as the user chose it, before it is fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum LifetimeChoice {
    /// The first fire ends it.
    Once,
    /// Until every target session has ended.
    UntilEnds,
    /// Until local midnight.
    Today,
    /// Until a time.
    Until {
        /// Epoch ms.
        #[serde(rename = "atMs")]
        #[ts(type = "number")]
        at_ms: i64,
    },
    /// Until the user removes it.
    Always,
    /// Not said: ADR-0036 §2.4's default.
    Unsaid,
}

impl LifetimeChoice {
    /// The lifetime it fixes: "today" ends at `next_midnight_ms`; unsaid
    /// is `Once` for finishing or ending on sessions, else "today".
    #[must_use]
    pub fn fix(
        self,
        targets: &[WatchTarget],
        conditions: &[Condition],
        next_midnight_ms: i64,
    ) -> Lifetime {
        match self {
            Self::Once => Lifetime::Once,
            Self::UntilEnds => Lifetime::UntilEnds,
            Self::Today => Lifetime::Until {
                at_ms: next_midnight_ms,
            },
            Self::Until { at_ms } => Lifetime::Until { at_ms },
            Self::Always => Lifetime::Always,
            Self::Unsaid => {
                let on_sessions =
                    !targets.is_empty() && targets.iter().all(|t| t.kind() == TargetKind::Session);
                let ending = !conditions.is_empty()
                    && conditions
                        .iter()
                        .all(|c| matches!(c.code.as_str(), "finishes" | "ends"));
                if on_sessions && ending {
                    Lifetime::Once
                } else {
                    Lifetime::Until {
                        at_ms: next_midnight_ms,
                    }
                }
            }
        }
    }
}

/// A watch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct Watch {
    /// Its id.
    pub id: WatchId,
    /// 1–10 targets.
    pub targets: Vec<WatchTarget>,
    /// Each, or all.
    pub matching: Matching,
    /// 1–4 conditions; any of them fires.
    pub conditions: Vec<Condition>,
    /// How long it lives.
    pub lifetime: Lifetime,
    /// Paused by the user: it hears nothing until resumed.
    #[serde(default)]
    pub paused: bool,
    /// "Even during quiet hours": the user's own switch, off by default
    /// (ADR-0036 §2.5); quiet hours hold its alerts otherwise.
    #[serde(default)]
    pub even_in_quiet_hours: bool,
    /// When it was made, epoch ms.
    #[ts(type = "number")]
    pub created_ms: i64,
}

/// Why a watch was refused. Codes only, never words of the user's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchError {
    /// The id isn't `w-` and hex.
    BadId,
    /// No targets, or more than [`MAX_TARGETS`].
    TargetCount,
    /// The same target twice, "any session" beside another, or an id
    /// that is empty or too long.
    BadTarget,
    /// No conditions, or more than [`MAX_CONDITIONS`].
    ConditionCount,
    /// Not in the closed set.
    UnknownCondition(String),
    /// The condition can't be watched on this kind of target.
    ConditionNotForTarget(String, TargetKind),
    /// The parameter is missing, of the wrong type or out of bounds.
    BadParam(String),
    /// "Until it ends" with a target that isn't a session.
    UntilEndsNeedsSessions,
    /// "Until" a time already past.
    UntilPast,
    /// [`MAX_ACTIVE`] watches are already active.
    TooMany,
}

impl WatchError {
    /// The code a refusal carries.
    #[must_use]
    pub fn code(&self) -> String {
        match self {
            Self::BadId => "bad_id".into(),
            Self::TargetCount => "target_count".into(),
            Self::BadTarget => "bad_target".into(),
            Self::ConditionCount => "condition_count".into(),
            Self::UnknownCondition(code) => format!("unknown_condition:{code}"),
            Self::ConditionNotForTarget(code, kind) => {
                format!("condition_not_for_target:{code}:{}", target_code(*kind))
            }
            Self::BadParam(code) => format!("bad_param:{code}"),
            Self::UntilEndsNeedsSessions => "until_ends_needs_sessions".into(),
            Self::UntilPast => "until_past".into(),
            Self::TooMany => "too_many".into(),
        }
    }
}

/// A target kind's code.
#[must_use]
pub const fn target_code(kind: TargetKind) -> &'static str {
    match kind {
        TargetKind::Session => "session",
        TargetKind::App => "app",
        TargetKind::Project => "project",
        TargetKind::Pool => "pool",
        TargetKind::Account => "account",
        TargetKind::Any => "any",
    }
}

fn id_ok(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= ID_MAX && !id.chars().any(char::is_control)
}

/// The parameter fits its spec.
fn param_ok(kind: ParamKind, required: bool, param: Option<&Param>) -> bool {
    match (kind, param) {
        (_, None) => !required,
        (ParamKind::Minutes { min, max }, Some(Param::Minutes(n))) => (min..=max).contains(n),
        (ParamKind::Percent { min, max }, Some(Param::Percent(n))) => (min..=max).contains(n),
        (ParamKind::TaskState, Some(Param::TaskState(_))) => true,
        (ParamKind::Agent, Some(Param::Agent(id))) => id_ok(id),
        _ => false,
    }
}

impl Watch {
    /// The watch is well-formed against the closed set: bounds, targets,
    /// each condition known, offered on each target's kind, its parameter
    /// typed and in bounds, and a lifetime that fits. Whether a target's
    /// App can report a condition (D-127) is the catalogue's, which knows
    /// the Apps.
    ///
    /// # Errors
    ///
    /// The first [`WatchError`] found.
    pub fn validate(&self, registry: &Registry, now_ms: i64) -> Result<(), WatchError> {
        if !self.id.is_valid() {
            return Err(WatchError::BadId);
        }
        if self.targets.is_empty() || self.targets.len() > MAX_TARGETS {
            return Err(WatchError::TargetCount);
        }
        let mut seen = std::collections::BTreeSet::new();
        for target in &self.targets {
            if !seen.insert(target) || target.id().is_some_and(|id| !id_ok(id)) {
                return Err(WatchError::BadTarget);
            }
        }
        if self.targets.len() > 1 && self.targets.contains(&WatchTarget::Any) {
            return Err(WatchError::BadTarget);
        }
        if self.conditions.is_empty() || self.conditions.len() > MAX_CONDITIONS {
            return Err(WatchError::ConditionCount);
        }
        for condition in &self.conditions {
            let rule = registry
                .get(&condition.code)
                .ok_or_else(|| WatchError::UnknownCondition(condition.code.clone()))?;
            for target in &self.targets {
                if !rule.targets().contains(&target.kind()) {
                    return Err(WatchError::ConditionNotForTarget(
                        condition.code.clone(),
                        target.kind(),
                    ));
                }
            }
            let fits = match rule.param() {
                Some(spec) => param_ok(spec.kind, spec.required, condition.param.as_ref()),
                None => condition.param.is_none(),
            };
            if !fits {
                return Err(WatchError::BadParam(condition.code.clone()));
            }
        }
        match self.lifetime {
            Lifetime::UntilEnds
                if !self.targets.iter().all(|t| t.kind() == TargetKind::Session) =>
            {
                Err(WatchError::UntilEndsNeedsSessions)
            }
            Lifetime::Until { at_ms } if at_ms <= now_ms => Err(WatchError::UntilPast),
            _ => Ok(()),
        }
    }
}

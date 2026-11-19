//! The watch engine (ADR-0036 §2.3): pure, no I/O, no clock of its own.
//! It hears observations in order and returns what fired and what ended;
//! the same observations always give the same fires (everything is kept
//! in ordered maps, and time is the observations' own). Deadlines (stuck
//! for 10 minutes, idle for 30, an "until" time) are armed when their
//! state is entered and disarmed when it's left; [`Engine::next_wake`]
//! says when the one timer should next wake, `None` when nothing is armed.
//!
//! It never branches on a condition: each rule answers for itself
//! ([`super::conditions`]). It never writes a watch either: it is handed
//! watches ([`Engine::add`]) and only says which ended.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::Serialize;

use super::conditions::{Memo, Registry, Verdict};
use super::model::{Lifetime, Matching, TargetKind, Watch, WatchId, WatchTarget};
use crate::features::observe::{Observation, Observed, Removed, Subject};

/// A target and condition fire at most once per entry into the state, and
/// again only after this long (the notification engine's rule): a
/// flickering agent is told once.
pub const REARM_MS: i64 = 5 * 60_000;

/// Why a watch ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum EndReason {
    /// A "once" watch told the user.
    FirstFire,
    /// Every session it watched "until it ends" has ended.
    TargetsEnded,
    /// Its time was up.
    TimeReached,
    /// What it watched was removed.
    TargetGone,
    /// AI Juggler can no longer watch for its condition (the closed set
    /// or an App's `observes` narrowed).
    NoLongerOffered,
}

impl EndReason {
    /// Its code, for the log.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::FirstFire => "first_fire",
            Self::TargetsEnded => "targets_ended",
            Self::TimeReached => "time_reached",
            Self::TargetGone => "target_gone",
            Self::NoLongerOffered => "no_longer_offered",
        }
    }
}

/// A watch fired: a typed alert for the notification path (AJ-E3817).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fire {
    /// The watch.
    pub watch: WatchId,
    /// The condition met (its code).
    pub condition: String,
    /// The target that met it (`None` for "all of").
    pub target: Option<WatchTarget>,
    /// The session it happened in, when one did.
    pub session: Option<String>,
    /// When.
    pub at_ms: i64,
    /// What happened (an observation's code, or `deadline`).
    pub cause: &'static str,
    /// The user's "Even during quiet hours", carried for the alert path.
    pub even_in_quiet_hours: bool,
    /// Heard from the journals on start.
    pub replayed: bool,
}

/// A watch ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Expiry {
    /// The watch.
    pub watch: WatchId,
    /// Why.
    pub reason: EndReason,
    /// When.
    pub at_ms: i64,
}

/// What one step did.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Outcome {
    /// What fired, in order.
    pub fires: Vec<Fire>,
    /// What ended, in order.
    pub ended: Vec<Expiry>,
}

impl Outcome {
    fn merge(&mut self, other: Self) {
        self.fires.extend(other.fires);
        self.ended.extend(other.ended);
    }

    /// Nothing happened.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fires.is_empty() && self.ended.is_empty()
    }
}

/// `(target index, member, condition index)`: one condition of one target
/// for one member (a session, or an account for the room).
type Key = (usize, String, usize);

#[derive(Clone, Debug, Default)]
struct Slot {
    memo: Memo,
    armed: Option<i64>,
    last_fire: Option<i64>,
}

#[derive(Clone, Debug)]
struct Live {
    watch: Watch,
    slots: BTreeMap<Key, Slot>,
    /// "All of": which targets have met a condition since it began.
    met: Vec<bool>,
    /// Session targets that ended (or are gone).
    ended: BTreeSet<String>,
    /// The last observation time it heard.
    cursor_ms: i64,
}

impl Live {
    fn new(watch: Watch) -> Self {
        let met = vec![false; watch.targets.len()];
        Self {
            watch,
            slots: BTreeMap::new(),
            met,
            ended: BTreeSet::new(),
            cursor_ms: 0,
        }
    }

    fn sessions_all_ended(&self) -> bool {
        self.watch
            .targets
            .iter()
            .all(|t| matches!(t, WatchTarget::Session(id) if self.ended.contains(id)))
    }
}

/// Where a target is indexed: `(kind, id)`, any session under `""`.
type IndexKey = (TargetKind, String);

fn index_key(target: &WatchTarget) -> IndexKey {
    (target.kind(), target.id().unwrap_or_default().to_owned())
}

/// The target covers what the observation is about.
fn covers(target: &WatchTarget, subject: &Subject) -> bool {
    match target {
        WatchTarget::Session(id) => subject.session.as_deref() == Some(id),
        WatchTarget::App(id) => subject.apps.iter().any(|a| a == id),
        WatchTarget::Project(id) => subject.project.as_deref() == Some(id),
        WatchTarget::Pool(id) => subject.pools.iter().any(|p| p == id),
        WatchTarget::Account(id) => subject.account.as_deref() == Some(id),
        WatchTarget::Any => true,
    }
}

/// The member an observation is about, within a target: its session, else
/// its account (the room), else its first pool.
fn member(subject: &Subject) -> String {
    subject
        .session
        .clone()
        .or_else(|| subject.account.clone())
        .or_else(|| subject.pools.first().cloned())
        .unwrap_or_default()
}

/// The watch engine.
#[derive(Clone, Debug)]
pub struct Engine {
    registry: Arc<Registry>,
    live: BTreeMap<WatchId, Live>,
    index: BTreeMap<IndexKey, BTreeSet<WatchId>>,
}

impl Engine {
    /// An engine over a closed set.
    #[must_use]
    pub fn new(registry: Arc<Registry>) -> Self {
        Self {
            registry,
            live: BTreeMap::new(),
            index: BTreeMap::new(),
        }
    }

    /// The watches it holds, by id.
    pub fn ids(&self) -> impl Iterator<Item = &WatchId> {
        self.live.keys()
    }

    /// How many it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.live.len()
    }

    /// It holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }

    /// The last observation time a watch heard (its cursor, for the
    /// replay on start).
    #[must_use]
    pub fn cursor(&self, id: &WatchId) -> Option<i64> {
        self.live.get(id).map(|live| live.cursor_ms)
    }

    /// Starts hearing for `watch` (already validated and stored). A session
    /// target's conditions that count from the start are armed now.
    pub fn add(&mut self, watch: Watch, now_ms: i64) {
        self.remove(&watch.id);
        for target in &watch.targets {
            self.index
                .entry(index_key(target))
                .or_default()
                .insert(watch.id.clone());
        }
        let mut live = Live::new(watch);
        live.cursor_ms = now_ms;
        if !live.watch.paused {
            self.start(&mut live, now_ms);
        }
        self.live.insert(live.watch.id.clone(), live);
    }

    fn start(&self, live: &mut Live, now_ms: i64) {
        for (t, target) in live.watch.targets.iter().enumerate() {
            let WatchTarget::Session(session) = target else {
                continue;
            };
            for (c, condition) in live.watch.conditions.iter().enumerate() {
                let Some(rule) = self.registry.get(&condition.code) else {
                    continue;
                };
                if let Verdict::Arm { at_ms } = rule.on_start(condition.param.as_ref(), now_ms) {
                    live.slots.entry((t, session.clone(), c)).or_default().armed = Some(at_ms);
                }
            }
        }
    }

    /// Stops hearing for a watch.
    pub fn remove(&mut self, id: &WatchId) {
        if let Some(live) = self.live.remove(id) {
            for target in &live.watch.targets {
                let key = index_key(target);
                if let Some(ids) = self.index.get_mut(&key) {
                    ids.remove(id);
                    if ids.is_empty() {
                        self.index.remove(&key);
                    }
                }
            }
        }
    }

    /// Pauses or resumes a watch: paused, it hears nothing and keeps
    /// nothing armed; resumed, it starts afresh.
    pub fn set_paused(&mut self, id: &WatchId, paused: bool, now_ms: i64) {
        let Some(mut live) = self.live.remove(id) else {
            return;
        };
        live.watch.paused = paused;
        live.slots.clear();
        live.met.iter_mut().for_each(|m| *m = false);
        if !paused {
            self.start(&mut live, now_ms);
        }
        self.live.insert(id.clone(), live);
    }

    /// When the timer should next wake: the earliest armed deadline or
    /// "until" time; `None` when nothing waits.
    #[must_use]
    pub fn next_wake(&self) -> Option<i64> {
        self.live
            .values()
            .flat_map(|live| {
                let until = match live.watch.lifetime {
                    Lifetime::Until { at_ms } => Some(at_ms),
                    _ => None,
                };
                let armed = live
                    .slots
                    .values()
                    .filter(|_| !live.watch.paused)
                    .filter_map(|slot| slot.armed);
                until.into_iter().chain(armed)
            })
            .min()
    }

    /// The time is now `now_ms`: what was due by then fires or ends, in
    /// time order.
    pub fn tick(&mut self, now_ms: i64) -> Outcome {
        let mut out = Outcome::default();
        loop {
            let due = self.earliest_due(now_ms);
            let Some((at_ms, id, key)) = due else {
                break;
            };
            match key {
                None => self.end(&id, EndReason::TimeReached, at_ms, &mut out),
                Some(key) => {
                    let Some(mut live) = self.live.remove(&id) else {
                        continue;
                    };
                    if let Some(slot) = live.slots.get_mut(&key) {
                        slot.armed = None;
                    }
                    let session = live
                        .watch
                        .targets
                        .get(key.0)
                        .and_then(|t| matches!(t, WatchTarget::Session(_)).then(|| key.1.clone()))
                        .or_else(|| (!key.1.is_empty()).then(|| key.1.clone()));
                    let ended =
                        Self::met(&mut live, &key, at_ms, "deadline", session, false, &mut out);
                    live.cursor_ms = live.cursor_ms.max(at_ms);
                    self.live.insert(id.clone(), live);
                    if ended {
                        self.end(&id, EndReason::FirstFire, at_ms, &mut out);
                    }
                }
            }
        }
        out
    }

    /// The earliest thing due by `now_ms`: `(at, watch, slot)`, the slot
    /// `None` for an "until" time. Ties go by watch id, then slot.
    fn earliest_due(&self, now_ms: i64) -> Option<(i64, WatchId, Option<Key>)> {
        let mut best: Option<(i64, WatchId, Option<Key>)> = None;
        for (id, live) in &self.live {
            if let Lifetime::Until { at_ms } = live.watch.lifetime
                && at_ms <= now_ms
            {
                let candidate = (at_ms, id.clone(), None);
                if best
                    .as_ref()
                    .is_none_or(|b| (candidate.0, &candidate.1) < (b.0, &b.1))
                {
                    best = Some(candidate);
                }
            }
            if live.watch.paused {
                continue;
            }
            for (key, slot) in &live.slots {
                if let Some(at_ms) = slot.armed.filter(|at| *at <= now_ms) {
                    let candidate = (at_ms, id.clone(), Some(key.clone()));
                    if best.as_ref().is_none_or(|b| {
                        (candidate.0, &candidate.1, &candidate.2) < (b.0, &b.1, &b.2)
                    }) {
                        best = Some(candidate);
                    }
                }
            }
        }
        best
    }

    /// A condition was met for `key`: the fire, unless told within
    /// [`REARM_MS`]; with "all of", once every target has met one. Returns
    /// whether a "once" watch must end.
    fn met(
        live: &mut Live,
        key: &Key,
        at_ms: i64,
        cause: &'static str,
        session: Option<String>,
        replayed: bool,
        out: &mut Outcome,
    ) -> bool {
        let slot = live.slots.entry(key.clone()).or_default();
        slot.armed = None;
        if slot.last_fire.is_some_and(|last| at_ms - last < REARM_MS) {
            return false;
        }
        slot.last_fire = Some(at_ms);
        let target = match live.watch.matching {
            Matching::Each => live.watch.targets.get(key.0).cloned(),
            Matching::All => {
                if let Some(met) = live.met.get_mut(key.0) {
                    *met = true;
                }
                if !live.met.iter().all(|m| *m) {
                    return false;
                }
                live.met.iter_mut().for_each(|m| *m = false);
                None
            }
        };
        out.fires.push(Fire {
            watch: live.watch.id.clone(),
            condition: live
                .watch
                .conditions
                .get(key.2)
                .map(|c| c.code.clone())
                .unwrap_or_default(),
            target,
            session,
            at_ms,
            cause,
            even_in_quiet_hours: live.watch.even_in_quiet_hours,
            replayed,
        });
        live.watch.lifetime == Lifetime::Once
    }

    fn end(&mut self, id: &WatchId, reason: EndReason, at_ms: i64, out: &mut Outcome) {
        if self.live.contains_key(id) {
            self.remove(id);
            out.ended.push(Expiry {
                watch: id.clone(),
                reason,
                at_ms,
            });
        }
    }

    /// Hears one observation: first what fell due before it, then the
    /// observation itself for every watch whose target covers it.
    pub fn observe(&mut self, obs: &Observation) -> Outcome {
        let mut out = self.tick(obs.at_ms);
        if let Observed::Removed { what, id } = &obs.event {
            out.merge(self.removed(*what, id, obs.at_ms));
            return out;
        }
        for id in self.candidates(&obs.subject) {
            let Some(mut live) = self.live.remove(&id) else {
                continue;
            };
            let once_done = if live.watch.paused {
                false
            } else {
                self.hear(&mut live, obs, &mut out)
            };
            live.cursor_ms = live.cursor_ms.max(obs.at_ms);
            let ends = Self::ends_with(&mut live, obs);
            self.live.insert(id.clone(), live);
            if once_done {
                self.end(&id, EndReason::FirstFire, obs.at_ms, &mut out);
            } else if ends {
                self.end(&id, EndReason::TargetsEnded, obs.at_ms, &mut out);
            }
        }
        out
    }

    /// Every watch a target of which may cover the subject, in id order.
    fn candidates(&self, subject: &Subject) -> BTreeSet<WatchId> {
        let mut keys: Vec<IndexKey> = vec![(TargetKind::Any, String::new())];
        keys.extend(
            subject
                .session
                .iter()
                .map(|s| (TargetKind::Session, s.clone())),
        );
        keys.extend(subject.apps.iter().map(|a| (TargetKind::App, a.clone())));
        keys.extend(
            subject
                .project
                .iter()
                .map(|p| (TargetKind::Project, p.clone())),
        );
        keys.extend(
            subject
                .account
                .iter()
                .map(|a| (TargetKind::Account, a.clone())),
        );
        keys.extend(subject.pools.iter().map(|p| (TargetKind::Pool, p.clone())));
        keys.iter()
            .filter_map(|key| self.index.get(key))
            .flatten()
            .cloned()
            .collect()
    }

    /// Runs every condition of every covering target. Returns whether a
    /// "once" watch fired (it ends; nothing more of it is heard).
    fn hear(&self, live: &mut Live, obs: &Observation, out: &mut Outcome) -> bool {
        let who = member(&obs.subject);
        let targets: Vec<usize> = live
            .watch
            .targets
            .iter()
            .enumerate()
            .filter(|(_, target)| covers(target, &obs.subject))
            .map(|(t, _)| t)
            .collect();
        let conditions = live.watch.conditions.clone();
        for t in targets {
            for (c, condition) in conditions.iter().enumerate() {
                let Some(rule) = self.registry.get(&condition.code) else {
                    continue;
                };
                let key = (t, who.clone(), c);
                let slot = live.slots.entry(key.clone()).or_default();
                let verdict = rule.on(condition.param.as_ref(), &mut slot.memo, obs);
                let met = match verdict {
                    Verdict::Ignore => false,
                    Verdict::Met => true,
                    Verdict::Arm { at_ms } if at_ms <= obs.at_ms => true,
                    Verdict::Arm { at_ms } => {
                        slot.armed = Some(at_ms);
                        false
                    }
                    Verdict::Disarm => {
                        slot.armed = None;
                        false
                    }
                };
                if met
                    && Self::met(
                        live,
                        &key,
                        obs.at_ms,
                        obs.event.code(),
                        obs.subject.session.clone(),
                        obs.replayed,
                        out,
                    )
                {
                    return true;
                }
            }
        }
        false
    }

    /// "Until it ends": the session targets that ended, and whether all
    /// have now.
    fn ends_with(live: &mut Live, obs: &Observation) -> bool {
        if !matches!(obs.event, Observed::Ended { .. }) {
            return false;
        }
        let Some(session) = &obs.subject.session else {
            return false;
        };
        if live
            .watch
            .targets
            .contains(&WatchTarget::Session(session.clone()))
        {
            live.ended.insert(session.clone());
        }
        live.watch.lifetime == Lifetime::UntilEnds && live.sessions_all_ended()
    }

    /// Something a watch may target is gone: a session's end for its
    /// session targets (the watch ends once all of its targets are gone
    /// sessions); any other target ends its watches. Never silent: each
    /// ending is in the outcome, with its reason.
    fn removed(&mut self, what: Removed, id: &str, at_ms: i64) -> Outcome {
        let mut out = Outcome::default();
        let kind = match what {
            Removed::Session => TargetKind::Session,
            Removed::App => TargetKind::App,
            Removed::Project => TargetKind::Project,
            Removed::Pool => TargetKind::Pool,
            Removed::Account => TargetKind::Account,
        };
        let ids: Vec<WatchId> = self
            .index
            .get(&(kind, id.to_owned()))
            .map(|ids| ids.iter().cloned().collect())
            .unwrap_or_default();
        for watch in ids {
            let gone = if kind == TargetKind::Session {
                self.live.get_mut(&watch).is_some_and(|live| {
                    live.ended.insert(id.to_owned());
                    live.sessions_all_ended()
                })
            } else {
                true
            };
            if gone {
                self.end(&watch, EndReason::TargetGone, at_ms, &mut out);
            }
        }
        out
    }
}

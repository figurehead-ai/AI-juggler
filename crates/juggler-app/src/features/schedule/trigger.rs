//! The trigger maths (ADR-0039 §2, §3, §10): when a schedule is next due,
//! and what a due time that passed means under its missed-run policy.
//! Pure: no I/O, no clock of its own; the zone and the time are the
//! caller's, so every rule is tested exhaustively.
//!
//! - **Occurrences** are the trigger's raw times: `At` once; `Every` in
//!   absolute time from its wall-clock anchor (never from launch);
//!   `Calendar` in the user's local wall time; `AfterEvent` a delay after
//!   the event that armed it; `WhenIdle` the earliest it may run (its
//!   deadline is that plus `max_wait`).
//! - **Shaping** turns an occurrence into its due time: a stable jitter
//!   (the same for the same schedule and occurrence, so a recompute moves
//!   nothing), then the window (a time outside it waits for its next
//!   opening).
//! - **DST** (§10): a wall time in the spring gap runs at the first valid
//!   time after it; one the autumn overlap gives twice runs once, at the
//!   first. `Every` is absolute; `Calendar` and windows follow wall time.
//! - **Missed runs** (§3): of the occurrences between the last handled one
//!   and now, `RunOnce` runs one, `Skip` runs only an on-time one, and
//!   `RunEach` each up to its cap.

use jiff::Timestamp;
use jiff::civil::{Date, DateTime, Time};
use jiff::tz::{AmbiguousOffset, TimeZone};
use serde::{Deserialize, Serialize};

/// A minute.
pub const MINUTE_MS: i64 = 60_000;
/// A day of absolute time.
pub const DAY_MS: i64 = 86_400_000;
/// A run this late or less is on time (the scheduler's own lateness);
/// later, its occurrence was missed (asleep, closed, a clock change).
pub const ON_TIME_MS: i64 = MINUTE_MS;
/// The largest jitter.
pub const JITTER_MAX_MS: i64 = 3_600_000;
/// The shortest `Every` (sub-minute work stays a tick, §1).
pub const EVERY_MIN_MS: i64 = MINUTE_MS;
/// The longest `Every`.
pub const EVERY_MAX_MS: i64 = 366 * DAY_MS;
/// The longest delay or wait (`AfterEvent`, `WhenIdle`).
pub const WAIT_MAX_MS: i64 = 30 * DAY_MS;
/// The largest `RunEach` cap.
pub const RUN_EACH_MAX: u32 = 100;
/// How many days a calendar search looks ahead (a monthly rule's worst
/// case is two months; a year's leap day, eight years).
const SEARCH_DAYS: u32 = 3_000;
/// How many occurrences a missed-run count walks before it stops counting
/// (the newest is found either way).
const COUNT_MAX: u32 = 10_000;

/// A local wall-clock time, `HH:MM`.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ts_rs::TS,
)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ClockTime {
    /// 0–23.
    pub hour: u8,
    /// 0–59.
    pub minute: u8,
}

impl ClockTime {
    /// `hour:minute`.
    #[must_use]
    pub const fn new(hour: u8, minute: u8) -> Self {
        Self { hour, minute }
    }

    const fn valid(self) -> bool {
        self.hour < 24 && self.minute < 60
    }

    const fn minutes(self) -> u32 {
        self.hour as u32 * 60 + self.minute as u32
    }

    fn time(self) -> Option<Time> {
        #[allow(clippy::cast_possible_wrap, reason = "validated: below 60")]
        Time::new(self.hour as i8, self.minute as i8, 0, 0).ok()
    }
}

/// A calendar rule: the small RRULE subset people say (§2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "repeat", rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum CalendarRule {
    /// Every day at a time.
    Daily {
        /// The time.
        at: ClockTime,
    },
    /// Monday to Friday at a time.
    Weekdays {
        /// The time.
        at: ClockTime,
    },
    /// On some weekdays (1 = Monday … 7 = Sunday) at a time.
    Weekly {
        /// The days, ISO numbers.
        days: Vec<u8>,
        /// The time.
        at: ClockTime,
    },
    /// On a day of the month at a time; a month without that day (the 31st
    /// in April) uses its last day.
    Monthly {
        /// 1–31.
        day: u8,
        /// The time.
        at: ClockTime,
    },
}

impl CalendarRule {
    /// The rule's time of day.
    #[must_use]
    pub const fn at(&self) -> ClockTime {
        match self {
            Self::Daily { at }
            | Self::Weekdays { at }
            | Self::Weekly { at, .. }
            | Self::Monthly { at, .. } => *at,
        }
    }

    fn matches(&self, date: Date) -> bool {
        let weekday = date.weekday().to_monday_one_offset();
        match self {
            Self::Daily { .. } => true,
            Self::Weekdays { .. } => weekday <= 5,
            Self::Weekly { days, .. } => days.iter().any(|d| i16::from(*d) == i16::from(weekday)),
            Self::Monthly { day, .. } => {
                i16::from(date.day()) == i16::from(*day).min(i16::from(date.days_in_month()))
            }
        }
    }

    fn validate(&self) -> Result<(), &'static str> {
        if !self.at().valid() {
            return Err("time");
        }
        match self {
            Self::Weekly { days, .. }
                if days.is_empty()
                    || days.len() > 7
                    || days.iter().any(|d| !(1..=7).contains(d)) =>
            {
                Err("days")
            }
            Self::Monthly { day, .. } if !(1..=31).contains(day) => Err("day"),
            _ => Ok(()),
        }
    }
}

/// When a schedule comes due (§2, a closed set).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(
    export,
    export_to = "../../../ui/src/types/generated/",
    rename = "ScheduleTrigger"
)]
pub enum Trigger {
    /// Once, at an instant.
    At {
        /// Epoch ms.
        #[ts(type = "number")]
        at_ms: i64,
    },
    /// Every interval, in absolute time from a wall-clock anchor.
    Every {
        /// The interval.
        #[ts(type = "number")]
        interval_ms: i64,
        /// An instant the occurrences line up with.
        #[ts(type = "number")]
        anchor_ms: i64,
    },
    /// A calendar rule in the user's local time.
    Calendar {
        /// The rule.
        rule: CalendarRule,
    },
    /// A delay after a typed event, cancelled if `unless` comes first.
    AfterEvent {
        /// The event's code.
        event: String,
        /// The delay.
        #[ts(type = "number")]
        delay_ms: i64,
        /// The event that cancels it.
        #[serde(default)]
        #[ts(optional = nullable)]
        unless: Option<String>,
    },
    /// At the next idle moment from `after_ms` past its arming, or at
    /// `max_wait_ms` past that, whichever is first.
    WhenIdle {
        /// The longest it waits for idle.
        #[ts(type = "number")]
        max_wait_ms: i64,
        /// How long after its arming (its creation, or its last run) it may
        /// run at all.
        #[serde(default)]
        #[ts(type = "number")]
        after_ms: i64,
    },
}

/// Only between two local times (§2's `window`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(
    export,
    export_to = "../../../ui/src/types/generated/",
    rename = "ScheduleWindow"
)]
pub enum Window {
    /// From `from` until `until` (across midnight when `from` is later).
    Between {
        /// Opens.
        from: ClockTime,
        /// Closes.
        until: ClockTime,
    },
    /// Outside the user's quiet hours (by reference: a change to them
    /// applies at once).
    OutsideQuietHours,
}

/// How long a schedule lives (ADR-0036's lifetimes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(
    export,
    export_to = "../../../ui/src/types/generated/",
    rename = "ScheduleLifetime"
)]
pub enum Lifetime {
    /// One run, then it ends.
    Once,
    /// No occurrence from this instant on.
    Until {
        /// Epoch ms.
        #[ts(type = "number")]
        at_ms: i64,
    },
    /// Until removed.
    Always,
}

/// The shared modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(
    export,
    export_to = "../../../ui/src/types/generated/",
    rename = "ScheduleModifiers"
)]
pub struct Modifiers {
    /// Only inside this window.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub window: Option<Window>,
    /// A random delay up to this, the same for each occurrence.
    #[serde(default)]
    #[ts(type = "number")]
    pub jitter_ms: i64,
    /// How long it lives.
    pub lifetime: Lifetime,
}

impl Modifiers {
    /// One run, no window, no jitter.
    #[must_use]
    pub const fn once() -> Self {
        Self {
            window: None,
            jitter_ms: 0,
            lifetime: Lifetime::Once,
        }
    }

    /// Until removed, no window, no jitter.
    #[must_use]
    pub const fn always() -> Self {
        Self {
            window: None,
            jitter_ms: 0,
            lifetime: Lifetime::Always,
        }
    }
}

/// What a due time that passed means (§3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum MissedPolicy {
    /// One run, however many were missed.
    #[default]
    RunOnce,
    /// Only an on-time run; missed ones move on to the next.
    Skip,
    /// Each missed occurrence, up to `cap`.
    RunEach {
        /// The most runs at once.
        cap: u32,
    },
}

impl MissedPolicy {
    /// The stored code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RunOnce => "run_once",
            Self::Skip => "skip",
            Self::RunEach { .. } => "run_each",
        }
    }
}

/// A schedule's timing: its trigger, modifiers and missed-run policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    /// When.
    pub trigger: Trigger,
    /// The modifiers.
    pub modifiers: Modifiers,
    /// What a passed due time means.
    pub missed: MissedPolicy,
}

impl Spec {
    /// Checks every bound.
    ///
    /// # Errors
    ///
    /// The code of the first field out of bounds.
    pub fn validate(&self) -> Result<(), &'static str> {
        match &self.trigger {
            Trigger::At { at_ms } if *at_ms <= 0 => return Err("at"),
            Trigger::Every { interval_ms, .. }
                if !(EVERY_MIN_MS..=EVERY_MAX_MS).contains(interval_ms) =>
            {
                return Err("interval");
            }
            Trigger::Calendar { rule } => rule.validate()?,
            Trigger::AfterEvent {
                event,
                delay_ms,
                unless,
            } => {
                let code = |c: &str| juggler_core::storage::memory::code_like(c, 64);
                if !code(event) || unless.as_deref().is_some_and(|u| !code(u)) {
                    return Err("event");
                }
                if !(0..=WAIT_MAX_MS).contains(delay_ms) {
                    return Err("delay");
                }
            }
            Trigger::WhenIdle {
                max_wait_ms,
                after_ms,
            } => {
                // A repeating idle job waits at least a minute between runs.
                let least = if matches!(self.modifiers.lifetime, Lifetime::Once) {
                    0
                } else {
                    MINUTE_MS
                };
                if !(MINUTE_MS..=WAIT_MAX_MS).contains(max_wait_ms)
                    || !(least..=WAIT_MAX_MS).contains(after_ms)
                {
                    return Err("wait");
                }
            }
            _ => {}
        }
        if !(0..=JITTER_MAX_MS).contains(&self.modifiers.jitter_ms) {
            return Err("jitter");
        }
        if let Some(Window::Between { from, until }) = self.modifiers.window
            && (!from.valid() || !until.valid())
        {
            return Err("window");
        }
        if let Lifetime::Until { at_ms } = self.modifiers.lifetime
            && at_ms <= 0
        {
            return Err("until");
        }
        if let MissedPolicy::RunEach { cap } = self.missed
            && !(1..=RUN_EACH_MAX).contains(&cap)
        {
            return Err("cap");
        }
        Ok(())
    }

    /// The trigger repeats (a run doesn't end it by itself).
    #[must_use]
    pub const fn repeats(&self) -> bool {
        !matches!(self.trigger, Trigger::At { .. })
            && !matches!(self.modifiers.lifetime, Lifetime::Once)
    }
}

/// What the maths needs from outside: the user's zone, and their quiet
/// hours (`OutsideQuietHours` reads them by reference).
#[derive(Clone, Debug)]
pub struct Context {
    /// The user's local time zone.
    pub zone: TimeZone,
    /// Quiet hours, `(from, until)`, when the user has them.
    pub quiet: Option<(ClockTime, ClockTime)>,
}

impl Context {
    /// In `zone`, with no quiet hours.
    #[must_use]
    pub const fn in_zone(zone: TimeZone) -> Self {
        Self { zone, quiet: None }
    }
}

/// Where a schedule is: the occurrence it last handled (or its creation),
/// and when an `AfterEvent` or `WhenIdle` was armed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    /// Occurrences after this are still to come.
    pub base_ms: i64,
    /// When it was armed, for the triggers that are.
    pub armed_ms: Option<i64>,
}

/// The next occurrence and when it is due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Next {
    /// The raw occurrence.
    pub occurrence_ms: i64,
    /// When it is due: jittered, then moved into the window.
    pub due_ms: i64,
}

/// What firing a due schedule now means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fired {
    /// How many runs to make now.
    pub runs: u32,
    /// How many occurrences went unrun (missed, under the policy).
    pub missed: u32,
    /// The newest occurrence was due within [`ON_TIME_MS`].
    pub on_time: bool,
    /// Where the schedule is now.
    pub place: Place,
    /// What comes next; `None` once the schedule is over.
    pub next: Option<Next>,
}

// ============================================================================ local time

/// `ms` as a wall-clock date and time in `zone`.
#[must_use]
pub fn local(ms: i64, zone: &TimeZone) -> Option<DateTime> {
    Some(
        Timestamp::from_millisecond(ms)
            .ok()?
            .to_zoned(zone.clone())
            .datetime(),
    )
}

/// The instant a wall time happens in `zone` (§10): in the spring gap,
/// the first valid time after it (the transition); in the autumn overlap,
/// the first of the two.
#[must_use]
pub fn instant_of(dt: DateTime, zone: &TimeZone) -> Option<i64> {
    let at = match zone.to_ambiguous_timestamp(dt).offset() {
        AmbiguousOffset::Unambiguous { offset } => offset.to_timestamp(dt).ok()?,
        AmbiguousOffset::Fold { before, .. } => before.to_timestamp(dt).ok()?,
        AmbiguousOffset::Gap { after, .. } => {
            // Read with the later offset, the wall time lands inside the
            // span before the gap; the transition after it is the gap's end.
            let early = after.to_timestamp(dt).ok()?;
            zone.following(early).next()?.timestamp()
        }
    };
    Some(at.as_millisecond())
}

/// The first occurrence of `rule` strictly after `after_ms`.
#[must_use]
pub fn calendar_after(rule: &CalendarRule, after_ms: i64, zone: &TimeZone) -> Option<i64> {
    let time = rule.at().time()?;
    // A day back: the overlap or a gap may put the wall time's instant on
    // the far side of the date boundary of `after_ms`.
    let mut day = local(after_ms, zone)?.date().yesterday().ok()?;
    for _ in 0..SEARCH_DAYS {
        if rule.matches(day)
            && let Some(at) = instant_of(day.to_datetime(time), zone)
            && at > after_ms
        {
            return Some(at);
        }
        day = day.tomorrow().ok()?;
    }
    None
}

fn minutes_of(dt: DateTime) -> u32 {
    u32::try_from(i32::from(dt.hour()) * 60 + i32::from(dt.minute())).unwrap_or(0)
}

/// The window's local times, with `OutsideQuietHours` read from `ctx`;
/// `None` when it is always open.
fn resolved(window: Option<Window>, ctx: &Context) -> Option<(ClockTime, ClockTime)> {
    let (from, until) = match window? {
        Window::Between { from, until } => (from, until),
        Window::OutsideQuietHours => {
            let (quiet_from, quiet_until) = ctx.quiet?;
            (quiet_until, quiet_from)
        }
    };
    (from != until).then_some((from, until))
}

/// `ms`'s local time is inside `from..until` (across midnight when `from`
/// is later).
#[must_use]
pub fn in_window(ms: i64, from: ClockTime, until: ClockTime, zone: &TimeZone) -> bool {
    let Some(dt) = local(ms, zone) else {
        return true;
    };
    let (m, f, u) = (minutes_of(dt), from.minutes(), until.minutes());
    match f.cmp(&u) {
        std::cmp::Ordering::Equal => true,
        std::cmp::Ordering::Less => f <= m && m < u,
        std::cmp::Ordering::Greater => m >= f || m < u,
    }
}

/// `ms` if the window is open then, else the window's next opening.
fn into_window(ms: i64, window: Option<(ClockTime, ClockTime)>, zone: &TimeZone) -> i64 {
    let Some((from, until)) = window else {
        return ms;
    };
    if in_window(ms, from, until, zone) {
        return ms;
    }
    calendar_after(&CalendarRule::Daily { at: from }, ms, zone).unwrap_or(ms)
}

// ============================================================================ jitter

/// A stable seed for a schedule id (FNV-1a).
#[must_use]
pub fn seed_of(id: &str) -> u64 {
    id.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The jitter for one occurrence: in `0..=jitter_ms`, the same every time
/// for the same seed and occurrence (splitmix64).
#[must_use]
pub fn jitter_of(seed: u64, occurrence_ms: i64, jitter_ms: i64) -> i64 {
    if jitter_ms <= 0 {
        return 0;
    }
    #[allow(clippy::cast_sign_loss, reason = "bits mixed, not a quantity")]
    let mut z = seed ^ (occurrence_ms as u64);
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    #[allow(clippy::cast_sign_loss, reason = "jitter_ms > 0 here")]
    let span = jitter_ms as u64 + 1;
    i64::try_from(z % span).unwrap_or(0)
}

// ============================================================================ occurrences

/// The trigger's first raw occurrence strictly after `place.base_ms`.
#[must_use]
pub fn occurrence_after(trigger: &Trigger, place: Place, zone: &TimeZone) -> Option<i64> {
    let base = place.base_ms;
    match trigger {
        Trigger::At { at_ms } => (*at_ms > base).then_some(*at_ms),
        Trigger::Every {
            interval_ms,
            anchor_ms,
        } => {
            if *interval_ms <= 0 {
                return None;
            }
            let k = base.checked_sub(*anchor_ms)?.div_euclid(*interval_ms) + 1;
            anchor_ms.checked_add(k.checked_mul(*interval_ms)?)
        }
        Trigger::Calendar { rule } => calendar_after(rule, base, zone),
        Trigger::AfterEvent { delay_ms, .. } => place
            .armed_ms
            .and_then(|armed| armed.checked_add(*delay_ms))
            .filter(|at| *at > base),
        Trigger::WhenIdle { after_ms, .. } => place
            .armed_ms
            .and_then(|armed| armed.checked_add(*after_ms))
            .filter(|at| *at > base),
    }
}

/// A `WhenIdle` occurrence's deadline: it runs then even if never idle.
#[must_use]
pub fn idle_deadline(trigger: &Trigger, occurrence_ms: i64) -> Option<i64> {
    match trigger {
        Trigger::WhenIdle { max_wait_ms, .. } => occurrence_ms.checked_add(*max_wait_ms),
        _ => None,
    }
}

fn shaped(spec: &Spec, occurrence_ms: i64, ctx: &Context, seed: u64) -> i64 {
    let jittered = occurrence_ms + jitter_of(seed, occurrence_ms, spec.modifiers.jitter_ms);
    into_window(jittered, resolved(spec.modifiers.window, ctx), &ctx.zone)
}

fn within_lifetime(spec: &Spec, occurrence_ms: i64) -> bool {
    match spec.modifiers.lifetime {
        Lifetime::Until { at_ms } => occurrence_ms < at_ms,
        Lifetime::Once | Lifetime::Always => true,
    }
}

/// The next occurrence after `place` and when it is due; `None` when
/// none is to come (an `At` that ran, a lifetime that ended, an event
/// not yet armed).
#[must_use]
pub fn next(spec: &Spec, place: Place, ctx: &Context, seed: u64) -> Option<Next> {
    let occurrence_ms = occurrence_after(&spec.trigger, place, &ctx.zone)?;
    within_lifetime(spec, occurrence_ms).then(|| Next {
        occurrence_ms,
        due_ms: shaped(spec, occurrence_ms, ctx, seed),
    })
}

/// The occurrences after `place` that are due by `now_ms`: how many
/// (up to [`COUNT_MAX`]) and the newest.
fn due_by(spec: &Spec, place: Place, now_ms: i64, ctx: &Context, seed: u64) -> (u32, Option<i64>) {
    let due = |occ: i64| within_lifetime(spec, occ) && shaped(spec, occ, ctx, seed) <= now_ms;
    if let Trigger::Every {
        interval_ms,
        anchor_ms,
    } = spec.trigger
        && interval_ms > 0
    {
        // Absolute time: counted, not walked.
        let index = |t: i64| t.saturating_sub(anchor_ms).div_euclid(interval_ms);
        let limit = match spec.modifiers.lifetime {
            Lifetime::Until { at_ms } => now_ms.min(at_ms.saturating_sub(1)),
            _ => now_ms,
        };
        let (first, mut last) = (index(place.base_ms) + 1, index(limit));
        let mut newest = anchor_ms.saturating_add(last.saturating_mul(interval_ms));
        // Shaping only delays: the newest may not be due yet, never more.
        if last >= first && !due(newest) {
            last -= 1;
            newest -= interval_ms;
        }
        if last < first {
            return (0, None);
        }
        let count = u32::try_from(last - first + 1).unwrap_or(u32::MAX);
        return (count, Some(newest));
    }
    let mut cursor = place;
    let (mut count, mut newest) = (0_u32, None);
    while let Some(occ) = occurrence_after(&spec.trigger, cursor, &ctx.zone) {
        if occ > now_ms || !due(occ) {
            break;
        }
        count = count.saturating_add(1);
        newest = Some(occ);
        cursor.base_ms = occ;
        if count >= COUNT_MAX {
            break;
        }
    }
    (count, newest)
}

/// A due schedule fired at `now_ms`: how many runs to make, how many
/// were missed, and what comes next. Nothing due: no runs, the place as
/// it was.
#[must_use]
pub fn fire(spec: &Spec, place: Place, now_ms: i64, ctx: &Context, seed: u64) -> Fired {
    let (count, newest) = due_by(spec, place, now_ms, ctx, seed);
    let Some(newest) = newest.filter(|_| count > 0) else {
        return Fired {
            runs: 0,
            missed: 0,
            on_time: true,
            place,
            next: next(spec, place, ctx, seed),
        };
    };
    let on_time = now_ms - shaped(spec, newest, ctx, seed) <= ON_TIME_MS;
    let runs = match spec.missed {
        MissedPolicy::RunOnce => 1,
        MissedPolicy::Skip => u32::from(on_time),
        MissedPolicy::RunEach { cap } => count.min(cap.max(1)),
    };
    // What arms itself again: an idle job from this run; an event's delay
    // waits for its next event.
    let armed_ms = match spec.trigger {
        Trigger::WhenIdle { .. } => Some(now_ms),
        Trigger::AfterEvent { .. } => None,
        _ => place.armed_ms,
    };
    let after = Place {
        base_ms: match spec.trigger {
            Trigger::WhenIdle { .. } => now_ms,
            _ => newest,
        },
        armed_ms,
    };
    let over = matches!(spec.modifiers.lifetime, Lifetime::Once);
    Fired {
        runs,
        missed: count - runs,
        on_time,
        place: after,
        next: if over {
            None
        } else {
            next(spec, after, ctx, seed)
        },
    }
}

/// The first place of a schedule made at `now_ms`: occurrences after now,
/// an idle job armed from now. An `At` already past is due at once.
#[must_use]
pub const fn first_place(trigger: &Trigger, now_ms: i64) -> Place {
    match trigger {
        Trigger::At { at_ms } if *at_ms <= now_ms => Place {
            base_ms: *at_ms - 1,
            armed_ms: None,
        },
        Trigger::WhenIdle { .. } => Place {
            base_ms: now_ms - 1,
            armed_ms: Some(now_ms),
        },
        _ => Place {
            base_ms: now_ms,
            armed_ms: None,
        },
    }
}

#[cfg(test)]
#[path = "trigger_tests.rs"]
mod tests;

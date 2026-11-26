//! Schedules as typed rows (ADR-0039 §3, §4, §7): made, edited, paused
//! and removed only here, each change checked against its handler and
//! written with its `schedule` log event (ADR-0037's append-only log,
//! codes only). The user may change only the schedules in their own
//! list; an owner module alone changes its app-made ones.

use juggler_core::storage::memory::{Actor, NewEvent, Stream};
use juggler_core::storage::{ScheduleRow, ScheduleTiming, Storage};
use serde_json::Value;

use super::registry::Registry;
use super::trigger::{self, Context, MissedPolicy, Modifiers, Place, Spec, Trigger};

/// The log's subject kind for a schedule.
pub const SUBJECT: &str = "schedule";
/// AI Juggler's own schedules (later, other tools' are listed beside).
pub const SOURCE_AI_JUGGLER: &str = "ai_juggler";
/// The most live schedules in the user's list.
pub const USER_MAX: u32 = 200;
/// The longest title.
pub const TITLE_MAX: usize = 200;

/// Who made a schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreatedBy {
    /// The user, in the UI.
    User,
    /// AI Juggler's code.
    App,
    /// Bot's draft the user confirmed (the confirmation is the user's;
    /// `bot` itself is reserved for 0.6's scheduled prompts).
    Bot,
}

impl CreatedBy {
    /// The stored code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::App => "app",
            Self::Bot => "bot",
        }
    }

    fn actor(self) -> Actor {
        match self {
            Self::App => Actor::App,
            Self::User | Self::Bot => Actor::User,
        }
    }
}

/// Why a change was refused.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ScheduleError {
    /// No handler by that id.
    #[error("no_handler")]
    NoHandler,
    /// The handler isn't one the user's own schedules may use.
    #[error("not_for_users")]
    NotForUsers,
    /// The payload or the timing doesn't fit (a code).
    #[error("invalid: {0}")]
    Invalid(&'static str),
    /// The title is empty or too long.
    #[error("title")]
    Title,
    /// The user's list is full.
    #[error("full")]
    Full,
    /// No such schedule.
    #[error("not_found")]
    NotFound,
    /// Not the user's to change (an app schedule, or ended).
    #[error("not_yours")]
    NotYours,
    /// Storage.
    #[error("storage: {0}")]
    Storage(String),
}

impl From<juggler_core::error::CoreError> for ScheduleError {
    fn from(error: juggler_core::error::CoreError) -> Self {
        Self::Storage(error.to_string())
    }
}

/// A schedule to make.
#[derive(Clone, Debug)]
pub struct NewSchedule {
    /// The owning module.
    pub owner: &'static str,
    /// The handler.
    pub handler: &'static str,
    /// The handler's data.
    pub payload: Value,
    /// Its timing.
    pub spec: Spec,
    /// In the user's list.
    pub user_visible: bool,
    /// Who made it.
    pub created_by: CreatedBy,
    /// Plain words.
    pub title: String,
}

/// A stored schedule, typed.
#[derive(Clone, Debug, PartialEq)]
pub struct Schedule {
    /// The row.
    pub row: ScheduleRow,
    /// Its timing.
    pub spec: Spec,
    /// Its data.
    pub payload: Value,
}

impl Schedule {
    /// The typed view of a row; `None` when its JSON doesn't parse (a row
    /// a later version wrote).
    #[must_use]
    pub fn of(row: ScheduleRow) -> Option<Self> {
        let spec = Spec {
            trigger: serde_json::from_str(&row.trigger).ok()?,
            modifiers: serde_json::from_str(&row.modifiers).ok()?,
            missed: serde_json::from_str(&row.missed_policy).ok()?,
        };
        let payload = serde_json::from_str(&row.payload).ok()?;
        Some(Self { row, spec, payload })
    }

    /// Where it is in time.
    #[must_use]
    pub const fn place(&self) -> Place {
        Place {
            base_ms: self.row.base_ms,
            armed_ms: self.row.armed_ms,
        }
    }

    /// The user may change it: in their list and not ended.
    #[must_use]
    pub const fn users(&self) -> bool {
        self.row.user_visible && self.row.ended_ms.is_none()
    }
}

/// A fresh id: `s-`, the time and a counter, in hex.
#[must_use]
pub fn fresh_id(now_ms: i64) -> String {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut extra = [0_u8; 2];
    let _ = getrandom::fill(&mut extra);
    format!(
        "s-{:011x}{:04x}{:02x}{:02x}",
        now_ms.max(0),
        n & 0xffff,
        extra[0],
        extra[1]
    )
}

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// The timing a place and its next give.
#[must_use]
pub fn timing(place: Place, next: Option<trigger::Next>) -> ScheduleTiming {
    ScheduleTiming {
        next_due_ms: next.map(|n| n.due_ms),
        occurrence_ms: next.map(|n| n.occurrence_ms),
        base_ms: place.base_ms,
        armed_ms: place.armed_ms,
    }
}

/// Appends a `schedule` log event (codes only); a database without the
/// log's tables (V54) skips it.
pub fn log(
    storage: &Storage,
    verb: &'static str,
    row: &ScheduleRow,
    actor: Actor,
    payload: Value,
    now_ms: i64,
) {
    if !juggler_core::storage::memory::installed(storage.raw_conn()).unwrap_or(false) {
        return;
    }
    let mut event = NewEvent::new(now_ms, Stream::Schedule, verb, actor);
    event.subject = Some((SUBJECT, row.id.clone()));
    let mut body = serde_json::json!({ "handler": row.handler, "owner": row.owner });
    if let (Some(map), Value::Object(more)) = (body.as_object_mut(), payload) {
        map.extend(more);
    }
    event.payload = body;
    if let Err(error) = storage.ai_log().append(&event, None) {
        tracing::debug!(%error, verb, "schedule: the log refused an event");
    }
}

fn checked(
    registry: &Registry,
    handler: &str,
    payload: &Value,
    spec: &Spec,
    title: &str,
    user: bool,
) -> Result<(), ScheduleError> {
    let found = registry.get(handler).ok_or(ScheduleError::NoHandler)?;
    if user && !found.user_made() {
        return Err(ScheduleError::NotForUsers);
    }
    found.validate(payload).map_err(ScheduleError::Invalid)?;
    spec.validate().map_err(ScheduleError::Invalid)?;
    if title.trim().is_empty() || title.chars().count() > TITLE_MAX {
        return Err(ScheduleError::Title);
    }
    Ok(())
}

/// Makes a schedule: checked against its handler, its first due time
/// computed and persisted, logged.
///
/// # Errors
///
/// [`ScheduleError`]: no handler, one the user may not use, a payload or
/// timing out of bounds, a bad title, the user's list full, storage.
pub fn create(
    storage: &Storage,
    registry: &Registry,
    new: NewSchedule,
    ctx: &Context,
    now_ms: i64,
) -> Result<Schedule, ScheduleError> {
    let user = new.created_by != CreatedBy::App;
    checked(
        registry,
        new.handler,
        &new.payload,
        &new.spec,
        &new.title,
        user,
    )?;
    if new.user_visible && storage.schedules().live_user_count()? >= USER_MAX {
        return Err(ScheduleError::Full);
    }
    let id = fresh_id(now_ms);
    let place = trigger::first_place(&new.spec.trigger, now_ms);
    let next = trigger::next(&new.spec, place, ctx, trigger::seed_of(&id));
    let t = timing(place, next);
    let row = ScheduleRow {
        id,
        owner: new.owner.to_owned(),
        handler: new.handler.to_owned(),
        payload: json(&new.payload),
        trigger: json(&new.spec.trigger),
        modifiers: json(&new.spec.modifiers),
        missed_policy: json(&new.spec.missed),
        next_due_ms: t.next_due_ms,
        occurrence_ms: t.occurrence_ms,
        base_ms: t.base_ms,
        armed_ms: t.armed_ms,
        last_run_ms: None,
        last_outcome: None,
        retries: 0,
        paused: false,
        user_visible: new.user_visible,
        created_by: new.created_by.code().to_owned(),
        source: SOURCE_AI_JUGGLER.to_owned(),
        title: new.title.trim().to_owned(),
        created_ms: now_ms,
        updated_ms: now_ms,
        ended_ms: None,
    };
    storage.schedules().insert(&row)?;
    log(
        storage,
        "created",
        &row,
        new.created_by.actor(),
        serde_json::json!({ "by": new.created_by.code() }),
        now_ms,
    );
    Ok(Schedule {
        row,
        spec: new.spec,
        payload: new.payload,
    })
}

/// One app-made schedule per owner and handler, kept in step with what the
/// owner asks: made when missing, its timing changed when it differs, left
/// alone (its place and history kept) when it is the same.
///
/// # Errors
///
/// As [`create`].
pub fn ensure_app(
    storage: &Storage,
    registry: &Registry,
    new: NewSchedule,
    ctx: &Context,
    now_ms: i64,
) -> Result<Schedule, ScheduleError> {
    let existing = storage
        .schedules()
        .of_handler(new.owner, new.handler)?
        .into_iter()
        .filter(|row| row.created_by == CreatedBy::App.code() && row.ended_ms.is_none())
        .find_map(Schedule::of);
    let Some(found) = existing else {
        return create(storage, registry, new, ctx, now_ms);
    };
    if found.spec == new.spec && found.payload == new.payload && found.row.title == new.title {
        return Ok(found);
    }
    checked(
        registry,
        new.handler,
        &new.payload,
        &new.spec,
        &new.title,
        false,
    )?;
    rewrite(
        storage,
        &found,
        &new.spec,
        &new.payload,
        &new.title,
        ctx,
        now_ms,
        Actor::App,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "one edit's fields, written together"
)]
fn rewrite(
    storage: &Storage,
    found: &Schedule,
    spec: &Spec,
    payload: &Value,
    title: &str,
    ctx: &Context,
    now_ms: i64,
    actor: Actor,
) -> Result<Schedule, ScheduleError> {
    let place = trigger::first_place(&spec.trigger, now_ms);
    let next = trigger::next(spec, place, ctx, trigger::seed_of(&found.row.id));
    let t = timing(place, next);
    storage.schedules().edit(
        &found.row.id,
        &json(&spec.trigger),
        &json(&spec.modifiers),
        &json(payload),
        title.trim(),
        t,
        now_ms,
    )?;
    let row = storage
        .schedules()
        .get(&found.row.id)?
        .ok_or(ScheduleError::NotFound)?;
    log(storage, "edited", &row, actor, Value::Null, now_ms);
    Schedule::of(row).ok_or(ScheduleError::NotFound)
}

fn users(storage: &Storage, id: &str) -> Result<Schedule, ScheduleError> {
    let row = storage
        .schedules()
        .get(id)?
        .ok_or(ScheduleError::NotFound)?;
    let found = Schedule::of(row).ok_or(ScheduleError::NotFound)?;
    if found.users() {
        Ok(found)
    } else {
        Err(ScheduleError::NotYours)
    }
}

/// The user edits one of their schedules: its time and title (its handler
/// and data stay).
///
/// # Errors
///
/// Not found, not theirs, out of bounds, storage.
#[allow(
    clippy::too_many_arguments,
    reason = "the user's edit: which, its new time and title, with the context"
)]
pub fn edit_users(
    storage: &Storage,
    registry: &Registry,
    id: &str,
    trigger: Trigger,
    modifiers: Modifiers,
    title: &str,
    ctx: &Context,
    now_ms: i64,
) -> Result<Schedule, ScheduleError> {
    let found = users(storage, id)?;
    let spec = Spec {
        trigger,
        modifiers,
        missed: found.spec.missed,
    };
    checked(
        registry,
        &found.row.handler,
        &found.payload,
        &spec,
        title,
        true,
    )?;
    rewrite(
        storage,
        &found,
        &spec,
        &found.payload,
        title,
        ctx,
        now_ms,
        Actor::User,
    )
}

/// The user pauses or resumes one of their schedules.
///
/// # Errors
///
/// Not found, not theirs, storage.
pub fn pause_users(
    storage: &Storage,
    id: &str,
    paused: bool,
    now_ms: i64,
) -> Result<Schedule, ScheduleError> {
    let found = users(storage, id)?;
    if storage.schedules().set_paused(id, paused, now_ms)? {
        let verb = if paused { "paused" } else { "resumed" };
        log(storage, verb, &found.row, Actor::User, Value::Null, now_ms);
    }
    let row = storage
        .schedules()
        .get(id)?
        .ok_or(ScheduleError::NotFound)?;
    Schedule::of(row).ok_or(ScheduleError::NotFound)
}

/// Pause all (or resume all) in the user's list; how many changed.
///
/// # Errors
///
/// Storage.
pub fn pause_all_users(
    storage: &Storage,
    paused: bool,
    now_ms: i64,
) -> Result<usize, ScheduleError> {
    Ok(storage.schedules().set_paused_user(paused, now_ms)?)
}

/// The user removes one of their schedules (an ended one too).
///
/// # Errors
///
/// Not found, not theirs, storage.
pub fn remove_users(storage: &Storage, id: &str, now_ms: i64) -> Result<(), ScheduleError> {
    let row = storage
        .schedules()
        .get(id)?
        .ok_or(ScheduleError::NotFound)?;
    if !row.user_visible {
        return Err(ScheduleError::NotYours);
    }
    storage.schedules().remove(id)?;
    log(storage, "removed", &row, Actor::User, Value::Null, now_ms);
    Ok(())
}

/// An owner arms its `AfterEvent` schedules on their event (now), or
/// disarms them on their `unless`.
///
/// # Errors
///
/// Storage.
pub fn on_event(
    storage: &Storage,
    owner: &str,
    handler: &str,
    event: &str,
    ctx: &Context,
    now_ms: i64,
) -> Result<usize, ScheduleError> {
    let mut changed = 0;
    for found in storage
        .schedules()
        .of_handler(owner, handler)?
        .into_iter()
        .filter(|row| row.ended_ms.is_none())
        .filter_map(Schedule::of)
    {
        let Trigger::AfterEvent {
            event: arms,
            unless,
            ..
        } = &found.spec.trigger
        else {
            continue;
        };
        let place = if arms == event {
            Place {
                base_ms: found.row.base_ms.min(now_ms),
                armed_ms: Some(now_ms),
            }
        } else if unless.as_deref() == Some(event) {
            Place {
                base_ms: found.row.base_ms,
                armed_ms: None,
            }
        } else {
            continue;
        };
        let next = trigger::next(&found.spec, place, ctx, trigger::seed_of(&found.row.id));
        storage
            .schedules()
            .set_timing(&found.row.id, timing(place, next), now_ms)?;
        changed += 1;
    }
    Ok(changed)
}

/// Every live schedule's next due time computed again from where it is
/// (launch, wake, a clock or zone change); how many moved. A retry keeps
/// its own time.
///
/// # Errors
///
/// Storage.
pub fn recompute_all(
    storage: &Storage,
    ctx: &Context,
    now_ms: i64,
) -> Result<usize, ScheduleError> {
    let mut moved = 0;
    let mut after = String::new();
    loop {
        let rows = storage.schedules().live_after(&after, 200)?;
        let Some(last) = rows.last() else {
            break;
        };
        after.clone_from(&last.id);
        for found in rows.into_iter().filter_map(Schedule::of) {
            if found.row.retries > 0 {
                continue;
            }
            let next = trigger::next(
                &found.spec,
                found.place(),
                ctx,
                trigger::seed_of(&found.row.id),
            );
            let t = timing(found.place(), next);
            if t.next_due_ms != found.row.next_due_ms || t.occurrence_ms != found.row.occurrence_ms
            {
                storage.schedules().set_timing(&found.row.id, t, now_ms)?;
                moved += 1;
            }
        }
    }
    Ok(moved)
}

/// The policy's code, for the log.
#[must_use]
pub const fn policy_code(policy: MissedPolicy) -> &'static str {
    policy.code()
}

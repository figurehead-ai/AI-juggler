//! The scheduler at run time (ADR-0039 §3, §5, §7): one thread that
//! sleeps until the soonest due time (at most [`TICK_MS`], so a wake or a
//! clock change is noticed even where the system can't say), takes what
//! is due, and hands each run to a small pool.
//!
//! - **Run controls (§5):** one run at a time per schedule (a schedule
//!   still running when it comes due again skips that occurrence, logged);
//!   at most [`RUNS_AT_ONCE`] runs at once, the user's reminders first,
//!   then the app's jobs, then idle work, each on a thread of the lowest
//!   priority; a run past its handler's timeout is abandoned and counted a
//!   failure; a failure the handler marks retryable is tried again with
//!   backoff (×2 from a minute, at most [`RETRIES_MAX`] times, never past
//!   the next regular due time).
//! - **Missed runs (§3):** what came due while asleep, closed or across a
//!   clock change is fired under the schedule's policy ([`trigger::fire`]).
//! - **Recompute (§3):** every live schedule's next due time on launch,
//!   on a wake, and on a clock or time-zone change (the system's
//!   notification on macOS; elsewhere the wall clock checked against the
//!   monotonic one, [`juggler_platform::clock::JumpCheck`]).
//! - **Writer holds stay short (ADR-0030):** the store's lock is taken for
//!   each read or write, never across a run.
//! - **Logged (§7):** every run, skip, miss, retry and failure, with its
//!   handler, outcome and duration, on the `schedule` stream.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock, Weak};
use std::time::Instant;

use juggler_core::storage::memory::Actor;
use juggler_core::storage::{ScheduleRow, ScheduleTiming, Storage};
use serde_json::{Value, json};

use super::registry::{Outcome, Priority, Registry, RunCtx, ScheduleHandler};
use super::store::{self, Schedule};
use super::trigger::{self, Context, Trigger};
use super::{idle, zone};
use crate::features::observe::bell::Bell;
use crate::state::AppState;

/// The longest the thread sleeps between looks (a wake or a clock change
/// is noticed by then where the system can't say; an idle job is
/// re-checked by then).
pub const TICK_MS: i64 = 30_000;
/// At most this many runs at once.
pub const RUNS_AT_ONCE: usize = 2;
/// The most retries of one failure.
pub const RETRIES_MAX: u32 = 5;
/// The first retry's wait (doubling, up to [`RETRY_CAP_MS`]).
pub const RETRY_FIRST_MS: i64 = 60_000;
/// The longest retry wait.
pub const RETRY_CAP_MS: i64 = 3_600_000;
/// Idle work waits this long after launch (ADR-0037 §2.3: two minutes
/// after start, so it never competes with the first paint).
pub const IDLE_AFTER_LAUNCH_MS: i64 = 120_000;

/// When the `n`th retry (from 0) runs after a failure at `now_ms`.
#[must_use]
pub fn retry_at(now_ms: i64, n: u32) -> i64 {
    let wait = RETRY_FIRST_MS.saturating_mul(1_i64 << n.min(20));
    now_ms.saturating_add(wait.min(RETRY_CAP_MS))
}

/// One run handed to the pool.
pub struct Dispatch {
    /// The schedule.
    pub schedule: Schedule,
    /// Its handler.
    pub handler: Arc<dyn ScheduleHandler>,
    /// When it was due.
    pub due_ms: i64,
    /// How many runs to make (more than one only under `RunEach`).
    pub runs: u32,
    /// Occurrences missed.
    pub missed: u32,
    /// Past its due time.
    pub late: bool,
    /// Retries so far (this run is a retry when above 0).
    pub retries: u32,
    /// Where the schedule is once this run is done.
    pub after: ScheduleTiming,
    /// Its lifetime is over after this run.
    pub ends: bool,
}

impl std::fmt::Debug for Dispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dispatch")
            .field("id", &self.schedule.row.id)
            .field("handler", &self.schedule.row.handler)
            .field("runs", &self.runs)
            .finish_non_exhaustive()
    }
}

fn stop(storage: &Storage, found: &Schedule, outcome: &'static str, now_ms: i64) {
    let t = ScheduleTiming {
        next_due_ms: None,
        occurrence_ms: None,
        base_ms: found.row.base_ms,
        armed_ms: found.row.armed_ms,
    };
    let _ = storage
        .schedules()
        .record_run(&found.row.id, None, outcome, 0, t, None);
    store::log(
        storage,
        "failed",
        &found.row,
        Actor::App,
        json!({ "outcome": outcome }),
        now_ms,
    );
}

/// What is due by `now_ms`, at most `room` runs, the most urgent first;
/// each run's next due time is written before it is handed out, so a
/// restart mid-run never runs it twice. What isn't run now (a schedule
/// still running, a missed occurrence its policy skips) is written and
/// logged here.
#[allow(
    clippy::too_many_lines,
    reason = "one pass over the due rows; each case is a few lines and they share the row"
)]
pub fn take_due<S: std::hash::BuildHasher>(
    storage: &Storage,
    registry: &Registry,
    ctx: &Context,
    now_ms: i64,
    idle: Option<bool>,
    running: &HashSet<String, S>,
    room: usize,
) -> Vec<Dispatch> {
    let Ok(rows) = storage.schedules().due(now_ms, 64) else {
        return Vec::new();
    };
    let mut ready: Vec<(Priority, Dispatch)> = Vec::new();
    for row in rows {
        let Some(found) = Schedule::of(row.clone()) else {
            if let Some(found) = Schedule::of(stub(row)) {
                stop(storage, &found, "invalid", now_ms);
            }
            continue;
        };
        let Some(handler) = registry.get(&found.row.handler).cloned() else {
            stop(storage, &found, "no_handler", now_ms);
            continue;
        };
        if handler.validate(&found.payload).is_err() {
            stop(storage, &found, "invalid", now_ms);
            continue;
        }
        let seed = trigger::seed_of(&found.row.id);
        let place = found.place();
        if running.contains(&found.row.id) {
            // §5: one run at a time; this occurrence is skipped.
            let fired = trigger::fire(&found.spec, place, now_ms, ctx, seed);
            let t = store::timing(fired.place, fired.next);
            let _ = storage.schedules().set_timing(&found.row.id, t, now_ms);
            store::log(
                storage,
                "skipped",
                &found.row,
                Actor::App,
                json!({ "outcome": "still_running" }),
                now_ms,
            );
            continue;
        }
        // An idle job waits for idle, or for its deadline; none runs in
        // the first minutes after launch (`idle` is `None` then).
        if let Trigger::WhenIdle { .. } = found.spec.trigger
            && idle.is_none()
        {
            continue;
        }
        if let Trigger::WhenIdle { .. } = found.spec.trigger
            && idle == Some(false)
            && found
                .row
                .occurrence_ms
                .and_then(|occ| trigger::idle_deadline(&found.spec.trigger, occ))
                .is_some_and(|deadline| now_ms < deadline)
        {
            continue;
        }
        let due_ms = found.row.next_due_ms.unwrap_or(now_ms);
        if found.row.retries > 0 {
            let next = trigger::next(&found.spec, place, ctx, seed);
            ready.push((
                handler.priority(),
                Dispatch {
                    after: store::timing(place, next),
                    ends: next.is_none() && !found.spec.repeats(),
                    runs: 1,
                    missed: 0,
                    late: false,
                    retries: found.row.retries,
                    due_ms,
                    handler,
                    schedule: found,
                },
            ));
            continue;
        }
        let fired = trigger::fire(&found.spec, place, now_ms, ctx, seed);
        let after = store::timing(fired.place, fired.next);
        let ends =
            fired.next.is_none() && !matches!(found.spec.trigger, Trigger::AfterEvent { .. });
        if fired.runs == 0 {
            let ended = (ends && fired.missed > 0).then_some(now_ms);
            let _ = storage
                .schedules()
                .record_run(&found.row.id, None, "missed", 0, after, ended);
            if fired.missed > 0 {
                store::log(
                    storage,
                    "missed",
                    &found.row,
                    Actor::App,
                    json!({ "policy": found.spec.missed.code(), "missed": fired.missed }),
                    now_ms,
                );
            }
            continue;
        }
        ready.push((
            handler.priority(),
            Dispatch {
                after,
                ends,
                runs: fired.runs,
                missed: fired.missed,
                late: !fired.on_time,
                retries: 0,
                due_ms,
                handler,
                schedule: found,
            },
        ));
    }
    ready.sort_by_key(|(priority, d)| (*priority, d.due_ms));
    let taken: Vec<Dispatch> = ready.into_iter().take(room).map(|(_, d)| d).collect();
    for d in &taken {
        // Handed out: not picked again while it runs.
        let t = ScheduleTiming {
            next_due_ms: d.after.next_due_ms.filter(|_| !d.ends),
            ..d.after
        };
        let _ = storage
            .schedules()
            .set_timing(&d.schedule.row.id, t, now_ms);
        if d.missed > 0 {
            store::log(
                storage,
                "missed",
                &d.schedule.row,
                Actor::App,
                json!({ "policy": d.schedule.spec.missed.code(), "missed": d.missed }),
                now_ms,
            );
        }
    }
    taken
}

/// How a handed-out run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// The handler returned.
    Returned(Outcome),
    /// It ran past its timeout and was abandoned.
    TimedOut,
}

/// Writes a run's end and logs it: the outcome, a retry when the handler
/// asked for one (with backoff, never past the next regular due time),
/// and the schedule's end when its lifetime is over.
pub fn finish(storage: &Storage, d: &Dispatch, ended: Ended, ran_ms: i64, took_ms: i64) {
    let row = &d.schedule.row;
    let id = row.id.as_str();
    let base = json!({ "duration_ms": took_ms, "runs": d.runs, "late": d.late });
    let with = |extra: Value| {
        let mut value = base.clone();
        if let (Some(map), Value::Object(more)) = (value.as_object_mut(), extra) {
            map.extend(more);
        }
        value
    };
    match ended {
        Ended::Returned(Outcome::Done) => {
            let _ = storage.schedules().record_run(
                id,
                Some(ran_ms),
                "ok",
                0,
                d.after,
                d.ends.then_some(ran_ms),
            );
            store::log(
                storage,
                "ran",
                row,
                Actor::App,
                with(json!({ "outcome": "ok" })),
                ran_ms,
            );
        }
        Ended::Returned(Outcome::Failed { retry: true, code }) if d.retries < RETRIES_MAX => {
            let at = retry_at(ran_ms, d.retries);
            let regular = d.after.next_due_ms.filter(|_| !d.ends);
            if regular.is_none_or(|next| at < next) {
                let t = ScheduleTiming {
                    next_due_ms: Some(at),
                    ..d.after
                };
                let _ = storage.schedules().record_run(
                    id,
                    Some(ran_ms),
                    "failed",
                    d.retries + 1,
                    t,
                    None,
                );
                store::log(
                    storage,
                    "retried",
                    row,
                    Actor::App,
                    with(json!({ "outcome": code, "retry": d.retries + 1 })),
                    ran_ms,
                );
            } else {
                failed(storage, d, code, ran_ms, with(json!({ "outcome": code })));
            }
        }
        Ended::Returned(Outcome::Failed { code, .. }) => {
            failed(storage, d, code, ran_ms, with(json!({ "outcome": code })));
        }
        Ended::TimedOut => {
            failed(
                storage,
                d,
                "timeout",
                ran_ms,
                with(json!({ "outcome": "timeout" })),
            );
        }
    }
}

fn failed(storage: &Storage, d: &Dispatch, code: &'static str, ran_ms: i64, payload: Value) {
    let outcome = if code == "timeout" {
        "timeout"
    } else {
        "failed"
    };
    let _ = storage.schedules().record_run(
        &d.schedule.row.id,
        Some(ran_ms),
        outcome,
        0,
        d.after,
        d.ends.then_some(ran_ms),
    );
    store::log(
        storage,
        outcome,
        &d.schedule.row,
        Actor::App,
        payload,
        ran_ms,
    );
}

/// Every live schedule's next due time computed again, logged once with
/// why (launch, wake, clock, zone) and how many moved.
pub fn recompute(storage: &Storage, ctx: &Context, why: &'static str, now_ms: i64) -> usize {
    let moved = store::recompute_all(storage, ctx, now_ms).unwrap_or(0);
    tracing::debug!(why, moved, "schedule: recomputed");
    moved
}

// ============================================================================ the thread

#[derive(Default)]
struct Inner {
    closed: bool,
    recompute: Option<&'static str>,
    /// Schedules with a run on a thread (an abandoned one until it returns).
    running: HashSet<String>,
    /// Runs counted against the pool (an abandoned one no longer is).
    active: usize,
    /// Transient app schedules, in memory only (the digest's window).
    transient: HashMap<&'static str, i64>,
}

type Shared = Bell<Inner>;

/// The scheduler's run-time state, held by the app's state.
#[derive(Default)]
pub struct Scheduler {
    shared: Arc<Shared>,
    registry: OnceLock<Arc<Registry>>,
    thread: OnceLock<()>,
}

impl std::fmt::Debug for Scheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scheduler")
            .field("registry", &self.registry.get())
            .finish_non_exhaustive()
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.shared.close(|inner| inner.closed = true);
    }
}

impl Scheduler {
    /// The registered handlers (empty before start).
    #[must_use]
    pub fn registry(&self) -> Option<&Arc<Registry>> {
        self.registry.get()
    }

    /// Installs the handlers (once; the start does it). `false` when they
    /// were already.
    pub fn install(&self, registry: Registry) -> bool {
        self.registry.set(Arc::new(registry)).is_ok()
    }

    /// Something changed (a schedule made, edited, paused, armed): look
    /// again now.
    pub fn changed(&self) {
        self.shared.ring();
    }

    /// The times moved under the schedules (a wake, a clock or zone
    /// change): recompute, then look again.
    pub fn time_moved(&self, why: &'static str) {
        if let Ok(mut inner) = self.shared.lock() {
            inner.recompute = Some(why);
        }
        self.shared.ring();
    }

    /// A transient schedule's next due time, if it has one.
    #[must_use]
    pub fn transient_due(&self, key: &str) -> Option<i64> {
        self.shared
            .lock()
            .ok()
            .and_then(|inner| inner.transient.get(key).copied())
    }

    /// A transient schedule (in memory, never stored): `key`'s next due
    /// time, or none. The digest publishes its window this way (§11).
    pub fn set_transient(&self, key: &'static str, due_ms: Option<i64>) {
        if let Ok(mut inner) = self.shared.lock() {
            let before = inner.transient.get(key).copied();
            match due_ms {
                Some(at) => {
                    inner.transient.insert(key, at);
                }
                None => {
                    inner.transient.remove(key);
                }
            }
            if before == due_ms {
                return;
            }
        }
        self.shared.ring();
    }
}

/// The context now: the user's zone, read fresh, and their quiet hours.
#[must_use]
pub fn context(storage: &Storage) -> Context {
    let quiet = crate::settings_policy::quiet_hours(storage);
    Context {
        zone: zone::local(),
        quiet: (quiet.start != quiet.end).then(|| {
            (
                trigger::ClockTime::new(u8::try_from(quiet.start).unwrap_or(0), 0),
                trigger::ClockTime::new(u8::try_from(quiet.end).unwrap_or(0), 0),
            )
        }),
    }
}

/// Starts the scheduler with `registry`: recomputes on launch, watches the
/// system's wake and clock changes, and runs what comes due.
pub fn start(state: &Arc<AppState>, registry: Registry) {
    let scheduler = &state.schedules;
    if !scheduler.install(registry) {
        return;
    }
    scheduler.thread.get_or_init(|| {
        if let Ok(mut inner) = scheduler.shared.lock() {
            inner.recompute = Some("launch");
        }
        let weak = Arc::downgrade(state);
        let tell = weak.clone();
        let _ = juggler_window_events::watch_time_changes(move |change| {
            if let Some(state) = tell.upgrade() {
                state.schedules.time_moved(match change {
                    juggler_window_events::TimeChange::Wake => "wake",
                    juggler_window_events::TimeChange::Clock => "clock",
                    juggler_window_events::TimeChange::Zone => "zone",
                });
            }
        });
        let shared = scheduler.shared.clone();
        let _ = std::thread::Builder::new()
            .name("schedule".into())
            .spawn(move || run(&shared, &weak));
    });
}

fn run(shared: &Arc<Shared>, state: &Weak<AppState>) {
    let _ = juggler_platform::thread::lowest_priority();
    let mut jump =
        juggler_platform::clock::JumpCheck::new(crate::commands::now_millis(), Instant::now());
    let launched = crate::commands::now_millis();
    loop {
        let now = crate::commands::now_millis();
        let Some(app) = state.upgrade() else {
            return;
        };
        let (why, running, room) = {
            let Ok(mut inner) = shared.lock() else {
                return;
            };
            if inner.closed {
                return;
            }
            if let Some(change) = jump.check(now, Instant::now()) {
                inner.recompute.get_or_insert(match change {
                    juggler_platform::clock::ClockChange::Ahead => "ahead",
                    juggler_platform::clock::ClockChange::Back => "back",
                });
            }
            (
                inner.recompute.take(),
                inner.running.clone(),
                RUNS_AT_ONCE.saturating_sub(inner.active),
            )
        };
        let Some(registry) = app.schedules.registry.get().cloned() else {
            return;
        };
        let idle = (now - launched >= IDLE_AFTER_LAUNCH_MS).then(|| idle::now(&app));
        let (taken, soonest) = match app.storage.lock() {
            Ok(storage) => {
                let ctx = context(&storage);
                if let Some(why) = why {
                    recompute(&storage, &ctx, why, now);
                }
                let taken = if room > 0 {
                    take_due(&storage, &registry, &ctx, now, idle, &running, room)
                } else {
                    Vec::new()
                };
                (taken, storage.schedules().soonest().ok().flatten())
            }
            Err(_) => return,
        };
        fire_transient(shared, &app, now);
        for d in taken {
            dispatch(shared, &app, d);
        }
        let Ok(inner) = shared.lock() else {
            return;
        };
        if inner.closed {
            return;
        }
        let next_transient = inner.transient.values().min().copied();
        let wake = [soonest.filter(|at| *at > now), next_transient]
            .into_iter()
            .flatten()
            .min()
            .map_or(now + TICK_MS, |at| at.min(now + TICK_MS));
        drop(app);
        if shared.sleep(inner, Some(wake), now).is_none() {
            return;
        }
    }
}

/// Fires the transient schedules due by now, each through its owner's
/// own call.
fn fire_transient(shared: &Arc<Shared>, state: &Arc<AppState>, now: i64) {
    let due: Vec<&'static str> = {
        let Ok(mut inner) = shared.lock() else {
            return;
        };
        let due: Vec<&'static str> = inner
            .transient
            .iter()
            .filter(|(_, at)| **at <= now)
            .map(|(key, _)| *key)
            .collect();
        for key in &due {
            inner.transient.remove(key);
        }
        due
    };
    for key in due {
        super::transient(state, key);
    }
}

fn dispatch(shared: &Arc<Shared>, state: &Arc<AppState>, d: Dispatch) {
    let id = d.schedule.row.id.clone();
    if let Ok(mut inner) = shared.lock() {
        inner.running.insert(id.clone());
        inner.active += 1;
    }
    let shared = Arc::clone(shared);
    let state = Arc::clone(state);
    let _ = std::thread::Builder::new()
        .name("schedule-run".into())
        .spawn(move || {
            let _ = juggler_platform::thread::lowest_priority();
            supervise(&shared, &state, &d);
            if let Ok(mut inner) = shared.lock() {
                inner.running.remove(&id);
            }
            shared.ring();
        });
}

/// Runs `d` on its own thread and waits up to its timeout: past it, the
/// run is abandoned (counted a failure, its pool place freed) but the
/// schedule stays "running" until the thread returns, so it never runs
/// twice at once.
fn supervise(shared: &Arc<Shared>, state: &Arc<AppState>, d: &Dispatch) {
    let started = Instant::now();
    let ran_ms = crate::commands::now_millis();
    let (tx, rx) = std::sync::mpsc::channel();
    let handler = Arc::clone(&d.handler);
    let payload = d.schedule.payload.clone();
    let id = d.schedule.row.id.clone();
    let title = d.schedule.row.title.clone();
    let (due_ms, late, missed, runs) = (d.due_ms, d.late, d.missed, d.runs);
    let worker_state = Arc::clone(state);
    let worker = std::thread::Builder::new()
        .name("schedule-handler".into())
        .spawn(move || {
            let _ = juggler_platform::thread::lowest_priority();
            let busy = || crate::features::ai::run::busy(crate::features::ai::service::shared());
            let mut outcome = Outcome::Done;
            for _ in 0..runs.max(1) {
                let ctx = RunCtx {
                    state: &worker_state,
                    id: &id,
                    title: &title,
                    due_ms,
                    now_ms: crate::commands::now_millis(),
                    late,
                    missed,
                    busy: &busy,
                };
                outcome = handler.run(&payload, &ctx);
                if outcome != Outcome::Done {
                    break;
                }
            }
            let _ = tx.send(outcome);
        });
    let ended = match worker {
        Ok(worker) => match rx.recv_timeout(d.handler.timeout()) {
            Ok(outcome) => {
                let _ = worker.join();
                Ended::Returned(outcome)
            }
            Err(_) => Ended::TimedOut,
        },
        Err(_) => Ended::Returned(Outcome::Failed {
            retry: true,
            code: "no_thread",
        }),
    };
    let took = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    if let Ok(storage) = state.storage.lock() {
        finish(&storage, d, ended, ran_ms, took);
    }
    if let Ok(mut inner) = shared.lock() {
        inner.active = inner.active.saturating_sub(1);
    }
    shared.ring();
    if ended == Ended::TimedOut {
        // The abandoned run's thread ends on its own; the schedule stays
        // marked running until then (see `dispatch`).
        let _ = rx.recv();
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;

/// A row whose JSON a later version wrote, read as a spent `At` with no
/// data, only so it can be stopped and logged.
fn stub(mut row: ScheduleRow) -> ScheduleRow {
    row.trigger = json!({ "kind": "at", "at_ms": 1 }).to_string();
    row.modifiers = json!({ "lifetime": { "kind": "once" } }).to_string();
    row.missed_policy = json!({ "kind": "run_once" }).to_string();
    "null".clone_into(&mut row.payload);
    row
}

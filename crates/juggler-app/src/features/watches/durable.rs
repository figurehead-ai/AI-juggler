//! G12 (AJ-E2027, ADR-0039 §11): watches' timed conditions (idle for N
//! minutes, looks stuck after N, an "until" time) survive a restart and a
//! sleep. Each watch's engine state is saved as a unit, with its schedule,
//! in one row of the one scheduler (one statement, so one transaction):
//! the row's data is the state, its `At` the watch's earliest deadline.
//!
//! - **On start** the engine is loaded from the stored watches as before,
//!   then each watch's saved state is put back ([`restore_all`]): its
//!   deadlines as they were armed, not again from the start.
//! - **After every engine step** what changed is saved ([`save`]); a watch
//!   no longer heard loses its row.
//! - **When a deadline falls due** the scheduler runs [`Deadline`], which
//!   wakes the watches' thread: after a sleep (its own timer paused with
//!   the Mac) the fire comes at once, still with the deadline's own time,
//!   and once (the engine disarms what fired).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use juggler_core::storage::{ScheduleRow, Storage};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::engine::{Engine, Saved};
use super::model::WatchId;
use crate::features::schedule::registry::{Outcome, Priority, RunCtx, ScheduleHandler};
use crate::features::schedule::store::SOURCE_AI_JUGGLER;
use crate::features::schedule::trigger::{Lifetime, MissedPolicy, Modifiers, Trigger};
use crate::state::AppState;

/// The module that owns these schedules.
pub const OWNER: &str = "watches";
/// The handler that wakes the watches when a deadline is due.
pub const HANDLER: &str = "watch.deadline";
/// The largest saved state, in bytes.
const STATE_MAX: usize = 256 * 1024;

/// A watch's row data: which watch, and its engine state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    /// The watch.
    pub watch: WatchId,
    /// Its engine state.
    pub state: Saved,
}

/// A watch's schedule id.
#[must_use]
pub fn row_id(watch: &WatchId) -> String {
    format!("wd-{}", watch.0)
}

/// Wakes the watches when a deadline is due (the engine fires it).
pub struct Deadline;

impl ScheduleHandler for Deadline {
    fn validate(&self, payload: &Value) -> Result<(), &'static str> {
        let parsed: Payload = serde_json::from_value(payload.clone()).map_err(|_| "payload")?;
        let size = serde_json::to_string(&parsed.state).map_or(usize::MAX, |s| s.len());
        if parsed.watch.0.is_empty() || parsed.watch.0.len() > 128 || size > STATE_MAX {
            return Err("payload");
        }
        Ok(())
    }
    fn run(&self, _payload: &Value, ctx: &RunCtx<'_>) -> Outcome {
        super::runtime::changed(ctx.state);
        Outcome::Done
    }
    // A watch the user set is told before the app's own jobs; waking the
    // thread takes no time.
    fn priority(&self) -> Priority {
        Priority::User
    }
    fn timeout(&self) -> Duration {
        Duration::from_secs(10)
    }
    fn id(&self) -> &'static str {
        HANDLER
    }
    fn owner(&self) -> &'static str {
        OWNER
    }
}

/// The handler, for the scheduler's registry.
#[must_use]
pub fn handler() -> Arc<dyn ScheduleHandler> {
    Arc::new(Deadline)
}

/// Each live watch's state and earliest wake, as the engine holds them now.
#[must_use]
pub fn snapshot(engine: &Engine) -> BTreeMap<WatchId, (Saved, Option<i64>)> {
    engine
        .ids()
        .filter_map(|id| Some((id.clone(), (engine.saved(id)?, engine.wake_of(id)))))
        .collect()
}

/// The saved states, by watch.
#[must_use]
pub fn rows(storage: &Storage) -> BTreeMap<WatchId, (ScheduleRow, Saved)> {
    storage
        .schedules()
        .of_handler(OWNER, HANDLER)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let payload: Payload = serde_json::from_str(&row.payload).ok()?;
            Some((payload.watch, (row, payload.state)))
        })
        .collect()
}

/// Puts each watch's saved state back into a freshly loaded engine; how
/// many came back.
pub fn restore_all(storage: &Storage, engine: &mut Engine) -> usize {
    rows(storage)
        .iter()
        .filter(|(watch, (_, saved))| engine.restore(watch, saved))
        .count()
}

fn row_of(watch: &WatchId, saved: &Saved, wake: Option<i64>, now_ms: i64) -> Option<ScheduleRow> {
    let payload = serde_json::to_string(&Payload {
        watch: watch.clone(),
        state: saved.clone(),
    })
    .ok()
    .filter(|p| p.len() <= STATE_MAX)?;
    let trigger = Trigger::At {
        at_ms: wake.unwrap_or(i64::MAX / 2),
    };
    let id = row_id(watch);
    Some(ScheduleRow {
        owner: OWNER.to_owned(),
        handler: HANDLER.to_owned(),
        payload,
        trigger: serde_json::to_string(&trigger).ok()?,
        modifiers: serde_json::to_string(&Modifiers {
            lifetime: Lifetime::Always,
            ..Modifiers::once()
        })
        .ok()?,
        missed_policy: serde_json::to_string(&MissedPolicy::RunOnce).ok()?,
        next_due_ms: wake,
        occurrence_ms: wake,
        base_ms: wake.map_or(now_ms, |at| at - 1),
        armed_ms: None,
        last_run_ms: None,
        last_outcome: None,
        retries: 0,
        paused: false,
        user_visible: false,
        created_by: "app".to_owned(),
        source: SOURCE_AI_JUGGLER.to_owned(),
        title: "A watch's timer".to_owned(),
        created_ms: now_ms,
        updated_ms: now_ms,
        ended_ms: None,
        id,
    })
}

/// Saves what changed since `before` (the last saved snapshot): each
/// changed watch's state and its next wake in one row; a watch no longer
/// heard loses its row. Returns whether the scheduler should look again.
pub fn save(
    storage: &Storage,
    before: &BTreeMap<WatchId, (Saved, Option<i64>)>,
    now: &BTreeMap<WatchId, (Saved, Option<i64>)>,
    now_ms: i64,
) -> bool {
    let mut any = false;
    for (watch, (saved, wake)) in now {
        if before.get(watch) == Some(&(saved.clone(), *wake)) {
            continue;
        }
        if let Some(row) = row_of(watch, saved, *wake, now_ms)
            && storage.schedules().put(&row).is_ok()
        {
            any = true;
        }
    }
    let gone: BTreeSet<&WatchId> = before.keys().filter(|w| !now.contains_key(*w)).collect();
    for watch in gone {
        if storage.schedules().remove(&row_id(watch)).unwrap_or(false) {
            any = true;
        }
    }
    any
}

/// Saves the engine's state as it is now (start: every watch, rows of
/// watches no longer held removed).
pub fn save_all(
    state: &Arc<AppState>,
    snapshot_now: &BTreeMap<WatchId, (Saved, Option<i64>)>,
    now_ms: i64,
) {
    let Ok(storage) = state.storage.lock() else {
        return;
    };
    let before: BTreeMap<WatchId, (Saved, Option<i64>)> = rows(&storage)
        .into_iter()
        .map(|(watch, (row, saved))| (watch, (saved, row.next_due_ms)))
        .collect();
    let moved = save(&storage, &before, snapshot_now, now_ms);
    drop(storage);
    if moved {
        state.schedules.changed();
    }
}

//! The Floor (SPEC §9.7, ADR-0026): the backend's side.
//!
//! - One [`FloorSnapshot`] of the company, built by [`feed`], for the
//!   Floor window and the main window's Floor list alike.
//! - Lazy: nothing runs until the Floor or its list first asks
//!   ([`current`]). From then on the events the app already emits mark it
//!   stale, and a 250 ms pass rebuilds it at most 4 times a second, sends
//!   it to the render thread's subscribers and tells the window
//!   (`floor://changed`).

pub mod card;
pub mod commands;
pub mod controls;
#[cfg(feature = "floor-harness")]
pub mod drive;
pub mod feed;
#[cfg(feature = "floor-harness")]
pub mod harness;
pub mod light;
pub mod logos;
pub mod render;
pub mod store;
pub mod window;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use juggler_floor::snapshot::FloorSnapshot;

use crate::error::AppResult;
use crate::state::AppState;

/// The window's event: the snapshot changed (its payload is the new one).
pub const EVENT_CHANGED: &str = "floor://changed";

/// The events that make the snapshot stale.
pub const SOURCES: &[&str] = &[
    crate::EVENT_SESSIONS_UPDATED,
    crate::EVENT_SESSIONS_QUOTA,
    crate::features::tasks::EVENT_CHANGED,
    crate::features::notifications::views::EVENT_CHANGED,
    crate::features::conversations::EVENT_CHANGED,
    crate::features::runtime::EVENT_CHANGED,
    crate::features::places::EVENT_CHANGED,
];

/// The events that come every few seconds with readings (the processes'
/// CPU, This Mac's load): what they change shows in steps over minutes (a
/// fire after 2 minutes hot, the rack's tenths), so they rebuild the
/// snapshot at most every [`SLOW_EVERY`] (SPEC §9.7: next to nothing at
/// rest).
pub const SLOW_SOURCES: &[&str] = &[
    crate::EVENT_THIS_MAC,
    crate::features::processes::EVENT_CHANGED,
];

/// How often the readings may rebuild the snapshot.
pub const SLOW_EVERY: std::time::Duration = std::time::Duration::from_secs(30);

/// The Floor's state in [`AppState`].
#[derive(Default)]
pub struct Floor {
    app: OnceLock<tauri::AppHandle>,
    started: AtomicBool,
    stale: Arc<AtomicBool>,
    stale_slow: Arc<AtomicBool>,
    last: RwLock<Option<Arc<FloorSnapshot>>>,
    hot: Mutex<BTreeMap<String, i64>>,
    subscribers: Mutex<Vec<Subscriber>>,
    window: Mutex<Option<window::Open>>,
    published: Mutex<Option<Arc<FloorSnapshot>>>,
}

impl std::fmt::Debug for Floor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Floor")
            .field("started", &self.started.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

/// Called with each new snapshot; `false` when it wants no more.
pub type Subscriber = Box<dyn Fn(Arc<FloorSnapshot>) -> bool + Send>;

impl Floor {
    /// Keeps the app's handle for when the Floor first runs (at setup).
    pub fn attach(&self, app: tauri::AppHandle) {
        let _ = self.app.set(app);
    }

    /// Calls `on` with each new snapshot until it returns `false`.
    pub fn subscribe(&self, on: Subscriber) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(on);
        }
    }

    fn publish(&self, snapshot: &Arc<FloorSnapshot>) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.retain(|on| on(Arc::clone(snapshot)));
        }
    }
}

/// Builds the snapshot now and keeps it.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn rebuild(state: &Arc<AppState>) -> AppResult<Arc<FloorSnapshot>> {
    let at = std::time::Instant::now();
    let now = juggler_core::model::Timestamp::now().millis();
    let inputs = feed::gather(state, now)?;
    let snapshot = {
        let mut hot = state
            .floor
            .hot
            .lock()
            .map_err(|_| crate::error::AppError::StatePoisoned)?;
        Arc::new(feed::assemble(&inputs, now, &mut hot))
    };
    if let Ok(mut last) = state.floor.last.write() {
        *last = Some(Arc::clone(&snapshot));
    }
    render::COUNTERS.feed_ns.fetch_add(
        render::nanos(at.elapsed()),
        std::sync::atomic::Ordering::Relaxed,
    );
    Ok(snapshot)
}

/// The snapshot as it is (built now the first time, which also starts
/// keeping it current). At most one pass (250 ms) behind what changed.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn current(state: &Arc<AppState>) -> AppResult<Arc<FloorSnapshot>> {
    if let Some(app) = state.floor.app.get() {
        start(state, app.clone());
    }
    let kept = state.floor.last.read().ok().and_then(|last| last.clone());
    match kept {
        Some(snapshot) => Ok(snapshot),
        None => rebuild(state),
    }
}

/// Marks the snapshot stale: the next pass rebuilds it (a change the
/// events don't carry, such as a logo the window drew).
pub fn mark_stale(state: &AppState) {
    state.floor.stale.store(true, Ordering::Release);
}

/// What made the last snapshots news (the harness's record): the first
/// part that differed, newest last.
pub static NEWS: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());

/// The same company as drawn, whenever each was built: figures compared
/// within the steps the Floor shows them at (the rack's tenths, a
/// battery's whole percent, a fire's 5 % only while hot, times to the
/// minute) against the last one published, so a reading that wiggles
/// wakes nothing (SPEC §9.7: no frames at rest).
#[cfg(test)]
fn same(a: &FloorSnapshot, b: &FloorSnapshot) -> bool {
    differs(a, b).is_none()
}

/// The first part of `b` that differs from `a` as drawn.
fn differs(a: &FloorSnapshot, b: &FloorSnapshot) -> Option<&'static str> {
    const MINUTE: i64 = 60_000;
    let near = |x: f32, y: f32, step: f32| (x - y).abs() < step;
    let minute = |x: Option<i64>, y: Option<i64>| match (x, y) {
        (Some(x), Some(y)) => (x - y).abs() < MINUTE,
        (x, y) => x == y,
    };
    if !near(a.machine.load, b.machine.load, 0.1)
        || a.machine.alert_since_ms != b.machine.alert_since_ms
        || a.machine.alert != b.machine.alert
    {
        return Some("machine");
    }
    if a.apps.len() != b.apps.len() {
        return Some("apps");
    }
    for (x, y) in a.apps.iter().zip(&b.apps) {
        let hot = x.hot_since_ms.is_some() || y.hot_since_ms.is_some();
        let cpu = |p: f32, q: f32| !hot || near(p, q, 5.0);
        let (mut x2, mut y2) = (x.clone(), y.clone());
        if !cpu(x.cpu_pct, y.cpu_pct) || !minute(x.last_active_ms, y.last_active_ms) {
            return Some("apps");
        }
        x2.cpu_pct = 0.0;
        y2.cpu_pct = 0.0;
        x2.last_active_ms = None;
        y2.last_active_ms = None;
        if x2.sessions.len() != y2.sessions.len() {
            return Some("sessions");
        }
        for (s, t) in x2.sessions.iter_mut().zip(y2.sessions.iter_mut()) {
            if !cpu(s.cpu_pct, t.cpu_pct) || !minute(Some(s.last_ms), Some(t.last_ms)) {
                return Some("sessions");
            }
            s.cpu_pct = 0.0;
            t.cpu_pct = 0.0;
            s.last_ms = 0;
            t.last_ms = 0;
        }
        if x2 != y2 {
            return Some(if x2.sessions == y2.sessions {
                "apps"
            } else {
                "sessions"
            });
        }
    }
    if a.accounts.len() != b.accounts.len() {
        return Some("accounts");
    }
    for (x, y) in a.accounts.iter().zip(&b.accounts) {
        let pct = match (x.left_pct, y.left_pct) {
            (Some(p), Some(q)) => near(p, q, 1.0),
            (p, q) => p == q,
        };
        let (mut x2, mut y2) = (x.clone(), y.clone());
        x2.left_pct = None;
        y2.left_pct = None;
        x2.last_used_ms = None;
        y2.last_used_ms = None;
        if !pct || !minute(x.last_used_ms, y.last_used_ms) || x2 != y2 {
            return Some("accounts");
        }
    }
    if a.pools != b.pools {
        return Some("pools");
    }
    if a.notices != b.notices {
        return Some("notices");
    }
    // a limit the clock alone passes (smoke out, done to idle, an hour
    // unused): drawn without waiting for anything else to change
    if juggler_floor::scene::time_limits(a) != juggler_floor::scene::time_limits(b) {
        return Some("limits");
    }
    None
}

/// Whether `snapshot` is news since the last one published (and, if so,
/// it is now that one).
fn news(floor: &Floor, snapshot: &Arc<FloorSnapshot>) -> bool {
    floor.published.lock().is_ok_and(|mut published| {
        let why = published
            .as_ref()
            .map_or(Some("first"), |before| differs(before, snapshot));
        let fresh = why.is_some();
        if let (Some(why), Ok(mut news)) = (why, NEWS.lock()) {
            if news.len() >= 64 {
                news.remove(0);
            }
            news.push(why);
        }
        if fresh {
            *published = Some(Arc::clone(snapshot));
        }
        fresh
    })
}

/// Starts keeping the snapshot current: once, on first use.
fn start(state: &Arc<AppState>, app: tauri::AppHandle) {
    use tauri::{Emitter, Listener};
    if state.floor.started.swap(true, Ordering::AcqRel) {
        return;
    }
    // The first pass builds and publishes; after it, only news.
    state.floor.stale.store(true, Ordering::Release);
    for name in SOURCES {
        let mark = Arc::clone(&state.floor.stale);
        app.listen_any(*name, move |_| mark.store(true, Ordering::Release));
    }
    for name in SLOW_SOURCES {
        let mark = Arc::clone(&state.floor.stale_slow);
        app.listen_any(*name, move |_| mark.store(true, Ordering::Release));
    }
    let mut slow_at = std::time::Instant::now();
    let weak = Arc::downgrade(state);
    // poll: drains the stale mark the events set, so a burst of them
    // rebuilds the snapshot once (at most 4 a second); ends with the app.
    crate::ticks::every_while(
        "floor-feed",
        std::time::Duration::from_millis(250),
        move || {
            let Some(state) = weak.upgrade() else {
                return false;
            };
            let slow = slow_at.elapsed() >= SLOW_EVERY
                && state.floor.stale_slow.swap(false, Ordering::AcqRel);
            if slow {
                slow_at = std::time::Instant::now();
            }
            if state.floor.stale.swap(false, Ordering::AcqRel) | slow {
                // The 2 s process tick marks it stale often: only a real
                // change wakes the Floor and the window.
                if let Ok(snapshot) = rebuild(&state)
                    && news(&state.floor, &snapshot)
                {
                    state.floor.publish(&snapshot);
                    let _ = app.emit(EVENT_CHANGED, snapshot.as_ref());
                }
            }
            true
        },
    );
}

#[cfg(test)]
mod tests;

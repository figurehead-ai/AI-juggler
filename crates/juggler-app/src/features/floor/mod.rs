//! The Floor (SPEC §9.7, ADR-0026): the backend's side.
//!
//! - One [`FloorSnapshot`] of the company, built by [`feed`], for the
//!   Floor window and the main window's Floor list alike.
//! - Lazy: nothing runs until the Floor or its list first asks
//!   ([`current`]). From then on the events the app already emits mark it
//!   stale, and a 250 ms pass rebuilds it at most 4 times a second, sends
//!   it to the render thread's subscribers and tells the window
//!   (`floor://changed`).

pub mod commands;
pub mod feed;

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
    crate::EVENT_THIS_MAC,
    crate::features::processes::EVENT_CHANGED,
    crate::features::tasks::EVENT_CHANGED,
    crate::features::notifications::views::EVENT_CHANGED,
    crate::features::conversations::EVENT_CHANGED,
    crate::features::runtime::EVENT_CHANGED,
    crate::features::places::EVENT_CHANGED,
];

/// The Floor's state in [`AppState`].
#[derive(Default)]
pub struct Floor {
    app: OnceLock<tauri::AppHandle>,
    started: AtomicBool,
    stale: Arc<AtomicBool>,
    last: RwLock<Option<Arc<FloorSnapshot>>>,
    hot: Mutex<BTreeMap<String, i64>>,
    subscribers: Mutex<Vec<std::sync::mpsc::Sender<Arc<FloorSnapshot>>>>,
    published: Mutex<Option<Arc<FloorSnapshot>>>,
}

impl std::fmt::Debug for Floor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Floor")
            .field("started", &self.started.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl Floor {
    /// Keeps the app's handle for when the Floor first runs (at setup).
    pub fn attach(&self, app: tauri::AppHandle) {
        let _ = self.app.set(app);
    }

    /// A channel the render thread reads new snapshots from.
    pub fn subscribe(&self) -> std::sync::mpsc::Receiver<Arc<FloorSnapshot>> {
        let (tx, rx) = std::sync::mpsc::channel();
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(tx);
        }
        rx
    }

    fn publish(&self, snapshot: &Arc<FloorSnapshot>) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.retain(|tx| tx.send(Arc::clone(snapshot)).is_ok());
        }
    }
}

/// Builds the snapshot now and keeps it.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn rebuild(state: &Arc<AppState>) -> AppResult<Arc<FloorSnapshot>> {
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

/// The same company, whenever each was built.
fn same(a: &FloorSnapshot, b: &FloorSnapshot) -> bool {
    a.apps == b.apps
        && a.accounts == b.accounts
        && a.pools == b.pools
        && a.machine == b.machine
        && a.notices == b.notices
}

/// Whether `snapshot` is news since the last one published (and, if so,
/// it is now that one).
fn news(floor: &Floor, snapshot: &Arc<FloorSnapshot>) -> bool {
    floor.published.lock().is_ok_and(|mut published| {
        let fresh = published
            .as_ref()
            .is_none_or(|before| !same(before, snapshot));
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
            if state.floor.stale.swap(false, Ordering::AcqRel) {
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

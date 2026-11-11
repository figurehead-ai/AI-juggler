//! ADR-0032 amendment 4: what runs reaches the window within the bound.
//! A full look at the process table at least every
//! [`RECONCILE_EVERY_MS`] (the places tick) and at once when an app
//! launches or quits (`NSWorkspace`, `places::spawn`).

use std::sync::Arc;

use crate::features::conversations::places::HostRegistry;
use crate::features::conversations::procs;
use crate::state::AppState;

/// ADR-0032: the longest between two full looks at what runs (an app's
/// launch and quit come sooner, by `NSWorkspace`'s notifications).
pub const RECONCILE_EVERY_MS: i64 = 5_000;

/// ADR-0032: looks at what runs now, and tells the window when the set
/// of running desktop Apps changed (`force`: look now, whenever the last
/// look was). Returns whether it changed.
pub fn reconcile(state: &Arc<AppState>, now: i64, force: bool) -> bool {
    {
        let Ok(last) = state.places.presence.lock() else {
            return false;
        };
        if !force && now.saturating_sub(last.1) < RECONCILE_EVERY_MS {
            return false;
        }
    }
    let hosts = HostRegistry::of(&state.registry);
    let home = juggler_platform::user::home().unwrap_or_default();
    let running: std::collections::BTreeSet<String> =
        crate::features::presence::running(&procs::table(), &hosts, &home)
            .into_keys()
            .collect();
    let moved = state.places.presence.lock().is_ok_and(|mut last| {
        let moved = last.0 != running;
        *last = (running, now);
        moved
    });
    if moved {
        crate::features::places::changed(state);
    }
    moved
}

/// ADR-0032: the desktop Apps running at the last reconcile (at most
/// [`RECONCILE_EVERY_MS`] old, sooner on a launch or quit), without a
/// look at the process table: the Floor's lit offices are the rail's
/// Open now.
pub fn running_at_last_look(state: &AppState) -> std::collections::BTreeSet<String> {
    state
        .places
        .presence
        .lock()
        .map(|last| last.0.clone())
        .unwrap_or_default()
}

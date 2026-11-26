//! One scheduler for everything time-based (ADR-0039, D-151; AJ-E2027):
//! durable, typed, visible triggers. Sub-minute observation loops stay
//! ticks (`ticks.rs`); anything due at a wall-clock time, every minute or
//! longer with a missed run that matters, kept across a restart or seen
//! by the user is a schedule here.
//!
//! - **The trigger maths** ([`trigger`]): pure; next due, missed runs,
//!   DST, windows, jitter, in the user's local time zone (`jiff`, read
//!   fresh by [`zone`]).
//! - **Handlers** ([`registry`]): only registered, typed handlers run; a
//!   schedule names one and carries data, never code. Agent text never
//!   makes or edits one.
//! - **The table** ([`store`]): V57's `schedules`, every change checked
//!   against its handler and logged on the `schedule` stream.
//! - **At run time** ([`service`]): one thread, run controls, missed runs
//!   by policy, recompute on launch, wake and a clock or zone change.
//! - **Idle** ([`idle`]): defined once, for `WhenIdle` work.
//!
//! Nothing runs outside the running app (§9): no launchd item, no
//! background task, no wake timer. What fell due while AI Juggler was
//! closed runs under its missed-run policy on the next launch.

pub mod commands;
pub mod idle;
pub mod registry;
pub mod remind_draft;
pub mod reminder;
pub mod service;
pub mod store;
pub mod trigger;
pub mod zone;

use std::sync::Arc;

pub use service::Scheduler;

use crate::state::AppState;

/// Every handler this build registers (ADR-0019: each module's own).
#[must_use]
pub fn handlers() -> registry::Registry {
    let mut registry = registry::Registry::default();
    let _ = registry.register(Arc::new(reminder::Reminder));
    registry
}

/// A transient schedule came due (in memory only, [`Scheduler::set_transient`]):
/// its owner's call.
pub fn transient(state: &Arc<AppState>, key: &'static str) {
    let _ = (state, key);
}

/// Starts the scheduler (once, at launch).
pub fn start(state: &Arc<AppState>) {
    service::start(state, handlers());
}

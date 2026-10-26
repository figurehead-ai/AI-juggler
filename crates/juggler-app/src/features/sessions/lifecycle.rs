//! The lifecycle forwarder: what the supervisor says of each session it
//! runs (started, exited, a helper left behind) becomes the session's
//! stored status, with what follows from it: AJ-E405's exit
//! classification (a rate-limit signature is
//! [`SessionStatus::QuotaBlocked`](juggler_core::model::SessionStatus),
//! not a crash), AJ-E410's snapshot of what it left running, AJ-E1510's
//! notification and AJ-E1509's restart.
//!
//! One thread reads the events for as long as AI Juggler runs. A quiet
//! spell never ends it: it ended after its first quiet half-second until
//! 0.1.7 (2026-10-26 lab), and every exit after that went unrecorded.

use std::sync::Arc;
use std::time::Duration;

use juggler_supervisor::{ExitKind, LifecycleEvent};

use crate::state::AppState;

/// How long one wait for an event lasts before it waits again.
const WAIT: Duration = Duration::from_millis(500);

/// Starts the forwarder: every lifecycle event goes through [`forward`],
/// for as long as the app runs.
pub fn spawn_forwarder(state: Arc<AppState>) {
    // None is a quiet spell, not an end: wait again. An event whose
    // handling panics is that event's loss only (`ticks::once`).
    crate::ticks::every_from_now("lifecycle", Duration::ZERO, move || {
        if let Some(event) = state.supervisor.recv_timeout(WAIT) {
            forward(&state, event);
        }
    });
}

/// One lifecycle event, stored with everything that follows from it.
pub fn forward(state: &Arc<AppState>, event: LifecycleEvent) {
    let stopped_by_user = state.supervisor.was_stopped(event.session());
    // AJ-E405: classify non-zero exits against the captured
    // output tail BEFORE the status mapping runs — a rate-limit
    // signature is QuotaBlocked (immediate), not a crash.
    let quota_session = event.session().clone();
    let quota_blocked = match &event {
        LifecycleEvent::Exited { kind, session, .. } => {
            if stopped_by_user || kind.is_success() {
                None
            } else if let ExitKind::Code(code) = kind {
                let tail = state
                    .supervisor
                    .output_tail(session, 4096)
                    .unwrap_or_default();
                state
                    .heuristics
                    .classify_exit(*code, &tail, crate::commands::now_millis(), None)
            } else {
                None
            }
        }
        LifecycleEvent::Started { .. } | LifecycleEvent::HelperOrphaned { .. } => None,
    };
    // AJ-E1509: how it ended, for the restarter (after the status
    // is stored, with storage free again).
    let ending = match &event {
        LifecycleEvent::Exited { kind, session, pid } => {
            // AJ-E410: freeze the pre-exit descendant rows into the
            // orphan tracker + journal the durable allow-list BEFORE
            // the status mapping consumes the event.
            crate::commands::record_exit_snapshot(state, session, *pid, kind);
            let tail = if kind.is_success() {
                String::new()
            } else {
                state
                    .supervisor
                    .output_tail(session, 4096)
                    .unwrap_or_default()
            };
            Some(super::restart::Ending::of(
                kind,
                stopped_by_user,
                quota_blocked.is_some(),
                &tail,
            ))
        }
        _ => None,
    };
    let mut now = None;
    if let Ok(storage) = state.storage.lock() {
        now = crate::commands::apply_lifecycle_event(
            &storage,
            event,
            stopped_by_user,
            quota_blocked.as_ref().map(|transition| &transition.reason),
        );
        // A quota exit ends the process — forget debounce.
        if quota_blocked.is_some() {
            state.heuristics.forget(&quota_session);
        }
    }
    // AJ-E1510: finished or at its limit, as a notification when
    // asked for.
    if let Some(record) = now {
        crate::features::notifications::session_status(
            state,
            record.id.as_str(),
            record.status.as_str(),
        );
    }
    if let Some(ending) = ending {
        super::restart::after_exit(state, &quota_session, ending);
    }
}

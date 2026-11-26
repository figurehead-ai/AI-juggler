//! The app's own schedules (ADR-0039 §11), each kept in step by its
//! owner at start ([`ensure_all`]) and shown read-only in Settings ›
//! Advanced:
//!
//! - **ADR-0037's maintenance** (`WhenIdle`): the memory store's and Bot's
//!   world model's upkeep, gyrus's idle pass ("dreaming") among it, at the
//!   next idle moment an hour after the last (at most an hour later still),
//!   in bounded batches that stop when an AI job arrives
//!   ([`crate::features::ai::world::maintain::pass`]).

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use super::registry::{Outcome, Priority, Registry, RunCtx, ScheduleHandler};
use super::store::{self, CreatedBy, NewSchedule};
use super::trigger::{MissedPolicy, Modifiers, Spec, Trigger};
use crate::state::AppState;

/// The owner of the app's upkeep schedules.
pub const OWNER: &str = "upkeep";
/// The memory store's maintenance.
pub const MAINTAIN: &str = "memory.maintain";
/// An hour.
const HOUR_MS: i64 = 3_600_000;

fn empty(payload: &Value) -> Result<(), &'static str> {
    if payload.as_object().is_some_and(serde_json::Map::is_empty) {
        Ok(())
    } else {
        Err("payload")
    }
}

/// ADR-0037 §2.3's maintenance, Bot's world model's upkeep and its idle
/// pass among it.
pub struct Maintain;

impl ScheduleHandler for Maintain {
    fn id(&self) -> &'static str {
        MAINTAIN
    }
    fn owner(&self) -> &'static str {
        OWNER
    }
    fn timeout(&self) -> Duration {
        Duration::from_mins(10)
    }
    fn priority(&self) -> Priority {
        Priority::Idle
    }
    fn validate(&self, payload: &Value) -> Result<(), &'static str> {
        empty(payload)
    }
    fn run(&self, _payload: &Value, ctx: &RunCtx<'_>) -> Outcome {
        let _ = crate::features::ai::world::maintain::pass(ctx.state, ctx.busy);
        Outcome::Done
    }
}

/// The upkeep handlers.
pub fn register(registry: &mut Registry) {
    let own: [Arc<dyn ScheduleHandler>; 1] = [Arc::new(Maintain)];
    for handler in own {
        let _ = registry.register(handler);
    }
}

/// The app's own schedules as their owners want them.
#[must_use]
pub fn wanted() -> Vec<NewSchedule> {
    let app = |handler, spec, title: &str| NewSchedule {
        owner: OWNER,
        handler,
        payload: json!({}),
        spec,
        user_visible: false,
        created_by: CreatedBy::App,
        title: title.to_owned(),
    };
    vec![app(
        MAINTAIN,
        Spec {
            trigger: Trigger::WhenIdle {
                max_wait_ms: HOUR_MS,
                after_ms: HOUR_MS,
            },
            modifiers: Modifiers::always(),
            missed: MissedPolicy::RunOnce,
        },
        "Tidy what Bot remembers",
    )]
}

/// Makes or keeps each app schedule (at start).
pub fn ensure_all(state: &Arc<AppState>, registry: &Registry) {
    let Ok(storage) = state.storage.lock() else {
        return;
    };
    let ctx = super::service::context(&storage);
    let now = crate::commands::now_millis();
    for new in wanted() {
        if let Err(error) = store::ensure_app(&storage, registry, new, &ctx, now) {
            tracing::debug!(%error, "schedule: an app schedule couldn't be kept");
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]

    use super::*;

    #[test]
    fn the_app_schedules_are_kept_once_each_and_read_only_for_the_user() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::seven_sessions(dir.path());
        let mut registry = Registry::default();
        register(&mut registry);
        ensure_all(&state, &registry);
        ensure_all(&state, &registry);
        let storage = state.storage.lock().unwrap();
        for handler in [MAINTAIN] {
            let rows = storage.schedules().of_handler(OWNER, handler).unwrap();
            assert_eq!(rows.len(), 1, "{handler}: one, kept in step");
            assert!(!rows[0].user_visible);
            assert_eq!(rows[0].created_by, "app");
            assert!(rows[0].next_due_ms.is_some());
        }
        assert_eq!(
            store::pause_users(
                &storage,
                &storage.schedules().of_handler(OWNER, MAINTAIN).unwrap()[0].id,
                true,
                0
            ),
            Err(store::ScheduleError::NotYours)
        );
    }

    #[test]
    fn maintenance_waits_for_idle_an_hour_after_its_last_run() {
        let maintain = wanted()
            .into_iter()
            .find(|n| n.handler == MAINTAIN)
            .unwrap();
        assert_eq!(
            maintain.spec.trigger,
            Trigger::WhenIdle {
                max_wait_ms: HOUR_MS,
                after_ms: HOUR_MS
            }
        );
        assert_eq!(maintain.spec.validate(), Ok(()));
        assert_eq!(Maintain.priority(), Priority::Idle);
        assert!(!Maintain.user_made());
    }
}

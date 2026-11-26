//! The app's own schedules (ADR-0039 §11), each kept in step by its
//! owner at start ([`ensure_all`]) and shown read-only in Settings ›
//! Advanced:
//!
//! - **ADR-0037's maintenance** (`WhenIdle`): the memory store's and Bot's
//!   world model's upkeep, gyrus's idle pass ("dreaming") among it, at the
//!   next idle moment an hour after the last (at most an hour later still),
//!   in bounded batches that stop when an AI job arrives
//!   ([`crate::features::ai::world::maintain::pass`]).
//! - **Cloud backup** (`Every` hour, on the wall clock's hour; `RunOnce`):
//!   the same check as before, which backs up only when Cloud backup is on
//!   and a backup is due by the user's own schedule.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use super::registry::{Outcome, Priority, Registry, RunCtx, ScheduleHandler};
use super::store::{self, CreatedBy, NewSchedule};
use super::trigger::{self, MissedPolicy, Modifiers, Spec, Trigger};
use crate::state::AppState;

/// The owner of the app's upkeep schedules.
pub const OWNER: &str = "upkeep";
/// The memory store's maintenance.
pub const MAINTAIN: &str = "memory.maintain";
/// Cloud backup's hourly check.
pub const BACKUP: &str = "cloud_backup";
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

/// Cloud backup's hourly check (AJ-E1205).
pub struct Backup;

impl ScheduleHandler for Backup {
    fn id(&self) -> &'static str {
        BACKUP
    }
    fn owner(&self) -> &'static str {
        OWNER
    }
    fn timeout(&self) -> Duration {
        Duration::from_mins(15)
    }
    fn priority(&self) -> Priority {
        Priority::App
    }
    fn validate(&self, payload: &Value) -> Result<(), &'static str> {
        empty(payload)
    }
    fn run(&self, _payload: &Value, ctx: &RunCtx<'_>) -> Outcome {
        backup_check(ctx.state, &crate::edition::current().vault_keychain())
    }
}

/// The hourly backup check, unchanged from the thread it replaces: the
/// tick decides (Cloud backup on, the user's daily or weekly schedule
/// due, a failure retried an hour on), so the run is done either way.
pub fn backup_check(state: &Arc<AppState>, vault: &dyn juggler_account::SecretStore) -> Outcome {
    let _ = crate::cloud_backup::cloud_backup_tick(state, vault);
    Outcome::Done
}

/// The upkeep handlers.
pub fn register(registry: &mut Registry) {
    let own: [Arc<dyn ScheduleHandler>; 2] = [Arc::new(Maintain), Arc::new(Backup)];
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
    vec![
        app(
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
        ),
        app(
            BACKUP,
            Spec {
                trigger: Trigger::Every {
                    interval_ms: HOUR_MS,
                    anchor_ms: 0,
                },
                modifiers: Modifiers {
                    jitter_ms: 5 * trigger::MINUTE_MS,
                    ..Modifiers::always()
                },
                missed: MissedPolicy::RunOnce,
            },
            "Check whether a cloud backup is due",
        ),
    ]
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

    use jiff::tz::TimeZone;

    use super::*;
    use crate::features::schedule::trigger::{Context, Place, next};

    #[test]
    fn the_app_schedules_are_kept_once_each_and_read_only_for_the_user() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::seven_sessions(dir.path());
        let mut registry = Registry::default();
        register(&mut registry);
        ensure_all(&state, &registry);
        ensure_all(&state, &registry);
        let storage = state.storage.lock().unwrap();
        for handler in [MAINTAIN, BACKUP] {
            let rows = storage.schedules().of_handler(OWNER, handler).unwrap();
            assert_eq!(rows.len(), 1, "{handler}: one, kept in step");
            assert!(!rows[0].user_visible);
            assert_eq!(rows[0].created_by, "app");
            assert!(rows[0].next_due_ms.is_some());
        }
        assert_eq!(
            store::pause_users(
                &storage,
                &storage.schedules().of_handler(OWNER, BACKUP).unwrap()[0].id,
                true,
                0
            ),
            Err(store::ScheduleError::NotYours)
        );
    }

    #[test]
    fn the_backup_check_is_hourly_on_the_wall_clock_not_from_launch() {
        let backup = wanted().into_iter().find(|n| n.handler == BACKUP).unwrap();
        let ctx = Context::in_zone(TimeZone::UTC);
        // Launched at 10:47:13: due by 11:05 (the hour, then up to five
        // minutes' jitter), not at 11:47 as the old thread's sleep was.
        let launched = 1_800_000_000_000 - 1_800_000_000_000 % HOUR_MS + 47 * 60_000 + 13_000;
        let n = next(
            &backup.spec,
            Place {
                base_ms: launched,
                armed_ms: None,
            },
            &ctx,
            7,
        )
        .unwrap();
        assert_eq!(n.occurrence_ms % HOUR_MS, 0);
        assert!(n.due_ms - n.occurrence_ms <= 5 * 60_000);
        assert!(n.occurrence_ms - launched < HOUR_MS);
        assert_eq!(
            backup.spec.missed,
            MissedPolicy::RunOnce,
            "asleep or closed: one check on wake"
        );
    }

    #[test]
    fn the_backup_check_does_what_the_thread_did_nothing_while_cloud_backup_is_off() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::seven_sessions(dir.path());
        let vault = juggler_account::MemoryStore::default();
        // Off (no schedule chosen): the tick does nothing, and neither does
        // the scheduled check that calls it.
        assert!(!crate::cloud_backup::cloud_backup_tick(&state, &vault));
        assert_eq!(backup_check(&state, &vault), Outcome::Done);
        assert!(
            crate::cloud_backup::cloud_backup_status_impl(&state, &vault)
                .last
                .is_none()
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
        assert!(!Maintain.user_made() && !Backup.user_made());
    }
}

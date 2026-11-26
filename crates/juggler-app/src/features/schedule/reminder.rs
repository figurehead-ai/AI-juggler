//! `RemindAt` (ADR-0039 §11.1): a reminder is an `At` schedule with this
//! handler. Its data is only what it is about (a session, a pool, a
//! project, an Account or an App, by id) or nothing; its words are the
//! title the user saw when they set it. When it comes due it is told
//! through the bell and the system notifications
//! ([`crate::features::notifications::watch_alerts::reminded`]); one that
//! came due while AI Juggler was closed or the Mac slept is told late,
//! once, and says so.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::registry::{Outcome, Priority, RunCtx, ScheduleHandler};
use crate::features::watches::model::WatchTarget;

/// The handler's id.
pub const HANDLER: &str = "reminder";
/// The module that owns reminders.
pub const OWNER: &str = "reminders";
/// The longest id a reminder names.
const ID_MAX: usize = 128;

/// A reminder's data.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ReminderPayload {
    /// What it is about; none for a plain reminder.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub about: Option<WatchTarget>,
}

impl ReminderPayload {
    /// Checks the data.
    ///
    /// # Errors
    ///
    /// `about` when it names "anything" or an id out of bounds.
    pub fn check(&self) -> Result<(), &'static str> {
        match &self.about {
            None => Ok(()),
            Some(WatchTarget::Any) => Err("about"),
            Some(target) => match target.id() {
                Some(id) if !id.is_empty() && id.len() <= ID_MAX => Ok(()),
                _ => Err("about"),
            },
        }
    }
}

/// The reminder handler.
pub struct Reminder;

impl ScheduleHandler for Reminder {
    fn id(&self) -> &'static str {
        HANDLER
    }
    fn owner(&self) -> &'static str {
        OWNER
    }
    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
    fn priority(&self) -> Priority {
        Priority::User
    }
    fn user_made(&self) -> bool {
        true
    }
    fn validate(&self, payload: &Value) -> Result<(), &'static str> {
        serde_json::from_value::<ReminderPayload>(payload.clone())
            .map_err(|_| "payload")?
            .check()
    }
    fn run(&self, payload: &Value, ctx: &RunCtx<'_>) -> Outcome {
        let Ok(data) = serde_json::from_value::<ReminderPayload>(payload.clone()) else {
            return Outcome::Failed {
                retry: false,
                code: "payload",
            };
        };
        crate::features::notifications::watch_alerts::reminded(
            ctx.state,
            ctx.id,
            ctx.due_ms,
            ctx.title,
            data.about.as_ref(),
            ctx.late,
        );
        Outcome::Done
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_reminder_carries_only_what_it_is_about() {
        let r = Reminder;
        assert_eq!(r.validate(&json!({})), Ok(()));
        assert_eq!(
            r.validate(&json!({ "about": { "kind": "session", "id": "s1" } })),
            Ok(())
        );
        assert_eq!(
            r.validate(&json!({ "about": { "kind": "any" } })),
            Err("about")
        );
        assert_eq!(
            r.validate(&json!({ "about": { "kind": "session", "id": "" } })),
            Err("about")
        );
        assert_eq!(
            r.validate(&json!({ "run": "rm -rf /" })),
            Err("payload"),
            "no other field"
        );
        assert!(r.user_made());
        assert_eq!(r.priority(), Priority::User);
    }
}

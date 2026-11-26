//! Typed handlers, never code (ADR-0039 §4). A module registers its
//! handlers at start (ADR-0019): an id, the module that owns it, a
//! payload validator, a timeout, a priority, and whether the user's own
//! schedules may use it. A schedule names a handler by id and carries
//! data; only a registered handler runs, and its validator checks the
//! data before the schedule is saved and again before each run.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::state::AppState;

/// Which runs go first when several are due (§5): what the user asked to
/// be told, then the app's jobs, then idle work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// A reminder the user set.
    User,
    /// The app's own jobs.
    App,
    /// Idle work.
    Idle,
}

/// How a run went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It did its work.
    Done,
    /// It failed: `retry` when it may succeed if tried again soon.
    Failed {
        /// Try again with backoff.
        retry: bool,
        /// Why, a code for the log.
        code: &'static str,
    },
}

impl Outcome {
    /// The stored code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Done => "ok",
            Self::Failed { .. } => "failed",
        }
    }
}

/// What a run knows.
pub struct RunCtx<'a> {
    /// The app.
    pub state: &'a Arc<AppState>,
    /// The schedule's id.
    pub id: &'a str,
    /// Its title, plain words.
    pub title: &'a str,
    /// When it was due.
    pub due_ms: i64,
    /// Now.
    pub now_ms: i64,
    /// Runs past their due time (asleep, closed, a clock change).
    pub late: bool,
    /// Occurrences missed before this one.
    pub missed: u32,
    /// An AI job arrived: idle work stops at its next batch.
    pub busy: &'a (dyn Fn() -> bool + Sync),
}

/// A registered handler.
pub trait ScheduleHandler: Send + Sync {
    /// Its id, a code (`reminder`, `cloud_backup`).
    fn id(&self) -> &'static str;
    /// The module that owns it; it alone changes its app-made schedules.
    fn owner(&self) -> &'static str;
    /// A run past this is abandoned and counted a failure.
    fn timeout(&self) -> Duration;
    /// Its default priority.
    fn priority(&self) -> Priority;
    /// The user's own schedules may use it.
    fn user_made(&self) -> bool {
        false
    }
    /// Checks a payload: the code of what is wrong.
    ///
    /// # Errors
    ///
    /// The payload doesn't fit this handler.
    fn validate(&self, payload: &Value) -> Result<(), &'static str>;
    /// Runs once.
    fn run(&self, payload: &Value, ctx: &RunCtx<'_>) -> Outcome;
}

/// The registered handlers.
#[derive(Default)]
pub struct Registry {
    handlers: BTreeMap<&'static str, Arc<dyn ScheduleHandler>>,
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.handlers.keys()).finish()
    }
}

impl Registry {
    /// Adds a handler.
    ///
    /// # Errors
    ///
    /// `taken` when its id is already registered.
    pub fn register(&mut self, handler: Arc<dyn ScheduleHandler>) -> Result<(), &'static str> {
        let id = handler.id();
        if !juggler_core::storage::memory::code_like(id, 64) {
            return Err("id");
        }
        if self.handlers.contains_key(id) {
            return Err("taken");
        }
        self.handlers.insert(id, handler);
        Ok(())
    }

    /// The handler with this id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Arc<dyn ScheduleHandler>> {
        self.handlers.get(id)
    }

    /// The registered ids.
    pub fn ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.handlers.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(&'static str);
    impl ScheduleHandler for Fake {
        fn id(&self) -> &'static str {
            self.0
        }
        fn owner(&self) -> &'static str {
            "test"
        }
        fn timeout(&self) -> Duration {
            Duration::from_secs(1)
        }
        fn priority(&self) -> Priority {
            Priority::App
        }
        fn validate(&self, _payload: &Value) -> Result<(), &'static str> {
            Ok(())
        }
        fn run(&self, _payload: &Value, _ctx: &RunCtx<'_>) -> Outcome {
            Outcome::Done
        }
    }

    #[test]
    fn a_handler_registers_once_by_a_code_id() {
        let mut registry = Registry::default();
        assert_eq!(registry.register(Arc::new(Fake("a.b"))), Ok(()));
        assert_eq!(registry.register(Arc::new(Fake("a.b"))), Err("taken"));
        assert_eq!(registry.register(Arc::new(Fake("run this"))), Err("id"));
        assert!(registry.get("a.b").is_some());
        assert!(registry.get("rm -rf").is_none());
        assert_eq!(registry.ids().collect::<Vec<_>>(), ["a.b"]);
        assert!(Priority::User < Priority::App && Priority::App < Priority::Idle);
    }
}

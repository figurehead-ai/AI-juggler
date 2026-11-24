//! The IPC edge's shared mechanics (ENGINEERING-GUIDE T1): a command that
//! touches SQLite, files, the network or processes is `async` and runs
//! that work off the main thread through [`blocking`].

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// AJ-E1552: how long a command may take before the app log says so. A
/// screen waits on most of them (its skeleton shows until they answer, and
/// gives up at 12 s), so a slow one is named with its time, never silent.
pub const BUDGET: Duration = Duration::from_secs(2);

/// Whether a command that took `took` went over [`BUDGET`].
#[must_use]
pub fn over_budget(took: Duration) -> bool {
    took > BUDGET
}

/// What a command's time is measured against (0.5.1 F30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// The machine's work only: past [`BUDGET`] is slow and logged.
    Machine,
    /// The call waits for the person (a priming sheet, the system's
    /// dialog after it): their time is theirs, never a slow call.
    Person,
}

/// The app log's line for a command that took `took`, if any: one over
/// [`BUDGET`] that waited only on the machine.
#[must_use]
pub fn budget_line(context: &str, took: Duration, wait: Wait) -> Option<String> {
    (wait == Wait::Machine && over_budget(took)).then(|| {
        format!(
            "{context}: took {} ms, over its {} ms budget",
            took.as_millis(),
            BUDGET.as_millis()
        )
    })
}

/// Runs `body` on the blocking pool with the app state; a task that
/// could not finish (it panicked or was cancelled) is an `Io` error named
/// by `context`, never a silent default.
///
/// # Errors
///
/// Whatever `body` returns, or [`AppError::Io`] when the task fails.
pub async fn blocking<T: Send + 'static>(
    state: &Arc<AppState>,
    context: &'static str,
    body: impl FnOnce(&Arc<AppState>) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    blocking_with(state, context, Wait::Machine, body).await
}

/// [`blocking`] with what its time is measured against: a call that
/// waits for the person says [`Wait::Person`] at the call (0.5.1 F30: a
/// priming sheet's 24 s wait for Not now was logged as a slow call), and
/// its time is never held against [`BUDGET`].
///
/// # Errors
///
/// As [`blocking`].
pub async fn blocking_with<T: Send + 'static>(
    state: &Arc<AppState>,
    context: &'static str,
    wait: Wait,
    body: impl FnOnce(&Arc<AppState>) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let state = Arc::clone(state);
    // AJ-E2013: what a command logs carries its span (`command>…`), and a
    // command that fails says so in the app log.
    let span = tracing::info_span!("command", what = context);
    tauri::async_runtime::spawn_blocking(move || {
        let _entered = span.enter();
        let started = Instant::now();
        let result = body(&state);
        if let Some(line) = budget_line(context, started.elapsed(), wait) {
            tracing::warn!("{line}");
        }
        if let Err(error) = &result {
            tracing::warn!("{context}: {error}");
        }
        result
    })
    .await
    .map_err(|error| AppError::Io {
        context,
        message: error.to_string(),
    })?
}

/// [`blocking`] for a list read under the storage lock at one "now" (ms):
/// a storage that can't be locked gives an empty list, never an error.
///
/// # Errors
///
/// [`AppError::Io`] only when the task fails.
pub async fn blocking_list<T: Send + 'static>(
    state: &Arc<AppState>,
    context: &'static str,
    body: impl FnOnce(&Arc<AppState>, &juggler_core::storage::Storage, i64) -> Vec<T> + Send + 'static,
) -> AppResult<Vec<T>> {
    blocking(state, context, move |state| {
        let Ok(storage) = state.storage.lock() else {
            return Ok(Vec::new());
        };
        let now = juggler_core::model::Timestamp::now().millis();
        Ok(body(state, &storage, now))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AJ-E1552: a command past its budget is logged; one within it isn't.
    #[test]
    fn a_command_over_its_budget_is_named() {
        assert!(!over_budget(Duration::from_millis(40)));
        assert!(!over_budget(BUDGET));
        assert!(over_budget(BUDGET + Duration::from_millis(1)));
    }

    /// 0.5.1 F30: a call marked as waiting for the person is never a slow
    /// call, however long they take; the machine's own work still is.
    #[test]
    fn a_wait_for_the_person_is_not_a_slow_call() {
        let long = Duration::from_millis(24_161);
        assert_eq!(budget_line("os prompt request", long, Wait::Person), None);
        assert_eq!(
            budget_line("os prompt request", long, Wait::Machine).as_deref(),
            Some("os prompt request: took 24161 ms, over its 2000 ms budget")
        );
        assert_eq!(
            budget_line("the pools", Duration::from_millis(40), Wait::Machine),
            None
        );
    }
}

//! AJ-E2302..E2308 (D-40, ADR-0022): how the sessions AI Juggler starts
//! run, and what their runtime says.
//!
//! - **Which runtime** ([`settings`]): the session's choice, else the
//!   profile's, else the tool's default; a Beta only when picked for that
//!   session with its switch on. Data from the adapter, never a tool id.
//! - **Starting** ([`launch`]): a driven session is a supervisor child like
//!   any other (piped, or serving a socket in `runtime/`), connected through
//!   its adapter's codec; anything that fails starts the terminal instead,
//!   with the reason.
//! - **Driving** ([`driver`]): a thread per driven session keeps a snapshot
//!   for the UI and the stored state exact, and carries the user's own
//!   commands. Messages are held in memory to show them and never stored.
//! - **Answers are the user's** (D-28 as amended by D-40): only
//!   [`commands::runtime_answer`], run by the user's click, reaches
//!   `Driver::answer` (`tests/runtime_guardrails.rs` checks the source).

pub mod commands;
pub mod driver;
pub mod dto;
pub mod launch;
pub mod pickup;
pub mod reports;
pub mod settings;
pub mod trust;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use juggler_core::model::SessionId;

use driver::Driver;
use dto::RuntimeInfo;

/// Tells the UI a session's runtime changed, and whether its stored state
/// did too (the app sets it at start; tests leave it unset).
pub type Notifier = Arc<dyn Fn(&str, bool) + Send + Sync>;

/// Event: driven sessions' runtimes changed (payload: their ids).
pub const EVENT_CHANGED: &str = "runtime://changed";

/// The runtimes of the sessions AI Juggler started.
#[derive(Default)]
pub struct Runtimes {
    drivers: Mutex<HashMap<String, Arc<Driver>>>,
    /// Why a session runs in the terminal although driven was chosen.
    fallbacks: Mutex<HashMap<String, String>>,
    /// What terminal sessions' hooks said last (AJ-E2306).
    observed: Mutex<HashMap<String, reports::Observed>>,
    /// Driven sessions AI Juggler stopped during a turn (AJ-E2310).
    cut_off: Mutex<HashMap<String, pickup::CutOff>>,
    notifier: Mutex<Option<Notifier>>,
}

impl std::fmt::Debug for Runtimes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtimes").finish_non_exhaustive()
    }
}

impl Runtimes {
    /// Sets what tells the UI.
    pub fn set_notifier(&self, notifier: Notifier) {
        if let Ok(mut slot) = self.notifier.lock() {
            *slot = Some(notifier);
        }
    }

    /// Tells the UI `session` changed (`stored`: its stored state too).
    pub fn notify(&self, session: &str, stored: bool) {
        let notifier = self.notifier.lock().ok().and_then(|slot| slot.clone());
        if let Some(notifier) = notifier {
            notifier(session, stored);
        }
    }

    /// Holds a new driver.
    pub fn insert(&self, driver: Arc<Driver>) {
        if let Ok(mut drivers) = self.drivers.lock() {
            drivers.insert(driver.session.to_string(), driver);
        }
    }

    /// The driver of `session`, when it is driven.
    #[must_use]
    pub fn driver(&self, session: &str) -> Option<Arc<Driver>> {
        self.drivers.lock().ok()?.get(session).cloned()
    }

    /// Whether `session` is driven.
    #[must_use]
    pub fn is_driven(&self, session: &SessionId) -> bool {
        self.driver(session.as_str()).is_some()
    }

    /// Whether `session`'s state is its tool's own word, never a guess: it
    /// is driven, or its hooks report.
    #[must_use]
    pub fn is_exact(&self, session: &SessionId) -> bool {
        self.is_driven(session)
            || self.is_cut_off(session.as_str())
            || self
                .observed
                .lock()
                .is_ok_and(|observed| observed.contains_key(session.as_str()))
    }

    /// Notes that AI Juggler stopped during `session`'s turn (AJ-E2310).
    pub fn note_cut_off(&self, session: &SessionId, how: pickup::CutOff) {
        if let Ok(mut cut) = self.cut_off.lock() {
            cut.insert(session.to_string(), how);
        }
    }

    /// Whether `session`'s turn was cut off and waits for Resume.
    #[must_use]
    pub fn is_cut_off(&self, session: &str) -> bool {
        self.cut_off
            .lock()
            .is_ok_and(|cut| cut.contains_key(session))
    }

    /// `session` is resumed or ended: no longer cut off.
    pub fn clear_cut_off(&self, session: &str) {
        if let Ok(mut cut) = self.cut_off.lock() {
            cut.remove(session);
        }
    }

    /// Takes in a hook's report for a terminal session: its exact state,
    /// the stored state, and its conversation's link.
    pub fn observe(
        &self,
        state: &Arc<AppState>,
        session: &SessionId,
        hook: &juggler_agentbridge::wire::HookReport,
    ) {
        let now = {
            let Ok(mut observed) = self.observed.lock() else {
                return;
            };
            let next = reports::next(session, observed.get(session.as_str()), hook);
            observed.insert(session.to_string(), next.clone());
            next
        };
        if let Some(native_id) = &hook.tool_session {
            driver::link(state, session, native_id, hook.transcript.as_deref());
        }
        driver::store_ask(state, session, now.ask.as_ref(), now.state == "ended");
    }

    /// Every driver, for the asks across sessions.
    #[must_use]
    pub fn drivers(&self) -> Vec<Arc<Driver>> {
        let mut all: Vec<Arc<Driver>> = self
            .drivers
            .lock()
            .map(|drivers| drivers.values().cloned().collect())
            .unwrap_or_default();
        all.sort_by(|a, b| a.session.as_str().cmp(b.session.as_str()));
        all
    }

    /// Notes why `session` runs in the terminal.
    pub fn fell_back(&self, session: &SessionId, why: String) {
        if let Ok(mut fallbacks) = self.fallbacks.lock() {
            fallbacks.insert(session.to_string(), why);
        }
    }

    /// Forgets a driver whose session ended.
    pub fn forget(&self, session: &str) {
        if let Ok(mut drivers) = self.drivers.lock() {
            drivers.remove(session);
        }
        if let Ok(mut fallbacks) = self.fallbacks.lock() {
            fallbacks.remove(session);
        }
        if let Ok(mut observed) = self.observed.lock() {
            observed.remove(session);
        }
        self.clear_cut_off(session);
    }

    /// The runtime of `session`, as the UI shows it; `None` for a session
    /// AI Juggler knows nothing more about than its process.
    #[must_use]
    pub fn info(&self, session: &str) -> Option<RuntimeInfo> {
        if let Some(driver) = self.driver(session) {
            return Some(driver.info());
        }
        if let Some(how) = self
            .cut_off
            .lock()
            .ok()
            .and_then(|c| c.get(session).copied())
        {
            return Some(RuntimeInfo {
                kind: "driven".into(),
                label: Some(how.label.to_owned()),
                beta: how.beta,
                cut_off: Some(pickup::CUT_OFF_WORDS.to_owned()),
                ..RuntimeInfo::default()
            });
        }
        let why = self
            .fallbacks
            .lock()
            .ok()
            .and_then(|f| f.get(session).cloned());
        let seen = self
            .observed
            .lock()
            .ok()
            .and_then(|o| o.get(session).cloned());
        if why.is_none() && seen.is_none() {
            return None;
        }
        Some(RuntimeInfo {
            kind: "terminal".into(),
            fell_back: why,
            state: seen.as_ref().map(|s| s.state.to_owned()),
            asks: seen
                .as_ref()
                .and_then(|s| s.ask.clone())
                .into_iter()
                .collect(),
            error: seen.and_then(|s| s.error),
            ..RuntimeInfo::default()
        })
    }
}

/// Tells the window what changed, at most every 150 ms: the runtimes'
/// ids on [`EVENT_CHANGED`], and the session list when a stored state
/// changed (an ask opened or closed). A burst of deltas is one emission.
pub fn start_emitter(state: &Arc<AppState>, app: tauri::AppHandle) {
    use tauri::Emitter;
    type Dirty = Arc<Mutex<(std::collections::BTreeSet<String>, bool)>>;
    let dirty: Dirty = Arc::default();
    let marks = Arc::clone(&dirty);
    state.runtime.set_notifier(Arc::new(move |session, stored| {
        if let Ok(mut dirty) = marks.lock() {
            dirty.0.insert(session.to_owned());
            dirty.1 |= stored;
        }
    }));
    let weak = Arc::downgrade(state);
    let _ = std::thread::Builder::new()
        .name("runtime-emitter".into())
        .spawn(move || {
            loop {
                // poll: coalesces a driven session's events for the window;
                // ends with the app.
                std::thread::sleep(std::time::Duration::from_millis(150));
                let Some(state) = weak.upgrade() else { return };
                let (ids, stored) = dirty.lock().map_or_else(
                    |_| (Vec::new(), false),
                    |mut d| {
                        (
                            std::mem::take(&mut d.0).into_iter().collect::<Vec<_>>(),
                            std::mem::take(&mut d.1),
                        )
                    },
                );
                if !ids.is_empty() {
                    let _ = app.emit(EVENT_CHANGED, ids);
                }
                if stored {
                    let _ = crate::commands::emit_sessions(&app, &state);
                }
            }
        });
}

/// AJ-E2305: lets a driven session go before a switch takes its
/// conversation: when its turn ends ("Finish this turn", at most `limit`)
/// or now ("Stop now", an interrupt), then its process ends. `false` for a
/// session that isn't driven (the switch freezes it as before).
///
/// # Errors
///
/// [`crate::error::AppError::Refused`] when the turn didn't end in time.
pub fn release_for_switch(
    state: &Arc<AppState>,
    session: &SessionId,
    finish_turn: bool,
    limit: std::time::Duration,
) -> crate::error::AppResult<bool> {
    use std::time::{Duration, Instant};
    let Some(driver) = state.runtime.driver(session.as_str()) else {
        return Ok(false);
    };
    let how = if finish_turn {
        juggler_adapter::runtime::Release::AfterTurn(limit)
    } else {
        juggler_adapter::runtime::Release::Now
    };
    let outcome = driver
        .release(how)
        .map_err(crate::error::AppError::Refused)?;
    if finish_turn && !outcome.at_boundary {
        return Err(crate::error::AppError::Refused(
            "The session is still working: choose Stop now, or try again once its turn is done"
                .to_owned(),
        ));
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while !state.supervisor.is_finished(session) && Instant::now() < deadline {
        // poll: the tool exits within 5 s of its input closing.
        std::thread::sleep(Duration::from_millis(20));
    }
    if !state.supervisor.is_finished(session) {
        let _ = state.supervisor.stop(session, Duration::from_secs(2));
    }
    state.runtime.forget(session.as_str());
    Ok(true)
}

/// Each row with its runtime (the session read model's last step).
#[must_use]
pub fn described(
    state: &Arc<AppState>,
    mut rows: Vec<crate::features::sessions::dto::InstanceInfo>,
) -> Vec<crate::features::sessions::dto::InstanceInfo> {
    for row in &mut rows {
        row.runtime = state.runtime.info(&row.id);
    }
    rows
}

use crate::state::AppState;

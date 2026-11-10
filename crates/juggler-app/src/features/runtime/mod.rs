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

use crate::events::{AppEvent, EmitEvent as _};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use juggler_core::model::SessionId;

use driver::Driver;
use dto::RuntimeInfo;

/// Tells the UI a session's runtime changed, and whether its stored state
/// did too (the app sets it at start; tests leave it unset).
pub type Notifier = Arc<dyn Fn(&str, bool) + Send + Sync>;

/// Arms a session's first-report timer (see [`reports::arm_first_reports`]):
/// what it returns cancels the timer when dropped.
pub type FirstReportTimer = Arc<dyn Fn(&SessionId) -> Option<reports::Cancel> + Send + Sync>;

/// A terminal session whose hooks will report, until its first report.
struct Awaiting {
    /// When it started (epoch ms).
    since: i64,
    /// Its one-shot timer at [`reports::FIRST_REPORT_MS`]: dropped (so
    /// cancelled) with the entry, by the first report, the process's end
    /// or the session's.
    _timer: Option<reports::Cancel>,
}

/// Event: driven sessions' runtimes changed (payload: their ids).
pub const EVENT_CHANGED: &str = AppEvent::RuntimeChanged.name();

/// The runtimes of the sessions AI Juggler started.
#[derive(Default)]
pub struct Runtimes {
    drivers: Mutex<HashMap<String, Arc<Driver>>>,
    /// Why a session runs in the terminal although driven was chosen.
    fallbacks: Mutex<HashMap<String, String>>,
    /// What terminal sessions' hooks said last (AJ-E2306).
    observed: Mutex<HashMap<String, reports::Observed>>,
    /// Terminal sessions whose hooks will report, until their first
    /// report.
    awaiting: Mutex<HashMap<String, Awaiting>>,
    /// What arms a first-report timer (the app sets it at start; tests
    /// set a clock of their own, or none: the 15 s safety net only).
    first_report_timer: Mutex<Option<FirstReportTimer>>,
    /// A session's stored state changed (an ask opened or closed): Needs
    /// you is worked out again at once, not on its 5 s tick.
    needs_changed: AtomicBool,
    /// Driven sessions AI Juggler stopped during a turn (AJ-E2310).
    cut_off: Mutex<HashMap<String, pickup::CutOff>>,
    /// Terminal sessions whose tool's folder screen was given the user's
    /// own answer (once each: `sessions::attention::screen_asks`).
    screen_answered: Mutex<std::collections::HashSet<String>>,
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

    /// Sets what arms a session's first-report timer.
    pub fn set_first_report_timer(&self, timer: FirstReportTimer) {
        if let Ok(mut slot) = self.first_report_timer.lock() {
            *slot = Some(timer);
        }
    }

    /// Whether `session`'s folder screen is yet to get the user's answer:
    /// `true` once, then `false` (the screen redraws before it moves on).
    pub fn first_screen_answer(&self, session: &SessionId) -> bool {
        self.screen_answered
            .lock()
            .is_ok_and(|mut answered| answered.insert(session.to_string()))
    }

    /// A session's stored state changed: Needs you follows at once.
    pub fn mark_needs_changed(&self) {
        self.needs_changed.store(true, Ordering::Release);
    }

    /// Whether Needs you is due to be worked out again (taken once).
    pub fn take_needs_changed(&self) -> bool {
        self.needs_changed.swap(false, Ordering::AcqRel)
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
    /// is driven, or its hooks report (or will: before the first report,
    /// see [`reports::FIRST_REPORT_MS`]).
    #[must_use]
    pub fn is_exact(&self, session: &SessionId) -> bool {
        self.is_driven(session)
            || self.is_cut_off(session.as_str())
            || self
                .observed
                .lock()
                .is_ok_and(|observed| observed.contains_key(session.as_str()))
            || self
                .awaiting
                .lock()
                .is_ok_and(|awaiting| awaiting.contains_key(session.as_str()))
    }

    /// Notes that `session`'s hooks will report, from `now_ms`: until the
    /// first report it reads Idle, never Working (the rc19 walk: a resume
    /// waiting in Claude's own Resume picker read Working). Its one-shot
    /// timer takes it in as waiting at [`reports::FIRST_REPORT_MS`] (the
    /// rc22 walk: on the 15 s tick alone, a folder-trust question showed
    /// in Needs you only about 40 s in).
    pub fn expect_reports(&self, session: &SessionId, now_ms: i64) {
        let arm = self.first_report_timer.lock().ok().and_then(|t| t.clone());
        let timer = arm.and_then(|arm| arm(session));
        if let Ok(mut awaiting) = self.awaiting.lock() {
            awaiting.insert(
                session.to_string(),
                Awaiting {
                    since: now_ms,
                    _timer: timer,
                },
            );
        }
    }

    /// `session`'s first-report timer ran out: taken in as waiting on the
    /// tool's own screen, when it is still awaited and nothing has
    /// reported.
    pub fn take_in_first_report(
        &self,
        session: &SessionId,
        now_ms: i64,
    ) -> Option<reports::Observed> {
        let awaited = self
            .awaiting
            .lock()
            .ok()?
            .remove(session.as_str())
            .is_some();
        if !awaited {
            return None;
        }
        let mut observed = self.observed.lock().ok()?;
        if observed.contains_key(session.as_str()) {
            return None;
        }
        let now = reports::first_report_overdue(session, now_ms);
        observed.insert(session.to_string(), now.clone());
        Some(now)
    }

    /// The sessions whose first report is overdue at `now_ms`, each taken
    /// in as waiting on the tool's own screen (see
    /// [`reports::first_report_overdue`]).
    pub fn take_in_overdue(&self, now_ms: i64) -> Vec<(SessionId, reports::Observed)> {
        let due: Vec<String> = {
            let Ok(mut awaiting) = self.awaiting.lock() else {
                return Vec::new();
            };
            let due: Vec<String> = awaiting
                .iter()
                .filter(|(_, awaited)| now_ms - awaited.since >= reports::FIRST_REPORT_MS)
                .map(|(session, _)| session.clone())
                .collect();
            for session in &due {
                awaiting.remove(session);
            }
            due
        };
        let Ok(mut observed) = self.observed.lock() else {
            return Vec::new();
        };
        let mut taken = Vec::new();
        for session in due {
            if observed.contains_key(&session) {
                continue;
            }
            let session = SessionId::new(session);
            let now = reports::first_report_overdue(&session, now_ms);
            observed.insert(session.to_string(), now.clone());
            taken.push((session, now));
        }
        taken
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
        if let Ok(mut awaiting) = self.awaiting.lock() {
            awaiting.remove(session.as_str());
        }
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

    /// `session`'s process has ended: what its hooks said and whether a
    /// first report is awaited are forgotten, so nothing it asked before it
    /// ended (the overdue stand-in included) outlives it. A cut-off turn
    /// and the driver stay theirs.
    pub fn process_ended(&self, session: &str) {
        if let Ok(mut observed) = self.observed.lock() {
            observed.remove(session);
        }
        if let Ok(mut answered) = self.screen_answered.lock() {
            answered.remove(session);
        }
        if let Ok(mut awaiting) = self.awaiting.lock() {
            awaiting.remove(session);
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
        if let Ok(mut awaiting) = self.awaiting.lock() {
            awaiting.remove(session);
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
        // Its hooks haven't reported yet: Idle until they do (or until
        // the first report is overdue), never the status's Working.
        let seen = self
            .observed
            .lock()
            .ok()
            .and_then(|o| o.get(session).cloned())
            .or_else(|| {
                self.awaiting
                    .lock()
                    .ok()
                    .filter(|awaiting| awaiting.contains_key(session))
                    .map(|_| reports::Observed {
                        state: "idle",
                        ask: None,
                        error: None,
                    })
            });
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
    // poll: coalesces a driven session's events for the window; ends with
    // the app.
    crate::ticks::every_while(
        "runtime-emitter",
        std::time::Duration::from_millis(150),
        move || {
            let Some(state) = weak.upgrade() else {
                return false;
            };
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
                app.emit_event(AppEvent::RuntimeChanged, ids);
            }
            if stored {
                let _ = crate::features::sessions::read_model::emit_sessions(&app, &state);
            }
            // An ask opened or closed: Needs you (the rail, the badge, the
            // menu-bar item) follows now, not on its 5 s tick (the rc22
            // walk: a cleared ask stayed about 6 s).
            if state.runtime.take_needs_changed() {
                crate::features::notifications::needs::refresh(&state);
            }
            true
        },
    );
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

/// The rc19 walk: a resume that sat in Claude Code's own Resume picker
/// read Working, because nothing had reported and the status guessed.
#[cfg(test)]
mod first_report_tests {
    use super::{Runtimes, reports};
    use crate::features::sessions::rows::Rank;
    use juggler_agentbridge::wire::HookReport;
    use juggler_core::model::{SessionId, SessionStatus};

    fn rank(runtimes: &Runtimes, session: &SessionId) -> Rank {
        let info = runtimes.info(session.as_str());
        Rank::of_session(
            SessionStatus::Running,
            info.as_ref().and_then(|info| info.state.as_deref()),
        )
    }

    #[test]
    fn a_session_whose_hooks_havent_reported_never_reads_working() {
        let runtimes = Runtimes::default();
        let session = SessionId::new("sess-picker");
        // Without reports, the status guesses (a tool with no hooks).
        assert_eq!(rank(&runtimes, &session), Rank::Working);
        runtimes.expect_reports(&session, 1_000);
        assert!(runtimes.is_exact(&session), "no heuristic guesses over it");
        assert_eq!(
            runtimes.info("sess-picker").unwrap().state.as_deref(),
            Some("idle")
        );
        assert_eq!(rank(&runtimes, &session), Rank::Open);
        // Not overdue yet.
        assert!(
            runtimes
                .take_in_overdue(1_000 + reports::FIRST_REPORT_MS - 1)
                .is_empty()
        );
        // Overdue: it waits on the tool's own screen, for the person.
        let taken = runtimes.take_in_overdue(1_000 + reports::FIRST_REPORT_MS);
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].1.state, "asks_you");
        let info = runtimes.info("sess-picker").unwrap();
        assert_eq!(info.state.as_deref(), Some("asks_you"));
        assert!(info.asks[0].summary.contains("its terminal"));
        assert_eq!(rank(&runtimes, &session), Rank::NeedsYou);
        // Taken in once.
        assert!(runtimes.take_in_overdue(i64::MAX).is_empty());
        // Once the tool opens its session, its own word: idle.
        let start = HookReport {
            event: "SessionStart".to_owned(),
            ..HookReport::default()
        };
        let before = runtimes
            .observed
            .lock()
            .unwrap()
            .get("sess-picker")
            .cloned();
        assert_eq!(
            reports::next(&session, before.as_ref(), &start).state,
            "idle"
        );
    }

    fn overdue() -> reports::Observed {
        reports::first_report_overdue(&SessionId::new("sess-x"), 5_000)
    }

    fn hook(event: &str, kind: Option<&str>) -> HookReport {
        HookReport {
            event: event.to_owned(),
            kind: kind.map(str::to_owned),
            ..HookReport::default()
        }
    }

    /// The rc20 walk: after the trust question was answered and the hooks
    /// reported, the overdue "Asks you" stayed. A report that keeps the
    /// state before (a Notification of another kind) or its ask (a
    /// permission prompt) never keeps the stand-in.
    #[test]
    fn a_report_never_keeps_the_overdue_stand_in() {
        let session = SessionId::new("sess-x");
        let before = overdue();
        assert!(before.is_first_report_overdue());
        let other = reports::next(
            &session,
            Some(&before),
            &hook("Notification", Some("auth_success")),
        );
        assert_eq!((other.state, other.ask.is_none()), ("idle", true));
        let none = reports::next(&session, Some(&before), &hook("Notification", None));
        assert_eq!((none.state, none.ask.is_none()), ("idle", true));
        // The tool's own later ask is its own, never the stand-in's words.
        let asks = reports::next(
            &session,
            Some(&before),
            &hook("Notification", Some("permission_prompt")),
        );
        assert_eq!(asks.state, "asks_you");
        assert!(!asks.is_first_report_overdue());
        // …and a real ask before is still kept, as it was.
        let kept = reports::next(
            &session,
            Some(&asks),
            &hook("Notification", Some("auth_success")),
        );
        assert_eq!(kept, asks);
    }

    fn seeded(dir: &std::path::Path, ids: &[&str]) -> std::sync::Arc<crate::state::AppState> {
        use juggler_core::model::{
            Account, AccountId, HarnessId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId,
            ProfileId, Timestamp,
        };
        let state = std::sync::Arc::new(crate::state::AppState::init(dir.join("data")).unwrap());
        {
            let storage = state.storage.lock().unwrap();
            storage
                .accounts()
                .insert(&Account {
                    id: AccountId::new("acc"),
                    harness: HarnessId::new("claude-code"),
                    label: "Lab".into(),
                    plan: None,
                    state_dir: dir.join("state"),
                    cleanup_opt_in: false,
                    renewal_date: None,
                    preview_opt_in: false,
                    import_source_path: None,
                    import_origin: None,
                    imported_at: None,
                    in_place: false,
                })
                .unwrap();
            storage
                .profiles()
                .insert(&LaunchRecipe {
                    id: ProfileId::new("rec"),
                    name: "Main".into(),
                    account: AccountId::new("acc"),
                    settings_overlay: juggler_core::model::SettingsOverlay::default(),
                    default_cwd: None,
                })
                .unwrap();
            for id in ids {
                storage
                    .sessions()
                    .insert(&InstanceRecord {
                        id: SessionId::new(*id),
                        conversation: None,
                        profile: ProfileId::new("rec"),
                        machine: MachineId::new("this-mac"),
                        kind: InstanceKind::Terminal,
                        project: None,
                        cwd: dir.to_path_buf(),
                        status: SessionStatus::Running,
                        started_at: Timestamp::now(),
                        last_activity: None,
                        usage: juggler_core::model::UsageSnapshot::default(),
                        attention: None,
                        running: None,
                        transcript_ref: None,
                        window: None,
                    })
                    .unwrap();
            }
        }
        state
    }

    fn status(state: &crate::state::AppState, id: &str) -> (SessionStatus, bool) {
        let record = state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .get(&SessionId::new(id))
            .unwrap();
        (record.status, record.attention.is_some())
    }

    /// rc20: the overdue ask, stored, ends with the first real report (of
    /// any kind), with the tool's own later ask, and with the process.
    #[test]
    fn the_overdue_ask_ends_with_the_first_report_and_with_the_process() {
        let dir = tempfile::tempdir().unwrap();
        let state = seeded(dir.path(), &["sess-a", "sess-b"]);
        for id in ["sess-a", "sess-b"] {
            state.runtime.expect_reports(&SessionId::new(id), 0);
        }
        reports::overdue_tick(&state, reports::FIRST_REPORT_MS);
        assert_eq!(status(&state, "sess-a"), (SessionStatus::WaitingUser, true));
        assert_eq!(status(&state, "sess-b"), (SessionStatus::WaitingUser, true));

        // A: its first report, a Notification of another kind.
        let a = SessionId::new("sess-a");
        state
            .runtime
            .observe(&state, &a, &hook("Notification", Some("auth_success")));
        assert_eq!(status(&state, "sess-a"), (SessionStatus::Running, false));
        assert_eq!(
            state.runtime.info("sess-a").unwrap().state.as_deref(),
            Some("idle")
        );
        // Its own later ask, then its answer: cleared again.
        state
            .runtime
            .observe(&state, &a, &hook("Notification", Some("permission_prompt")));
        assert_eq!(status(&state, "sess-a"), (SessionStatus::WaitingUser, true));
        state
            .runtime
            .observe(&state, &a, &hook("PostToolUse", None));
        assert_eq!(status(&state, "sess-a"), (SessionStatus::Running, false));

        // B: stopped while it waits: nothing of the ask outlives it.
        crate::features::sessions::lifecycle::forward(
            &state,
            juggler_supervisor::LifecycleEvent::Exited {
                session: SessionId::new("sess-b"),
                pid: 1,
                kind: juggler_supervisor::ExitKind::Code(0),
            },
        );
        assert_eq!(status(&state, "sess-b"), (SessionStatus::Finished, false));
        assert!(
            state.runtime.info("sess-b").is_none(),
            "no runtime state outlives the process"
        );
    }

    /// A timer the test clock holds: when, its guard, what it runs.
    type Pending = (i64, std::sync::Weak<()>, Box<dyn FnOnce(i64) + Send>);

    /// A clock the test moves: its timers run when it passes their time,
    /// unless their guard was dropped first.
    #[derive(Default)]
    struct ManualClock {
        now: std::sync::Mutex<i64>,
        timers: std::sync::Mutex<Vec<Pending>>,
    }

    impl reports::OneShot for ManualClock {
        fn after(
            &self,
            wait: std::time::Duration,
            due: Box<dyn FnOnce(i64) + Send>,
        ) -> Option<reports::Cancel> {
            let at = *self.now.lock().unwrap() + i64::try_from(wait.as_millis()).unwrap();
            let guard = std::sync::Arc::new(());
            self.timers
                .lock()
                .unwrap()
                .push((at, std::sync::Arc::downgrade(&guard), due));
            Some(reports::Cancel::new(guard))
        }
    }

    impl ManualClock {
        fn advance_to(&self, now: i64) {
            *self.now.lock().unwrap() = now;
            let due: Vec<_> = {
                let mut timers = self.timers.lock().unwrap();
                let (due, later) = std::mem::take(&mut *timers)
                    .into_iter()
                    .partition(|(at, _, _)| *at <= now);
                *timers = later;
                due
            };
            for (_, guard, run) in due {
                if guard.upgrade().is_some() {
                    run(now);
                }
            }
        }

        /// Timers neither run nor cancelled.
        fn armed(&self) -> usize {
            self.timers
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, guard, _)| guard.upgrade().is_some())
                .count()
        }
    }

    fn clocked(
        dir: &std::path::Path,
        ids: &[&str],
    ) -> (
        std::sync::Arc<crate::state::AppState>,
        std::sync::Arc<ManualClock>,
    ) {
        let state = seeded(dir, ids);
        let clock = std::sync::Arc::new(ManualClock::default());
        reports::arm_first_reports(&state, clock.clone());
        for id in ids {
            state.runtime.expect_reports(&SessionId::new(*id), 0);
        }
        (state, clock)
    }

    fn state_of(state: &crate::state::AppState, id: &str) -> Option<String> {
        state.runtime.info(id).and_then(|info| info.state)
    }

    /// The rc22 walk: a session at its folder-trust question showed in
    /// Needs you only about 40 s in (20 s, then the 15 s tick, then Needs
    /// you's own 5 s). Its own timer says so at 20 s, with no tick.
    #[test]
    fn a_silent_session_asks_at_twenty_seconds_with_no_tick() {
        let dir = tempfile::tempdir().unwrap();
        let (state, clock) = clocked(dir.path(), &["sess-trust"]);
        assert_eq!(clock.armed(), 1, "one timer per launch");
        clock.advance_to(reports::FIRST_REPORT_MS - 1);
        assert_eq!(
            status(&state, "sess-trust"),
            (SessionStatus::Running, false)
        );
        assert_eq!(state_of(&state, "sess-trust").as_deref(), Some("idle"));
        assert!(!state.runtime.take_needs_changed());
        clock.advance_to(reports::FIRST_REPORT_MS);
        assert_eq!(
            status(&state, "sess-trust"),
            (SessionStatus::WaitingUser, true),
            "Asks you at 20 s exactly"
        );
        assert_eq!(state_of(&state, "sess-trust").as_deref(), Some("asks_you"));
        assert!(
            state.runtime.take_needs_changed(),
            "Needs you is worked out again now, not on its tick"
        );
        assert_eq!(clock.armed(), 0);
        // The safety net finds nothing more to take in.
        assert!(state.runtime.take_in_overdue(i64::MAX).is_empty());
    }

    #[test]
    fn a_report_at_nineteen_seconds_means_it_never_asks() {
        let dir = tempfile::tempdir().unwrap();
        let (state, clock) = clocked(dir.path(), &["sess-quick"]);
        clock.advance_to(19_000);
        state.runtime.observe(
            &state,
            &SessionId::new("sess-quick"),
            &hook("SessionStart", None),
        );
        assert_eq!(clock.armed(), 0, "the first report cancels the timer");
        clock.advance_to(10 * reports::FIRST_REPORT_MS);
        assert_eq!(
            status(&state, "sess-quick"),
            (SessionStatus::Running, false)
        );
        assert_eq!(state_of(&state, "sess-quick").as_deref(), Some("idle"));
    }

    #[test]
    fn the_process_ending_cancels_the_timer() {
        let dir = tempfile::tempdir().unwrap();
        let (state, clock) = clocked(dir.path(), &["sess-quit"]);
        crate::features::sessions::lifecycle::forward(
            &state,
            juggler_supervisor::LifecycleEvent::Exited {
                session: SessionId::new("sess-quit"),
                pid: 1,
                kind: juggler_supervisor::ExitKind::Code(0),
            },
        );
        assert_eq!(clock.armed(), 0, "the process's end cancels the timer");
        clock.advance_to(10 * reports::FIRST_REPORT_MS);
        assert_eq!(
            status(&state, "sess-quit"),
            (SessionStatus::Finished, false)
        );
        assert!(state.runtime.info("sess-quit").is_none());
    }

    /// The rc27 P1 walk: a switch's Codex at its own "Trust this folder?"
    /// read Working. Its screen's question asks at once, in its words;
    /// answered (the screen moved on), it works again; a question already
    /// taken in isn't taken in twice.
    #[test]
    fn a_tools_screen_question_asks_at_once_and_its_answer_ends_it() {
        use crate::features::sessions::attention::apply_screen_ask;
        let dir = tempfile::tempdir().unwrap();
        let state = seeded(dir.path(), &["sess-codex"]);
        let id = SessionId::new("sess-codex");
        let storage = state.storage.lock().unwrap();
        assert_eq!(
            apply_screen_ask(&storage, &id, Some("Trust this folder?"), 5),
            Some(SessionStatus::WaitingUser)
        );
        let record = storage.sessions().get(&id).unwrap();
        let reason = record.attention.unwrap();
        assert_eq!(
            (reason.rule.as_str(), reason.inputs, reason.since_ms),
            ("tool_asks", vec!["Trust this folder?".to_owned()], 5)
        );
        assert_eq!(
            apply_screen_ask(&storage, &id, Some("Trust this folder?"), 9),
            None,
            "already asking"
        );
        assert_eq!(
            apply_screen_ask(&storage, &id, None, 12),
            Some(SessionStatus::Running)
        );
        assert_eq!(apply_screen_ask(&storage, &id, None, 13), None);
    }

    /// The rc27 P1 walk: the original a switch stopped read "Stopped by
    /// you". A switch's stop ends it "stopped by the switch"; the mark is
    /// taken at its exit, so a later Stop of the user's is theirs again.
    #[test]
    fn a_switchs_stop_ends_it_stopped_by_the_switch_not_by_you() {
        use juggler_core::model::Ending;
        let dir = tempfile::tempdir().unwrap();
        let state = seeded(dir.path(), &["sess-original"]);
        let id = SessionId::new("sess-original");
        state.mark_switch_stop(&id);
        crate::features::sessions::lifecycle::forward(
            &state,
            juggler_supervisor::LifecycleEvent::Exited {
                session: id.clone(),
                pid: 1,
                kind: juggler_supervisor::ExitKind::Signaled(Some(15)),
            },
        );
        assert_eq!(
            status(&state, "sess-original"),
            (SessionStatus::Stopped, false)
        );
        let ending = state
            .storage
            .lock()
            .unwrap()
            .sessions()
            .endings()
            .unwrap()
            .remove("sess-original");
        assert_eq!(ending, Some((Ending::Switched, None)));
        assert!(!state.take_switch_stop(&id), "taken once, at its exit");
    }

    /// The rc22 walk: the ask cleared about 6 s after the first report
    /// (Needs you waited on its 5 s tick). The first report clears the
    /// stored ask and marks Needs you for now, with no tick.
    #[test]
    fn the_first_report_clears_the_ask_at_once() {
        let dir = tempfile::tempdir().unwrap();
        let (state, clock) = clocked(dir.path(), &["sess-answered"]);
        let told = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = told.clone();
        state
            .runtime
            .set_notifier(std::sync::Arc::new(move |id, stored| {
                seen.lock().unwrap().push((id.to_owned(), stored));
            }));
        clock.advance_to(reports::FIRST_REPORT_MS);
        assert_eq!(
            status(&state, "sess-answered"),
            (SessionStatus::WaitingUser, true)
        );
        assert!(state.runtime.take_needs_changed());
        told.lock().unwrap().clear();
        state.runtime.observe(
            &state,
            &SessionId::new("sess-answered"),
            &hook("SessionStart", None),
        );
        assert_eq!(
            status(&state, "sess-answered"),
            (SessionStatus::Running, false)
        );
        assert_eq!(state_of(&state, "sess-answered").as_deref(), Some("idle"));
        assert_eq!(
            told.lock().unwrap().as_slice(),
            [("sess-answered".to_owned(), true)],
            "the window is told at once"
        );
        assert!(
            state.runtime.take_needs_changed(),
            "Needs you follows at once, not on its tick"
        );
        assert!(!state.runtime.take_needs_changed(), "taken once");
    }

    #[test]
    fn a_session_that_reported_in_time_is_never_taken_as_waiting() {
        let dir = tempfile::tempdir().unwrap();
        let state =
            std::sync::Arc::new(crate::state::AppState::init(dir.path().join("data")).unwrap());
        let runtimes = &state.runtime;
        let session = SessionId::new("sess-fine");
        runtimes.expect_reports(&session, 0);
        let prompt = HookReport {
            event: "UserPromptSubmit".to_owned(),
            ..HookReport::default()
        };
        runtimes.observe(&state, &session, &prompt);
        assert!(runtimes.take_in_overdue(i64::MAX).is_empty());
        assert_eq!(
            runtimes.info("sess-fine").unwrap().state.as_deref(),
            Some("working")
        );
        // Forgotten with the session.
        runtimes.expect_reports(&SessionId::new("sess-gone"), 0);
        runtimes.forget("sess-gone");
        assert!(runtimes.take_in_overdue(i64::MAX).is_empty());
    }
}

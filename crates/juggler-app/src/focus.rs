//! The Focus Window service (AJ-E414, AJ-E2102): tiered focus that raises
//! the EXACT window a desktop Instance owns — activate the app, match the
//! recorded window identity, clear `AXMinimized`, then `AXRaise`.
//!
//! Mechanism pin. Tier (a), the app's activation, is by the owning
//! PROCESS (`NSRunningApplication`, safe objc2 calls, no permission):
//! never by bundle id, which brought the wrong instance forward when an
//! app runs several (all six of the maintainer's Claude profiles share
//! one, AJ-E2102). AI Juggler hands activation over first and checks the
//! app is frontmost afterwards ([`crate::activation`], 0.1.5). Tiers (b)
//! and (c) go through `/usr/bin/osascript` +
//! System Events (AX), and run ONLY when the user turned window focus on
//! in Settings: a click never raises a permission prompt (D-37).
//! Scriptable through the [`ScriptRunner`] seam so the tier logic is
//! unit-testable with no GUI, cross-platform, and 10/10-repeatable.

use std::sync::Arc;

use juggler_core::model::WindowIdentity;

use crate::activation::{self, Activation, Front, Request, Why};

/// Req.5 copy-exact outcome: the exact window was found and raised.
pub const OUTCOME_FOCUSED_WINDOW: &str = "Focused window";
/// Req.5 copy-exact outcome: the app was activated but no unique
/// window matched the recorded identity.
pub const OUTCOME_WINDOW_NOT_FOUND: &str = "App opened — window not found";
/// Req.5 copy-exact outcome: no live process owns this Instance.
pub const OUTCOME_NOT_RUNNING: &str = "App not running — Relaunch?";
/// Honest outcome on non-macOS builds (no window system to target).
pub const OUTCOME_UNSUPPORTED: &str = "Window focus unavailable on this platform";
/// AJ-E2102: the app came forward (checked: it's frontmost); the exact
/// window needs window focus turned on in Settings (never asked for from
/// a click).
pub const OUTCOME_APP_FORWARD: &str = "Brought the app forward";

/// 0.1.5: the app is running and stayed behind; the words say why when
/// it's known (the same as [`activation::behind_text`] for "the app").
#[must_use]
pub fn outcome_behind(why: Why) -> &'static str {
    match why {
        Why::NotInFront => {
            "macOS didn't bring the app forward: AI Juggler wasn't in front. Click it in the Dock"
        }
        Why::Refused => "macOS refused to bring the app forward. Click it in the Dock",
        Why::StayedBehind => "macOS didn't bring the app forward. Click it in the Dock",
    }
}

/// One focus attempt: the honest outcome, whether the Accessibility
/// grant is missing (degraded tier), and the (possibly enriched)
/// window identity to persist — first-sight titles and re-discovered
/// PIDs survive the attempt (Req.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusAttempt {
    /// Copy-exact outcome label.
    pub outcome: &'static str,
    /// True when exact-window focus was impossible because the
    /// Accessibility permission is missing (degraded to activation).
    pub permission_missing: bool,
    /// Identity as it should be recorded after this attempt.
    pub identity: Option<WindowIdentity>,
}

/// The OS operations the tiers need (tests script them): `AppleScript`
/// (shelled out), and [`Front`] to bring the app forward by its process
/// and read the frontmost app (no permission prompts, the right instance
/// of an app that runs several).
pub trait ScriptRunner: Front {
    /// Runs one `AppleScript` source against System Events;
    /// `Ok(stdout)` on exit 0, `Err(stderr)` otherwise.
    ///
    /// # Errors
    ///
    /// The stderr text of a failed script (non-zero exit or spawn
    /// failure) — the tiers match on it, never panic on it.
    fn osascript(&self, source: &str) -> Result<String, String>;
}

/// The real runner: `/usr/bin/osascript -e` and [`activation::Mac`].
#[derive(Debug, Default, Clone, Copy)]
pub struct OsascriptRunner;

impl Front for OsascriptRunner {
    fn request(&self, pid: u32) -> Request {
        activation::Mac.request(pid)
    }

    fn frontmost(&self) -> Option<u32> {
        activation::Mac.frontmost()
    }
}

impl ScriptRunner for OsascriptRunner {
    fn osascript(&self, source: &str) -> Result<String, String> {
        let out = std::process::Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(source)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

/// Escapes an `AppleScript` string literal body (backslash + quote).
fn applescript_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// True when an osascript stderr is the missing-Accessibility grant
/// (not a generic script failure) — the strings macOS emits for
/// untrusted AX senders.
fn is_permission_denied(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("assistive")
        || lower.contains("not authorized")
        || lower.contains("accessibility")
        || lower.contains("-1743")
        || lower.contains("-1719")
}

/// Parses an `AppleScript` `{a, b, c}`-style window-name list as printed
/// by osascript (comma-separated; quotes only when needed). Names
/// containing commas cannot be split unambiguously and are surfaced as
/// a parse failure → `window not found`, never a wrong-window raise.
fn parse_window_names(raw: &str) -> Option<Vec<String>> {
    let raw = raw.trim().trim_start_matches('{').trim_end_matches('}');
    if raw.is_empty() {
        return Some(Vec::new());
    }
    let mut names = Vec::new();
    for piece in raw.split(", ") {
        let name = piece.trim();
        if name.contains(',') {
            return None;
        }
        let unquoted = name.trim_matches('"');
        if unquoted.contains('"') {
            return None;
        }
        names.push(unquoted.to_string());
    }
    Some(names)
}

/// The tier engine. Pure over the [`ScriptRunner`] seam — every branch
/// is unit-testable without a GUI (the platform check lives in
/// [`focus`], the public entry).
pub struct FocusService {
    runner: Arc<dyn ScriptRunner>,
}

impl FocusService {
    /// Wraps a runner (the app passes [`OsascriptRunner`]; tests a
    /// scripted fixture).
    #[must_use]
    pub fn new(runner: Arc<dyn ScriptRunner>) -> Self {
        Self { runner }
    }

    /// Runs the tiers for one window identity (Req.1): activate FIRST
    /// (windows on another Space are invisible to AX until the app is
    /// frontmost — the recorded trap), then enumerate by recorded PID
    /// (stale → re-discover by bundle id, never assumed), match the
    /// recorded title (absent → adopt the first window at first
    /// sight; collision → `window not found`), then clear
    /// `AXMinimized` and `AXRaise` (clearing FIRST — `AXRaise` on a
    /// minimised window is a silent no-op).
    /// `exact_windows`: the user turned window focus on in Settings;
    /// without it only tier (a) runs, and System Events is never asked.
    #[must_use]
    pub fn attempt(&self, identity: &WindowIdentity, exact_windows: bool) -> FocusAttempt {
        let mut identity = identity.clone();
        // Tier (a): app activation by its process, always (Space reveal +
        // degrade path), checked: "forward" only once it's frontmost. Pid
        // 0 is a pre-v10 identity: nothing to activate until tier (b)
        // re-discovers it.
        let activated = if identity.pid == 0 {
            Activation::Gone
        } else {
            activation::bring(self.runner.as_ref(), identity.pid)
        };
        if !exact_windows {
            return FocusAttempt {
                outcome: match activated {
                    Activation::Forward => OUTCOME_APP_FORWARD,
                    Activation::Behind(why) => outcome_behind(why),
                    Activation::Gone => OUTCOME_NOT_RUNNING,
                },
                permission_missing: activated == Activation::Forward,
                identity: Some(identity),
            };
        }
        // Tier (b): enumerate the app's windows by recorded PID;
        // a stale PID re-discovers the process by bundle id. A denial
        // of the WINDOW read is classified HERE as well — the probe
        // above reads only process names, which macOS allows WITHOUT
        // the Accessibility grant (caught live on the real TextEdit
        // record: probe OK, enumeration denied -1728).
        let by_pid = format!(
            "tell application \"System Events\" to get name of every window of (first application process whose unix id is {})",
            identity.pid
        );
        let enumeration = match self.runner.osascript(&by_pid) {
            Ok(raw) => raw,
            Err(stderr) => {
                if is_permission_denied(&stderr) {
                    return FocusAttempt {
                        outcome: OUTCOME_WINDOW_NOT_FOUND,
                        permission_missing: true,
                        identity: Some(identity),
                    };
                }
                let Some(bundle) = identity.bundle_id.clone() else {
                    return not_running(identity);
                };
                let by_bundle = format!(
                    "tell application \"System Events\" to get name of every window of (first application process whose bundle identifier is \"{}\")",
                    applescript_escape(&bundle)
                );
                match self.runner.osascript(&by_bundle) {
                    Ok(raw) => {
                        // Re-discovered: the app relaunched under a new
                        // PID. Persist the fresh process id.
                        let pid = self.runner.osascript(&format!(
                            "tell application \"System Events\" to get unix id of (first application process whose bundle identifier is \"{}\")",
                            applescript_escape(&bundle)
                        ));
                        if let Ok(pid) = pid
                            && let Ok(pid) = pid.trim().parse::<u32>()
                        {
                            identity.pid = pid;
                            identity.start_time = None;
                            let _ = activation::bring(self.runner.as_ref(), pid);
                        }
                        raw
                    }
                    Err(stderr) => {
                        if is_permission_denied(&stderr) {
                            return FocusAttempt {
                                outcome: OUTCOME_WINDOW_NOT_FOUND,
                                permission_missing: true,
                                identity: Some(identity),
                            };
                        }
                        return not_running(identity);
                    }
                }
            }
        };
        let Some(names) = parse_window_names(&enumeration) else {
            return window_not_found(identity);
        };
        if names.is_empty() {
            return window_not_found(identity);
        }
        // Match the recorded title; first sight adopts window[0].
        let title = if let Some(recorded) = identity.title.clone() {
            let matches = names
                .iter()
                .filter(|name| name.as_str() == recorded)
                .count();
            // Stale title (renamed/closed) or a title collision without
            // a `CGWindow` id: reported, never guessed to a window
            // (re-adopting would raise the WRONG window — handover trap).
            if matches != 1 {
                return window_not_found(identity);
            }
            recorded
        } else {
            identity.title = Some(names[0].clone());
            names[0].clone()
        };
        self.raise(identity, &title)
    }

    /// Tier (c): clear `AXMinimized` FIRST, then `AXRaise`, then
    /// frontmost — one script so the order is atomic. Focused only once
    /// the app is frontmost.
    fn raise(&self, identity: WindowIdentity, title: &str) -> FocusAttempt {
        let raise = format!(
            "tell application \"System Events\"\ntell (first application process whose unix id is {})\nset value of attribute \"AXMinimized\" of window \"{}\" to false\nperform action \"AXRaise\" of window \"{}\"\nset frontmost to true\nend tell\nend tell",
            identity.pid,
            applescript_escape(title),
            applescript_escape(title)
        );
        if self.runner.osascript(&raise).is_err() {
            return window_not_found(identity);
        }
        // The raise said yes; the window counts as focused only once its
        // app is frontmost.
        let outcome = match activation::settle(self.runner.as_ref(), identity.pid, true) {
            Activation::Forward => OUTCOME_FOCUSED_WINDOW,
            Activation::Behind(why) => outcome_behind(why),
            Activation::Gone => OUTCOME_NOT_RUNNING,
        };
        FocusAttempt {
            outcome,
            permission_missing: false,
            identity: Some(identity),
        }
    }
}

fn not_running(identity: WindowIdentity) -> FocusAttempt {
    FocusAttempt {
        outcome: OUTCOME_NOT_RUNNING,
        permission_missing: false,
        identity: Some(identity),
    }
}

fn window_not_found(identity: WindowIdentity) -> FocusAttempt {
    FocusAttempt {
        outcome: OUTCOME_WINDOW_NOT_FOUND,
        permission_missing: false,
        identity: Some(identity),
    }
}

/// The public entry: the platform gate, then the tiers. Non-macOS
/// builds report the honest unsupported outcome instead of shelling
/// out to binaries that do not exist.
#[must_use]
pub fn focus(
    runner: Arc<dyn ScriptRunner>,
    identity: &WindowIdentity,
    exact_windows: bool,
) -> FocusAttempt {
    if !cfg!(target_os = "macos") {
        return FocusAttempt {
            outcome: OUTCOME_UNSUPPORTED,
            permission_missing: false,
            identity: Some(identity.clone()),
        };
    }
    FocusService::new(runner).attempt(identity, exact_windows)
}

#[cfg(test)]
mod tests {
    //! The scripted-fixture suites (quality gate): a two-window fake
    //! app, one minimised; every branch is driven through the
    //! [`ScriptRunner`] seam with no GUI and no real permission state.

    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A scriptable fake runner: answers osascript probes and records
    /// every activation/script for sequence asserts. Scenario knobs
    /// are atomics so tests can flip them after construction.
    struct FixtureRunner {
        activation_log: Mutex<Vec<String>>,
        script_log: Mutex<Vec<String>>,
        windows: Mutex<Vec<String>>,
        pid_alive: AtomicBool,
        bundle_gone: AtomicBool,
        permission_denied: AtomicBool,
        window_read_denied: AtomicBool,
        raise_fails: AtomicBool,
        /// macOS takes the request and another app stays in front.
        stays_behind: AtomicBool,
        /// The frontmost app's process (the last one that came forward).
        front: Mutex<Option<u32>>,
    }

    impl FixtureRunner {
        fn two_windows() -> Arc<Self> {
            Arc::new(Self {
                activation_log: Mutex::new(Vec::new()),
                script_log: Mutex::new(Vec::new()),
                windows: Mutex::new(vec!["notes-a".into(), "notes-b".into()]),
                pid_alive: AtomicBool::new(true),
                bundle_gone: AtomicBool::new(false),
                permission_denied: AtomicBool::new(false),
                window_read_denied: AtomicBool::new(false),
                raise_fails: AtomicBool::new(false),
                stays_behind: AtomicBool::new(false),
                front: Mutex::new(Some(1)),
            })
        }

        fn scripts(&self) -> Vec<String> {
            self.script_log.lock().unwrap().clone()
        }

        fn activations(&self) -> Vec<String> {
            self.activation_log.lock().unwrap().clone()
        }

        fn raises(&self) -> Vec<String> {
            self.scripts()
                .into_iter()
                .filter(|s| s.contains("AXRaise"))
                .collect()
        }
    }

    impl ScriptRunner for FixtureRunner {
        fn osascript(&self, source: &str) -> Result<String, String> {
            self.script_log.lock().unwrap().push(source.to_string());
            if self.permission_denied.load(Ordering::SeqCst) && source.contains("System Events") {
                return Err(
                    "execution error: Not authorized to send Apple events \"System Events\" (-1743)"
                        .into(),
                );
            }
            if source.contains("get name of every window") {
                if source.contains("unix id is 0") {
                    return Err("execution error: Can't get application process (-600)".into());
                }
                if !self.pid_alive.load(Ordering::SeqCst) && source.contains("unix id is") {
                    return Err("execution error: Can't get application process (-600)".into());
                }
                if !self.pid_alive.load(Ordering::SeqCst)
                    && self.bundle_gone.load(Ordering::SeqCst)
                    && source.contains("bundle identifier")
                {
                    return Err("execution error: Can't get application process (-600)".into());
                }
                if self.window_read_denied.load(Ordering::SeqCst) {
                    return Err(
                        "execution error: osascript is not allowed assistive access. (-1728)"
                            .into(),
                    );
                }
                let names = self.windows.lock().unwrap().join(", ");
                return Ok(names);
            }
            if source.contains("get unix id of") {
                return Ok("42424".into());
            }
            if source.contains("AXRaise") {
                if self.raise_fails.load(Ordering::SeqCst) {
                    return Err("execution error: Can't get window (-25204)".into());
                }
                return Ok(String::new());
            }
            Ok("System Events".into())
        }
    }

    impl Front for FixtureRunner {
        fn request(&self, pid: u32) -> Request {
            self.activation_log.lock().unwrap().push(pid.to_string());
            if !self.stays_behind.load(Ordering::SeqCst) {
                *self.front.lock().unwrap() = Some(pid);
            }
            Request::Asked { in_front: true }
        }

        fn frontmost(&self) -> Option<u32> {
            *self.front.lock().unwrap()
        }

        fn pause(&self, _span: std::time::Duration) {}
    }

    fn identity() -> WindowIdentity {
        WindowIdentity {
            pid: 111,
            bundle_id: Some("dev.aijuggler.fake-desktop".into()),
            title: Some("notes-b".into()),
            cgwindow_id: None,
            start_time: None,
        }
    }

    /// Quality gate: the two-window fixture (target = window 2, the
    /// minimised one) focuses the correct window 10/10, with the
    /// trap-order pinned: activation BEFORE enumeration, AXMinimized
    /// cleared BEFORE AXRaise in the same script.
    #[test]
    fn two_window_fixture_focuses_correct_window_10_of_10() {
        let runner = FixtureRunner::two_windows();
        let service = FocusService::new(runner.clone());
        for attempt in 1..=10 {
            let result = service.attempt(&identity(), true);
            assert_eq!(
                result.outcome, OUTCOME_FOCUSED_WINDOW,
                "attempt {attempt} must focus the window"
            );
            assert!(!result.permission_missing);
        }
        // Activation happened exactly once per attempt (tier a first).
        assert_eq!(runner.activations().len(), 10);
        // Every raise script: minimized cleared BEFORE AXRaise, then
        // frontmost — and it targets the recorded title, not window 1.
        let raises = runner.raises();
        assert_eq!(raises.len(), 10);
        for script in &raises {
            assert!(script.contains("window \"notes-b\""), "{script}");
            let minimized = script.find("AXMinimized").unwrap();
            let raise = script.find("AXRaise").unwrap();
            let frontmost = script.find("set frontmost").unwrap();
            assert!(minimized < raise && raise < frontmost, "{script}");
        }
    }

    /// First sight: no recorded title → window[0] is adopted and
    /// persisted with the identity (never guessed beyond first sight).
    #[test]
    fn first_sight_adopts_and_persists_window_zero_title() {
        let runner = FixtureRunner::two_windows();
        let mut id = identity();
        id.title = None;
        let result = FocusService::new(runner).attempt(&id, true);
        assert_eq!(result.outcome, OUTCOME_FOCUSED_WINDOW);
        assert_eq!(result.identity.unwrap().title.as_deref(), Some("notes-a"));
    }

    /// Title collision (two windows, one name, no CGWindow id): the
    /// ambiguity is recorded as `window not found` — never a
    /// wrong-window raise (handover trap).
    #[test]
    fn title_collision_reports_window_not_found_never_guesses() {
        let runner = FixtureRunner::two_windows();
        {
            let mut windows = runner.windows.lock().unwrap();
            windows[1] = "notes-a".into();
        }
        let result = FocusService::new(runner.clone()).attempt(&identity(), true);
        assert_eq!(result.outcome, OUTCOME_WINDOW_NOT_FOUND);
        assert!(
            runner.raises().is_empty(),
            "no raise may run on an ambiguous match"
        );
    }

    /// Missing Accessibility permission: degrades to app activation
    /// with the honest outcome + flag, and never raises. No crash, no
    /// silent permission request (the probe is read-only).
    #[test]
    fn missing_permission_degrades_to_activation_with_flag() {
        let runner = FixtureRunner::two_windows();
        runner.permission_denied.store(true, Ordering::SeqCst);
        let result = FocusService::new(runner.clone()).attempt(&identity(), true);
        assert_eq!(result.outcome, OUTCOME_WINDOW_NOT_FOUND);
        assert!(result.permission_missing);
        assert_eq!(runner.activations(), vec!["111"]);
        assert!(
            runner.raises().is_empty(),
            "degraded tier must not attempt a raise"
        );
    }

    /// The live-caught case (real TextEdit record): process-name probe
    /// SUCCEEDS (allowed without the grant) but the window read is
    /// denied — classified as permission-missing, never "not running".
    #[test]
    fn window_read_denied_after_ok_probe_degrades_with_flag() {
        let runner = FixtureRunner::two_windows();
        runner.window_read_denied.store(true, Ordering::SeqCst);
        let mut id = identity();
        id.pid = 0; // pre-v10-style identity: pid path errs first
        let result = FocusService::new(runner.clone()).attempt(&id, true);
        assert_eq!(result.outcome, OUTCOME_WINDOW_NOT_FOUND);
        assert!(
            result.permission_missing,
            "must not masquerade as not-running"
        );
        assert!(runner.raises().is_empty());
    }

    /// Stale PID (app relaunched): re-discovers the process by bundle
    /// id and persists the fresh PID — never assumes the old one.
    #[test]
    fn stale_pid_is_rediscovered_by_bundle_id() {
        let runner = FixtureRunner::two_windows();
        runner.pid_alive.store(false, Ordering::SeqCst);
        let result = FocusService::new(runner).attempt(&identity(), true);
        assert_eq!(result.outcome, OUTCOME_FOCUSED_WINDOW);
        assert_eq!(result.identity.unwrap().pid, 42_424);
    }

    /// Process gone entirely (pid AND bundle find nothing): the honest
    /// not-running outcome with the relaunch prompt.
    #[test]
    fn dead_process_reports_not_running() {
        let runner = FixtureRunner::two_windows();
        runner.pid_alive.store(false, Ordering::SeqCst);
        runner.bundle_gone.store(true, Ordering::SeqCst);
        let result = FocusService::new(runner).attempt(&identity(), true);
        assert_eq!(result.outcome, OUTCOME_NOT_RUNNING);
    }

    /// Running app with no windows at all (all closed): window not
    /// found — the app was activated, there is just no window to raise.
    #[test]
    fn windowless_app_reports_window_not_found() {
        let runner = FixtureRunner::two_windows();
        runner.windows.lock().unwrap().clear();
        let result = FocusService::new(runner).attempt(&identity(), true);
        assert_eq!(result.outcome, OUTCOME_WINDOW_NOT_FOUND);
    }

    /// A raise that fails at the AX layer reports window-not-found,
    /// never success (honest outcome classification, Req.5).
    #[test]
    fn failed_raise_reports_window_not_found() {
        let runner = FixtureRunner::two_windows();
        runner.raise_fails.store(true, Ordering::SeqCst);
        let result = FocusService::new(runner).attempt(&identity(), true);
        assert_eq!(result.outcome, OUTCOME_WINDOW_NOT_FOUND);
    }

    /// Stale recorded title (window renamed/closed, app still running
    /// with other windows): re-enumerated, absence reported — the old
    /// title is never assumed to still exist.
    #[test]
    fn stale_title_reports_window_not_found() {
        let runner = FixtureRunner::two_windows();
        {
            let mut windows = runner.windows.lock().unwrap();
            windows[1] = "renamed".into();
        }
        let result = FocusService::new(runner.clone()).attempt(&identity(), true);
        assert_eq!(result.outcome, OUTCOME_WINDOW_NOT_FOUND);
        assert!(
            runner.raises().is_empty(),
            "a stale title must not raise a renamed window"
        );
    }

    /// Unparseable enumeration (comma inside a title): reported as
    /// window-not-found, never a mis-split raise.
    #[test]
    fn unparseable_names_degrade_to_window_not_found() {
        let runner = FixtureRunner::two_windows();
        {
            let mut windows = runner.windows.lock().unwrap();
            windows[0] = "a,b".into();
        }
        let mut id = identity();
        id.title = None;
        let result = FocusService::new(runner).attempt(&id, true);
        assert_eq!(result.outcome, OUTCOME_WINDOW_NOT_FOUND);
    }

    /// AJ-E2102: window focus off in Settings — the app comes forward by
    /// its process, and System Events is never asked (no prompt from a
    /// click). The outcome says the window step needs the setting.
    #[test]
    fn without_window_focus_only_the_process_is_activated() {
        let runner = FixtureRunner::two_windows();
        let result = FocusService::new(runner.clone()).attempt(&identity(), false);
        assert_eq!(result.outcome, OUTCOME_APP_FORWARD);
        assert!(result.permission_missing);
        assert_eq!(runner.activations(), vec!["111"]);
        assert!(runner.scripts().is_empty(), "no AppleScript at all");
        // A pre-v10 identity (no pid) has nothing to bring forward.
        let mut legacy = identity();
        legacy.pid = 0;
        let result = FocusService::new(runner.clone()).attempt(&legacy, false);
        assert_eq!(result.outcome, OUTCOME_NOT_RUNNING);
        assert!(runner.scripts().is_empty());
    }

    /// 0.1.5: macOS took the request and another app stayed in front.
    /// Never "Brought the app forward" (and no permission explainer);
    /// with window focus on, a raise that said yes isn't "Focused
    /// window" either.
    #[test]
    fn an_app_that_stays_behind_is_never_reported_forward() {
        let runner = FixtureRunner::two_windows();
        runner.stays_behind.store(true, Ordering::SeqCst);
        let result = FocusService::new(runner.clone()).attempt(&identity(), false);
        assert_eq!(result.outcome, outcome_behind(Why::StayedBehind));
        assert!(!result.permission_missing);
        let result = FocusService::new(runner.clone()).attempt(&identity(), true);
        assert_eq!(result.outcome, outcome_behind(Why::StayedBehind));
        assert_eq!(runner.raises().len(), 1, "the raise ran; it isn't proof");
    }

    /// The focus words are Go to it's words, for "the app".
    #[test]
    fn the_focus_words_match_go_to() {
        for why in [Why::NotInFront, Why::Refused, Why::StayedBehind] {
            assert_eq!(outcome_behind(why), activation::behind_text("the app", why));
        }
    }

    /// The platform gate: on this build the entry either runs the
    /// tiers (macOS) or reports unsupported without touching the
    /// runner — cross-platform CI asserts the branch logic itself.
    #[test]
    fn public_entry_platform_gates_before_running() {
        let runner = FixtureRunner::two_windows();
        let bridged: Arc<dyn ScriptRunner> = runner.clone();
        let result = focus(bridged, &identity(), true);
        if cfg!(target_os = "macos") {
            assert_eq!(result.outcome, OUTCOME_FOCUSED_WINDOW);
        } else {
            assert_eq!(result.outcome, OUTCOME_UNSUPPORTED);
            assert!(runner.scripts().is_empty());
            assert!(runner.activations().is_empty());
        }
    }
}

/// AJ-E711 test seam: the runner the Fleet link uses, and a forced
/// screen-lock answer (production: the real runner and the real probe).
static LINK_RUNNER: std::sync::Mutex<Option<Arc<dyn ScriptRunner + Send + Sync>>> =
    std::sync::Mutex::new(None);
static LOCK_OVERRIDE: std::sync::Mutex<Option<bool>> = std::sync::Mutex::new(None);

static LINK_EXACT: std::sync::Mutex<Option<bool>> = std::sync::Mutex::new(None);

/// Whether a remote focus request may reach the exact window on this Mac:
/// the user turned window focus on here (a test can force it).
#[must_use]
pub fn link_exact_windows(state: &Arc<crate::state::AppState>) -> bool {
    LINK_EXACT
        .lock()
        .ok()
        .and_then(|slot| *slot)
        .unwrap_or_else(|| crate::os_access::exact_windows_allowed(state))
}

/// Test seam: force whether a remote focus may reach the exact window
/// (None = the user's setting).
#[doc(hidden)]
pub fn set_link_exact_windows_for_test(allowed: Option<bool>) {
    if let Ok(mut slot) = LINK_EXACT.lock() {
        *slot = allowed;
    }
}

/// The runner a remote focus request uses on this Mac.
#[must_use]
pub fn link_runner() -> Arc<dyn ScriptRunner> {
    LINK_RUNNER
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .map_or_else(
            || Arc::new(OsascriptRunner) as Arc<dyn ScriptRunner>,
            |runner| runner as Arc<dyn ScriptRunner>,
        )
}

/// Test seam: a scripted runner for remote focus (None = the real one).
#[doc(hidden)]
pub fn set_link_runner_for_test(runner: Option<Arc<dyn ScriptRunner + Send + Sync>>) {
    if let Ok(mut slot) = LINK_RUNNER.lock() {
        *slot = runner;
    }
}

/// Test seam: force the screen-lock answer (None = probe for real).
#[doc(hidden)]
pub fn set_screen_locked_for_test(locked: Option<bool>) {
    if let Ok(mut slot) = LOCK_OVERRIDE.lock() {
        *slot = locked;
    }
}

/// Whether this Mac's screen is locked (AJ-E711: a remote focus never
/// runs on a locked screen — it is reported instead). Reads the console
/// session's `CGSSessionScreenIsLocked` flag from `ioreg`.
///
/// # Errors
///
/// Returns the reason when the state can't be read (the caller refuses
/// rather than guess).
pub fn screen_locked() -> Result<bool, String> {
    if let Some(forced) = LOCK_OVERRIDE.lock().ok().and_then(|slot| *slot) {
        return Ok(forced);
    }
    if !cfg!(target_os = "macos") {
        return Err("screen-lock state is only readable on macOS".to_owned());
    }
    let out = std::process::Command::new("/usr/sbin/ioreg")
        .args(["-n", "Root", "-d1", "-a"])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned());
    }
    Ok(plist_screen_locked(&String::from_utf8_lossy(&out.stdout)))
}

/// `true` when the plist carries `CGSSessionScreenIsLocked` = true.
fn plist_screen_locked(plist: &str) -> bool {
    plist
        .split("<key>CGSSessionScreenIsLocked</key>")
        .skip(1)
        .any(|rest| rest.trim_start().starts_with("<true/>"))
}

#[cfg(test)]
mod lock_tests {
    use super::plist_screen_locked;

    #[test]
    fn the_lock_flag_is_read_from_the_session_plist() {
        let locked = "<dict><key>CGSSessionScreenIsLocked</key>\n\t<true/><key>x</key></dict>";
        let open = "<dict><key>CGSSessionScreenIsLocked</key><false/></dict>";
        assert!(plist_screen_locked(locked));
        assert!(!plist_screen_locked(open));
        assert!(!plist_screen_locked(
            "<dict><key>kCGSSessionOnConsoleKey</key><true/></dict>"
        ));
    }
}

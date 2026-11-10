//! A test's launch of the real binary (`AIJ_TEST_LAUNCH=<the test's
//! pid>`): headless, and bound to the test.
//!
//! rc27's preflight: `second_launch` started the real binary in throwaway
//! homes, and the launches that got as far as Tauri came up as ordinary
//! apps — a Dock icon, focus taken from whatever the maintainer was doing,
//! and the main window shown blank (a debug build loads the dev server's
//! page, which no test serves), so it looked frozen until the test killed
//! it. A launch the test never reaped (the test process killed outright:
//! no `Drop` runs) stayed up like that until the maintainer killed it.
//!
//! Such a launch now:
//! - is never activated and has no Dock icon (activation policy
//!   `Prohibited`, set before the event loop starts), and its main window
//!   is made hidden and is never brought forward;
//! - shows no alert (its alerts go to stderr, as `AIJ_NO_DIALOGS`'s do)
//!   and no startup window;
//! - ends by itself once the test that started it is gone, so nothing it
//!   started outlives the test however the test ends.
//!
//! A test seam (SECURITY-GUIDELINES, test seams): debug builds only; a
//! release build compiles none of it (`scripts/release_seams_check.sh`).
//! Everything that asks reads [`launcher`], the one place the variable is
//! read.

use crate::features::app::crash::started_at;

/// The variable a test sets on the launches it starts: its own pid.
pub const ENV: &str = "AIJ_TEST_LAUNCH";

/// The pid a launch's value names: a positive number, nothing else.
#[must_use]
pub fn parse(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok().filter(|pid| *pid > 1)
}

/// The test that started this launch, when a test did (debug builds).
#[cfg(debug_assertions)]
#[must_use]
pub fn launcher() -> Option<u32> {
    static LAUNCHER: std::sync::OnceLock<Option<u32>> = std::sync::OnceLock::new();
    *LAUNCHER.get_or_init(|| std::env::var(ENV).ok().as_deref().and_then(parse))
}

/// A release build: never a test's launch.
#[cfg(not(debug_assertions))]
#[must_use]
pub const fn launcher() -> Option<u32> {
    None
}

/// Whether this launch is a test's: no window, no alert, no activation.
#[must_use]
pub fn headless() -> bool {
    launcher().is_some()
}

/// A test's launch is never activated and has no Dock icon: called on
/// the built app before its event loop starts, which activates the app as
/// it launches. Nothing for any other launch.
#[cfg_attr(
    not(target_os = "macos"),
    allow(
        clippy::needless_pass_by_ref_mut,
        reason = "only one system has an activation policy"
    )
)]
pub fn keep_out_of_sight<R: tauri::Runtime>(app: &mut tauri::App<R>) {
    #[cfg(target_os = "macos")]
    if headless() {
        app.set_activation_policy(tauri::ActivationPolicy::Prohibited);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

/// How often a test's launch looks for its test.
const LOOK_EVERY: std::time::Duration = std::time::Duration::from_millis(250);

/// At the very start of a launch (before the watchdog, an alert or the
/// data): a test's launch ends at once if its test is already gone, and
/// otherwise as soon as it goes. Nothing for any other launch.
pub fn bind_to_launcher() {
    let Some(pid) = launcher() else {
        return;
    };
    let Some(born) = started_at(pid) else {
        crate::early_note("the test that started this launch is gone; ending");
        std::process::exit(0);
    };
    let spawned = std::thread::Builder::new()
        .name("test-launch".into())
        .spawn(move || {
            // poll: a process has no exit event another can wait on;
            // a test's launch only, for as long as it runs.
            while started_at(pid) == Some(born) {
                std::thread::sleep(LOOK_EVERY);
            }
            crate::early_note("the test that started this launch is gone; ending");
            std::process::exit(0);
        });
    if spawned.is_err() {
        // Unbound, it could outlive its test: it doesn't run at all.
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_real_pid_makes_a_test_launch() {
        assert_eq!(parse("4242"), Some(4242));
        assert_eq!(parse(" 4242\n"), Some(4242));
        for not_one in ["", "0", "1", "-5", "yes", "42x", "4294967296"] {
            assert_eq!(parse(not_one), None, "{not_one:?}");
        }
    }

    #[test]
    fn a_run_without_the_variable_is_no_test_launch() {
        // The test binary itself is never started with it.
        assert!(std::env::var_os(ENV).is_none());
        assert!(!headless());
        // Unbound: returns, and the process goes on.
        bind_to_launcher();
    }

    #[test]
    fn the_launchers_start_time_is_read_and_a_gone_one_has_none() {
        assert!(started_at(std::process::id()).is_some());
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        assert_eq!(started_at(pid), None, "a reaped child is gone");
    }
}

//! AJ-E1509 Req.4 — AI Juggler comes back after it quits unexpectedly.
//!
//! - **The run marker** (`last-run` in the data folder): `running` from
//!   the moment the data is opened, `clean` when AI Juggler quits as
//!   asked. Found `running` at the next start, the last run crashed (or
//!   was killed): the sessions that died with it are ended with that
//!   reason, and resumed where restarting is on.
//! - **The watchdog**: a second process of this same binary
//!   (`--aij-watch <pid> <data>`) that waits for the app to end. Ended
//!   without the clean mark, it starts AI Juggler again — at most
//!   [`RELAUNCHES`] times in [`RELAUNCH_WINDOW_MS`], so a crash at startup
//!   can't loop. Not launchd's `KeepAlive`: that covers only a process
//!   launchd started, and AI Juggler is usually opened from the Dock.
//!   Release builds only, when "Reopen AI Juggler if it quits
//!   unexpectedly" is on (debug builds: only with `AIJ_WATCHDOG=1`).

use std::path::{Path, PathBuf};
use std::time::Duration;

/// The marker's file, in the data folder.
const MARKER: &str = "last-run";
/// The watchdog's relaunches so far (epoch ms, one a line).
const RELAUNCH_LOG: &str = "crash-relaunches";
/// The argument that makes this binary a watchdog.
pub const WATCH_ARG: &str = "--aij-watch";
/// Relaunches allowed within [`RELAUNCH_WINDOW_MS`].
pub const RELAUNCHES: usize = 3;
/// The window relaunches are counted in: 10 minutes.
pub const RELAUNCH_WINDOW_MS: i64 = 10 * 60 * 1000;
/// How often the watchdog looks.
const LOOK_EVERY: Duration = Duration::from_millis(500);

/// Whether the last run ended without quitting as asked; then marks this
/// run as running.
#[must_use]
pub fn begin_run(data_dir: &Path) -> bool {
    let crashed =
        std::fs::read_to_string(data_dir.join(MARKER)).is_ok_and(|was| was.trim() == "running");
    let _ = std::fs::write(data_dir.join(MARKER), "running\n");
    crashed
}

/// This run ended as asked (a quit, or the state closed in a test).
pub fn end_run(data_dir: &Path) {
    let _ = std::fs::write(data_dir.join(MARKER), "clean\n");
}

/// Whether this build may watch itself: release builds, and debug builds
/// only for the crash test (`AIJ_WATCHDOG=1`).
#[must_use]
pub fn watchdog_allowed() -> bool {
    #[cfg(debug_assertions)]
    {
        std::env::var_os("AIJ_WATCHDOG").is_some_and(|value| value == "1")
    }
    #[cfg(not(debug_assertions))]
    {
        true
    }
}

/// Starts the watchdog for this process, when allowed. Its own process,
/// with nothing to read or say; it ends with the app.
#[must_use]
pub fn spawn_watchdog(data_dir: &Path) -> Option<std::process::Child> {
    if !watchdog_allowed() {
        return None;
    }
    spawn_watchdog_with(&std::env::current_exe().ok()?, data_dir)
}

/// The watchdog, from `exe` (this app's binary; the tests pass the one
/// they built).
#[must_use]
pub fn spawn_watchdog_with(exe: &Path, data_dir: &Path) -> Option<std::process::Child> {
    std::process::Command::new(exe)
        .arg(WATCH_ARG)
        .arg(std::process::id().to_string())
        .arg(data_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()
}

/// When this launch is a watchdog: becomes one, and never returns.
pub fn watch_if_asked() {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(WATCH_ARG)) {
        return;
    }
    let pid = args
        .next()
        .and_then(|pid| pid.to_str().and_then(|pid| pid.parse::<u32>().ok()));
    let data = args.next().map(PathBuf::from);
    if let (Some(pid), Some(data)) = (pid, data) {
        watch(pid, &data);
    }
    std::process::exit(0);
}

/// `pid`'s start time, while it is alive (a reused pid has another).
fn started_at(pid: u32) -> Option<u64> {
    let pid = sysinfo::Pid::from_u32(pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    system.process(pid).map(sysinfo::Process::start_time)
}

fn watch(pid: u32, data_dir: &Path) {
    let Some(born) = started_at(pid) else {
        return;
    };
    while started_at(pid) == Some(born) {
        std::thread::sleep(LOOK_EVERY);
    }
    let clean =
        std::fs::read_to_string(data_dir.join(MARKER)).map_or(true, |was| was.trim() != "running");
    if clean {
        return;
    }
    let now = crate::commands::now_millis();
    let earlier = relaunches(data_dir);
    if !may_relaunch(&earlier, now) {
        eprintln!(
            "ai-juggler: quit unexpectedly {RELAUNCHES} times in 10 minutes; not starting it again"
        );
        return;
    }
    let mut log: Vec<i64> = earlier
        .into_iter()
        .filter(|at| now.saturating_sub(*at) < RELAUNCH_WINDOW_MS)
        .collect();
    log.push(now);
    let lines = log
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(data_dir.join(RELAUNCH_LOG), lines);
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

fn relaunches(data_dir: &Path) -> Vec<i64> {
    std::fs::read_to_string(data_dir.join(RELAUNCH_LOG))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}

/// Pure: whether another relaunch fits in the window.
#[must_use]
pub fn may_relaunch(earlier: &[i64], now_ms: i64) -> bool {
    earlier
        .iter()
        .filter(|at| now_ms.saturating_sub(**at) < RELAUNCH_WINDOW_MS)
        .count()
        < RELAUNCHES
}

/// Starts or ends this app's watchdog as "Reopen AI Juggler if it quits
/// unexpectedly" says.
pub fn follow_setting(state: &std::sync::Arc<crate::state::AppState>, on: bool) {
    let Ok(mut watchdog) = state.watchdog.lock() else {
        return;
    };
    match (watchdog.is_some(), on) {
        (false, true) => *watchdog = spawn_watchdog(&state.data_dir),
        (true, false) => end_watchdog(&mut watchdog),
        _ => {}
    }
}

fn end_watchdog(watchdog: &mut Option<std::process::Child>) {
    if let Some(mut child) = watchdog.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// AI Juggler is quitting or restarting as asked (a quit, an update's
/// restart, a reopen): the run is marked clean and the watchdog ends
/// first, so it never outlives the app or takes a restart for a crash.
/// Tauri's `restart()` on the main thread relaunches without the exit
/// event, so its callers come here before it, as the exit path does.
pub fn before_exit(state: &crate::state::AppState) {
    end_run(&state.data_dir);
    if let Ok(mut watchdog) = state.watchdog.lock() {
        end_watchdog(&mut watchdog);
    }
}

/// Once the app is up after a crash: the sessions it ended are resumed
/// where restarting is on (each as a crash, counted like one).
pub fn resume_after_crash(state: &std::sync::Arc<crate::state::AppState>) {
    let ended = state
        .ended_by_crash
        .lock()
        .map(|mut ended| std::mem::take(&mut *ended))
        .unwrap_or_default();
    for session in ended {
        crate::features::sessions::restart::after_exit(
            state,
            &session,
            crate::features::sessions::restart::Ending::Crashed,
        );
    }
}

/// The reason a session that died with a crashed AI Juggler shows.
pub const QUIT_UNEXPECTEDLY_RULE: &str = "app_quit_unexpectedly";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_not_ended_as_asked_is_found_at_the_next_start() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!begin_run(dir.path()), "a first run");
        assert!(begin_run(dir.path()), "never ended: a crash");
        end_run(dir.path());
        assert!(!begin_run(dir.path()), "quit as asked");
    }

    #[test]
    fn three_relaunches_in_ten_minutes_then_no_more() {
        let now = 1_800_000_000_000;
        let minute = 60_000;
        assert!(may_relaunch(&[], now));
        assert!(may_relaunch(&[now - 2 * minute, now - minute], now));
        assert!(!may_relaunch(
            &[now - 3 * minute, now - 2 * minute, now - minute],
            now
        ));
        assert!(may_relaunch(
            &[now - 30 * minute, now - 20 * minute, now - 11 * minute],
            now
        ));
    }
}

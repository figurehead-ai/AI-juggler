//! Waits woken by the kernel, never by polling (AJ-E2324, the move
//! review's B4): a process's exit and a file's change.
//!
//! A move waits for two things after it quits the two apps: their
//! processes to end, and the files it touches to hold still. Polling them
//! (a process-table read every 250 ms, a walk of every file every 1.5 s)
//! cost about 1–2 s of CPU a move on a shared Mac and set a 1.5 s floor.
//! Here the kernel says when:
//!
//! - **macOS**: `kqueue` through nix's safe API (`nix::sys::event`):
//!   `EVFILT_PROC`/`NOTE_EXIT` on each pid, `EVFILT_VNODE` (write, extend,
//!   delete, rename) on the files and folders. No `unsafe` here.
//! - **Linux and Windows**: the same calls, with a bounded polling
//!   fallback ([`any_exit`] looks every [`LOOK`]; [`quiet_for`] waits its
//!   window and says [`Quiet::Unwatched`], so the caller's own look of
//!   sizes and times decides). Linux's pidfd and inotify, and Windows'
//!   wait handles and `ReadDirectoryChangesW`, go behind these same calls.
//!
//! A wake is a hint, never the truth: the caller checks the process's
//! identity (its start time) or the files' sizes and times after every
//! wake, so a lost or extra event costs time, never correctness.

use std::path::PathBuf;
use std::time::Duration;

/// The polling fallback's step, and the longest a wait sleeps when it
/// can't watch anything.
pub const LOOK: Duration = Duration::from_millis(100);

/// The most files and folders one [`quiet_for`] watches (each holds an
/// open descriptor while it waits): the paths given first, then the
/// folders below them, then their files. What is left out is the caller's
/// look's to catch.
pub const WATCH_MAX: usize = 32;

/// What [`quiet_for`] saw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quiet {
    /// A whole window passed with no change to anything watched.
    Calm,
    /// Changes kept coming until the cap.
    Busy,
    /// Nothing could be watched on this system: the window was waited,
    /// and the caller's own look decides.
    Unwatched,
}

/// Waits until one of `pids` ends (or is already gone), at most `cap`.
/// Returns whether one ended (`false`: the cap passed, or this system
/// couldn't tell; the caller looks again either way).
#[must_use]
pub fn any_exit(pids: &[u32], cap: Duration) -> bool {
    if pids.is_empty() || cap.is_zero() {
        return false;
    }
    imp::any_exit(pids, cap)
}

/// Waits until `window` passes with no change (a write, an append, a
/// delete or a rename) to any of `paths` or what is below them, at most
/// `cap` (each change starts the window again).
#[must_use]
pub fn quiet_for(paths: &[PathBuf], window: Duration, cap: Duration) -> Quiet {
    imp::quiet_for(&watch_list(paths), window, cap)
}

/// What [`quiet_for`] watches: `paths` as given, then the folders below
/// them (nearest first), then the files below them (the most recently
/// written first), at most [`WATCH_MAX`].
fn watch_list(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = paths.iter().take(WATCH_MAX).cloned().collect();
    let mut folders: std::collections::VecDeque<PathBuf> =
        paths.iter().filter(|p| p.is_dir()).cloned().collect();
    let mut files: Vec<(Option<std::time::SystemTime>, PathBuf)> = Vec::new();
    while let Some(dir) = folders.pop_front() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                if out.len() < WATCH_MAX && !out.contains(&path) {
                    out.push(path.clone());
                }
                folders.push_back(path);
            } else if kind.is_file() {
                let written = entry.metadata().ok().and_then(|m| m.modified().ok());
                files.push((written, path));
            }
        }
    }
    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    for (_, file) in files {
        if out.len() >= WATCH_MAX {
            break;
        }
        if !out.contains(&file) {
            out.push(file);
        }
    }
    out
}

#[cfg(target_os = "macos")]
mod imp {
    use std::os::fd::AsRawFd as _;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use nix::errno::Errno;
    use nix::sys::event::{EvFlags, EventFilter, FilterFlag, KEvent, Kqueue};

    use super::{LOOK, Quiet};

    fn timespec(d: Duration) -> nix::libc::timespec {
        nix::libc::timespec {
            tv_sec: nix::libc::time_t::try_from(d.as_secs()).unwrap_or(nix::libc::time_t::MAX),
            tv_nsec: nix::libc::c_long::from(d.subsec_nanos()),
        }
    }

    fn blank() -> KEvent {
        KEvent::new(
            0,
            EventFilter::EVFILT_USER,
            EvFlags::empty(),
            FilterFlag::empty(),
            0,
            0,
        )
    }

    pub(super) fn any_exit(pids: &[u32], cap: Duration) -> bool {
        let Ok(queue) = Kqueue::new() else {
            std::thread::sleep(cap.min(LOOK));
            return false;
        };
        let mut watched = 0;
        for pid in pids {
            let Ok(ident) = usize::try_from(*pid) else {
                continue;
            };
            let exit = KEvent::new(
                ident,
                EventFilter::EVFILT_PROC,
                EvFlags::EV_ADD | EvFlags::EV_ONESHOT,
                FilterFlag::NOTE_EXIT,
                0,
                0,
            );
            match queue.kevent(&[exit], &mut [], Some(timespec(Duration::ZERO))) {
                Ok(_) => watched += 1,
                // Gone before it could be watched: it ended.
                Err(Errno::ESRCH) => return true,
                Err(_) => {}
            }
        }
        if watched == 0 {
            std::thread::sleep(cap.min(LOOK));
            return false;
        }
        let mut out = vec![blank(); pids.len().min(16)];
        queue
            .kevent(&[], &mut out, Some(timespec(cap)))
            .is_ok_and(|n| n > 0)
    }

    pub(super) fn quiet_for(watch: &[PathBuf], window: Duration, cap: Duration) -> Quiet {
        let open: Vec<std::fs::File> = watch
            .iter()
            .filter_map(|path| std::fs::File::open(path).ok())
            .collect();
        let Ok(queue) = Kqueue::new() else {
            std::thread::sleep(window.min(cap));
            return Quiet::Unwatched;
        };
        let changes: Vec<KEvent> = open
            .iter()
            .filter_map(|file| usize::try_from(file.as_raw_fd()).ok())
            .map(|fd| {
                KEvent::new(
                    fd,
                    EventFilter::EVFILT_VNODE,
                    EvFlags::EV_ADD | EvFlags::EV_CLEAR,
                    FilterFlag::NOTE_WRITE
                        | FilterFlag::NOTE_EXTEND
                        | FilterFlag::NOTE_DELETE
                        | FilterFlag::NOTE_RENAME,
                    0,
                    0,
                )
            })
            .collect();
        if changes.is_empty()
            || queue
                .kevent(&changes, &mut [], Some(timespec(Duration::ZERO)))
                .is_err()
        {
            std::thread::sleep(window.min(cap));
            return Quiet::Unwatched;
        }
        let until = Instant::now() + cap;
        let mut out = vec![blank(); 16];
        loop {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Quiet::Busy;
            }
            let wait = window.min(left);
            match queue.kevent(&[], &mut out, Some(timespec(wait))) {
                Ok(0) if wait == window => return Quiet::Calm,
                Ok(0) => return Quiet::Busy,
                // A change (or a signal): the window starts again.
                Ok(_) | Err(Errno::EINTR) => {}
                Err(_) => {
                    std::thread::sleep(left.min(window));
                    return Quiet::Unwatched;
                }
            }
        }
    }
}

/// Where no file watcher is built yet (Linux's inotify, Windows' wait
/// handles): a bounded sleep, and the caller's look decides.
#[cfg(not(target_os = "macos"))]
mod unwatched {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::Quiet;

    pub(super) fn quiet_for(_: &[PathBuf], window: Duration, cap: Duration) -> Quiet {
        std::thread::sleep(window.min(cap));
        Quiet::Unwatched
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use std::time::{Duration, Instant};

    use nix::errno::Errno;
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    use super::LOOK;
    pub(super) use super::unwatched::quiet_for;

    // The bounded polling fallback (Linux's pidfd goes here): a look every
    // `LOOK` whether each pid still runs.
    pub(super) fn any_exit(pids: &[u32], cap: Duration) -> bool {
        let until = Instant::now() + cap;
        loop {
            let gone = pids.iter().any(|pid| {
                i32::try_from(*pid)
                    .is_ok_and(|raw| kill(Pid::from_raw(raw), None) == Err(Errno::ESRCH))
            });
            if gone {
                return true;
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            std::thread::sleep(left.min(LOOK));
        }
    }
}

#[cfg(not(unix))]
mod imp {
    use std::time::Duration;

    use super::LOOK;
    pub(super) use super::unwatched::quiet_for;

    // Windows' wait handles go here; until then a bounded sleep, and the
    // caller looks again.
    pub(super) fn any_exit(_: &[u32], cap: Duration) -> bool {
        std::thread::sleep(cap.min(LOOK));
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::process::{Child, Command};
    use std::time::Instant;

    /// How long `f` took and how much CPU this process used meanwhile (its
    /// user and system time, from `ps`): the waits' cost, measured.
    #[cfg(target_os = "macos")]
    fn cpu_of<T>(f: impl FnOnce() -> T) -> (T, Duration, Duration) {
        fn cpu() -> Duration {
            let out = std::process::Command::new("/bin/ps")
                .args(["-o", "time=", "-p", &std::process::id().to_string()])
                .output()
                .unwrap();
            // "m:ss.cc"
            let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            let (m, s) = text.split_once(':').unwrap();
            Duration::from_secs_f64(m.parse::<f64>().unwrap() * 60.0 + s.parse::<f64>().unwrap())
        }
        let (cpu0, t0) = (cpu(), Instant::now());
        let out = f();
        let (took, cpu1) = (t0.elapsed(), cpu());
        (out, took, cpu1.saturating_sub(cpu0))
    }

    fn sleeper(secs: &str) -> Child {
        Command::new("/bin/sleep").arg(secs).spawn().unwrap()
    }

    /// A process's exit wakes the wait at once (not at the next poll, not
    /// at the cap), and a pid already gone returns at once.
    #[test]
    fn an_exit_wakes_the_wait_at_once() {
        let mut child = sleeper("0.3");
        let pid = child.id();
        let reaper = std::thread::spawn(move || child.wait());
        let t0 = Instant::now();
        assert!(any_exit(&[pid], Duration::from_secs(10)));
        let took = t0.elapsed();
        reaper.join().unwrap().unwrap();
        assert!(took < Duration::from_secs(2), "{took:?}");
        // Gone now.
        let t0 = Instant::now();
        assert!(any_exit(&[pid], Duration::from_secs(10)));
        assert!(t0.elapsed() < Duration::from_secs(1));
    }

    /// One of several ending wakes it; none ending waits to the cap.
    #[test]
    fn the_first_of_several_exits_wakes_it_and_none_waits_to_the_cap() {
        let mut long = sleeper("30");
        let mut short = sleeper("0.2");
        let pids = [long.id(), short.id()];
        let reaper = std::thread::spawn(move || short.wait());
        assert!(any_exit(&pids, Duration::from_secs(10)));
        reaper.join().unwrap().unwrap();
        let t0 = Instant::now();
        assert!(!any_exit(&[long.id()], Duration::from_millis(300)));
        assert!(t0.elapsed() >= Duration::from_millis(250));
        long.kill().unwrap();
        long.wait().unwrap();
    }

    /// Files that hold still are calm after one window; a writer that
    /// keeps writing is busy at the cap, and one that stops is calm a
    /// window after its last write.
    #[test]
    fn quiet_files_are_calm_after_one_window_and_a_writer_keeps_it_busy() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("s.jsonl");
        std::fs::write(&file, b"{}\n").unwrap();
        let paths = vec![tmp.path().to_path_buf()];
        let window = Duration::from_millis(300);
        let t0 = Instant::now();
        let calm = quiet_for(&paths, window, Duration::from_secs(5));
        let took = t0.elapsed();
        assert_eq!(calm, Quiet::Calm);
        assert!(took >= window && took < Duration::from_secs(1), "{took:?}");

        let writing = file.clone();
        let writer = std::thread::spawn(move || {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(writing)
                .unwrap();
            for _ in 0..12 {
                std::thread::sleep(Duration::from_millis(100));
                f.write_all(b"{\"type\":\"cost-state\"}\n").unwrap();
            }
        });
        let busy = quiet_for(&paths, window, Duration::from_millis(800));
        assert_eq!(busy, Quiet::Busy);
        writer.join().unwrap();
        let t0 = Instant::now();
        assert_eq!(
            quiet_for(&paths, window, Duration::from_secs(5)),
            Quiet::Calm
        );
        assert!(t0.elapsed() < Duration::from_secs(1));
    }

    /// The watch list keeps the given paths first and stays bounded.
    #[test]
    fn the_watch_list_is_bounded_and_keeps_the_given_paths_first() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..100 {
            std::fs::write(tmp.path().join(format!("{i}.txt")), b"x").unwrap();
        }
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        let given = vec![tmp.path().to_path_buf()];
        let list = watch_list(&given);
        assert_eq!(list.len(), WATCH_MAX);
        assert_eq!(list[0], tmp.path());
        assert_eq!(list[1], tmp.path().join("sub"));
    }

    /// AJ-E2324 R4's CPU target, measured: the waits of a move (two apps'
    /// exits, then the files' quiet window) use at most 50 ms of CPU.
    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "a measurement: run alone (the whole test process's CPU is read)"]
    fn the_waits_use_under_50_ms_of_cpu() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("s.jsonl"), b"{}\n").unwrap();
        let mut a = sleeper("1");
        let mut b = sleeper("1.5");
        let pids = vec![a.id(), b.id()];
        let reapers = [
            std::thread::spawn(move || a.wait()),
            std::thread::spawn(move || b.wait()),
        ];
        let ((), took, cpu) = cpu_of(|| {
            let mut left: Vec<u32> = pids.clone();
            let until = Instant::now() + Duration::from_secs(10);
            while !left.is_empty() && Instant::now() < until {
                let _ = any_exit(&left, Duration::from_secs(1));
                left.retain(|pid| {
                    let pid = nix::unistd::Pid::from_raw(i32::try_from(*pid).unwrap());
                    nix::sys::signal::kill(pid, None).is_ok()
                });
            }
            let _ = quiet_for(
                &[tmp.path().to_path_buf()],
                Duration::from_millis(300),
                Duration::from_secs(9),
            );
        });
        for r in reapers {
            r.join().unwrap().unwrap();
        }
        eprintln!("waits: {took:?} wall, {cpu:?} CPU");
        assert!(cpu <= Duration::from_millis(50), "{cpu:?}");
    }
}

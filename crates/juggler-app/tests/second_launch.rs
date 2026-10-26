//! AJ-E1010 — a second launch while AI Juggler holds its data folder
//! (`juggler.lock`) hands over and never opens the data: no database
//! created, no migration, no recovery. The real binary is started as the
//! second launch, against a throwaway home whose data folder this test
//! holds the lock on (as a running AI Juggler would); its dialogs go to
//! stderr (`AIJ_NO_DIALOGS`).
//!
//! Nothing answers this launch's hand-over (the test only holds the lock,
//! like an AI Juggler that is still starting up), so it becomes the
//! single-instance plugin's "running instance" for a moment: it must let
//! go before it reports and quits — on macOS, no socket is left behind —
//! or the real one, once ready, would hand itself over to it and exit.
//!
//! A plain `cargo build` carries AI Juggler's own identifier, so its
//! single-instance socket is the installed app's: with AI Juggler open,
//! the test is skipped rather than bring it forward.

#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

/// The launched app, killed when the test ends however it ends (a failed
/// assertion included): nothing it started outlives the test. It leads
/// its own process group ([`launch`]), so whatever it started (a fresh
/// launch from Try again) goes with it.
struct Reaped(Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        kill_group(self.0.id());
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Ends process group `pgid` (a launch and all it started). Written
/// `kill -s KILL -- -<pgid>`: Linux's `kill` reads `-9 -<pgid>` as a
/// process, not a group (what hung main's load_budgets).
fn kill_group(pgid: u32) {
    let _ = Command::new("kill")
        .args(["-s", "KILL", "--", &format!("-{pgid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Starts a launch at the head of its own process group, its stderr read
/// by a [`Tap`] (each line also to `on_line`).
fn launch(command: &mut Command, on_line: impl FnMut(&str) + Send + 'static) -> (Reaped, Tap) {
    let mut child = Reaped(
        std::os::unix::process::CommandExt::process_group(command, 0)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let tap = Tap::new(child.0.stderr.take().unwrap(), on_line);
    (child, tap)
}

/// A launch's stderr, read on its own thread. Taken within a bound, never
/// by waiting for the pipe to close: anything the launch started (a fresh
/// launch, a D-Bus daemon a Linux runner starts on demand) can hold it
/// open for ever — #132's Linux CI hung 80 minutes on that.
struct Tap {
    text: Arc<Mutex<String>>,
    closed: mpsc::Receiver<()>,
}

/// How long a launch's stderr may stay open after the launch ended.
const TAP_GRACE: Duration = Duration::from_secs(10);

impl Tap {
    fn new(
        stderr: std::process::ChildStderr,
        mut on_line: impl FnMut(&str) + Send + 'static,
    ) -> Self {
        let text = Arc::new(Mutex::new(String::new()));
        let (done, closed) = mpsc::channel();
        let into = Arc::clone(&text);
        std::thread::spawn(move || {
            for line in
                std::io::BufRead::lines(std::io::BufReader::new(stderr)).map_while(Result::ok)
            {
                on_line(&line);
                let mut text = into.lock().unwrap_or_else(PoisonError::into_inner);
                text.push_str(&line);
                text.push('\n');
            }
            let _ = done.send(());
        });
        Self { text, closed }
    }

    /// What it said: once the pipe closed, or after `within` at most.
    fn text(&self, within: Duration) -> String {
        let _ = self.closed.recv_timeout(within);
        self.text
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// One test at a time: every launch here shares the single-instance
/// plugin's socket, so a launch in one test could hand over to another's.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Why these tests can't run here and now, said once per test: an AI
/// Juggler build is open (it shares the single-instance socket), or the
/// shared target's app binary isn't this build's any more.
fn cannot_run() -> bool {
    if ai_juggler_is_open() {
        eprintln!(
            "skipped: an AI Juggler build is open, and this build shares its single-instance name"
        );
        return true;
    }
    if !binary_is_this_builds() {
        eprintln!(
            "skipped: another checkout rebuilt the shared target's juggler-app after this test was built; run `cargo test -p juggler-app --test second_launch` alone"
        );
        return true;
    }
    false
}

/// Whether `target/debug/juggler-app` is still the one cargo built with
/// this test: not much newer than this test binary itself. Checkouts on
/// this Mac share one target folder, and another session's build can
/// replace it minutes into a full run (the coordinator's, and this
/// branch's, intermittent failures: the launch ran older code).
fn binary_is_this_builds() -> bool {
    let modified = |path: &std::path::Path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    };
    let (Some(app), Some(test)) = (
        modified(std::path::Path::new(env!("CARGO_BIN_EXE_juggler-app"))),
        std::env::current_exe().ok().as_deref().and_then(modified),
    ) else {
        return true;
    };
    app.duration_since(test)
        .map_or(true, |newer| newer < Duration::from_secs(120))
}

/// The app binary these tests launch: a copy of this build's, taken once,
/// as soon as the first test runs. Checkouts on this Mac share one target
/// folder, so `target/debug/juggler-app` is whichever checkout built it
/// last; another session's build could swap it mid-run and these tests
/// would launch its older code (the intermittent failures in full runs).
///
/// A static is never dropped, so a `TempDir` here leaked a 150 MB copy per
/// run (53 found in `$TMPDIR`, 2026-10-25). The copies now live in one
/// folder, named after the test process, and each run removes the copies
/// whose process has ended (never another checkout's running copy).
fn app_binary() -> &'static std::path::Path {
    static COPY: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    COPY.get_or_init(|| {
        let dir = std::env::temp_dir().join("aij-second-launch-bin");
        std::fs::create_dir_all(&dir).unwrap();
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let pid = entry
                .file_name()
                .to_string_lossy()
                .strip_prefix("juggler-app-")
                .and_then(|pid| pid.parse::<u32>().ok());
            let ended = pid.is_some_and(|pid| {
                !Command::new("/bin/kill")
                    .args(["-0", &pid.to_string()])
                    .stderr(Stdio::null())
                    .status()
                    .is_ok_and(|status| status.success())
            });
            if ended {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let copy = dir.join(format!("juggler-app-{}", std::process::id()));
        std::fs::copy(env!("CARGO_BIN_EXE_juggler-app"), &copy).unwrap();
        copy
    })
}

/// Where every checkout's second-launch tests queue: beside the
/// single-instance socket they all share.
const SERIAL_LOCK: &str = "/tmp/aij-second-launch-tests.lock";

/// How long a test waits for another checkout's to finish: all seven at
/// their limits.
const SERIAL_WAIT: Duration = Duration::from_secs(30 * 60);

/// One test at a time: in this binary (its mutex), and across every
/// checkout on this Mac (a file lock). Two checkouts' launches share the
/// single-instance socket, so one could hand over to the other's: the
/// coordinator saw all five fail in a full run beside another checkout's.
struct Serial {
    _local: std::sync::MutexGuard<'static, ()>,
    _machine: std::fs::File,
}

fn serial() -> Serial {
    let local = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let machine = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(SERIAL_LOCK)
        .unwrap();
    // Released when the file closes, even if a test panics. Bounded: a
    // checkout whose tests hang can't hold every other one for ever.
    let until = Instant::now() + SERIAL_WAIT;
    while machine.try_lock().is_err() {
        assert!(
            Instant::now() < until,
            "another checkout's second-launch tests held {SERIAL_LOCK} for {SERIAL_WAIT:?}"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
    Serial {
        _local: local,
        _machine: machine,
    }
}

/// Where a debug build (AI Juggler Dev) keeps its data under `home`.
fn dev_data_dir(home: &std::path::Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/AI Juggler Dev")
    } else {
        home.join(".local/share/AI Juggler Dev")
    }
}

/// The single-instance socket for this build's identifier (macOS).
#[cfg(target_os = "macos")]
const SOCKET: &str = "/tmp/ai_figurehead_juggler_si.sock";

fn ai_juggler_is_open() -> bool {
    // Executables only (`comm`): a command line that merely mentions the
    // app (a shell, an editor) doesn't count. Another checkout's build
    // counts too: it answers on the same single-instance socket, so a
    // launch here would hand over to it (two sessions' gates at once).
    // This test's own launches are all reaped before the next test. And
    // whatever answers on the single-instance socket (an AI Juggler Dev
    // another session has open) would take these launches' hand-overs.
    #[cfg(target_os = "macos")]
    if std::os::unix::net::UnixStream::connect(SOCKET).is_ok() {
        return true;
    }
    std::process::Command::new("ps")
        .args(["-axo", "comm="])
        .output()
        .is_ok_and(|out| {
            String::from_utf8_lossy(&out.stdout).lines().any(|exe| {
                exe.contains("AI Juggler.app/Contents/MacOS/") || exe.ends_with("/juggler-app")
            })
        })
}

#[test]
fn a_second_launch_never_opens_the_data() {
    let _serial = serial();
    if cannot_run() {
        return;
    }
    // A socket left by an AI Juggler that crashed (none is open now) would
    // look like this launch's: clear it first.
    #[cfg(target_os = "macos")]
    let _ = std::fs::remove_file(SOCKET);
    let home = tempfile::tempdir().unwrap();
    let data = dev_data_dir(home.path());
    std::fs::create_dir_all(&data).unwrap();
    // The running AI Juggler: it holds the data folder.
    let running = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(data.join("juggler.lock"))
        .unwrap();
    running.try_lock().unwrap();

    let (mut second, tap) = launch(
        Command::new(app_binary())
            .env("HOME", home.path())
            .env("XDG_DATA_HOME", home.path().join(".local/share"))
            .env("AIJ_NO_DIALOGS", "1"),
        |_| {},
    );
    let second = &mut second.0;
    let deadline = Instant::now() + LAUNCH_LIMIT;
    let status = loop {
        if let Some(status) = second.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = second.kill();
            panic!("the second launch didn't end within {LAUNCH_LIMIT:?}");
        }
        // It must never create anything there (a database would be the
        // first sign it opened the data): stop it at once if it does.
        if std::fs::read_dir(&data).unwrap().count() > 1 {
            let _ = second.kill();
            let _ = second.wait();
            panic!("the second launch opened the data folder");
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let stderr = tap.text(TAP_GRACE);

    let left: Vec<String> = std::fs::read_dir(&data)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        left,
        ["juggler.lock"],
        "the second launch touched the data folder (status {status}): {stderr}"
    );
    assert!(
        !status.success(),
        "nothing answered, so it can't have handed over"
    );
    assert!(
        stderr.contains("isn't answering") && stderr.contains("Force Quit")
            || !cfg!(target_os = "macos"),
        "it says what to do: {stderr}"
    );
    #[cfg(target_os = "macos")]
    assert!(
        !std::path::Path::new(SOCKET).exists(),
        "it let go of the single-instance socket before it quit"
    );
    drop(running);
}

/// Starts the real binary as a second launch against `home` (whose data
/// folder the caller holds) and returns its status, stderr and how long
/// it took.
fn second_launch(home: &std::path::Path) -> (std::process::ExitStatus, String, Duration) {
    let mut command = Command::new(app_binary());
    command
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("AIJ_NO_DIALOGS", "1");
    bounded(command)
}

/// How long a launch may take before it counts as hung. A debug build
/// starts in seconds on a quiet Mac, and in about a minute beside other
/// sessions' builds (the coordinator's full runs).
const LAUNCH_LIMIT: Duration = Duration::from_secs(180);

/// Runs a launch to its end, never past [`LAUNCH_LIMIT`]: past it, the
/// launch is killed and the test fails, never left running.
fn bounded(mut command: Command) -> (std::process::ExitStatus, String, Duration) {
    let started = Instant::now();
    let (mut child, tap) = launch(&mut command, |_| {});
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > LAUNCH_LIMIT {
            kill_group(child.0.id());
            panic!(
                "the launch didn't end within {LAUNCH_LIMIT:?}: {}",
                tap.text(TAP_GRACE)
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let took = started.elapsed();
    // Anything it left in its group ends now: its stderr closes.
    kill_group(child.0.id());
    (status, tap.text(TAP_GRACE), took)
}

/// #132's Linux CI: a launch ended, but something it started kept its
/// stderr open, and the test waited on the pipe for 80 minutes. Here a
/// helper in its group and one that left it (`setsid`, as a D-Bus daemon
/// started on demand does) both hold it: the test still has its words
/// within the grace, and the helper in its group is ended.
#[test]
fn a_launch_whose_helpers_hold_its_stderr_never_hangs_the_test() {
    let mut command = Command::new("sh");
    command.args([
        "-c",
        // An odd duration of its own: another `sleep 120` on the Mac
        // (any other program's) must not read as this helper left behind.
        "sleep 120.731 & python3 -c 'import os, time; os.setsid(); time.sleep(20)' & echo said >&2; exit 3",
    ]);
    let began = Instant::now();
    let (status, stderr, _) = bounded(command);
    assert_eq!(status.code(), Some(3));
    assert!(stderr.contains("said"), "{stderr}");
    // The helper that left holds the pipe for 20 s: waiting on the pipe
    // would take that long.
    assert!(
        began.elapsed() < TAP_GRACE + Duration::from_secs(5),
        "{:?}",
        began.elapsed()
    );
    let left = Command::new("pgrep")
        .args(["-f", "^sleep 120.731$"])
        .output()
        .unwrap();
    assert!(left.stdout.is_empty(), "the helper in its group was ended");
}

/// A home whose data folder is held, as by a running AI Juggler.
fn held_home() -> (tempfile::TempDir, PathBuf, std::fs::File) {
    let home = tempfile::tempdir().unwrap();
    let data = dev_data_dir(home.path());
    std::fs::create_dir_all(&data).unwrap();
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(data.join("juggler.lock"))
        .unwrap();
    lock.try_lock().unwrap();
    (home, data, lock)
}

/// 0.0.11 walk (Q8): the open AI Juggler is frozen — its hand-over socket
/// accepts but never answers. The second launch waits about 3 s, then
/// says it isn't answering, and quits without touching the data.
#[test]
fn a_second_launch_says_a_frozen_app_isnt_answering() {
    let _serial = serial();
    if cannot_run() {
        return;
    }
    let (home, data, lock) = held_home();
    // The frozen app's socket, where the app puts it: a 0700 directory of
    // ours in the per-user temp folder (the launch inherits our TMPDIR).
    let (socket, frozen) = frozen_socket(&data);
    let dir = socket.parent().unwrap().to_path_buf();
    let (status, stderr, took) = second_launch(home.path());
    if killed_from_outside(&status) {
        return;
    }
    assert!(!status.success(), "nothing answered: {stderr}");
    assert!(stderr.contains("isn't answering"), "it says so: {stderr}");
    assert!(
        took >= Duration::from_millis(2_900),
        "it waited for an answer: {took:?}"
    );
    assert!(took < LAUNCH_LIMIT, "it didn't hang: {took:?}");
    let left: Vec<String> = std::fs::read_dir(&data)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["juggler.lock"], "the data folder is untouched");
    let _ = std::fs::remove_file(&socket);
    let _ = std::fs::remove_dir(&dir);
    frozen.join().unwrap();
    drop(lock);
}

/// The open AI Juggler answers: the second launch quits at once, quietly.
#[test]
fn a_second_launch_hands_over_to_an_app_that_answers() {
    let _serial = serial();
    if cannot_run() {
        return;
    }
    let (home, data, lock) = held_home();
    let me = std::os::unix::fs::MetadataExt::uid(&std::fs::metadata(&data).unwrap());
    let socket = juggler_app::handover::socket_path("ai.figurehead.juggler", &data).unwrap();
    let handover = juggler_app::handover::Handover::at(socket.clone(), me);
    handover.serve(|| true).unwrap();
    let (status, stderr, took) = second_launch(home.path());
    if killed_from_outside(&status) {
        return;
    }
    assert!(status.success(), "it handed over ({status}): {stderr}");
    assert!(!stderr.contains("isn't answering"), "{stderr}");
    assert!(took < LAUNCH_LIMIT, "{took:?}");
    handover.stop();
    let _ = std::fs::remove_dir(socket.parent().unwrap());
    drop(lock);
}

/// A frozen app's hand-over socket: accepts every ask, never answers,
/// until its test is done (the socket's file is removed). Under load a
/// launch reaches it after a minute: with a fixed lifetime it found none,
/// handed over to no one and stayed running (the coordinator's run).
fn frozen_socket(data: &std::path::Path) -> (PathBuf, std::thread::JoinHandle<()>) {
    let socket = juggler_app::handover::socket_path("ai.figurehead.juggler", data).unwrap();
    std::os::unix::fs::DirBuilderExt::mode(&mut std::fs::DirBuilder::new(), 0o700)
        .create(socket.parent().unwrap())
        .unwrap();
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let path = socket.clone();
    let frozen = std::thread::spawn(move || {
        let mut held = Vec::new();
        let until = Instant::now() + LAUNCH_LIMIT + Duration::from_secs(60);
        while Instant::now() < until && path.exists() {
            if let Ok((stream, _)) = listener.accept() {
                held.push(stream); // read nothing, answer nothing
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
    (socket, frozen)
}

/// The processes whose environment carries `tag` (this test's launches,
/// the relaunched one included), killed; how many there were.
fn reap_tagged(tag: &str) -> usize {
    let out = std::process::Command::new("ps")
        .args(["eww", "-axo", "pid=,command="])
        .output()
        .unwrap();
    let pids: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|line| line.contains(tag) && !line.contains(" ps "))
        .filter_map(|line| line.split_whitespace().next().map(str::to_owned))
        .collect();
    for pid in &pids {
        let _ = std::process::Command::new("kill")
            .args(["-9", pid])
            .status();
    }
    pids.len()
}

/// Ends every process carrying the tag when dropped (see [`reap_tagged`]),
/// and the single-instance socket a killed launch leaves behind.
struct ReapTagged(String);

impl Drop for ReapTagged {
    fn drop(&mut self) {
        // Until nothing tagged is left and nothing of ours answers on the
        // single-instance socket: a relaunch that started late would take
        // the next test's hand-overs (seen under load).
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            let found = reap_tagged(&self.0);
            #[cfg(target_os = "macos")]
            let answering = std::os::unix::net::UnixStream::connect(SOCKET).is_ok();
            #[cfg(not(target_os = "macos"))]
            let answering = false;
            if (found == 0 && !answering) || Instant::now() >= until {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        std::thread::sleep(Duration::from_millis(200));
        // A launch killed mid-start leaves this build's socket: never the
        // installed app's (the test doesn't run while that's open).
        #[cfg(target_os = "macos")]
        if !ai_juggler_is_open() {
            let _ = std::fs::remove_file(SOCKET);
        }
    }
}

/// The "isn't answering" alert's two buttons, pressed on the real binary
/// (a debug build's `AIJ_DIALOG_ANSWER`): Quit quits, and touches nothing.
#[test]
fn the_isnt_answering_alerts_quit_quits() {
    let _serial = serial();
    if cannot_run() {
        return;
    }
    let (home, data, lock) = held_home();
    let (socket, frozen) = frozen_socket(&data);
    let mut command = Command::new(app_binary());
    command
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join(".local/share"))
        .env("AIJ_DIALOG_ANSWER", "Quit");
    let (status, stderr, _) = bounded(command);
    if killed_from_outside(&status) {
        return;
    }
    assert!(!status.success(), "it quit: {stderr}");
    assert!(
        stderr.contains("can't start yet") && stderr.contains("(answered: Quit)"),
        "{status}: {stderr}"
    );
    let left: Vec<String> = std::fs::read_dir(&data)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["juggler.lock"], "the data folder is untouched");
    let _ = std::fs::remove_file(&socket);
    let _ = std::fs::remove_dir(socket.parent().unwrap());
    frozen.join().unwrap();
    drop(lock);
}

/// Try again, once the open app has let go: this launch starts again in a
/// fresh one (the alert made it an app before Tauri's own) and quits.
#[test]
fn the_isnt_answering_alerts_try_again_starts_once_the_data_is_free() {
    let _serial = serial();
    if cannot_run() {
        return;
    }
    let (home, data, lock) = held_home();
    let (socket, frozen) = frozen_socket(&data);
    let tag = format!("AIJ_TEST_TAG=try-again-{}", std::process::id());
    // However the test ends, every process it started (the fresh launch
    // included) is ended, and the single-instance socket a fresh launch
    // may have taken goes with it.
    let _reaper = ReapTagged(tag.clone());
    let (key, value) = tag.split_once('=').unwrap();
    // The open app lets go once Try again is pressed.
    let mut lock = Some(lock);
    let (mut launched, tap) = launch(
        Command::new(app_binary())
            .env("HOME", home.path())
            .env("XDG_DATA_HOME", home.path().join(".local/share"))
            .env("AIJ_DIALOG_ANSWER", "Try again")
            .env(key, value),
        move |line| {
            if line.contains("(answered: Try again)") {
                drop(lock.take());
            }
        },
    );
    let deadline = Instant::now() + LAUNCH_LIMIT;
    let status = loop {
        if let Some(status) = launched.0.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            kill_group(launched.0.id());
            reap_tagged(&tag);
            panic!(
                "the launch didn't end within {LAUNCH_LIMIT:?}: {}",
                tap.text(TAP_GRACE)
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    // The data is held again at once, so the fresh launch (asking the
    // frozen socket for 3 s first) never starts the app for real: it's
    // found by its tag, and ended.
    // (If the fresh launch was quicker and took it, it starts the app for
    // real: that's ended below just the same.)
    let again = std::fs::OpenOptions::new()
        .write(true)
        .open(data.join("juggler.lock"))
        .unwrap();
    let _ = again.try_lock();
    let mut relaunched = 0;
    let until = Instant::now() + Duration::from_secs(60);
    // (Elsewhere the tag can't be seen: there, its word that it started
    // one is enough.)
    let said = || !cfg!(target_os = "macos") && tap.text(Duration::ZERO).contains("starting again");
    while relaunched == 0 && Instant::now() < until && !said() {
        std::thread::sleep(Duration::from_millis(100));
        relaunched = reap_tagged(&tag);
    }
    // The fresh launch is in this launch's group (Linux's `ps` shows no
    // environment, so the tag can't find it there): ended with it.
    kill_group(launched.0.id());
    let seen = tap.text(TAP_GRACE);
    if killed_from_outside(&status) {
        return;
    }
    assert!(seen.contains("(answered: Try again)"), "{status}: {seen}");
    assert!(status.success(), "it handed on to a fresh launch: {seen}");
    // On macOS the fresh launch is found by its tag. Elsewhere `ps` shows
    // no environment, and with no display (Linux CI) the fresh launch
    // ends at once: this launch's own word that it started one counts.
    assert!(
        relaunched >= 1 || (!cfg!(target_os = "macos") && seen.contains("starting again")),
        "a fresh launch started: {seen}"
    );
    drop(again);
    let _ = std::fs::remove_file(&socket);
    let _ = std::fs::remove_dir(socket.parent().unwrap());
    frozen.join().unwrap();
}

/// A quick double-click: the first launch holds the data and opens its
/// hand-over socket only after 2 s. The second waits for it and hands
/// over, never staying around without a window (the coordinator).
#[test]
fn a_launch_waits_for_one_still_starting_and_hands_over() {
    let _serial = serial();
    if cannot_run() {
        return;
    }
    let (home, data, lock) = held_home();
    let me = std::os::unix::fs::MetadataExt::uid(&std::fs::metadata(&data).unwrap());
    let socket = juggler_app::handover::socket_path("ai.figurehead.juggler", &data).unwrap();
    let handover = std::sync::Arc::new(juggler_app::handover::Handover::at(socket.clone(), me));
    let starting = {
        let handover = std::sync::Arc::clone(&handover);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            handover.serve(|| true).unwrap();
        })
    };
    let (status, stderr, _) = second_launch(home.path());
    starting.join().unwrap();
    if killed_from_outside(&status) || handed_to_another(&stderr) {
        return;
    }
    assert!(
        status.success(),
        "it handed over once the first was up ({status}): {stderr}"
    );
    assert!(!stderr.contains("isn't answering"), "{stderr}");
    handover.stop();
    let _ = std::fs::remove_dir(socket.parent().unwrap());
    drop(lock);
}

/// The data held and nothing answering at all: after its wait, the launch
/// shows the "isn't answering" alert, and Quit quits.
#[test]
fn with_nothing_answering_the_launch_asks_and_quit_quits() {
    let _serial = serial();
    if cannot_run() {
        return;
    }
    let (home, data, lock) = held_home();
    let mut command = Command::new(app_binary());
    command
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join(".local/share"))
        .env("AIJ_DIALOG_ANSWER", "Quit");
    let (status, stderr, took) = bounded(command);
    if killed_from_outside(&status) || handed_to_another(&stderr) {
        return;
    }
    assert!(!status.success(), "it quit: {stderr}");
    assert!(
        stderr.contains("can't start yet") && stderr.contains("(answered: Quit)"),
        "it asked ({status}): {stderr}"
    );
    assert!(
        took >= Duration::from_secs(10),
        "it waited for one starting first: {took:?}"
    );
    let left: Vec<String> = std::fs::read_dir(&data)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["juggler.lock"], "the data folder is untouched");
    #[cfg(target_os = "macos")]
    assert!(
        !std::path::Path::new(SOCKET).exists(),
        "no single-instance socket left"
    );
    drop(lock);
}

/// A launch something else on this Mac ended (SIGKILL, which no test
/// here sends while the launch runs): in a full run beside other sessions,
/// three launches ended with no output at all. Its result says nothing
/// about AI Juggler, so the test skips, saying so.
fn killed_from_outside(status: &std::process::ExitStatus) -> bool {
    let killed = std::os::unix::process::ExitStatusExt::signal(status) == Some(9);
    if killed {
        eprintln!("skipped: something outside this test killed the launch (SIGKILL)");
    }
    killed
}

/// Another session's AI Juggler build came up on the shared
/// single-instance socket while the launch waited, and the launch handed
/// over to it (as it should to an older AI Juggler): this test can't
/// tell its case apart then, so it says so and stops.
fn handed_to_another(stderr: &str) -> bool {
    let other = stderr.contains("an older AI Juggler answers on the single-instance socket");
    if other {
        eprintln!(
            "skipped: another AI Juggler build answered on the single-instance socket mid-test"
        );
    }
    other
}

#[cfg(target_os = "macos")]
/// This launch's process group: (pid, command line) of each member that
/// still runs (a zombie waiting to be reaped isn't one).
#[cfg(target_os = "macos")]
fn group_members(pgid: u32) -> Vec<(u32, String)> {
    let out = Command::new("ps")
        .args(["-axo", "pid=,pgid=,stat=,command="])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid: u32 = parts.next()?.parse().ok()?;
            let group: u32 = parts.next()?.parse().ok()?;
            let zombie = parts.next()?.starts_with('Z');
            (group == pgid && !zombie).then(|| (pid, parts.collect::<Vec<_>>().join(" ")))
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn wait_for(what: &str, limit: Duration, mut done: impl FnMut() -> bool) {
    let until = Instant::now() + limit;
    while !done() {
        assert!(Instant::now() < until, "{what} within {limit:?}");
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// AJ-E1509 Req.4 on the real binary: AI Juggler killed outright
/// (SIGKILL, so it never marks a clean quit) comes back by itself — its
/// watchdog sees it gone and starts it again, once. Everything it started
/// is ended with the test (the group).
#[test]
#[cfg(target_os = "macos")]
fn killed_outright_ai_juggler_comes_back_by_itself() {
    let _serial = serial();
    if cannot_run() {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let data = dev_data_dir(home.path());
    let (mut first, tap) = launch(
        Command::new(app_binary())
            .env("HOME", home.path())
            .env("XDG_DATA_HOME", home.path().join(".local/share"))
            .env("AIJ_NO_DIALOGS", "1")
            .env("AIJ_WATCHDOG", "1"),
        |_| {},
    );
    let pgid = first.0.id();
    let marker = || std::fs::read_to_string(data.join("last-run")).unwrap_or_default();
    let watchdogs = || {
        group_members(pgid)
            .into_iter()
            .filter(|(_, command)| command.contains("--aij-watch"))
            .count()
    };
    wait_for("the app up, its watchdog watching", LAUNCH_LIMIT, || {
        marker().trim() == "running" && watchdogs() >= 1
    });

    // Killed outright: no clean mark.
    kill_pid(pgid);
    let _ = first.0.wait();
    let apps = || {
        group_members(pgid)
            .into_iter()
            .filter(|(pid, command)| {
                *pid != pgid && !command.contains("--aij-watch") && command.contains("juggler-app")
            })
            .count()
    };
    wait_for("a new AI Juggler", LAUNCH_LIMIT, || apps() >= 1);
    let relaunches = std::fs::read_to_string(data.join("crash-relaunches")).unwrap_or_default();
    assert_eq!(
        relaunches.lines().count(),
        1,
        "started again once: {relaunches}"
    );
    wait_for("the new run's own watchdog", LAUNCH_LIMIT, || {
        watchdogs() >= 1
    });
    assert_eq!(marker().trim(), "running", "the new run holds the data");

    kill_group(pgid);
    wait_for("nothing of it left", Duration::from_secs(20), || {
        group_members(pgid).is_empty()
    });
    let _ = tap.text(Duration::ZERO);
    if !ai_juggler_is_open() {
        let _ = std::fs::remove_file(SOCKET);
    }
}

#[cfg(target_os = "macos")]
/// Kills one process, not its group.
#[cfg(target_os = "macos")]
fn kill_pid(pid: u32) {
    let _ = Command::new("kill")
        .args(["-s", "KILL", &pid.to_string()])
        .status();
}

/// AJ-E1509: after a quit as asked (the run marked clean), the watchdog
/// sees the app gone and ends by itself — it starts nothing.
#[test]
#[cfg(target_os = "macos")]
fn the_watchdog_ends_with_a_clean_quit_and_starts_nothing() {
    if !binary_is_this_builds() {
        return;
    }
    let data = tempfile::tempdir().unwrap();
    std::fs::write(data.path().join("last-run"), "clean\n").unwrap();
    // The "app": a process the watchdog watches.
    let mut app = Reaped(Command::new("sleep").arg("30").spawn().unwrap());
    let (mut watchdog, _tap) = launch(
        Command::new(app_binary())
            .arg("--aij-watch")
            .arg(app.0.id().to_string())
            .arg(data.path()),
        |_| {},
    );
    std::thread::sleep(Duration::from_millis(800));
    assert!(
        watchdog.0.try_wait().unwrap().is_none(),
        "it watches while the app runs"
    );
    kill_pid(app.0.id());
    let _ = app.0.wait();
    wait_for("the watchdog to end", Duration::from_secs(10), || {
        watchdog.0.try_wait().unwrap().is_some()
    });
    assert!(
        group_members(watchdog.0.id()).is_empty(),
        "nothing was started"
    );
    assert!(!data.path().join("crash-relaunches").exists());
}

/// AJ-E1509: an update's restart (or a reopen) goes through
/// `before_exit`, as a quit does: the watchdog is ended and the run marked
/// clean first, so the restart is never taken for a crash.
#[test]
fn before_a_restart_the_watchdog_ends_and_the_run_is_clean() {
    if !binary_is_this_builds() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("app-data");
    let state = juggler_app::state::AppState::init(data.clone()).unwrap();
    assert_eq!(
        std::fs::read_to_string(data.join("last-run"))
            .unwrap()
            .trim(),
        "running"
    );
    let child = juggler_app::features::app::crash::spawn_watchdog_with(app_binary(), &data)
        .expect("a watchdog");
    let pid = child.id();
    *state.watchdog.lock().unwrap() = Some(child);
    std::thread::sleep(Duration::from_millis(500));
    assert!(alive(pid), "it watches");
    juggler_app::features::app::crash::before_exit(&state);
    assert!(!alive(pid), "the watchdog ended before the restart");
    assert_eq!(
        std::fs::read_to_string(data.join("last-run"))
            .unwrap()
            .trim(),
        "clean"
    );
    assert!(state.watchdog.lock().unwrap().is_none());
}

/// Whether `pid` still runs (a reaped zombie doesn't).
fn alive(pid: u32) -> bool {
    let out = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    let stat = String::from_utf8_lossy(&out.stdout);
    !stat.trim().is_empty() && !stat.trim().starts_with('Z')
}

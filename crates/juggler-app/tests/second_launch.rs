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
use std::time::{Duration, Instant};

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
    // app (a shell, an editor) doesn't count.
    std::process::Command::new("ps")
        .args(["-axo", "comm="])
        .output()
        .is_ok_and(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .any(|exe| exe.contains("AI Juggler.app/Contents/MacOS/"))
        })
}

#[test]
fn a_second_launch_never_opens_the_data() {
    if ai_juggler_is_open() {
        eprintln!("skipped: AI Juggler is open, and this build shares its single-instance name");
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

    let mut second = std::process::Command::new(env!("CARGO_BIN_EXE_juggler-app"))
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join(".local/share"))
        .env("AIJ_NO_DIALOGS", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = second.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = second.kill();
            panic!("the second launch didn't end within a minute");
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
    let mut stderr = String::new();
    std::io::Read::read_to_string(&mut second.stderr.take().unwrap(), &mut stderr).unwrap();

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
    let started = Instant::now();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_juggler-app"))
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("AIJ_NO_DIALOGS", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .unwrap();
    (
        out.status,
        String::from_utf8_lossy(&out.stderr).into_owned(),
        started.elapsed(),
    )
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
    if ai_juggler_is_open() {
        eprintln!("skipped: AI Juggler is open, and this build shares its single-instance name");
        return;
    }
    let (home, data, lock) = held_home();
    // The frozen app's socket, where the app puts it: a 0700 directory of
    // ours in the per-user temp folder (the launch inherits our TMPDIR).
    let socket = juggler_app::handover::socket_path("ai.figurehead.juggler", &data).unwrap();
    let dir = socket.parent().unwrap().to_path_buf();
    std::os::unix::fs::DirBuilderExt::mode(&mut std::fs::DirBuilder::new(), 0o700)
        .create(&dir)
        .unwrap();
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let frozen = std::thread::spawn(move || {
        // Accept, read the ask, never reply (held well past the wait).
        let (mut stream, _) = listener.accept().unwrap();
        let mut ask = [0_u8; 6];
        let _ = std::io::Read::read_exact(&mut stream, &mut ask);
        std::thread::sleep(Duration::from_secs(6));
    });
    let (status, stderr, took) = second_launch(home.path());
    assert!(!status.success(), "nothing answered: {stderr}");
    assert!(stderr.contains("isn't answering"), "it says so: {stderr}");
    assert!(
        took >= Duration::from_millis(2_900),
        "it waited for an answer: {took:?}"
    );
    assert!(took < Duration::from_secs(20), "it didn't hang: {took:?}");
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
    if ai_juggler_is_open() {
        eprintln!("skipped: AI Juggler is open, and this build shares its single-instance name");
        return;
    }
    let (home, data, lock) = held_home();
    let me = std::os::unix::fs::MetadataExt::uid(&std::fs::metadata(&data).unwrap());
    let socket = juggler_app::handover::socket_path("ai.figurehead.juggler", &data).unwrap();
    let handover = juggler_app::handover::Handover::at(socket.clone(), me);
    handover.serve(|| true).unwrap();
    let (status, stderr, took) = second_launch(home.path());
    assert!(status.success(), "it handed over: {stderr}");
    assert!(!stderr.contains("isn't answering"), "{stderr}");
    assert!(took < Duration::from_secs(10), "{took:?}");
    handover.stop();
    let _ = std::fs::remove_dir(socket.parent().unwrap());
    drop(lock);
}

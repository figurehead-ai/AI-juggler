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

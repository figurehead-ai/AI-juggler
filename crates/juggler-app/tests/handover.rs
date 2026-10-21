//! 0.0.11 walk, Q8: a second launch waits for the running app's answer.
//! A socket that accepts but never replies (a frozen app) is "not
//! answering" after the wait; a live app answers; no socket falls back.
#![cfg(unix)]
#![allow(clippy::unwrap_used, reason = "test code")]

use std::io::Read;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use juggler_app::handover::{self, Answer};

fn socket(name: &str) -> (tempfile::TempDir, PathBuf) {
    // Short paths: a Unix socket path must stay under ~100 bytes.
    let dir = tempfile::Builder::new()
        .prefix("aij-ho")
        .tempdir_in("/tmp")
        .unwrap();
    let path = dir.path().join(name);
    (dir, path)
}

#[test]
fn a_socket_that_accepts_but_never_replies_is_not_answering() {
    let (_dir, path) = socket("frozen.sock");
    let listener = UnixListener::bind(&path).unwrap();
    // Accepts and reads, never answers — like a frozen app's socket.
    let held = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0_u8; 6];
        let _ = stream.read_exact(&mut buf);
        std::thread::sleep(Duration::from_secs(2));
    });
    let started = Instant::now();
    let answer = handover::ask(&path, Duration::from_millis(400));
    let waited = started.elapsed();
    assert_eq!(answer, Answer::NotAnswering);
    assert!(
        waited >= Duration::from_millis(350),
        "gave up too soon: {waited:?}"
    );
    assert!(
        waited < Duration::from_millis(1_500),
        "waited too long: {waited:?}"
    );
    held.join().unwrap();
}

#[test]
fn a_running_app_answers_once_it_came_forward() {
    let (_dir, path) = socket("live.sock");
    let raised = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let count = std::sync::Arc::clone(&raised);
    handover::serve(&path, move || {
        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        true
    });
    assert_eq!(
        handover::ask(&path, handover::ANSWER_WAIT),
        Answer::Answered
    );
    assert_eq!(
        handover::ask(&path, handover::ANSWER_WAIT),
        Answer::Answered
    );
    assert_eq!(raised.load(std::sync::atomic::Ordering::SeqCst), 2);
    handover::stop(&path);
    assert!(!path.exists());
}

#[test]
fn an_app_whose_window_could_not_come_forward_stays_silent() {
    let (_dir, path) = socket("stuck.sock");
    handover::serve(&path, || false);
    assert_eq!(
        handover::ask(&path, Duration::from_millis(400)),
        Answer::NotAnswering
    );
    handover::stop(&path);
}

#[test]
fn no_socket_falls_back_to_the_plugins_hand_over() {
    let (_dir, path) = socket("none.sock");
    assert_eq!(
        handover::ask(&path, Duration::from_millis(400)),
        Answer::NoneListening
    );
    // A file left by a crashed run, nobody listening: the same.
    drop(UnixListener::bind(&path).unwrap());
    assert!(path.exists());
    assert_eq!(
        handover::ask(&path, Duration::from_millis(400)),
        Answer::NoneListening
    );
}

#[test]
fn the_socket_is_per_data_folder_and_short() {
    let a = handover::socket_path(
        "ai.figurehead.juggler.dev",
        std::path::Path::new("/Users/a/Library/Application Support/AI Juggler Dev"),
    );
    let b = handover::socket_path(
        "ai.figurehead.juggler.dev",
        std::path::Path::new("/private/tmp/x/home/Library/Application Support/AI Juggler Dev"),
    );
    assert_ne!(a, b, "two homes, two sockets");
    assert_eq!(
        a,
        handover::socket_path(
            "ai.figurehead.juggler.dev",
            std::path::Path::new("/Users/a/Library/Application Support/AI Juggler Dev")
        )
    );
    assert!(a.as_os_str().len() < 100, "{}", a.display());
    assert!(a.starts_with("/tmp"));
}

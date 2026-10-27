//! 0.0.11 walk, Q8: a second launch waits for the running app's answer,
//! and the hand-over socket follows the local-IPC rules (SECURITY-GUIDELINES
//! C7): a 0700 directory of ours, a 0600 socket, peers checked by uid,
//! stale sockets removed only when ours, links never followed, and an
//! answer trusted only from a socket we own.
#![cfg(unix)]
#![allow(clippy::unwrap_used, reason = "test code")]

use std::io::Read;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use juggler_app::handover::{self, Answer, Handover, HandoverError};
use juggler_platform::user::{UserId, peer_allowed};

/// A fresh, short base folder (a socket path must stay under ~100 bytes)
/// and this user's uid.
fn base() -> (tempfile::TempDir, UserId) {
    let dir = tempfile::Builder::new()
        .prefix("aij-ho")
        .tempdir_in("/tmp")
        .unwrap();
    let me = UserId::from_uid(std::fs::metadata(dir.path()).unwrap().uid());
    (dir, me)
}

/// A socket path inside a `0700` directory of ours, as `serve` makes it.
fn our_socket(base: &Path) -> PathBuf {
    let dir = base.join("aij-test");
    std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    dir.join("ho.sock")
}

fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path).unwrap().mode() & 0o777
}

#[test]
fn a_socket_that_accepts_but_never_replies_is_not_answering() {
    let (base, me) = base();
    let socket = our_socket(base.path());
    let listener = UnixListener::bind(&socket).unwrap();
    // Accepts and reads, never answers — like a frozen app's socket.
    let held = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0_u8; 6];
        let _ = stream.read_exact(&mut buf);
        std::thread::sleep(Duration::from_secs(2));
    });
    let started = Instant::now();
    let answer = Handover::at(socket, me).ask(Duration::from_millis(400));
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
fn a_running_app_answers_once_it_came_forward_from_a_0600_socket_in_a_0700_dir() {
    let (base, me) = base();
    let socket = base.path().join("aij-new").join("ho.sock");
    let raised = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let count = std::sync::Arc::clone(&raised);
    let handover = Handover::at(socket.clone(), me);
    handover
        .serve(move || {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            true
        })
        .unwrap();
    assert_eq!(
        mode(socket.parent().unwrap()),
        0o700,
        "the directory is ours alone"
    );
    assert_eq!(mode(&socket), 0o600, "the socket is ours alone");
    assert_eq!(handover.ask(handover::ANSWER_WAIT), Answer::Answered);
    assert_eq!(handover.ask(handover::ANSWER_WAIT), Answer::Answered);
    assert_eq!(raised.load(std::sync::atomic::Ordering::SeqCst), 2);
    handover.stop();
    assert!(!socket.exists());
}

#[test]
fn an_app_whose_window_could_not_come_forward_stays_silent() {
    let (base, me) = base();
    let handover = Handover::at(our_socket(base.path()), me);
    handover.serve(|| false).unwrap();
    assert_eq!(
        handover.ask(Duration::from_millis(400)),
        Answer::NotAnswering
    );
    handover.stop();
}

#[test]
fn no_socket_falls_back_to_the_plugins_hand_over() {
    let (base, me) = base();
    let socket = our_socket(base.path());
    assert_eq!(
        Handover::at(socket.clone(), me).ask(Duration::from_millis(400)),
        Answer::NoneListening
    );
    // A socket file of ours left by a crashed run, nobody listening: the same.
    drop(UnixListener::bind(&socket).unwrap());
    assert!(socket.exists());
    assert_eq!(
        Handover::at(socket, me).ask(Duration::from_millis(400)),
        Answer::NoneListening
    );
}

#[test]
fn a_stale_socket_of_ours_is_cleaned_up_and_served_again() {
    let (base, me) = base();
    let socket = our_socket(base.path());
    drop(UnixListener::bind(&socket).unwrap());
    let handover = Handover::at(socket, me);
    handover.serve(|| true).unwrap();
    assert_eq!(handover.ask(handover::ANSWER_WAIT), Answer::Answered);
    handover.stop();
}

#[test]
fn a_symlink_at_the_socket_is_never_followed_or_removed() {
    let (base, me) = base();
    let socket = our_socket(base.path());
    let target = base.path().join("precious.txt");
    std::fs::write(&target, "keep me").unwrap();
    std::os::unix::fs::symlink(&target, &socket).unwrap();
    let handover = Handover::at(socket.clone(), me);
    assert!(matches!(
        handover.serve(|| true),
        Err(HandoverError::NotOurs(_))
    ));
    assert_eq!(handover.ask(Duration::from_millis(400)), Answer::Untrusted);
    handover.stop();
    assert!(
        std::fs::symlink_metadata(&socket)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the link stays"
    );
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "keep me",
        "and what it points at"
    );
}

#[test]
fn someone_elses_socket_or_directory_is_refused() {
    let (base, me) = base();
    let socket = our_socket(base.path());
    let listener = UnixListener::bind(&socket).unwrap();
    // Owned by another user (simulated: we ask as a uid that isn't the owner).
    let stranger = UserId::from_uid(me.uid().wrapping_add(1));
    let as_stranger = Handover::at(socket.clone(), stranger);
    assert_eq!(
        as_stranger.ask(Duration::from_millis(400)),
        Answer::Untrusted,
        "not asked"
    );
    assert!(matches!(
        as_stranger.serve(|| true),
        Err(HandoverError::NotOurs(_))
    ));
    as_stranger.stop();
    assert!(socket.exists(), "someone else's socket is never removed");
    drop(listener);

    // A directory anyone else can write to isn't trusted, nor chmod'ed into shape.
    let open_dir = base.path().join("aij-open");
    std::fs::create_dir(&open_dir).unwrap();
    std::fs::set_permissions(&open_dir, std::fs::Permissions::from_mode(0o777)).unwrap();
    let in_open = Handover::at(open_dir.join("ho.sock"), me);
    assert!(matches!(
        in_open.serve(|| true),
        Err(HandoverError::NotOurs(_))
    ));
    drop(UnixListener::bind(open_dir.join("ho.sock")).unwrap());
    assert_eq!(in_open.ask(Duration::from_millis(400)), Answer::Untrusted);
    assert_eq!(mode(&open_dir), 0o777, "left as it was");
}

#[test]
fn only_this_user_is_answered() {
    // The hand-over's listener answers through this rule (local_ipc).
    let me = UserId::from_uid(501);
    assert!(peer_allowed(Some(me), me));
    assert!(!peer_allowed(Some(UserId::from_uid(502)), me));
    assert!(
        !peer_allowed(None, me),
        "a peer we can't identify isn't answered"
    );
}

#[test]
fn the_socket_is_per_data_folder_short_and_in_the_users_temp_folder() {
    let temp = Path::new("/var/folders/zz/abcdefghijklmnopqrstuvwxyz0123/T");
    let a = handover::socket_path_in(
        temp,
        "ai.figurehead.juggler.dev",
        Path::new("/Users/a/Library/Application Support/AI Juggler Dev"),
    )
    .unwrap();
    let b = handover::socket_path_in(
        temp,
        "ai.figurehead.juggler.dev",
        Path::new("/private/tmp/x/home/Library/Application Support/AI Juggler Dev"),
    )
    .unwrap();
    let c = handover::socket_path_in(
        temp,
        "ai.figurehead.juggler",
        Path::new("/Users/a/Library/Application Support/AI Juggler Dev"),
    )
    .unwrap();
    assert_ne!(a, b, "two homes, two sockets");
    assert_ne!(a, c, "two editions, two sockets");
    assert_eq!(
        a,
        handover::socket_path_in(
            temp,
            "ai.figurehead.juggler.dev",
            Path::new("/Users/a/Library/Application Support/AI Juggler Dev")
        )
        .unwrap()
    );
    assert!(a.starts_with(temp));
    assert!(
        a.as_os_str().len() < juggler_platform::local_ipc::MAX_ENDPOINT_PATH,
        "{}",
        a.display()
    );
    // A temp folder too long for a socket address: no socket (the plugin's
    // hand-over is used).
    let long = PathBuf::from(format!("/{}", "x".repeat(90)));
    assert_eq!(
        handover::socket_path_in(&long, "ai.figurehead.juggler", Path::new("/d")),
        None
    );
}

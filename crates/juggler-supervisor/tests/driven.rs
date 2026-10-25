//! AJ-E2302: a driven session's process is owned like any other: the
//! supervisor spawns it (its own group, in the journal), hands its stdin
//! and stdout to the codec, reports its exit, and stops it. Never a PTY.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use juggler_adapter::{EnvMap, LaunchSpec};
use juggler_core::model::{MachineId, SessionId};
use juggler_supervisor::tree::SpawnContext;
use juggler_supervisor::{LifecycleEvent, Supervisor, process_alive};

fn fake_harness_bin() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_fake-harness") {
        return PathBuf::from(path);
    }
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    for profile in ["debug", "release"] {
        let candidate = target.join(profile).join("fake-harness");
        if candidate.exists() {
            return candidate;
        }
    }
    unreachable!("fake-harness binary not found under {}", target.display());
}

fn ctx() -> SpawnContext {
    SpawnContext {
        provider: "fake".into(),
        account_id: Some("acc-driven".into()),
        project_id: None,
        machine: MachineId::new("machine-driven"),
        role: "root",
    }
}

fn drive_spec(dir: &std::path::Path, needs_pty: bool) -> LaunchSpec {
    LaunchSpec {
        program: fake_harness_bin(),
        args: vec![
            "drive".into(),
            "--dir".into(),
            dir.join("transcripts").to_string_lossy().into_owned(),
            "--session-id".into(),
            "driven-1".into(),
        ],
        env: EnvMap::new(),
        cwd: dir.to_path_buf(),
        needs_pty,
    }
}

#[test]
fn a_driven_session_is_spawned_owned_and_its_pipes_handed_over() {
    let dir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::with_journal(dir.path()).unwrap();
    let session = SessionId::new("driven-session");
    assert!(
        supervisor
            .spawn_driven(&session, &drive_spec(dir.path(), true), &ctx())
            .is_err(),
        "a driven session never asks for a PTY"
    );
    let (spawned, pipes) = supervisor
        .spawn_driven(&session, &drive_spec(dir.path(), false), &ctx())
        .unwrap();
    assert!(supervisor.is_supervised(&session));
    assert_eq!(spawned.pgid, Some(spawned.pid), "its own process group");
    let mut stdin = pipes.stdin;
    let mut lines = BufReader::new(pipes.stdout).lines();
    let ready = lines.next().unwrap().unwrap();
    assert!(ready.contains(r#""ev":"ready""#), "{ready}");
    writeln!(stdin, r#"{{"op":"turn","text":"hello"}}"#).unwrap();
    stdin.flush().unwrap();
    let mut seen = Vec::new();
    while let Some(Ok(line)) = lines.next() {
        let end = line.contains("turn_end");
        seen.push(line);
        if end {
            break;
        }
    }
    assert!(seen.iter().any(|line| line.contains("turn_started")));
    // Closing stdin ends the tool; the supervisor reports the exit.
    drop(stdin);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut exited = false;
    while Instant::now() < deadline && !exited {
        if let Some(LifecycleEvent::Exited { session: s, .. }) =
            supervisor.recv_timeout(Duration::from_millis(100))
        {
            exited = s == session;
        }
    }
    assert!(exited, "the exit is reported");
    assert!(process_alive(spawned.pid).is_none());
}

#[test]
fn a_driven_session_is_stopped_with_its_session() {
    let dir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::with_journal(dir.path()).unwrap();
    let session = SessionId::new("driven-stop");
    let (spawned, pipes) = supervisor
        .spawn_driven(&session, &drive_spec(dir.path(), false), &ctx())
        .unwrap();
    assert!(process_alive(spawned.pid).is_some());
    supervisor.stop(&session, Duration::from_secs(2)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while process_alive(spawned.pid).is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(process_alive(spawned.pid).is_none(), "never left behind");
    drop(pipes);
}

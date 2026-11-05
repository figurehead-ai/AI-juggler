//! 0.3.9: the maintainer's failed switch of 2026-11-04, through AI
//! Juggler's own switch command at its real size: an 800 MB Claude app
//! conversation with a 1,719-file folder and 904 file-history files,
//! still written to while it switches. On 0.3.8 every such switch failed
//! with "Something went wrong"; the fixtures were a few kilobytes.
//!
//! It writes about 1.3 GB to the temp folder (removed after), so it's
//! opt-in:
//!
//! ```text
//! cargo test -p juggler-app --release --test switch_real_size -- --ignored --nocapture
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "support/set_up.rs"]
mod set_up;
#[path = "../../juggler-supervisor/tests/fixtures/stand_in.rs"]
mod stand_in;

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use juggler_adapter::AdapterRegistry;
use juggler_adapter::handover::{mangled_dir, sha256_file};
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;

const SESSION: &str = "1a967018-3bce-4925-b01a-0b46b11a35ea";

fn app_state(dir: &Path) -> Arc<AppState> {
    let stub = dir.join("claude-stub");
    stand_in::write(&stub, "#!/bin/sh\nexit 0\n");
    let mut registry = AdapterRegistry::new();
    let claude = Arc::new(ClaudeCodeAdapter::with_bin(&stub));
    registry.register(claude.clone());
    registry.register_handover(claude);
    Arc::new(AppState::init_with_registry(dir.join("app-data"), registry).unwrap())
}

fn config_dir_of(state: &Arc<AppState>, account_id: &str) -> PathBuf {
    state.accounts_dir().join(CLAUDE_CODE_ID).join(account_id)
}

fn turn(n: usize, cwd: &str, content: &str) -> String {
    serde_json::json!({
        "uuid": format!("t{n}"),
        "parentUuid": n.checked_sub(1).map(|p| format!("t{p}")),
        "sessionId": SESSION,
        "cwd": cwd,
        "type": if n.is_multiple_of(2) { "user" } else { "assistant" },
        "message": {"role": if n.is_multiple_of(2) { "user" } else { "assistant" }, "content": content},
    })
    .to_string()
}

/// The maintainer's shape in account A's folder.
fn seed(config: &Path, cwd: &str, megabytes: u64) -> PathBuf {
    let dir = config.join("projects").join(mangled_dir(cwd));
    std::fs::create_dir_all(&dir).unwrap();
    let main = dir.join(format!("{SESSION}.jsonl"));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&main).unwrap());
    let body = "w".repeat(4000);
    let (mut written, mut n) = (0u64, 0usize);
    while written < megabytes * 1024 * 1024 {
        let content = if n % 5000 == 4999 {
            "x".repeat(2 * 1024 * 1024)
        } else {
            body.clone()
        };
        let row = turn(n, cwd, &content);
        writeln!(out, "{row}").unwrap();
        written += row.len() as u64 + 1;
        n += 1;
    }
    out.flush().unwrap();
    let folder = dir.join(SESSION);
    std::fs::create_dir_all(folder.join("subagents")).unwrap();
    std::fs::create_dir_all(folder.join("tool-results")).unwrap();
    let chunk = "r".repeat(290 * 1024);
    for i in 0..1719_usize {
        if i.is_multiple_of(4) {
            std::fs::write(
                folder.join("subagents").join(format!("agent-{i}.jsonl")),
                format!("{}\n", turn(i, cwd, "sub")),
            )
            .unwrap();
        } else {
            std::fs::write(folder.join("tool-results").join(format!("{i}.txt")), &chunk).unwrap();
        }
    }
    let history = config.join("file-history").join(SESSION);
    std::fs::create_dir_all(&history).unwrap();
    for i in 0..904 {
        std::fs::write(history.join(format!("{i:016x}@v1")), "r".repeat(22 * 1024)).unwrap();
    }
    main
}

struct Lab {
    state: Arc<AppState>,
    session: String,
    target_profile: String,
    target_account: String,
    main: PathBuf,
    cwd: String,
}

fn lab(work: &Path, megabytes: u64) -> Lab {
    let state = app_state(work);
    let project = work.join("manuscript");
    std::fs::create_dir_all(&project).unwrap();
    let cwd = project.to_string_lossy().into_owned();
    let a = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "Default", None).unwrap(),
    );
    let b = set_up::set_up(
        &state,
        commands::create_account(&state, CLAUDE_CODE_ID, "personal-max", None).unwrap(),
    );
    let profile_a = commands::create_profile(&state, &a.id, "Main", None, None).unwrap();
    let profile_b = commands::create_profile(&state, &b.id, "Main", None, None).unwrap();
    let launched = commands::launch_profile(&state, &profile_a.id, Some(cwd.clone())).unwrap();
    let main = seed(&config_dir_of(&state, &a.id), &cwd, megabytes);
    {
        let storage = state.storage.lock().unwrap();
        let mut record = storage
            .sessions()
            .get(&juggler_core::model::SessionId::new(
                launched.session_id.clone(),
            ))
            .unwrap();
        record.transcript_ref = Some(juggler_core::model::TranscriptRef {
            path: main.clone(),
            native_id: Some(SESSION.to_owned()),
        });
        storage.sessions().update(&record).unwrap();
    }
    Lab {
        state,
        session: launched.session_id,
        target_profile: profile_b.id,
        target_account: b.id,
        main,
        cwd,
    }
}

fn megabytes() -> u64 {
    std::env::var("AJ_REAL_SIZE_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(800)
}

#[test]
#[ignore = "real size: writes about 1.3 GB to the temp folder"]
fn an_800_mb_conversation_switches_through_the_app() {
    let work = tempfile::tempdir().unwrap();
    let lab = lab(work.path(), megabytes());
    let before = std::fs::metadata(&lab.main).unwrap().len();
    // The Claude app is still writing while the switch runs.
    let writer = {
        let main = lab.main.clone();
        let cwd = lab.cwd.clone();
        std::thread::spawn(move || {
            let rounds = if std::env::var_os("AJ_NO_WRITER").is_some() {
                0
            } else {
                20
            };
            for n in 0..rounds {
                std::thread::sleep(std::time::Duration::from_millis(150));
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&main)
                    .unwrap();
                writeln!(file, "{}", turn(9_000_000 + n, &cwd, "written meanwhile")).unwrap();
            }
        })
    };
    let clock = Instant::now();
    juggler_handover::pipeline::on_progress(Box::new(move |step| {
        println!("  {:<10} at {:.1?}", step.as_str(), clock.elapsed());
    }));
    let switched = commands::switch_execute_impl(
        &lab.state,
        &lab.session,
        &lab.target_profile,
        "stop-now",
        None,
        &[],
    )
    .unwrap_or_else(|error| {
        let wire = error.wire();
        panic!(
            "the switch failed: {} / {:?}",
            wire.user_message, wire.detail
        )
    });
    println!("switched through the app in {:.1?}", clock.elapsed());
    writer.join().unwrap();
    let landed = config_dir_of(&lab.state, &lab.target_account)
        .join("projects")
        .join(mangled_dir(&lab.cwd));
    // What landed is the source as it was when it was frozen: at least
    // what it held when the switch began, whole lines, and exactly the
    // start of the source now.
    let landed_main = landed.join(format!("{SESSION}.jsonl"));
    let (hash, len) = sha256_file(&landed_main, None).unwrap();
    assert!(
        len >= before,
        "it carries everything there was: {len} < {before}"
    );
    assert_eq!(
        sha256_file(&lab.main, Some(len)).unwrap(),
        (hash, len),
        "a prefix of the source"
    );
    let whole = juggler_adapter::handover::BundleFile {
        name: String::new(),
        bytes: Vec::new(),
        on_disk: Some(landed_main),
    };
    assert!(whole.last_line_whole().unwrap(), "it ends on a whole line");
    let count = |dir: &Path| std::fs::read_dir(dir).unwrap().count();
    assert_eq!(
        count(&landed.join(SESSION).join("tool-results"))
            + count(&landed.join(SESSION).join("subagents")),
        1719
    );
    assert!(!switched.edge_id.is_empty());
}

/// The words, details and Retry a failure reaches the screen with, from
/// the app's own command: a target folder that can't be written.
#[test]
#[ignore = "real size: writes about 1.3 GB to the temp folder"]
fn a_switch_that_cant_write_says_why_and_offers_retry() {
    use std::os::unix::fs::PermissionsExt as _;
    let work = tempfile::tempdir().unwrap();
    let lab = lab(work.path(), 40);
    let projects = config_dir_of(&lab.state, &lab.target_account).join("projects");
    std::fs::create_dir_all(&projects).unwrap();
    std::fs::set_permissions(&projects, std::fs::Permissions::from_mode(0o500)).unwrap();
    let result = commands::switch_execute_impl(
        &lab.state,
        &lab.session,
        &lab.target_profile,
        "stop-now",
        None,
        &[],
    );
    std::fs::set_permissions(&projects, std::fs::Permissions::from_mode(0o700)).unwrap();
    let wire = result
        .expect_err("a folder that can't be written stops the switch")
        .wire();
    println!(
        "words: {}\ndetail: {:?}\nretryable: {}",
        wire.user_message, wire.detail, wire.retryable
    );
    assert!(
        wire.user_message.contains("wasn't allowed to write"),
        "it says why: {}",
        wire.user_message
    );
    assert!(
        wire.retryable,
        "fixing the folder and trying again can work"
    );
    assert!(wire.detail.is_some(), "Copy details has something to copy");
}

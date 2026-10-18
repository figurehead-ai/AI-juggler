//! Gate V1 mechanics dogfood (operational, run explicitly; M1-7 evidence).
//! Two imported accounts, one shared project directory, the REAL claude
//! binary under our supervisor/PTY — no automation of login (ADR-0003):
//! unauthenticated copies are expected to show Claude's own login screen,
//! which itself proves streaming, isolation, tracking, and clean teardown.
//!
//! ```text
//! cargo test -p juggler-app --test dogfood_v1 -- --ignored --nocapture
//! ```

// Test crates are pragmatic by policy (QUALITY.md §2: tests excepted).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(target_os = "macos")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_app::terminal;
use juggler_core::model::ProfileId;

fn expand(path: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").expect("HOME");
    std::path::Path::new(&home).join(path.strip_prefix("~/").unwrap_or(path))
}

#[test]
#[ignore = "operational: real-machine dogfood, see docs/qa/gate-M1.md"]
fn two_accounts_parallel_same_project_mechanics() {
    let data_dir = expand("Library/Application Support/AI Juggler");
    let project_dir = expand("Dev/AIJuggler/scratch/dogfood/project-x");
    let state = Arc::new(AppState::init(data_dir).expect("app state"));

    // The two dogfood accounts: `AIJ_DOGFOOD_ACCOUNTS="Label A,Label B"`
    // picks by label; otherwise the first two imported accounts.
    let (account_a, account_b) = {
        let storage = state.storage.lock().unwrap();
        let accounts = storage.accounts().list().unwrap();
        let wanted: Vec<String> = std::env::var("AIJ_DOGFOOD_ACCOUNTS")
            .map(|raw| {
                raw.split(',')
                    .map(|label| label.trim().to_owned())
                    .collect()
            })
            .unwrap_or_default();
        let pick = |index: usize| {
            wanted.get(index).map_or_else(
                || accounts.get(index).expect("two imported accounts").clone(),
                |label| {
                    accounts
                        .iter()
                        .find(|account| &account.label == label)
                        .unwrap_or_else(|| panic!("account {label} imported"))
                        .clone()
                },
            )
        };
        (pick(0).id, pick(1).id)
    };
    let profiles: Vec<ProfileId> = {
        let storage = state.storage.lock().unwrap();
        vec![
            storage
                .profiles()
                .list_by_account(&account_a)
                .unwrap()
                .first()
                .expect("Launch settings for the first account")
                .id
                .clone(),
            storage
                .profiles()
                .list_by_account(&account_b)
                .unwrap()
                .first()
                .expect("Launch settings for the second account")
                .id
                .clone(),
        ]
    };

    // Parallel launch of both accounts on the SAME directory.
    let launched: Vec<_> = profiles
        .iter()
        .map(|profile_id| {
            commands::launch_profile(
                &state,
                profile_id.as_str(),
                Some(project_dir.to_string_lossy().into_owned()),
            )
            .expect("launch")
        })
        .collect();
    println!(
        "LAUNCHED {} sessions: {:?}",
        launched.len(),
        launched.iter().map(|l| &l.session_id).collect::<Vec<_>>()
    );
    assert_eq!(launched.len(), 2);
    assert_ne!(launched[0].session_id, launched[1].session_id);

    // Both PTYs must stream Claude's own output. Unauthenticated copies
    // show Claude's login screen — that text IS the streaming proof.
    for result in &launched {
        let id = result.session_id.clone();
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut saw_output = false;
        while Instant::now() < deadline {
            let snapshot = terminal::terminal_snapshot_impl(&state, &id).unwrap();
            let bytes = base64_decode_for_test(&snapshot);
            let text = String::from_utf8_lossy(&bytes);
            if text.contains("login") || text.contains("Claude") || !text.trim().is_empty() {
                println!(
                    "STREAM OK {}: {} bytes, head: {:?}",
                    id,
                    bytes.len(),
                    text.trim().chars().take(80).collect::<String>()
                );
                saw_output = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(300));
        }
        assert!(saw_output, "no PTY output streamed for {id}");
    }

    // Graceful stop ladder for both.
    for result in &launched {
        terminal::terminal_kill_impl(&state, &result.session_id).unwrap();
    }
    for result in &launched {
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            let events = state.supervisor.try_recv();
            let mut done = false;
            if let Some(event) = events {
                let stopped_by_user = match &event {
                    juggler_supervisor::LifecycleEvent::Exited { session, .. } => {
                        state.supervisor.was_stopped(session)
                    }
                    _ => false,
                };
                commands::apply_lifecycle_event(
                    &state.storage.lock().unwrap(),
                    event.clone(),
                    stopped_by_user,
                    None,
                );
                if let juggler_supervisor::LifecycleEvent::Exited { session, .. } = event
                    && session.as_str() == result.session_id
                {
                    done = true;
                }
            }
            if done {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    // Project linkage: ONE project, both accounts in history.
    let projects = commands::projects_list_impl(&state).unwrap();
    let project = projects
        .iter()
        .find(|p| p.root.ends_with("project-x"))
        .expect("dogfood project tracked");
    let mut accounts_used = project.accounts_used.clone();
    accounts_used.sort();
    assert!(
        project.instance_count >= 2,
        "runs accumulate on the project"
    );
    assert!(
        accounts_used.len() >= 2,
        "both dogfood accounts appear in the project history"
    );
    println!(
        "PROJECT OK: {} | accounts: {:?} | instances: {}",
        project.name, project.accounts_used, project.instance_count
    );

    // Session records ended.
    let sessions = {
        let storage = state.storage.lock().unwrap();
        storage.sessions().list(None).unwrap()
    };
    for result in &launched {
        let record = sessions
            .iter()
            .find(|s| s.id.as_str() == result.session_id)
            .expect("session recorded");
        println!(
            "SESSION {} status: {} project: {:?}",
            record.id,
            record.status.as_str(),
            record.project
        );
        assert!(record.project.is_some());
        assert_eq!(
            record.status.as_str(),
            "stopped",
            "user stop ladder must not record as failed"
        );
    }

    println!("DOGFOOD MECHANICS: PASS");
}

fn base64_decode_for_test(data: &str) -> Vec<u8> {
    // Minimal base64 decode (standard alphabet, padding) for assertions.
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buf: u32 = 0;
    let mut bits = 0;
    for byte in data.bytes().filter(|b| !b.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        let value = TABLE.iter().position(|t| *t == byte).unwrap_or(0) as u32;
        buf = (buf << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    out
}

#[test]
#[ignore = "operational: backfills the user's real project history"]
fn backfill_real_projects() {
    let data_dir = expand("Library/Application Support/AI Juggler");
    let state = Arc::new(AppState::init(data_dir).expect("app state"));
    let enriched = commands::projects_backfill_impl(&state).expect("backfill");
    println!("BACKFILLED {enriched} projects from imported transcripts");
    for project in commands::projects_list_impl(&state).expect("projects") {
        println!(
            "  {} | via {:?} | {} prior sessions | active: {:?}",
            project.root,
            project.accounts_used,
            project.history_sessions,
            project
                .active_instance
                .as_ref()
                .map(|i| i.account_label.clone())
        );
    }
}

//! AJ-E512 gate dogfood (operational — run explicitly):
//! a REAL Claude Code Instance → its real helpers → verified
//! cleanup, through the real supervisor/PTY/ladder stack.
//!
//! Auth boundary (gate-M1 pattern): the fresh `CLAUDE_CONFIG_DIR` is
//! unauthenticated — Claude's own login screen is the expected
//! output. Nothing user-owned is touched: the config dir, project
//! dir, and app state are all throwaway copies; `~/.claude` and the
//! t3 profiles are never read or written. What this proves is the E5
//! spine on a real provider process: the tree lands in the durable
//! ledger, and the E507 verify→TERM→grace→KILL ladder takes it down
//! with an audit row — no `UnknownSession`, no survivors.
//!
//! ```text
//! cargo test -p juggler-app --test dogfood_e5 -- --ignored --nocapture
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
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use juggler_adapter::{AdapterRegistry, EnvMap, LaunchSpec};
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::Timestamp;

fn now_ms() -> i64 {
    Timestamp::now().millis()
}

/// Resolves the real claude binary (~/.local/bin/claude → PATH).
fn claude_binary() -> std::path::PathBuf {
    let local = std::env::var("HOME")
        .map(|home| std::path::Path::new(&home).join(".local/bin/claude"))
        .unwrap();
    if local.is_file() {
        return local;
    }
    let found = std::process::Command::new("which")
        .arg("claude")
        .output()
        .unwrap();
    std::path::PathBuf::from(String::from_utf8_lossy(&found.stdout).trim())
}

#[test]
#[ignore = "operational: real-claude gate dogfood, see docs/qa/gate-E5.md"]
fn real_claude_instance_real_helpers_verified_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("claude-config");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::create_dir_all(&project).unwrap();

    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    let session = juggler_core::model::SessionId::new("sess-dogfood-e5");
    let spec = LaunchSpec {
        program: claude_binary(),
        args: vec![],
        env: EnvMap::from([
            (
                "CLAUDE_CONFIG_DIR".to_string(),
                config.to_string_lossy().to_string(),
            ),
            ("TERM".to_string(), "xterm-256color".to_string()),
        ]),
        cwd: project.clone(),
        needs_pty: true,
    };
    let ctx = juggler_supervisor::tree::SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-dogfood-e5".into()),
        project_id: Some("proj-dogfood-e5".into()),
        machine: commands::local_machine_id(&state),
        role: "root",
    };
    let spawned = state.supervisor.spawn(&session, &spec, &ctx).unwrap();
    println!("[dogfood-e5] real claude root pid={}", spawned.pid);

    // 1. The real UI streams (unauthenticated: the login screen IS
    //    the expected honest output — gate-M1's auth boundary).
    let deadline = std::time::Instant::now() + Duration::from_secs(45);
    let mut saw_ui = false;
    while std::time::Instant::now() < deadline {
        let out = state.supervisor.output(&session).unwrap_or_default();
        let text = String::from_utf8_lossy(&out);
        if text.contains("Claude") {
            saw_ui = true;
            println!("[dogfood-e5] streamed {} bytes of real UI", text.len());
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    assert!(saw_ui, "the real claude UI never streamed");

    // 2. The real tree lands in the durable ledger (root + any real
    //    helpers claude boots — its runtime/MCP children).
    use juggler_supervisor::tree::LedgerStore as _;
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let helper_rows;
    loop {
        if let Ok(mut collector) = state.resources.lock() {
            collector.tick(&state.supervisor, now_ms());
        }
        let rows = state.ledger.session_rows(&session).unwrap_or_default();
        let helpers = rows.iter().filter(|row| row.role == "helper").count();
        if helpers > 0 || std::time::Instant::now() >= deadline {
            helper_rows = helpers;
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    let rows = state.ledger.session_rows(&session).unwrap();
    println!(
        "[dogfood-e5] ledger: {} rows ({} helpers) — real attribution {:?}",
        rows.len(),
        helper_rows,
        rows.first()
            .map(|row| (row.provider.clone(), row.exe_path.clone()))
    );

    // 3. VERIFIED cleanup through the E507 ladder (audit-first, real
    //    identity verify, TERM → grace → KILL only for survivors).
    let verify = commands::kill_verify_inner(&state, session.as_str(), spawned.pid).unwrap();
    println!(
        "[dogfood-e5] verify: verified={} members={}",
        verify.verified,
        verify.members.len()
    );
    assert!(verify.verified, "the real root verifies");
    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        session.as_str(),
        spawned.pid,
        "pgid",
        &Arc::new(AtomicBool::new(false)),
        None,
    )
    .unwrap();
    println!("[dogfood-e5] ladder: {outcome:?}");
    assert!(outcome.all_succeeded, "the ladder reports success");

    // The audit row exists (intent BEFORE signal, actor user:kill_sheet).
    let history = state.audit.history(0).unwrap();
    assert!(
        history.iter().any(|row| row.pid == spawned.pid),
        "the verified cleanup is audited"
    );

    // The tree is gone — root dead, no member survived the ladder.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        let gone = state
            .ledger
            .session_rows(&session)
            .unwrap()
            .iter()
            .filter(|row| row.pid.is_some())
            .all(|row| row.state == "exited");
        if gone {
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let final_rows = state.ledger.session_rows(&session).unwrap();
    for row in &final_rows {
        println!(
            "[dogfood-e5] final row: pid={:?} role={} state={} exit={:?}",
            row.pid, row.role, row.state, row.exit_kind
        );
    }
    assert!(
        final_rows
            .iter()
            .filter(|row| row.pid.is_some())
            .all(|row| row.state == "exited"),
        "every real process of the tree is exited"
    );
    println!("[dogfood-e5] PASS — real instance, real tree, verified cleanup, audited");
}

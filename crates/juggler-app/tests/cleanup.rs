//! AJ-E506 evidence: live dogfood of the read-only Cleanup Centre.
//! A fake Instance spawns a vite-like helper (an ephemeral port); the Instance
//! closes; after grace the inventory shows the orphan row with the
//! full attribution + the complete evidence chain — and the archived
//! inventory JSON proves ZERO kill fields exist anywhere in the DTO.
//! Run with `--nocapture` to archive the JSON.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_adapter::{AdapterRegistry, LaunchSpec};
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::SessionId;
use juggler_supervisor::tree::SpawnContext;

fn ctx(state: &Arc<AppState>) -> SpawnContext {
    SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-e506".into()),
        project_id: Some("proj-e506".into()),
        machine: commands::local_machine_id(state),
        role: "root",
    }
}

fn sh_spec(script: &str, cwd: &Path) -> LaunchSpec {
    LaunchSpec {
        program: "/bin/sh".into(),
        args: vec!["-c".into(), script.into()],
        env: Default::default(),
        cwd: cwd.to_path_buf(),
        needs_pty: false,
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn tick_until<F: FnMut() -> bool>(state: &Arc<AppState>, deadline_ms: u64, mut done: F) -> bool {
    let deadline = Instant::now() + Duration::from_millis(deadline_ms);
    while Instant::now() < deadline {
        if let Ok(mut collector) = state.resources.lock() {
            collector.tick(&state.supervisor, now_ms());
        }
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    false
}

#[test]
fn cleanup_centre_dogfood_orphan_row_with_evidence_and_zero_kill_fields() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(
        AppState::init_with_registry(dir.path().join("app-data"), AdapterRegistry::new()).unwrap(),
    );
    state.supervisor.set_orphan_grace("claude-code", 3);

    // The fake Instance + its vite-like helper on an EPHEMERAL port
    // (a fixed :5173 once collided with a leaked listener from an
    // earlier failed run and cascaded — ports are never assumed).
    let port: u16 = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let session = SessionId::new("sess-e506");
    let spawned = state
        .supervisor
        .spawn(
            &session,
            &sh_spec(
                &format!("python3 -m http.server {port} --bind 127.0.0.1 & wait"),
                dir.path(),
            ),
            &ctx(&state),
        )
        .unwrap();

    // Live phase: the inventory shows the healthy tree (all-managed)
    // WITH the helper row landed (a tree freezing at exit keeps only
    // its last pre-exit rows — the helper must be seen while live).
    let live_landed = tick_until(&state, 20_000, || {
        let info = commands::cleanup_inventory_inner(&state, "all-managed", false);
        {
            use juggler_supervisor::tree::LedgerStore as _;
            eprintln!(
                "[poll] ledger_rows={:?} degraded={}",
                state
                    .ledger
                    .session_rows(&juggler_core::model::SessionId::new("sess-e506"))
                    .map(|r| r
                        .iter()
                        .map(|r| (r.pid, r.role.clone(), r.state.clone()))
                        .collect::<Vec<_>>()),
                state.supervisor.ledger_degraded()
            );
        }
        eprintln!(
            "[poll] groups={} sess?={} rows={:?}",
            info.groups.len(),
            info.groups.iter().any(|g| g.session == "sess-e506"),
            info.groups
                .iter()
                .map(|g| (
                    g.healthy,
                    g.rows
                        .iter()
                        .map(|r| (r.pid, r.role.clone(), r.state.clone()))
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>()
        );
        info.groups.iter().any(|group| {
            group.session == "sess-e506"
                && group.healthy
                && group.rows.iter().any(|row| row.role == "helper")
        })
    });
    assert!(live_landed, "the live tree never showed as healthy");
    let live = commands::cleanup_inventory_inner(&state, "all-managed", false);
    println!(
        "[cleanup-live] {}",
        serde_json::to_string_pretty(&live).unwrap()
    );

    // The Instance closes WITHOUT the supervisor's ladder — since
    // AJ-E507 `stop()` TERMs the whole process group (the :5173
    // helper included), the orphan scenario needs the root to die
    // another way: an external SIGKILL to the root pid only (the
    // crash shape). The helper survives it and outlives grace.
    let _ = std::process::Command::new("kill")
        .arg("-9")
        .arg(spawned.pid.to_string())
        .status();
    let orphan_landed = tick_until(&state, 20_000, || {
        let info = commands::cleanup_inventory_inner(&state, "needs-attention", false);
        info.groups.iter().any(|group| {
            group.session == "sess-e506"
                && group.rows.iter().any(|row| {
                    row.state == "orphaned"
                        && row
                            .ports
                            .as_deref()
                            .is_some_and(|p| p.contains(&port.to_string()))
                })
        })
    });
    assert!(
        orphan_landed,
        "the orphan row with :5173 never appeared in needs-attention"
    );

    let info = commands::cleanup_inventory_inner(&state, "needs-attention", false);
    println!(
        "[cleanup-orphan] {}",
        serde_json::to_string_pretty(&info).unwrap()
    );
    assert!(!info.ledger_degraded);
    let group = info
        .groups
        .iter()
        .find(|group| group.session == "sess-e506")
        .unwrap();
    assert_eq!(group.provider, "claude-code");
    assert_eq!(group.account.as_deref(), Some("acc-e506"));
    assert_eq!(group.project.as_deref(), Some("proj-e506"));
    assert_eq!(group.machine, live.groups[0].machine);
    let orphan = group
        .rows
        .iter()
        .find(|row| row.state == "orphaned")
        .unwrap();
    // pass2 §2 label, VERBATIM.
    assert_eq!(
        orphan.state_label,
        "Still running after its Instance closed"
    );
    assert!(
        orphan
            .ports
            .as_deref()
            .unwrap_or("")
            .contains(&port.to_string())
    );
    // The complete evidence chain (Req.4).
    let evidence = &orphan.evidence;
    assert!(evidence.root_pid.is_some());
    assert!(evidence.pgid.is_some());
    assert!(evidence.first_seen > 0 && evidence.last_seen >= evidence.first_seen);
    assert!(!evidence.verification.verdict.is_empty());
    assert!(orphan.age_ms.is_some(), "kernel age from started_at");

    // Req.7 — zero kill surface: the ARCHIVED JSON contains no kill /
    // stop / terminate / signal field names anywhere, and the DTO
    // types carry none by construction.
    let json = serde_json::to_string(&info).unwrap();
    let lowered = json.to_lowercase();
    for banned in [
        "\"stop\"",
        "\"kill\"",
        "\"terminate\"",
        "\"signal\"",
        "free this port",
    ] {
        assert!(
            !lowered.contains(banned),
            "the read-only inventory JSON must never contain {banned}"
        );
    }

    // Cleanup: reap the orphan by pid.
    let _ = std::process::Command::new("kill")
        .arg(orphan.pid.to_string())
        .status();
}

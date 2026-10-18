//! AJ-E510 evidence: the crash drill — spawn trees, DROP the app
//! state without stopping anything (the crash), kill one root while
//! the app is down, restart, and assert the recovery pass: verified
//! live roots re-adopted WITH stop control (F-3 closed), the
//! down-period helper labelled `left behind while Juggler was
//! closed`, the forged-identity row `Unverifiable` (listed, never
//! killed), the exact one-time summary (2/1/1), the 24 h tier
//! suspension, and the `system:recovery` audit row. Run with
//! `--nocapture` to archive the evidence dumps.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use juggler_adapter::{AdapterRegistry, EnvMap, LaunchSpec};
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::Timestamp;
use juggler_supervisor::tree::LedgerStore as _;

fn alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn kill9(pid: u32) {
    let _ = std::process::Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .status();
}

fn sh_spec(script: &str, cwd: &Path) -> LaunchSpec {
    LaunchSpec {
        program: "/bin/sh".into(),
        args: vec!["-c".into(), script.into()],
        env: EnvMap::from([("JUGGLER_TEST".to_string(), "1".to_string())]),
        cwd: cwd.to_path_buf(),
        needs_pty: false,
    }
}

fn ctx(state: &Arc<AppState>) -> juggler_supervisor::tree::SpawnContext {
    juggler_supervisor::tree::SpawnContext {
        provider: "claude-code".into(),
        account_id: Some("acc-recovery".into()),
        project_id: Some("proj-recovery".into()),
        machine: commands::local_machine_id(state),
        role: "root",
    }
}

/// Ticks until one session has ≥`helpers` live helper rows.
fn tick_until_helpers(state: &Arc<AppState>, session: &str, helpers: usize) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        if let Ok(mut collector) = state.resources.lock() {
            collector.tick(&state.supervisor, Timestamp::now().millis());
        }
        if let Ok(rows) = state
            .ledger
            .session_rows(&juggler_core::model::SessionId::new(session.to_string()))
            && rows
                .iter()
                .filter(|row| row.role == "helper" && row.state == "running")
                .count()
                >= helpers
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    false
}

#[test]
fn crash_drill_readopts_verifies_and_reports() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("app-data");
    // ---- BEFORE the crash -------------------------------------------
    let state1 =
        Arc::new(AppState::init_with_registry(data.clone(), AdapterRegistry::new()).unwrap());
    // Two survivors (adopted = 2): root + a helper each.
    let a = juggler_core::model::SessionId::new("sess-a");
    let spawned_a = state1
        .supervisor
        .spawn(&a, &sh_spec("sleep 300 & wait", dir.path()), &ctx(&state1))
        .unwrap();
    let b = juggler_core::model::SessionId::new("sess-b");
    state1
        .supervisor
        .spawn(&b, &sh_spec("sleep 300 & wait", dir.path()), &ctx(&state1))
        .unwrap();
    assert!(
        tick_until_helpers(&state1, "sess-a", 1),
        "A's helper landed"
    );
    assert!(
        tick_until_helpers(&state1, "sess-b", 1),
        "B's helper landed"
    );

    // The down-period session: root + a python helper. The root dies
    // WHILE the app is down (external kill after the drop below).
    let c = juggler_core::model::SessionId::new("sess-c");
    let port: u16 = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let spawned_c = state1
        .supervisor
        .spawn(
            &c,
            &sh_spec(
                &format!("python3 -m http.server {port} --bind 127.0.0.1 & wait"),
                dir.path(),
            ),
            &ctx(&state1),
        )
        .unwrap();
    assert!(
        tick_until_helpers(&state1, "sess-c", 1),
        "C's dev-server helper landed"
    );

    // The FORGED row (unverifiable = 1): a ledger helper row for a
    // REAL live pid with a WRONG start time — a recycled-pid stranger
    // by construction.
    let forged_dir = tempfile::tempdir().unwrap();
    let mut outer = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("( exec sleep 300 ) & echo $! > target.pid; exit 0")
        .current_dir(forged_dir.path())
        .spawn()
        .unwrap();
    let _ = outer.wait();
    let forged_pid = std::fs::read_to_string(forged_dir.path().join("target.pid"))
        .unwrap()
        .trim()
        .parse::<u32>()
        .unwrap();
    assert!(alive(forged_pid), "forged fixture process is live");
    let boot_id = state1.ledger.boot_id().to_string();
    state1
        .ledger
        .upsert_helpers(
            &a,
            &[juggler_core::storage::LedgerRow {
                pid: Some(forged_pid),
                pgid: None,
                started_at: Some(1), // WRONG by construction
                exe_path: "/bin/sleep".into(),
                cmdline_hash: "forged".into(),
                cmdline_trunc: "sleep 300 (forged identity)".into(),
                boot_id: boot_id.clone(),
                session_id: "sess-a".into(),
                instance_id: None,
                provider: "claude-code".into(),
                account_id: Some("acc-recovery".into()),
                project_id: Some("proj-recovery".into()),
                machine_id: commands::local_machine_id(&state1).as_str().to_string(),
                root_pid: Some(spawned_a.pid),
                parent_pid: Some(spawned_a.pid),
                depth: 1,
                role: "helper".into(),
                state: "running".into(),
                listens_on: None,
                cwd: dir.path().to_string_lossy().to_string(),
                peak_rss: None,
                peak_cpu: None,
                exit_kind: None,
                reaped_by_us: None,
                escaped_group: false,
                down_period: false,
                first_seen: Timestamp::now().millis(),
                last_seen: Timestamp::now().millis(),
            }],
        )
        .unwrap();

    // Debug: the root row's verify inputs, pre-crash.
    {
        use juggler_supervisor::tree::{ProcessIdentity, verify, verify_start_time};
        let rows = state1.ledger.session_rows(&a).unwrap();
        let root = rows.iter().find(|row| row.role == "root").unwrap();
        let identity = ProcessIdentity {
            pid: root.pid.unwrap(),
            pgid: root.pgid,
            start_time: root.started_at.unwrap_or_default(),
            exe: std::path::PathBuf::from(&root.exe_path),
            boot_id: root.boot_id.clone(),
        };
        eprintln!(
            "[pre-crash] A root: pid={:?} started_at={:?} exe={} boot={} | verify={:?} start_time_only={:?}",
            root.pid,
            root.started_at,
            root.exe_path,
            root.boot_id,
            verify(root.pid.unwrap(), &identity),
            verify_start_time(root.pid.unwrap(), &identity),
        );
    }

    // ---- THE CRASH: drop everything without stopping a thing -------
    drop(state1);
    // C's root dies while Juggler is down (the down-period premise).
    kill9(spawned_c.pid);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while alive(spawned_c.pid) && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(!alive(spawned_c.pid), "C's root died while down");

    // ---- THE RESTART -------------------------------------------------
    let state2 =
        Arc::new(AppState::init_with_registry(data.clone(), AdapterRegistry::new()).unwrap());

    // Debug: C's rows at restart.
    {
        let rows = state2.ledger.session_rows(&c).unwrap();
        for row in &rows {
            eprintln!(
                "[restart] C row: pid={:?} role={} state={} started_at={:?} exe={} alive={}",
                row.pid,
                row.role,
                row.state,
                row.started_at,
                row.exe_path,
                row.pid.map(alive).unwrap_or(false)
            );
        }
    }

    // Req.3: the exact one-time summary (2/1/1).
    let summary = commands::recovery_summary_get_arc(&state2)
        .unwrap()
        .expect("summary pending");
    assert_eq!(
        summary.summary,
        "2 Instances re-adopted, 1 helpers left behind, 1 processes could not be verified and were left alone."
    );
    assert_eq!(
        (summary.adopted, summary.left_behind, summary.unverifiable),
        (2, 1, 1)
    );
    println!("[recovery] {summary:?}");
    // The audit row names the trigger.
    let audit = state2.audit.history(0).unwrap();
    assert!(
        audit.iter().any(|row| row.actor == "system:recovery"),
        "the recovery is audited: {audit:#?}"
    );
    // AJ-E508 Req.5: the tiers stay suspended after the recovery.
    assert!(
        commands::cleanup_status_inner(&state2).unwrap().suspended,
        "tiers suspended after recovery"
    );

    // F-3 closed: the adopted root has a WORKING stop (no
    // UnknownSession) — and it kills the whole verified tree.
    let members_before = juggler_supervisor::pgid_members(spawned_a.pid);
    assert!(members_before.len() >= 2, "A's tree is alive pre-stop");
    state2
        .supervisor
        .stop(&a, Duration::from_secs(5))
        .expect("adopted stop works (F-3)");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut all_gone = false;
    while std::time::Instant::now() < deadline {
        if members_before.iter().all(|pid| !alive(*pid)) {
            all_gone = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(all_gone, "the adopted tree died via the verified ladder");
    // B stays re-adopted and alive (untouched).
    assert!(
        state2.supervisor.alive_adopted_sessions().contains(&b),
        "B remains re-adopted"
    );

    // Req.2: the down-period helper is `orphaned` with the distinct
    // label; the forged row is `unverifiable`, listed, still alive.
    let inventory = commands::cleanup_inventory_inner(&state2, "all-managed", false);
    let c_row = inventory
        .groups
        .iter()
        .flat_map(|group| group.rows.iter())
        .find(|row| row.cmdline.contains("http.server"))
        .expect("C's helper listed");
    assert_eq!(c_row.state, "orphaned");
    assert!(c_row.down_period, "the down-period marker is set");
    assert_eq!(c_row.state_label, "left behind while Juggler was closed");
    let forged = inventory
        .groups
        .iter()
        .flat_map(|group| group.rows.iter())
        .find(|row| row.pid == forged_pid)
        .expect("the forged row is listed");
    assert_eq!(forged.state, "unverifiable");
    assert_eq!(forged.state_label, "Unverifiable — will not be touched");
    assert!(alive(forged_pid), "the unverified row was NEVER touched");
    println!("[recovery] left-behind: {c_row:?}");

    // Dismissed once ⇒ gone forever (until a new recovery).
    commands::recovery_summary_get_arc(&state2).unwrap();
    // (dismiss via the storage directly — the command needs tauri State)
    {
        let storage = state2.storage.lock().unwrap();
        storage
            .app_settings()
            .set("recovery_summary_consumed", "1")
            .unwrap();
    }
    assert!(
        commands::recovery_summary_get_arc(&state2)
            .unwrap()
            .is_none(),
        "shown exactly once"
    );

    // Cleanup: B's tree, C's helper, the forged fixture.
    state2.supervisor.stop(&b, Duration::from_secs(5)).unwrap();
    if let Ok(rows) = state2.ledger.session_rows(&c) {
        for row in rows.iter().filter_map(|row| row.pid) {
            kill9(row);
        }
    }
    kill9(forged_pid);
}

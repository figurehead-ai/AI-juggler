//! AJ-E512 guardrails suite (app level): EVERY kill path × the full
//! adversarial-fixture matrix, asserted on trap logs — never on UI
//! state. Paths: manual ladder (pid scope, pgid scope, the REFUSED
//! stranger), Tier 1, Tier 2, and crash recovery.
//!
//! The collateral law (D-01, zero false cleanup kills): the
//! unmanaged user server, the shared-PGID shell sibling, the
//! MCP-role holder, the Unverifiable PID-reuse stranger, and the
//! foreign-machine row receive ZERO signals from every path that is
//! not an explicit, warned, user-confirmed group kill; only the
//! intended verified DevServer/Watcher orphan is TERMed (KILL only
//! after grace when it ignores TERM). The ONE exception is AJ-E507
//! law, not a deviation: manual pgid/tree scope TERMs the confirmed
//! group — the sheet NAMES every shell/MCP member first
//! (`Includes a {role} that may hold unsaved state.`), and even
//! there everything OUTSIDE the confirmed group stays untouched.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

#[path = "../../juggler-supervisor/tests/fixtures/mod.rs"]
mod fixtures;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use fixtures::{FixtureSet, alive, kill9, port_free, spawn_unmanaged, unmanaged_fixture};
use juggler_adapter::{AdapterRegistry, EnvMap, LaunchSpec};
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::{
    Account, AccountId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId, ProfileId,
    SessionId, SessionStatus, Timestamp,
};
use juggler_supervisor::tree::LedgerStore as _;

fn now_ms() -> i64 {
    Timestamp::now().millis()
}

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
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
        account_id: Some("acc-guard".into()),
        project_id: Some("proj-guard".into()),
        machine: commands::local_machine_id(state),
        role: "root",
    }
}

fn seed_account(state: &Arc<AppState>, dir: &Path, id: &str, tier1: bool, tier2: bool) {
    {
        let storage = state.storage.lock().unwrap();
        storage
            .accounts()
            .insert(&Account {
                id: AccountId::new(id.to_string()),
                harness: juggler_core::model::HarnessId::new("claude-code"),
                label: id.into(),
                plan: None,
                state_dir: dir.join(format!("state-{id}")),
                cleanup_opt_in: tier1,
                renewal_date: None,
                preview_opt_in: false,
                import_source_path: None,
                import_origin: None,
                imported_at: None,
            })
            .unwrap();
        storage
            .profiles()
            .insert(&LaunchRecipe {
                id: ProfileId::new(format!("rec-{id}")),
                name: id.into(),
                account: AccountId::new(id.to_string()),
                settings_overlay: Default::default(),
                default_cwd: None,
            })
            .unwrap();
    }
    state
        .cleanup_settings
        .set(
            id,
            &juggler_core::storage::CleanupTierSettings {
                tier1_opt_in: tier1,
                tier2_opt_in: tier2,
            },
        )
        .unwrap();
}

fn seed_instance(state: &Arc<AppState>, dir: &Path, id: &str, account: &str) -> SessionId {
    let session = SessionId::new(id.to_string());
    state
        .storage
        .lock()
        .unwrap()
        .sessions()
        .insert(&InstanceRecord {
            id: session.clone(),
            conversation: None,
            profile: ProfileId::new(format!("rec-{account}")),
            machine: MachineId::new("this-mac"),
            kind: InstanceKind::Terminal,
            project: None,
            cwd: dir.to_path_buf(),
            status: SessionStatus::Running,
            started_at: Timestamp::now(),
            last_activity: Some(Timestamp::now()),
            usage: Default::default(),
            attention: None,
            running: None,
            transcript_ref: None,
            window: None,
        })
        .unwrap();
    session
}

/// Ticks until the session's helper rows include one per `needles`.
fn tick_until_helpers(state: &Arc<AppState>, session: &SessionId, needles: &[&str]) -> Vec<u32> {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(mut collector) = state.resources.lock() {
            collector.tick(&state.supervisor, now_ms());
        }
        if let Ok(rows) = state.ledger.session_rows(session) {
            let pids: Vec<Option<u32>> = needles
                .iter()
                .map(|needle| {
                    rows.iter()
                        .find(|row| row.role == "helper" && row.cmdline_trunc.contains(needle))
                        .and_then(|row| row.pid)
                })
                .collect();
            if pids.iter().all(|pid| pid.is_some()) {
                return pids.into_iter().map(|pid| pid.unwrap()).collect();
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "fixtures never landed in the ledger"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Ticks until the session's helpers are `orphaned` in the ledger
/// (the sweep's candidate source); returns the root's last_seen.
fn wait_orphans(state: &Arc<AppState>, session: &SessionId) -> i64 {
    state.supervisor.set_orphan_grace("claude-code", 3);
    let deadline = std::time::Instant::now() + Duration::from_secs(35);
    loop {
        if let Ok(mut collector) = state.resources.lock() {
            collector.tick(&state.supervisor, now_ms());
        }
        if let Ok(rows) = state.ledger.session_rows(session) {
            // The forged stranger flips `unverifiable` by design —
            // it is never an orphan candidate; exclude it.
            let helpers_orphaned = rows
                .iter()
                .filter(|row| row.role == "helper" && !row.cmdline_trunc.contains("(stranger)"))
                .all(|row| row.state == "orphaned" || row.state == "exited");
            if helpers_orphaned {
                return rows
                    .iter()
                    .find(|row| row.role == "root")
                    .map(|row| row.last_seen)
                    .unwrap_or_default();
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "orphans never surfaced"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// The PID-reuse stranger: a live fixture whose ledger row carries a
/// WRONG start time (forged identity — Unverifiable forever).
fn forge_stranger(state: &Arc<AppState>, session: &SessionId, root_pid: u32, dir: &Path) -> u32 {
    let trap = dir.join("stranger.traps");
    let ready = dir.join("stranger.ready");
    let pid = fixtures::spawn_detached(&fixtures::write_helper(dir), &[], &trap, &ready);
    state
        .ledger
        .upsert_helpers(
            session,
            &[juggler_core::storage::LedgerRow {
                pid: Some(pid),
                pgid: None,
                started_at: Some(1), // WRONG by construction
                exe_path: "/bin/python3".into(),
                cmdline_hash: "stranger".into(),
                cmdline_trunc: "python3 trap_helper.py (stranger)".into(),
                boot_id: state.ledger.boot_id().to_string(),
                session_id: session.as_str().to_string(),
                instance_id: None,
                provider: "claude-code".into(),
                account_id: Some("acc-guard".into()),
                project_id: Some("proj-guard".into()),
                machine_id: commands::local_machine_id(state).as_str().to_string(),
                root_pid: Some(root_pid),
                parent_pid: Some(root_pid),
                depth: 1,
                role: "helper".into(),
                state: "running".into(),
                listens_on: None,
                cwd: dir.to_string_lossy().to_string(),
                peak_rss: None,
                peak_cpu: None,
                exit_kind: None,
                reaped_by_us: None,
                escaped_group: false,
                down_period: false,
                first_seen: now_ms(),
                last_seen: now_ms(),
            }],
        )
        .unwrap();
    pid
}

/// A live fixture whose ledger row carries a WRONG absolute exe and
/// NO recorded start time — isolates the reconciler's EXE leg (the
/// start-time stranger is [`forge_stranger`]). `first_seen` is the
/// caller's: young rows pin the exec-settle window (AJ-E508), aged
/// rows pin the frozen-identity PID-reuse guard.
fn forge_exe_row(
    state: &Arc<AppState>,
    session: &SessionId,
    root_pid: u32,
    dir: &Path,
    tag: &str,
    first_seen: i64,
) -> u32 {
    let trap = dir.join(format!("{tag}.traps"));
    let ready = dir.join(format!("{tag}.ready"));
    let pid = fixtures::spawn_detached(&fixtures::write_helper(dir), &[], &trap, &ready);
    state
        .ledger
        .upsert_helpers(
            session,
            &[juggler_core::storage::LedgerRow {
                pid: Some(pid),
                pgid: None,
                started_at: None, // isolates the exe leg
                exe_path: "/bin/not-the-real-image".into(),
                cmdline_hash: tag.into(),
                cmdline_trunc: format!("python3 trap_helper.py ({tag})"),
                boot_id: state.ledger.boot_id().to_string(),
                session_id: session.as_str().to_string(),
                instance_id: None,
                provider: "claude-code".into(),
                account_id: Some("acc-guard".into()),
                project_id: Some("proj-guard".into()),
                machine_id: commands::local_machine_id(state).as_str().to_string(),
                root_pid: Some(root_pid),
                parent_pid: Some(root_pid),
                depth: 1,
                role: "helper".into(),
                state: "running".into(),
                listens_on: None,
                cwd: dir.to_string_lossy().to_string(),
                peak_rss: None,
                peak_cpu: None,
                exit_kind: None,
                reaped_by_us: None,
                escaped_group: false,
                down_period: false,
                first_seen,
                last_seen: now_ms(),
            }],
        )
        .unwrap();
    pid
}

fn cancel_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

/// Widens the ladder grace for this state (12 s): on a loaded
/// developer box a fixture's TERM handler can be starved past the
/// spec's 5 s — the SIGNAL still arrives (the trap log is the
/// evidence); only the KILL deadline moves. Production default is
/// untouched.
fn widen_grace(state: &Arc<AppState>) {
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set("kill_grace_secs", "12")
        .unwrap();
}

/// The suite spawns ~40 real processes; parallel tests contend for
/// sysinfo and identity reads flake (rows flip `unverifiable` under
/// load — a harness artefact, never a product verdict). One matrix
/// at a time: ~40 s total, deterministic.
static SEQUENTIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn serialize() -> std::sync::MutexGuard<'static, ()> {
    SEQUENTIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Collateral kinds asserted untouched after EVERY automated path
/// (the law; the manual pgid-scope exception is asserted separately).
const COLLATERAL: &[&str] = &["mcp", "shell", "term-ignorer", "escapee"];

#[test]
fn tier1_sweep_spares_every_collateral_fixture() {
    let _serial = serialize();
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-guard", true, false);
    let kit = FixtureSet::new(dir.path());

    let a = seed_instance(&state, dir.path(), "sess-t1", "acc-guard");
    let spawned_a = state
        .supervisor
        .spawn(
            &a,
            &sh_spec(
                &format!("{}\nsleep 3; exit 0", kit.root_script()),
                dir.path(),
            ),
            &ctx(&state),
        )
        .unwrap();
    // A still-healthy NEIGHBOUR tree of the SAME account: live roots
    // are never sweep candidates.
    let b = seed_instance(&state, dir.path(), "sess-t1-b", "acc-guard");
    let b_dir = dir.path().join("b");
    std::fs::create_dir_all(&b_dir).unwrap();
    let kit_b = FixtureSet::new(&b_dir);
    state
        .supervisor
        .spawn(
            &b,
            &sh_spec(&format!("{}\nwait", kit_b.root_script()), &b_dir),
            &ctx(&state),
        )
        .unwrap();
    let unmanaged_pid = spawn_unmanaged(dir.path());
    let stranger_pid = forge_stranger(&state, &a, spawned_a.pid, dir.path());

    let pids = tick_until_helpers(
        &state,
        &a,
        &["--port", "mcp-server", "--ignore-term", "touch", "--setsid"],
    );
    let vite_pid = pids[0];
    assert!(kit.wait_ready());
    let port = kit.vite_port();

    let root_exit = wait_orphans(&state, &a);
    commands::auto_cleanup_sweep(&state, root_exit + 61_000);

    // Intended: the verified DevServer orphan, TERMed, port freed.
    assert!(
        fixtures::wait_for(5_000, || !alive(vite_pid)),
        "the dev-server orphan is auto-TERMed"
    );
    assert_eq!(kit.get("vite").signals(), vec!["TERM".to_string()]);
    assert!(port_free(port));

    // COLLATERAL: zero signals, still alive.
    kit.assert_untouched(COLLATERAL, "tier-1 sweep");
    kit_b.assert_untouched(&["vite", "mcp"], "tier-1 sweep (neighbour session)");
    assert!(
        unmanaged_fixture(dir.path()).untouched(),
        "unmanaged user server signalled by the sweep"
    );
    for pid in [stranger_pid, unmanaged_pid] {
        assert!(alive(pid), "pid {pid} must survive the sweep");
    }
    let stranger_traps =
        std::fs::read_to_string(dir.path().join("stranger.traps")).unwrap_or_default();
    assert!(
        stranger_traps.trim().is_empty(),
        "the stranger was signalled: {stranger_traps}"
    );

    println!(
        "[tier1-collateral] vite {vite_pid} TERMed + port free; stranger {stranger_pid}, unmanaged {unmanaged_pid}, neighbour, mcp/shell/ignorer/escapee: zero signals"
    );

    // Cleanup.
    let _ = state.supervisor.stop(&b, Duration::from_secs(3));
    for pid in pids {
        kill9(pid);
    }
    kill9(stranger_pid);
    kill9(unmanaged_pid);
}

#[test]
fn tier2_crash_flow_spares_every_collateral_fixture() {
    let _serial = serialize();
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-guard", true, true);
    let kit = FixtureSet::new(dir.path());

    let a = seed_instance(&state, dir.path(), "sess-t2", "acc-guard");
    let spawned_a = state
        .supervisor
        .spawn(
            &a,
            &sh_spec(
                // The crash shape: the root dies hard, helpers survive.
                &format!("{}\nsleep 3; kill -9 $$", kit.root_script()),
                dir.path(),
            ),
            &ctx(&state),
        )
        .unwrap();
    let unmanaged_pid = spawn_unmanaged(dir.path());
    let stranger_pid = forge_stranger(&state, &a, spawned_a.pid, dir.path());

    let pids = tick_until_helpers(
        &state,
        &a,
        &["--port", "mcp-server", "--ignore-term", "touch", "--setsid"],
    );
    let vite_pid = pids[0];
    assert!(kit.wait_ready());
    let port = kit.vite_port();

    let root_exit = wait_orphans(&state, &a);
    // Before +5 min: NOTHING (no notice, no signal).
    commands::auto_cleanup_sweep(&state, root_exit + 4 * 60_000);
    kit.assert_untouched(&["vite"], "tier-2 before the 5 min crash grace");
    assert!(alive(vite_pid));
    // The notice appears, the Undo window lapses, THEN the signal.
    commands::auto_cleanup_sweep(&state, root_exit + 5 * 60_000 + 1_000);
    let pending = commands::cleanup_status_inner(&state)
        .unwrap()
        .tier2_pending;
    assert_eq!(pending.len(), 1, "exactly one Tier-2 notice");
    commands::auto_cleanup_sweep(&state, root_exit + 5 * 60_000 + 40_000);

    assert!(
        fixtures::wait_for(5_000, || !alive(vite_pid)),
        "the crash orphan is TERMed after the lapsed Undo window"
    );
    assert_eq!(kit.get("vite").signals(), vec!["TERM".to_string()]);
    assert!(port_free(port));

    kit.assert_untouched(COLLATERAL, "tier-2 crash flow");
    assert!(
        unmanaged_fixture(dir.path()).untouched(),
        "unmanaged user server signalled by tier 2"
    );
    assert!(alive(stranger_pid) && alive(unmanaged_pid));

    println!(
        "[tier2-collateral] vite {vite_pid} TERMed after notice+lapse; stranger/unmanaged/mcp/shell/ignorer/escapee: zero signals"
    );

    for pid in pids {
        kill9(pid);
    }
    kill9(stranger_pid);
    kill9(unmanaged_pid);
}

#[test]
fn manual_pid_scope_signals_only_the_verified_pid() {
    let _serial = serialize();
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-guard", false, false);
    widen_grace(&state);
    let kit = FixtureSet::new(dir.path());

    let a = seed_instance(&state, dir.path(), "sess-pid", "acc-guard");
    let spawned_a = state
        .supervisor
        .spawn(
            &a,
            &sh_spec(&format!("{}\nwait", kit.root_script()), dir.path()),
            &ctx(&state),
        )
        .unwrap();
    let unmanaged_pid = spawn_unmanaged(dir.path());
    let stranger_pid = forge_stranger(&state, &a, spawned_a.pid, dir.path());

    let pids = tick_until_helpers(
        &state,
        &a,
        &["--port", "mcp-server", "--ignore-term", "touch", "--setsid"],
    );
    let vite_pid = pids[0];
    assert!(kit.wait_ready());
    let port = kit.vite_port();

    // Step 1 — verify NOW: verified, with the port fact.
    let verify = commands::kill_verify_inner(&state, a.as_str(), vite_pid).unwrap();
    assert!(verify.verified);

    // Step 2+3 — the narrowest scope: THIS process only.
    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        a.as_str(),
        vite_pid,
        "pid",
        &cancel_flag(),
        Some(port),
    )
    .unwrap();
    assert!(outcome.all_succeeded, "{outcome:?}");
    assert!(
        fixtures::wait_for(5_000, || !alive(vite_pid)),
        "the verified orphan died on TERM"
    );
    assert_eq!(kit.get("vite").signals(), vec!["TERM".to_string()]);
    assert!(port_free(port));

    // COLLATERAL — including the SAME-GROUP shell/mcp siblings: the
    // pid scope is one process, nothing else.
    kit.assert_untouched(COLLATERAL, "manual pid scope");
    assert!(unmanaged_fixture(dir.path()).untouched());
    assert!(alive(stranger_pid) && alive(unmanaged_pid));

    // The stranger: REFUSED with zero signals and no audit row for it.
    let audit_before = state.audit.history(0).unwrap().len();
    let refused = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        a.as_str(),
        stranger_pid,
        "pid",
        &cancel_flag(),
        None,
    )
    .unwrap();
    assert_eq!(
        refused.refusal.as_deref(),
        Some(commands::KILL_REFUSAL_COPY)
    );
    assert_eq!(refused.members[0].outcome, "refused");
    assert!(alive(stranger_pid), "the stranger is never signalled");
    assert_eq!(state.audit.history(0).unwrap().len(), audit_before);

    println!(
        "[manual-pid] vite {vite_pid} only; same-group shell/mcp, stranger, unmanaged: zero signals"
    );

    let _ = state.supervisor.stop(&a, Duration::from_secs(3));
    for pid in pids {
        kill9(pid);
    }
    kill9(stranger_pid);
    kill9(unmanaged_pid);
}

#[test]
fn manual_pgid_scope_is_warned_and_confined_to_the_group() {
    let _serial = serialize();
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-guard", false, false);
    widen_grace(&state);
    let kit = FixtureSet::new(dir.path());

    let c = seed_instance(&state, dir.path(), "sess-pgid", "acc-guard");
    let spawned_c = state
        .supervisor
        .spawn(
            &c,
            &sh_spec(&format!("{}\nwait", kit.root_script()), dir.path()),
            &ctx(&state),
        )
        .unwrap();
    // A neighbour tree that must not hear the group TERM.
    let b = seed_instance(&state, dir.path(), "sess-pgid-b", "acc-guard");
    let b_dir = dir.path().join("b");
    std::fs::create_dir_all(&b_dir).unwrap();
    let kit_b = FixtureSet::new(&b_dir);
    state
        .supervisor
        .spawn(&b, &sh_script_in(&kit_b, &b_dir), &ctx(&state))
        .unwrap();
    let unmanaged_pid = spawn_unmanaged(dir.path());
    let stranger_pid = forge_stranger(&state, &c, spawned_c.pid, dir.path());

    let pids = tick_until_helpers(
        &state,
        &c,
        &["--port", "mcp-server", "--ignore-term", "touch", "--setsid"],
    );
    let (vite_pid, mcp_pid, ignorer_pid, shell_pid) = (pids[0], pids[1], pids[2], pids[3]);
    assert!(kit.wait_ready());
    let port = kit.vite_port();

    // Step 1 — the sheet NAMES every group member; shell + MCP carry
    // the unsaved-state warning (E507 Req.5, FMEA C3).
    let verify = commands::kill_verify_inner(&state, c.as_str(), vite_pid).unwrap();
    assert!(verify.verified);
    let shell_member = verify
        .members
        .iter()
        .find(|member| member.pid == shell_pid)
        .expect("the shell sibling is listed on the sheet");
    assert_eq!(
        shell_member.unsaved_state_warning.as_deref(),
        Some("Includes a TerminalShell that may hold unsaved state.")
    );
    let mcp_member = verify
        .members
        .iter()
        .find(|member| member.pid == mcp_pid)
        .expect("the MCP member is listed on the sheet");
    assert!(mcp_member.unsaved_state_warning.is_some(), "MCP warned");

    // Step 2+3 — the user-confirmed GROUP kill (the E507-law
    // exception): group members take the TERM; everything OUTSIDE
    // the confirmed group is untouched.
    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        c.as_str(),
        vite_pid,
        "pgid",
        &cancel_flag(),
        Some(port),
    )
    .unwrap();
    assert!(outcome.all_succeeded, "{outcome:?}");
    for pid in [vite_pid, mcp_pid, shell_pid, ignorer_pid] {
        assert!(
            fixtures::wait_for(8_000, || !alive(pid)),
            "group member {pid} died in the confirmed kill"
        );
    }
    // Every group member was TERMed once; the TERM-ignorer then fell
    // to the post-grace KILL (the ladder's contract) — that is why it
    // is an in-group member here, not collateral.
    for kind in ["vite", "mcp", "shell", "term-ignorer"] {
        let signals = kit.get(kind).signals();
        if signals != vec!["TERM".to_string()] {
            println!(
                "[pgid-diag] {kind}: signals={signals:?} outcome_members={:?}",
                outcome.members
            );
        }
        assert_eq!(signals, vec!["TERM".to_string()]);
    }
    assert!(port_free(port));

    // OUTSIDE the confirmed group: zero signals, still alive.
    kit.assert_untouched(&["escapee"], "manual pgid scope");
    kit_b.assert_untouched(&["vite", "mcp"], "manual pgid scope (neighbour session)");
    assert!(unmanaged_fixture(dir.path()).untouched());
    assert!(alive(stranger_pid) && alive(unmanaged_pid));

    println!(
        "[manual-pgid] confirmed group only ({vite_pid}/{mcp_pid}/{shell_pid}); escapee, neighbour, stranger, unmanaged: zero signals"
    );

    let _ = state.supervisor.stop(&b, Duration::from_secs(3));
    for pid in pids {
        kill9(pid);
    }
    kill9(stranger_pid);
    kill9(unmanaged_pid);
}

/// Helper: the neighbour tree's script, seeded in its own dir.
fn sh_script_in(kit: &FixtureSet, dir: &Path) -> LaunchSpec {
    sh_spec(&format!("{}\nwait", kit.root_script()), dir)
}

#[test]
fn crash_recovery_signals_nothing_and_stop_stays_scoped() {
    let _serial = serialize();
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("app-data");

    // ---- before the crash: one tree + the stranger + unmanaged ----
    let state1 = app_state(dir.path());
    seed_account(&state1, dir.path(), "acc-guard", false, false);
    let kit = FixtureSet::new(dir.path());
    let a = seed_instance(&state1, dir.path(), "sess-rec", "acc-guard");
    let spawned_a = state1
        .supervisor
        .spawn(
            &a,
            &sh_spec(&format!("{}\nwait", kit.root_script()), dir.path()),
            &ctx(&state1),
        )
        .unwrap();
    let unmanaged_pid = spawn_unmanaged(dir.path());
    let stranger_pid = forge_stranger(&state1, &a, spawned_a.pid, dir.path());
    let _ = tick_until_helpers(
        &state1,
        &a,
        &["--port", "mcp-server", "--ignore-term", "touch", "--setsid"],
    );
    assert!(kit.wait_ready());
    let port = kit.vite_port();

    // ---- THE CRASH ---------------------------------------------------
    drop(state1);

    // ---- THE RESTART: recovery adopts/labels, signals NOTHING -------
    let state2 =
        Arc::new(AppState::init_with_registry(data.clone(), AdapterRegistry::new()).unwrap());
    kit.assert_untouched(IN_TREE_ALL, "crash recovery itself");
    assert!(
        unmanaged_fixture(dir.path()).untouched(),
        "recovery signalled the unmanaged server"
    );
    let stranger_traps =
        std::fs::read_to_string(dir.path().join("stranger.traps")).unwrap_or_default();
    assert!(stranger_traps.trim().is_empty());
    let summary = commands::recovery_summary_get_arc(&state2)
        .unwrap()
        .expect("recovery summary pending");
    println!("[recovery-collateral] {summary:?}");

    // Diagnostic: every fixture's row state + liveness before the
    // stop (the zero-signal assertions below must be explainable).
    if let Ok(rows) = state2.ledger.session_rows(&a) {
        for row in &rows {
            println!(
                "[recovery-pre-stop] pid={:?} role={} state={} exe={} alive={}",
                row.pid,
                row.role,
                row.state,
                row.exe_path,
                row.pid.map(fixtures::alive).unwrap_or(false)
            );
        }
    }

    // The adopted stop: the verified OWN group only. In-group
    // fixtures die; the escapee, the stranger, the unmanaged server
    // never hear it.
    state2
        .supervisor
        .stop(&a, Duration::from_secs(3))
        .expect("adopted stop works");
    for kind in ["vite", "mcp", "shell", "term-ignorer"] {
        assert_eq!(
            kit.get(kind).signals(),
            vec!["TERM".to_string()],
            "{kind}: group member, TERMed once"
        );
    }
    kit.assert_untouched(&["escapee"], "adopted stop");
    assert!(
        unmanaged_fixture(dir.path()).signals().is_empty()
            || unmanaged_fixture(dir.path()).untouched(),
        "unmanaged untouched by the adopted stop"
    );
    assert!(alive(stranger_pid) && alive(unmanaged_pid));
    assert!(port_free(port), "the port is honestly free after the stop");

    println!(
        "[recovery-collateral] adopted stop scoped to the group; escapee/stranger/unmanaged: zero signals"
    );

    if let Ok(rows) = state2.ledger.session_rows(&a) {
        for row in rows.iter().filter_map(|row| row.pid) {
            kill9(row);
        }
    }
    kill9(stranger_pid);
    kill9(unmanaged_pid);
}

const IN_TREE_ALL: &[&str] = &["vite", "term-ignorer", "mcp", "shell", "escapee"];

#[test]
fn term_ignorer_gets_term_then_kill_only_after_grace() {
    let _serial = serialize();
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-guard", false, false);
    widen_grace(&state);
    let kit = FixtureSet::new(dir.path());

    let a = seed_instance(&state, dir.path(), "sess-ign", "acc-guard");
    state
        .supervisor
        .spawn(
            &a,
            &sh_spec(&format!("{}\nwait", kit.root_script()), dir.path()),
            &ctx(&state),
        )
        .unwrap();
    let pids = tick_until_helpers(&state, &a, &["--ignore-term"]);
    let ignorer_pid = pids[0];
    assert!(kit.wait_ready());
    let _port = kit.vite_port();

    // The intended target IS the TERM-ignorer this time: it logs
    // TERM, survives the grace window, and only KILL takes it.
    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        a.as_str(),
        ignorer_pid,
        "pid",
        &cancel_flag(),
        None,
    )
    .unwrap();
    assert!(
        outcome
            .members
            .iter()
            .any(|member| member.pid == ignorer_pid && member.outcome == "needed KILL"),
        "the ladder reports the post-grace KILL: {:?}",
        outcome.members
    );
    assert!(
        fixtures::wait_for(10_000, || !alive(ignorer_pid)),
        "KILL after grace"
    );
    assert_eq!(kit.get("term-ignorer").signals(), vec!["TERM".to_string()]);
    // Everyone else: zero signals.
    kit.assert_untouched(&["vite", "mcp", "shell", "escapee"], "TERM-ignorer drill");

    println!(
        "[term-ignorer] {ignorer_pid}: TERM logged, survived grace, KILLed; collateral zero signals"
    );

    let _ = state.supervisor.stop(&a, Duration::from_secs(3));
    for pid in pids {
        kill9(pid);
    }
}

/// AJ-E512 Req.3: a hog is NEVER auto-signalled at any tier — the
/// Tier-1 sweep stands down for hog-flagged candidates (Tier 3 =
/// alert + staged sheet only), while the manual sheet stays the only
/// path. The alert text is the exact template.
#[test]
fn hog_is_never_auto_signalled_and_alerts_with_exact_copy() {
    let _serial = serialize();
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-guard", true, false);
    // Lower the hog threshold so the fixture (~12 MiB) trips
    // `helper-rss>…` without faking gigabytes — 2 MiB keeps every
    // ring sample above it even under memory pressure (sustained-ness
    // is all-samples).
    {
        let storage = state.storage.lock().unwrap();
        storage
            .app_settings()
            .set("hog_helper_rss_limit_mb", "2")
            .unwrap();
    }
    let kit = FixtureSet::new(dir.path());

    let a = seed_instance(&state, dir.path(), "sess-hog", "acc-guard");
    state
        .supervisor
        .spawn(
            &a,
            &sh_spec(
                // The root outlives the sampling window: hog
                // sustained-ness needs >=2 ring samples while the
                // tree is LIVE (rows freeze at root death).
                &format!("{}\nsleep 25; exit 0", kit.root_script()),
                dir.path(),
            ),
            &ctx(&state),
        )
        .unwrap();
    let pids = tick_until_helpers(
        &state,
        &a,
        &["--port", "mcp-server", "--ignore-term", "touch", "--setsid"],
    );
    let vite_pid = pids[0];
    assert!(kit.wait_ready());
    let port = kit.vite_port();
    // Sustained-ness needs ≥2 ring samples in the window, every one
    // above the lowered limit.
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(mut collector) = state.resources.lock() {
            collector.tick(&state.supervisor, now_ms());
        }
        let hogs = commands::cleanup_alerts_inner(&state);
        // ANY hog alert for the session: the alert names the TOP-RSS
        // helper, which need not be the vite fixture specifically.
        if hogs
            .alerts
            .iter()
            .any(|alert| alert.kind == "hog" && alert.session == a.as_str())
        {
            break;
        }
        let top_rss = state
            .resources
            .lock()
            .ok()
            .and_then(|collector| {
                collector
                    .rows(&a)
                    .into_iter()
                    .flatten()
                    .skip(1)
                    .map(|row| row.rss_bytes)
                    .max()
            })
            .unwrap_or_default();
        println!("[hog-poll] waiting: top helper rss = {top_rss} bytes");
        assert!(
            std::time::Instant::now() < deadline,
            "the hog alert never fired (top helper rss = {top_rss} bytes)"
        );
        std::thread::sleep(Duration::from_millis(750));
    }

    // The hog alert: exact template, naming the top helper and the
    // Instance (the seeded account label).
    let alerts = commands::cleanup_alerts_inner(&state).alerts;
    let hog = alerts
        .iter()
        .find(|alert| alert.kind == "hog" && alert.session == a.as_str())
        .expect("hog alert present");
    // The exact template `{helper} under {Instance} holds {RSS} —
    // review in Cleanup Centre.` — the helper name is the platform's
    // process name (macOS: `Python`), the RSS token is GiB/MiB.
    let (head, tail) = hog
        .text
        .split_once(" under acc-guard holds ")
        .unwrap_or_else(|| panic!("template instance segment: {}", hog.text));
    assert!(!head.is_empty(), "names the helper: {}", hog.text);
    let rss = tail
        .strip_suffix(" — review in Cleanup Centre.")
        .unwrap_or_else(|| panic!("template tail: {}", hog.text));
    assert!(
        rss.ends_with(" MiB") || rss.ends_with(" GiB"),
        "RSS token: {rss}"
    );

    // The sweep: root exits cleanly, the orphan WOULD be Tier-1
    // eligible (a verified DevServer, opted in, past grace) — but it
    // is a hog: ZERO signals, still alive.
    let root_exit = wait_orphans(&state, &a);
    commands::auto_cleanup_sweep(&state, root_exit + 61_000);
    assert!(alive(vite_pid), "a hog is never auto-signalled");
    assert_eq!(
        kit.get("vite").signals(),
        Vec::<String>::new(),
        "zero signals to the hog"
    );
    kit.assert_untouched(&["mcp", "shell", "term-ignorer", "escapee"], "hog sweep");

    // The manual path still works (app prepares, user executes).
    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        a.as_str(),
        vite_pid,
        "pid",
        &cancel_flag(),
        Some(port),
    )
    .unwrap();
    assert!(outcome.all_succeeded, "{outcome:?}");
    assert!(fixtures::wait_for(5_000, || !alive(vite_pid)));
    assert_eq!(kit.get("vite").signals(), vec!["TERM".to_string()]);
    assert!(port_free(port));

    println!("[hog] alert text: {}", hog.text);
    let _ = state.supervisor.stop(&a, Duration::from_secs(3));
    for pid in pids {
        kill9(pid);
    }
}

/// AJ-E512 Req.3: idle-orphan nudges fire at most ONCE per orphan,
/// only past the idle age (CPU≈0 + age only) — the second read
/// (same or any later time) surfaces nothing new for it.
#[test]
fn idle_nudge_fires_exactly_once_per_orphan() {
    let _serial = serialize();
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-guard", true, false);
    {
        let storage = state.storage.lock().unwrap();
        // Tests shrink the 1 h idle age to 1 s.
        storage
            .app_settings()
            .set("idle_nudge_min_age_ms", "1000")
            .unwrap();
    }
    let kit = FixtureSet::new(dir.path());

    let a = seed_instance(&state, dir.path(), "sess-idle", "acc-guard");
    state
        .supervisor
        .spawn(
            &a,
            &sh_spec(
                &format!("{}\nsleep 3; exit 0", kit.root_script()),
                dir.path(),
            ),
            &ctx(&state),
        )
        .unwrap();
    let pids = tick_until_helpers(&state, &a, &["--port"]);
    let vite_pid = pids[0];
    assert!(kit.wait_ready());
    let _port = kit.vite_port();
    let _ = wait_orphans(&state, &a);

    // Age the orphan past the shrunk threshold.
    std::thread::sleep(Duration::from_millis(1_200));
    let first = commands::cleanup_alerts_inner(&state).alerts;
    let nudge = first
        .iter()
        .find(|alert| alert.kind == "idle" && alert.pid == vite_pid)
        .expect("the idle nudge fired");
    assert!(
        nudge.text.contains(" has been idle for ")
            && nudge.text.ends_with(" — review it in the Cleanup Centre."),
        "nudge copy: {}",
        nudge.text
    );
    // The once-flag: the second read surfaces nothing for it.
    let second = commands::cleanup_alerts_inner(&state).alerts;
    assert!(
        !second
            .iter()
            .any(|alert| alert.kind == "idle" && alert.pid == vite_pid),
        "the nudge fires exactly once per orphan"
    );

    println!("[idle] nudge text: {}", nudge.text);
    let _ = state.supervisor.stop(&a, Duration::from_secs(3));
    for pid in pids {
        kill9(pid);
    }
}

/// AJ-E508 exec-settle window (regression — CI flake PR #83,
/// 2026-10-16, `idle_nudge_fires_exactly_once_per_orphan`): a helper
/// discovered between fork and exec records the WRAPPER's exe; the
/// upsert refines it inside the 10 s settle window, but its 2 s
/// throttle races the reconciler's every-tick mismatch verdict — the
/// row flipped `unverifiable` BEFORE the refinement could land (it
/// only touches `running` rows) and orphans never surfaced. The exe
/// leg now withholds its verdict for the SAME window; the start-time
/// leg (the true PID-reuse signal) never waits. Pinned both ways:
/// young wrong-exe rows hold, aged ones flip, and the window CLOSES.
#[test]
fn exe_mismatch_holds_inside_settle_window_and_flips_after_it() {
    let _serial = serialize();
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-guard", false, false);
    let a = seed_instance(&state, dir.path(), "sess-settle", "acc-guard");
    // A live root anchors the forged rows (orphan detection needs an
    // EXITED root — this one never exits during the test).
    let spawned = state
        .supervisor
        .spawn(&a, &sh_spec("sleep 300", dir.path()), &ctx(&state))
        .unwrap();

    let young_pid = forge_exe_row(&state, &a, spawned.pid, dir.path(), "young", now_ms());
    let aged_pid = forge_exe_row(
        &state,
        &a,
        spawned.pid,
        dir.path(),
        "aged",
        now_ms() - juggler_core::storage::EXEC_SETTLE_MS - 1_000,
    );

    // Well past TWO consecutive mismatch ticks: the AGED row flips;
    // the YOUNG row holding proves the WINDOW (not the two-tick
    // discipline) withholds this verdict.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(mut collector) = state.resources.lock() {
            collector.tick(&state.supervisor, now_ms());
        }
        if let Ok(rows) = state.ledger.session_rows(&a) {
            let aged = rows.iter().find(|row| row.pid == Some(aged_pid));
            let young = rows.iter().find(|row| row.pid == Some(young_pid));
            if let Some(aged) = aged
                && aged.state == "unverifiable"
            {
                assert!(
                    young.is_some_and(|row| row.state == "running"),
                    "young row flipped inside the settle window: {:?}",
                    young.map(|row| row.state.clone())
                );
                break;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "aged wrong-exe row never flipped"
        );
        std::thread::sleep(Duration::from_millis(150));
    }

    // The window CLOSES — the guard is deferred, never disabled: the
    // same young row, once past the window, flips on the same fact.
    std::thread::sleep(Duration::from_millis(
        u64::try_from(juggler_core::storage::EXEC_SETTLE_MS).unwrap() + 1_500,
    ));
    let flipped_after_window = fixtures::wait_for(10_000, || {
        if let Ok(mut collector) = state.resources.lock() {
            collector.tick(&state.supervisor, now_ms());
        }
        state
            .ledger
            .session_rows(&a)
            .map(|rows| {
                rows.iter()
                    .any(|row| row.pid == Some(young_pid) && row.state == "unverifiable")
            })
            .unwrap_or(false)
    });
    assert!(
        flipped_after_window,
        "young row never flipped after the settle window closed"
    );

    let _ = state.supervisor.stop(&a, Duration::from_secs(3));
    for pid in [young_pid, aged_pid] {
        kill9(pid);
    }
}

/// AJ-E511 Req.2 end-to-end: a FOREIGN-machine orphan row (a
/// future-synced record) is refused by the typed WrongMachine path —
/// its own copy, zero signals, no audit row — and the all-machines
/// inventory lists it display-only while the local tree stays fully
/// actionable.
#[test]
fn foreign_machine_row_is_refused_with_zero_signals_and_listed_display_only() {
    let _serial = serialize();
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-guard", true, false);
    let kit = FixtureSet::new(dir.path());

    let a = seed_instance(&state, dir.path(), "sess-fleet", "acc-guard");
    let spawned_a = state
        .supervisor
        .spawn(
            &a,
            &sh_spec(&format!("{}\nwait", kit.root_script()), dir.path()),
            &ctx(&state),
        )
        .unwrap();
    let pids = tick_until_helpers(
        &state,
        &a,
        &["--port", "mcp-server", "--ignore-term", "touch", "--setsid"],
    );
    let vite_pid = pids[0];
    assert!(kit.wait_ready());

    // The foreign record: a LIVE local process carrying a FOREIGN
    // machine id — the future-sync shape (this node never owns it).
    let foreign_dir = dir.path().join("foreign");
    std::fs::create_dir_all(&foreign_dir).unwrap();
    let foreign_pid = fixtures::spawn_detached(
        &fixtures::write_helper(&foreign_dir),
        &[],
        &foreign_dir.join("f.traps"),
        &foreign_dir.join("f.ready"),
    );
    let foreign_row = juggler_core::storage::LedgerRow {
        pid: Some(foreign_pid),
        pgid: None,
        started_at: Some(1),
        exe_path: "/bin/python3".into(),
        cmdline_hash: "foreign".into(),
        cmdline_trunc: "python3 trap_helper.py (foreign machine)".into(),
        boot_id: state.ledger.boot_id().to_string(),
        session_id: a.as_str().to_string(),
        instance_id: None,
        provider: "claude-code".into(),
        account_id: Some("acc-guard".into()),
        project_id: Some("proj-guard".into()),
        machine_id: "machine-other".into(),
        root_pid: Some(spawned_a.pid),
        parent_pid: Some(spawned_a.pid),
        depth: 1,
        role: "helper".into(),
        state: "orphaned".into(),
        listens_on: None,
        cwd: dir.path().to_string_lossy().to_string(),
        peak_rss: None,
        peak_cpu: None,
        exit_kind: None,
        reaped_by_us: None,
        escaped_group: false,
        down_period: false,
        first_seen: now_ms(),
        last_seen: now_ms(),
    };
    state.ledger.upsert_helpers(&a, &[foreign_row]).unwrap();

    // The sheet: refused with the TYPED copy, no members listed.
    let verify = commands::kill_verify_inner(&state, a.as_str(), foreign_pid).unwrap();
    assert!(!verify.verified);
    assert_eq!(
        verify.refusal.as_deref(),
        Some(commands::KILL_WRONG_MACHINE_COPY)
    );
    assert_eq!(verify.machine_id, "machine-other");
    assert!(
        verify.members.is_empty(),
        "no member listing for a foreign row"
    );

    // The ladder: refused, ZERO signals, NO audit row.
    let audit_before = state.audit.history(0).unwrap().len();
    let outcome = commands::kill_execute_audited(
        &state,
        state.audit.as_ref(),
        a.as_str(),
        foreign_pid,
        "pid",
        &cancel_flag(),
        None,
    )
    .unwrap();
    assert_eq!(
        outcome.refusal.as_deref(),
        Some(commands::KILL_WRONG_MACHINE_COPY)
    );
    assert_eq!(outcome.members[0].outcome, "refused");
    let foreign_traps = std::fs::read_to_string(foreign_dir.join("f.traps")).unwrap_or_default();
    assert!(
        foreign_traps.trim().is_empty(),
        "foreign row signalled: {foreign_traps}"
    );
    assert!(fixtures::alive(foreign_pid));
    assert_eq!(state.audit.history(0).unwrap().len(), audit_before);

    // The sweep never sees it (candidates are machine-scoped) — run
    // it against the live tree; nothing is eligible, and the foreign
    // row must survive regardless.
    commands::auto_cleanup_sweep(&state, now_ms() + 61_000);
    assert!(
        fixtures::alive(foreign_pid),
        "the sweep never signals a foreign-machine row"
    );

    // The all-machines inventory lists it (display-only in the UI)
    // beside the local tree, and names THIS node.
    let local = commands::cleanup_inventory_inner(&state, "all-managed", false);
    assert!(
        local.groups.iter().all(|group| group
            .rows
            .iter()
            .all(|row| { row.machine_id == local.local_machine })),
        "the default scope lists only this machine"
    );
    let all = commands::cleanup_inventory_inner(&state, "all-managed", true);
    assert_ne!(all.local_machine, "machine-other");
    assert!(
        all.groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .any(|row| row.pid == foreign_pid && row.machine_id == "machine-other"),
        "the all-machines scope lists the foreign row"
    );

    // The LOCAL tree is unaffected: still fully actionable.
    let local_verify = commands::kill_verify_inner(&state, a.as_str(), vite_pid).unwrap();
    assert!(local_verify.verified, "local rows verify normally");

    println!(
        "[fleet] foreign {foreign_pid} refused + zero signals + listed display-only; local tree actionable"
    );

    let _ = state.supervisor.stop(&a, Duration::from_secs(3));
    for pid in pids {
        kill9(pid);
    }
    kill9(foreign_pid);
}

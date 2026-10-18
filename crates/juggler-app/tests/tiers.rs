//! AJ-E508 evidence: the operating tiers — the Tier-1 drill (a
//! clean-exit tree with one dev-server and one MCP-role helper: only
//! the dev-server is auto-TERMed, with the audit intent written
//! BEFORE the signal), the default-install guardrail (Tier 0 signals
//! nothing), the pause kill-switch, per-Account isolation, the
//! Tier-2 crash flow (5 min → ONE notification → ≥30 s Undo window →
//! signal only if not undone), demotion on re-verification failure,
//! and the 24 h crash suspension. Run with `--nocapture` to archive
//! the evidence dumps.

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
use juggler_core::model::{
    Account, AccountId, InstanceKind, InstanceRecord, LaunchRecipe, MachineId, ProfileId,
    SessionId, SessionStatus, Timestamp,
};

fn now_ms() -> i64 {
    Timestamp::now().millis()
}

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

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
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
        account_id: Some("acc-t1".into()),
        project_id: Some("proj-t1".into()),
        machine: commands::local_machine_id(state),
        role: "root",
    }
}

/// Seeds one Account (id, tier toggles) + recipe + Instance row so
/// the sweep can resolve session → account.
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
    let settings = juggler_core::storage::CleanupTierSettings {
        tier1_opt_in: tier1,
        tier2_opt_in: tier2,
    };
    state.cleanup_settings.set(id, &settings).unwrap();
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

/// One drill tree: a dev-server helper (python http.server on an
/// ephemeral port), an MCP-role helper (`mcp-server` in its argv),
/// and a root that exits CLEANLY (code 0) or CRASHES (SIGKILL on
/// itself) after the helpers are discoverable. Returns the session.
fn drill_tree(
    state: &Arc<AppState>,
    dir: &Path,
    id: &str,
    account: &str,
    clean: bool,
) -> SessionId {
    let session = seed_instance(state, dir, id, account);
    let port = free_port();
    let exit = if clean { "exit 0" } else { "kill -9 $$" };
    let script = format!(
        "python3 -m http.server {port} --bind 127.0.0.1 & \
         python3 -c 'import time; time.sleep(300)' mcp-server & \
         sleep 3; {exit}"
    );
    state
        .supervisor
        .spawn(&session, &sh_spec(&script, dir), &ctx(state))
        .unwrap();
    session
}

/// Splits the orphaned rows into (dev-server row, mcp row).
fn split_roles(
    rows: &[juggler_core::storage::LedgerRow],
) -> (
    juggler_core::storage::LedgerRow,
    juggler_core::storage::LedgerRow,
) {
    let dev = rows
        .iter()
        .find(|row| row.cmdline_trunc.contains("http.server"))
        .cloned()
        .expect("dev-server orphan row");
    let mcp = rows
        .iter()
        .find(|row| row.cmdline_trunc.contains("mcp-server"))
        .cloned()
        .expect("mcp orphan row");
    (dev, mcp)
}

/// Drives the reconciler ticks until the session's HELPER rows flip
/// `orphaned` (the ledger verdict — the sweep's candidate source),
/// then returns (root_last_seen, orphaned helper pids).
fn wait_ledger_orphans(
    state: &Arc<AppState>,
    session: &str,
) -> Option<(i64, Vec<juggler_core::storage::LedgerRow>)> {
    use juggler_supervisor::tree::LedgerStore as _;
    state.supervisor.set_orphan_grace("claude-code", 3);
    let deadline = std::time::Instant::now() + Duration::from_secs(25);
    while std::time::Instant::now() < deadline {
        if let Ok(mut collector) = state.resources.lock() {
            collector.tick(&state.supervisor, now_ms());
        }
        if let Ok(rows) = state
            .ledger
            .session_rows(&SessionId::new(session.to_string()))
        {
            let orphaned: Vec<_> = rows
                .iter()
                .filter(|row| row.role == "helper" && row.state == "orphaned")
                .cloned()
                .collect();
            let root_last = rows
                .iter()
                .find(|row| row.role == "root")
                .map(|root| root.last_seen);
            if orphaned.len() >= 2
                && let Some(root_last_seen) = root_last
            {
                return Some((root_last_seen, orphaned));
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    None
}

fn wait_gone(pid: u32) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(4);
    while std::time::Instant::now() < deadline {
        if !alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn app_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState::init_with_registry(dir.join("app-data"), AdapterRegistry::new()).unwrap())
}

/// The guardrail alongside the SPEC §1 invariant: a DEFAULT install
/// (Tier 0 everywhere) signals nothing across the fixture suite.
#[test]
fn default_install_signals_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-t1", false, false);
    let session = drill_tree(&state, dir.path(), "sess-default", "acc-t1", true);
    let (root_exit, rows) =
        wait_ledger_orphans(&state, "sess-default").expect("orphans never surfaced in the ledger");
    let (dev, mcp) = split_roles(&rows);
    commands::auto_cleanup_sweep(&state, root_exit + 61_000);
    assert!(
        alive(dev.pid.unwrap()),
        "Tier 0 default: the dev server survives"
    );
    assert!(
        alive(mcp.pid.unwrap()),
        "Tier 0 default: the MCP helper survives"
    );
    assert!(
        state.audit.history(0).unwrap().is_empty(),
        "no audit rows without opt-in"
    );
    // Manual cleanup still works at Tier 0 (the sheet is always there).
    let outcome = commands::orphan_stop_inner(&state, &session, dev.pid.unwrap()).unwrap();
    assert_eq!(
        outcome.state, "refused - not a recorded descendant",
        "the E410 allow-list no longer knows this pid — the manual sheet (AJ-E507) is the path"
    );
    kill9(dev.pid.unwrap());
    kill9(mcp.pid.unwrap());
}

/// The Tier-1 drill (pass2 §14.5): clean-exit tree, Tier 1 ON — the
/// dev-server is auto-TERMed after grace with the audit intent
/// written before the signal; the MCP-role helper is untouched.
#[test]
fn tier1_drill_kills_only_the_dev_server() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-t1", true, false);
    drill_tree(&state, dir.path(), "sess-drill", "acc-t1", true);
    let (root_exit, rows) =
        wait_ledger_orphans(&state, "sess-drill").expect("orphans never surfaced");
    let (dev, mcp) = split_roles(&rows);
    let (dev_pid, mcp_pid) = (dev.pid.unwrap(), mcp.pid.unwrap());
    commands::auto_cleanup_sweep(&state, root_exit + 61_000);

    assert!(wait_gone(dev_pid), "the dev-server must be auto-TERMed");
    assert!(alive(mcp_pid), "the MCP-role helper must never be touched");
    // The audit: intent BEFORE signal, actor auto:tier-1, completed.
    let history = state.audit.history(0).unwrap();
    let row = history
        .iter()
        .find(|row| row.pid == dev_pid)
        .unwrap_or_else(|| panic!("no audit row for the dev-server: {history:#?}"));
    assert_eq!(row.actor, "auto:tier-1");
    assert!(row.automatic, "the history reads it as automatic");
    assert!(row.outcome.contains("exited on TERM"));
    // The journal names the trigger (the D-08 audit trail).
    let journal = std::fs::read_to_string(state.supervisor.journal_ref().unwrap().path()).unwrap();
    assert!(journal.contains("auto-tier1"), "journal names the trigger");
    println!("[tier1-drill] dev {dev_pid} gone, mcp {mcp_pid} alive; audit: {row:#?}");
    kill9(mcp_pid);
}

/// Pause ON ⇒ zero signals even for a fully-eligible candidate; the
/// flag takes effect within one tick (the sweep reads it first).
#[test]
fn pause_blocks_every_signal() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-t1", true, true);
    drill_tree(&state, dir.path(), "sess-paused", "acc-t1", true);
    let (root_exit, rows) =
        wait_ledger_orphans(&state, "sess-paused").expect("orphans never surfaced");
    let (dev, mcp) = split_roles(&rows);
    let (dev_pid, mcp_pid) = (dev.pid.unwrap(), mcp.pid.unwrap());
    let status = commands::cleanup_pause_set_arc(&state, true).unwrap();
    assert!(status.paused, "the banner's data source");
    commands::auto_cleanup_sweep(&state, root_exit + 61_000);
    assert!(alive(dev_pid), "paused: zero signals");
    assert!(alive(mcp_pid));
    assert!(state.audit.history(0).unwrap().is_empty());
    // Unpausing resumes within one tick.
    commands::cleanup_pause_set_arc(&state, false).unwrap();
    commands::auto_cleanup_sweep(&state, root_exit + 61_000);
    assert!(wait_gone(dev_pid), "resumed: the sweep proceeds");
    kill9(mcp_pid);
}

/// Per-Account isolation: an opt-in on Account A never cleans Account
/// B's equally-eligible orphan.
#[test]
fn account_isolation_only_cleans_the_opted_in_account() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-a", true, false);
    seed_account(&state, dir.path(), "acc-b", false, false);
    drill_tree(&state, dir.path(), "sess-a", "acc-a", true);
    drill_tree(&state, dir.path(), "sess-b", "acc-b", true);
    let (exit_a, rows_a) = wait_ledger_orphans(&state, "sess-a").expect("A's orphans");
    let (exit_b, rows_b) = wait_ledger_orphans(&state, "sess-b").expect("B's orphans");
    let (dev_a, mcp_a) = split_roles(&rows_a);
    let (dev_b, mcp_b) = split_roles(&rows_b);
    commands::auto_cleanup_sweep(&state, exit_a.max(exit_b) + 61_000);
    assert!(
        wait_gone(dev_a.pid.unwrap()),
        "Account A (opted in): cleaned"
    );
    assert!(
        alive(dev_b.pid.unwrap()),
        "Account B (not opted in): never touched"
    );
    kill9(mcp_a.pid.unwrap());
    kill9(dev_b.pid.unwrap());
    kill9(mcp_b.pid.unwrap());
}

/// The Tier-2 crash flow: a crash root needs BOTH 5 minutes AND one
/// notification AND an elapsed ≥30 s Undo window; Undo dismisses it
/// forever (exactly one notification per crash-orphan).
#[test]
fn tier2_crash_flow_notification_undo_and_lapse() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-t1", false, true);
    drill_tree(&state, dir.path(), "sess-crash", "acc-t1", false);
    let (parent_exit, rows) =
        wait_ledger_orphans(&state, "sess-crash").expect("crash orphans never surfaced");
    let (dev, mcp) = split_roles(&rows);
    let dev_pid = dev.pid.unwrap();
    let mcp_pid = mcp.pid.unwrap();

    // Before 5 minutes: nothing (not even a notification).
    commands::auto_cleanup_sweep(&state, parent_exit + 4 * 60_000);
    assert!(alive(dev_pid));
    assert!(
        commands::cleanup_status_inner(&state)
            .unwrap()
            .tier2_pending
            .is_empty(),
        "no notification before the 5-minute grace"
    );

    // At 5 min + 1 s: exactly ONE notification, still no signal.
    commands::auto_cleanup_sweep(&state, parent_exit + 5 * 60_000 + 1_000);
    let status = commands::cleanup_status_inner(&state).unwrap();
    assert_eq!(status.tier2_pending.len(), 1, "one notification");
    assert!(status.tier2_pending[0].pid == dev_pid);
    assert!(alive(dev_pid), "the Undo window defers the signal");
    // No repeat notifications while the window is open.
    commands::auto_cleanup_sweep(&state, parent_exit + 5 * 60_000 + 2_000);
    assert_eq!(
        commands::cleanup_status_inner(&state)
            .unwrap()
            .tier2_pending
            .len(),
        1,
        "exactly one notification per crash-orphan"
    );

    // UNDO: the user keeps it running — never signalled.
    commands::cleanup_tier2_decide_arc(&state, dev_pid, true).unwrap();
    commands::auto_cleanup_sweep(&state, parent_exit + 5 * 60_000 + 60_000);
    assert!(alive(dev_pid), "Undo wins: never signalled");
    assert!(
        commands::cleanup_status_inner(&state)
            .unwrap()
            .tier2_pending
            .is_empty(),
        "the notice is dismissed"
    );

    // A SECOND crash orphan, left to LAPSE: signalled after the window
    // with the auto:tier-2 audit actor.
    let dir2 = tempfile::tempdir().unwrap();
    let state2 = app_state(dir2.path());
    seed_account(&state2, dir2.path(), "acc-t1", false, true);
    drill_tree(&state2, dir2.path(), "sess-lapse", "acc-t1", false);
    let (exit2, rows2) =
        wait_ledger_orphans(&state2, "sess-lapse").expect("crash orphans never surfaced");
    let (dev_row2, mcp_row2) = split_roles(&rows2);
    let (dev2, mcp2) = (dev_row2.pid.unwrap(), mcp_row2.pid.unwrap());
    commands::auto_cleanup_sweep(&state2, exit2 + 5 * 60_000 + 1_000);
    assert!(alive(dev2), "window open: no signal yet");
    commands::auto_cleanup_sweep(&state2, exit2 + 5 * 60_000 + 31_000);
    assert!(wait_gone(dev2), "window elapsed without Undo: signalled");
    let history = state2.audit.history(0).unwrap();
    let row = history
        .iter()
        .find(|row| row.pid == dev2)
        .unwrap_or_else(|| panic!("no tier-2 audit row: {history:#?}"));
    assert_eq!(row.actor, "auto:tier-2");
    println!("[tier2-lapse] dev {dev2} gone after the window; audit: {row:#?}");
    kill9(mcp_pid);
    kill9(mcp2);
}

/// A failed same-tick re-verification DEMOTES the candidate to Tier 0
/// (list-only) with an audit note — never a signal on stale evidence.
#[test]
fn verification_failure_demotes_to_tier0_with_audit_note() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-t1", true, false);
    drill_tree(&state, dir.path(), "sess-demote", "acc-t1", true);
    let (root_exit, rows) =
        wait_ledger_orphans(&state, "sess-demote").expect("orphans never surfaced");
    let (dev, mcp) = split_roles(&rows);
    let dev_pid = dev.pid.unwrap();
    // The pid is gone by sweep time (the PID-reuse shape): the ledger
    // row stays orphaned, verification fails CLOSED.
    kill9(dev_pid);
    assert!(wait_gone(dev_pid));
    commands::auto_cleanup_sweep(&state, root_exit + 61_000);
    let history = state.audit.history(0).unwrap();
    let row = history
        .iter()
        .find(|row| row.pid == dev_pid)
        .unwrap_or_else(|| panic!("no demote note: {history:#?}"));
    assert!(
        row.outcome.contains("demoted"),
        "the note names it: {}",
        row.outcome
    );
    kill9(mcp.pid.unwrap());
}

/// The 24 h crash suspension: while suspended, the sweep stands down;
/// the user's re-confirm (the Centre's button) resumes it.
#[test]
fn crash_suspension_blocks_until_reconfirmed() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-t1", true, false);
    {
        let storage = state.storage.lock().unwrap();
        storage
            .app_settings()
            .set(
                "cleanup_suspended_until",
                &(now_ms() + 3_600_000).to_string(),
            )
            .unwrap();
    }
    let status = commands::cleanup_status_inner(&state).unwrap();
    assert!(status.suspended, "the banner's data source");
    assert!(status.suspended_until_ms.is_some());

    drill_tree(&state, dir.path(), "sess-susp", "acc-t1", true);
    let (root_exit, rows) =
        wait_ledger_orphans(&state, "sess-susp").expect("orphans never surfaced");
    let (dev, mcp) = split_roles(&rows);
    let dev_pid = dev.pid.unwrap();
    commands::auto_cleanup_sweep(&state, root_exit + 61_000);
    assert!(alive(dev_pid), "suspended: zero signals");

    commands::cleanup_resume_after_crash_arc(&state).unwrap();
    assert!(
        !commands::cleanup_status_inner(&state).unwrap().suspended,
        "re-confirmed: the banner clears"
    );
    commands::auto_cleanup_sweep(&state, root_exit + 61_000);
    assert!(wait_gone(dev_pid), "resumed after re-confirm");
    kill9(mcp.pid.unwrap());
}

/// The two toggles persist independently through their command, and
/// the legacy single-flag surface mirrors Tier 1 (one honest source).
#[test]
fn tier_toggles_persist_independently() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path());
    seed_account(&state, dir.path(), "acc-t1", false, false);
    let set = commands::cleanup_tiers_set_arc(&state, "acc-t1", true, false).unwrap();
    assert!(set.tier1_opt_in && !set.tier2_opt_in);
    let read = commands::cleanup_tiers_get_arc(&state, "acc-t1").unwrap();
    assert_eq!(read, set);
    // Tier 2 is a separate consent.
    let set = commands::cleanup_tiers_set_arc(&state, "acc-t1", true, true).unwrap();
    assert!(set.tier1_opt_in && set.tier2_opt_in);
    // The legacy account flag mirrors Tier 1.
    let account = {
        let storage = state.storage.lock().unwrap();
        storage.accounts().get(&AccountId::new("acc-t1")).unwrap()
    };
    assert!(account.cleanup_opt_in, "the legacy flag mirrors Tier 1");
    // Off again.
    commands::cleanup_tiers_set_arc(&state, "acc-t1", false, false).unwrap();
    assert!(
        !commands::cleanup_tiers_get_arc(&state, "acc-t1")
            .unwrap()
            .tier1_opt_in
    );
}

//! Ledger rows for a test's own processes: the one record every stop
//! path verifies (pid + start time + exe + boot) before any signal.

use std::sync::Arc;

use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_core::model::SessionId;
use juggler_supervisor::tree::{IdentityProbe as _, LedgerStore as _, OsIdentityProbe};

/// What the kernel says about `pid` now: its start time (epoch s) and
/// executable; `None` when it is not readable.
#[must_use]
pub fn live_identity(pid: u32) -> Option<(i64, String)> {
    let probed = OsIdentityProbe.probe(pid)?;
    Some((probed.start_time, probed.exe.to_string_lossy().into_owned()))
}

/// Records `pid` as a helper of `session` on this Mac, in `row_state`
/// (`orphaned`, `running`), with the given identity. Seen ten minutes
/// ago, so the exec-settle window never applies.
///
/// # Errors
///
/// The ledger's own error when the row cannot be written.
pub fn seed_helper(
    state: &Arc<AppState>,
    session: &SessionId,
    pid: u32,
    started_at: Option<i64>,
    exe_path: &str,
    row_state: &str,
) -> Result<(), juggler_supervisor::tree::LedgerError> {
    let seen = juggler_core::model::Timestamp::now().millis() - 600_000;
    state.ledger.upsert_helpers(
        session,
        &[juggler_core::storage::LedgerRow {
            pid: Some(pid),
            pgid: None,
            started_at,
            exe_path: exe_path.to_string(),
            cmdline_hash: format!("test-{pid}"),
            cmdline_trunc: format!("test fixture {pid}"),
            boot_id: state.ledger.boot_id().to_string(),
            session_id: session.as_str().to_string(),
            instance_id: None,
            provider: "claude-code".into(),
            account_id: Some("acc-test".into()),
            project_id: Some("proj-test".into()),
            machine_id: commands::local_machine_id(state).as_str().to_string(),
            root_pid: None,
            parent_pid: None,
            depth: 1,
            role: "helper".into(),
            state: row_state.to_string(),
            listens_on: None,
            cwd: "/".into(),
            peak_rss: None,
            peak_cpu: None,
            exit_kind: None,
            reaped_by_us: None,
            escaped_group: false,
            down_period: false,
            first_seen: seen,
            last_seen: seen,
        }],
    )
}

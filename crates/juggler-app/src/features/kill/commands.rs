//! The kill commands. Each runs off the main thread.

use std::sync::Arc;

use crate::error::AppResult;
use crate::features::kill::dto::{KillExecuteInfo, KillVerifyInfo};
use crate::features::kill::{kill_execute_audited, kill_verify_inner};
use crate::state::AppState;

/// The verify sheet's step-1 command: re-runs the E503 identity check
/// NOW (not from the last tick), re-reads exe/cmdline/cwd/ports, and
/// enumerates the PGID collateral (Req.1/Req.5). Zero signalling.
///
/// # Errors
///
/// [`AppError::NotFound`](crate::error::AppError::NotFound) when the pid has no ledger row for the
/// session (the UI never offers a kill on unmanaged rows — Req.2 —
/// so this is a contract breach, not a UI state).
#[tauri::command]
pub async fn kill_verify(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    pid: u32,
) -> AppResult<KillVerifyInfo> {
    crate::ipc::blocking(state.inner(), "kill verify", move |state| {
        kill_verify_inner(state, &session_id, pid)
    })
    .await
}

/// The kill sheet's step 3: the audited ladder. Guardrails, in
/// order: (1) same-tick re-verification of EVERY signalled pid's
/// anchor — a reused PID gets ZERO signals and the exact refusal
/// copy; (2) the AJ-E509 audit intent BEFORE the first signal (no
/// audit, no kill); (3) the ladder — TERM, cancellable
/// grace, KILL only survivors; (4) the honest post-ladder port
/// re-probe (still-held is a fact, never a failure to hide).
///
/// # Errors
///
/// [`AppError::NotFound`](crate::error::AppError::NotFound) when no ledger row exists; validation when
/// `scope` is not one of `pid`|`pgid`|`tree`;
/// [`AppError::AuditUnavailable`](crate::error::AppError::AuditUnavailable) when the audit intent fails (the
/// kill is refused, zero signals sent).
#[tauri::command]
pub async fn kill_execute(
    state: tauri::State<'_, Arc<AppState>>,
    session_id: String,
    pid: u32,
    scope: String,
    attempt: String,
    goal_port: Option<u16>,
) -> AppResult<KillExecuteInfo> {
    crate::ipc::blocking(state.inner(), "kill execute", move |state| {
        let flag = state.register_kill_cancel(&attempt);
        let result = kill_execute_audited(
            state,
            state.audit.as_ref(),
            &session_id,
            pid,
            &scope,
            &flag,
            goal_port,
        );
        state.unregister_kill_cancel(&attempt);
        result
    })
    .await
}

/// Cancels a running ladder's grace window (the UI's Cancel button).
/// Idempotent; unknown attempts are no-ops (the ladder already
/// finished).
#[tauri::command]
///
/// # Errors
///
/// The work could not finish off the main thread.
pub async fn kill_cancel(state: tauri::State<'_, Arc<AppState>>, attempt: String) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "kill cancel", move |state| {
        state.flip_kill_cancel(&attempt);
        Ok(())
    })
    .await
}

/// The grace window's per-process exit ticks (Req.3): liveness of the
/// ladder's known members, probed cheaply (one targeted check per
/// pid — no scans). The sheet polls this while the ladder runs; the
/// AUTHORITATIVE outcomes still come from `kill_execute`'s report.
#[tauri::command]
pub async fn kill_liveness(pids: Vec<u32>) -> Vec<bool> {
    pids.iter()
        .map(|pid| juggler_supervisor::process_alive(*pid).is_some())
        .collect()
}

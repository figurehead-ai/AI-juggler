//! Embedded terminal surface (M1-4, spec §9): PTY sessions streamed to the
//! webview as base64 chunks over Tauri events, with keyboard input, resize,
//! snapshot replay for panel reopen, and guided login terminals.
//!
//! The forwarder batches chunks with a 16 ms flush so bursty harness output
//! coalesces into at most ~60 events/second/terminal — the UI frame budget.

use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use juggler_core::model::SessionId;

use crate::commands::{find_account_arc, unique_suffix};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Event: a batch of PTY output for one terminal.
pub const EVENT_TERMINAL_OUTPUT: &str = "terminal://output";
/// Event: a terminal's process exited.
pub const EVENT_TERMINAL_EXIT: &str = "terminal://exit";

/// How long the forwarder waits before flushing a partial batch (the UI
/// frame budget; also bounds idle wakeups).
const FLUSH_INTERVAL: Duration = Duration::from_millis(16);

/// Payload of [`EVENT_TERMINAL_OUTPUT`].
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TerminalChunk {
    /// Terminal (session) id.
    pub id: String,
    /// Base64-encoded PTY bytes since the previous chunk.
    pub data: String,
}

/// A spawned terminal's identity (also the exit-event payload).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TerminalInfo {
    /// Terminal (session) id — used for input/resize/kill/attach.
    pub id: String,
}

/// Spawns the vendor's guided login in an embedded terminal against the
/// account's isolated config dir (ADR-0003: we spawn; the user completes
/// the vendor's flow).
///
/// # Errors
///
/// Returns the typed [`AppError`] message on failure.
pub fn spawn_login_terminal_impl(
    app: &AppHandle,
    state: &Arc<AppState>,
    account_id: &str,
) -> AppResult<TerminalInfo> {
    let account = find_account_arc(state, account_id)?;
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.as_str().to_string()))?;
    let spec = adapter.login_spec(&account.state_dir)?;
    let id = SessionId::new(format!("login-{}", unique_suffix()));
    state.supervisor.spawn(
        &id,
        &spec,
        &juggler_supervisor::tree::SpawnContext {
            provider: account.harness.to_string(),
            account_id: Some(account.id.to_string()),
            project_id: None,
            machine: crate::commands::local_machine_id(state),
            role: juggler_core::storage::ROLE_LOGIN,
        },
    )?;
    attach_forwarder(app, state, &id);
    Ok(TerminalInfo {
        id: id.as_str().to_string(),
    })
}

/// Starts streaming a supervised session to the webview (idempotent: one
/// forwarder per terminal regardless of how often the UI re-attaches —
/// e.g. after a panel close/reopen).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the session is unknown.
pub fn attach_forwarder(app: &AppHandle, state: &Arc<AppState>, id: &SessionId) {
    {
        let Ok(mut taps) = state.terminal_taps.lock() else {
            return;
        };
        if !taps.insert(id.as_str().to_string()) {
            return; // already streaming
        }
    }
    let Ok(rx) = state.supervisor.tap(id) else {
        if let Ok(mut taps) = state.terminal_taps.lock() {
            taps.remove(id.as_str());
        }
        return;
    };
    let app = app.clone();
    let state = Arc::clone(state);
    let id = id.clone();
    std::thread::spawn(move || {
        forward_loop(&app, &state, &id, rx);
    });
}

// The receiver is the thread's sole input source — ownership is the
// interface (clippy wants &Receiver, which would be wrong here).
#[allow(clippy::needless_pass_by_value)]
fn forward_loop(
    app: &AppHandle,
    state: &Arc<AppState>,
    id: &SessionId,
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
) {
    loop {
        let mut batch: Vec<u8> = Vec::new();
        match rx.recv_timeout(FLUSH_INTERVAL) {
            Ok(chunk) => {
                batch.extend_from_slice(&chunk);
                // Drain whatever else is ready without blocking so a
                // single event carries the whole burst.
                while let Ok(more) = rx.try_recv() {
                    batch.extend_from_slice(&more);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if state.supervisor.is_finished(id) {
                    emit_exit(app, state, id);
                    return;
                }
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                emit_exit(app, state, id);
                return;
            }
        }
        let _ = app.emit(
            EVENT_TERMINAL_OUTPUT,
            TerminalChunk {
                id: id.as_str().to_string(),
                data: base64::engine::general_purpose::STANDARD.encode(&batch),
            },
        );
    }
}

fn emit_exit(app: &AppHandle, state: &Arc<AppState>, id: &SessionId) {
    if let Ok(mut taps) = state.terminal_taps.lock() {
        taps.remove(id.as_str());
    }
    let _ = app.emit(
        EVENT_TERMINAL_EXIT,
        TerminalInfo {
            id: id.as_str().to_string(),
        },
    );
}

// ---- command cores (thin #[tauri::command] wrappers live in commands.rs) --

/// Feeds keystrokes to a terminal.
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the session is unknown or
/// not a PTY.
pub fn terminal_write_impl(state: &Arc<AppState>, id: &str, data: &str) -> AppResult<()> {
    state
        .supervisor
        .write_input(&SessionId::new(id), data.as_bytes())?;
    Ok(())
}

/// Resizes a terminal.
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the session is unknown or
/// not a PTY.
pub fn terminal_resize_impl(
    state: &Arc<AppState>,
    id: &str,
    cols: u16,
    rows: u16,
) -> AppResult<()> {
    state
        .supervisor
        .resize_pty(&SessionId::new(id), cols, rows)?;
    Ok(())
}

/// Stops a terminal's process (graceful ladder).
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the session is unknown.
pub fn terminal_kill_impl(state: &Arc<AppState>, id: &str) -> AppResult<()> {
    state
        .supervisor
        .stop(&SessionId::new(id), Duration::from_secs(5))?;
    Ok(())
}

/// The retained output tail (base64) — replayed into xterm.js when a
/// terminal panel reopens.
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the session is unknown.
pub fn terminal_snapshot_impl(state: &Arc<AppState>, id: &str) -> AppResult<String> {
    let bytes = state.supervisor.output(&SessionId::new(id))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

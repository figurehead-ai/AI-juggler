//! Embedded terminal surface (M1-4, spec §9): PTY sessions streamed to the
//! webview as base64 chunks over Tauri events, with keyboard input, resize,
//! snapshot replay for panel reopen, and guided login terminals.
//!
//! The forwarder batches chunks with a 16 ms flush so bursty harness output
//! coalesces into at most ~60 events/second/terminal — the UI frame budget.

use crate::events::{AppEvent, EmitEvent as _};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use juggler_core::model::SessionId;

use crate::commands::find_account_arc;
use crate::error::{AppError, AppResult};
use crate::ids::unique_suffix;
use crate::state::AppState;

/// Event: a batch of PTY output for one terminal.
pub const EVENT_TERMINAL_OUTPUT: &str = AppEvent::TerminalOutput.name();
/// Event: a terminal's process exited.
pub const EVENT_TERMINAL_EXIT: &str = AppEvent::TerminalExit.name();

/// A sign-in terminal's id begins so (`login-<suffix>`).
pub const LOGIN_TERMINAL_PREFIX: &str = "login-";

/// Whether terminal `id` is a sign-in's ([`spawn_login_terminal_impl`]).
#[must_use]
pub fn is_login_terminal(id: &str) -> bool {
    id.starts_with(LOGIN_TERMINAL_PREFIX)
}

/// How long the forwarder waits before flushing a partial batch (the UI
/// frame budget; also bounds idle wakeups).
const FLUSH_INTERVAL: Duration = Duration::from_millis(16);

/// Payload of [`EVENT_TERMINAL_OUTPUT`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TerminalChunk {
    /// Terminal (session) id.
    pub id: String,
    /// Base64-encoded PTY bytes since the previous chunk.
    pub data: String,
    /// SK-23: where these bytes end in the terminal's whole output (the
    /// count of bytes it has written through them), so a view joins them
    /// to the kept output ([`TerminalSnapshot`]) by place: no byte missing
    /// or twice, whichever it hears first.
    #[ts(type = "number")]
    pub end: u64,
}

/// The kept output of a terminal, replayed when its view opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TerminalSnapshot {
    /// Base64-encoded PTY bytes: the tail AI Juggler keeps while it runs.
    pub data: String,
    /// SK-23: where those bytes end in the terminal's whole output (see
    /// [`TerminalChunk::end`]).
    #[ts(type = "number")]
    pub end: u64,
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
    let id = SessionId::new(format!("{LOGIN_TERMINAL_PREFIX}{}", unique_suffix()));
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

/// AJ-E2304: the real terminal UI on a driven session's conversation (its
/// adapter's terminal view: `codex --remote …` attached live, or the
/// tool's resume after the driver let go), in AI Juggler's terminal. An
/// owned child like a sign-in terminal, under the session's account.
///
/// # Errors
///
/// The supervisor's spawn error.
pub fn spawn_view_terminal(
    app: &AppHandle,
    state: &Arc<AppState>,
    account: &juggler_core::model::Account,
    spec: &juggler_adapter::LaunchSpec,
) -> AppResult<TerminalInfo> {
    let id = SessionId::new(format!("view-{}", unique_suffix()));
    // Its account's key, through the one spawn path.
    crate::features::sessions::spawn::spawn_view(state, &id, account, spec)?;
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
    let Ok((start, rx)) = state.supervisor.tap_at(id) else {
        if let Ok(mut taps) = state.terminal_taps.lock() {
            taps.remove(id.as_str());
        }
        return;
    };
    let app = app.clone();
    let state = Arc::clone(state);
    let id = id.clone();
    std::thread::spawn(move || {
        forward_loop(&app, &state, &id, start, rx);
    });
}

/// The event for `batch`, the bytes after `*end` in terminal `id`'s
/// output; `*end` moves past them (SK-23: each chunk says where it ends).
fn chunk_after(id: &SessionId, end: &mut u64, batch: &[u8]) -> TerminalChunk {
    *end = end.saturating_add(u64::try_from(batch.len()).unwrap_or(u64::MAX));
    TerminalChunk {
        id: id.as_str().to_string(),
        data: base64::engine::general_purpose::STANDARD.encode(batch),
        end: *end,
    }
}

// The receiver is the thread's sole input source — ownership is the
// interface (clippy wants &Receiver, which would be wrong here).
#[allow(clippy::needless_pass_by_value)]
fn forward_loop(
    app: &AppHandle,
    state: &Arc<AppState>,
    id: &SessionId,
    start: u64,
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
) {
    let mut end = start;
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
        app.emit_event(AppEvent::TerminalOutput, chunk_after(id, &mut end, &batch));
    }
}

fn emit_exit(app: &AppHandle, state: &Arc<AppState>, id: &SessionId) {
    if let Ok(mut taps) = state.terminal_taps.lock() {
        taps.remove(id.as_str());
    }
    // AJ-E1612: a sign-in ended: its App's sign-in is read again now, so
    // Needs you drops "needs its sign-in" before the window hears of it.
    if is_login_terminal(id.as_str()) {
        crate::features::notifications::needs::sign_in_ended(state);
    }
    app.emit_event(
        AppEvent::TerminalExit,
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
    let session = SessionId::new(id);
    // D-29: the user's typing holds a delivered prompt back (an escape
    // sequence alone, such as an arrow or a focus report, types nothing).
    if !data.starts_with('\x1b') {
        crate::features::messages::delivery::set_typing(state, &session, true);
    }
    write(state, &session, data.as_bytes())
}

/// D-29: a delivered message's keys, typed as `session`'s next prompt at
/// its idle prompt (`features::runtime::typed`, the only caller).
pub(crate) fn type_prompt(state: &AppState, session: &SessionId, keys: &[u8]) -> AppResult<()> {
    write(state, session, keys)
}

/// The one write to a session's terminal: the user's typing, or a
/// delivered prompt.
fn write(state: &AppState, session: &SessionId, bytes: &[u8]) -> AppResult<()> {
    state.supervisor.write_input(session, bytes)?;
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

/// The retained output tail (base64) and where it ends — replayed into
/// xterm.js when a terminal's view opens, and joined to the live chunks
/// by place (SK-23). With `from`, only what's kept after that place: a
/// view that has shown up to there fills a gap with it.
///
/// # Errors
///
/// Returns the typed [`AppError`] message when the session is unknown.
pub fn terminal_snapshot_impl(
    state: &Arc<AppState>,
    id: &str,
    from: Option<u64>,
) -> AppResult<TerminalSnapshot> {
    let (bytes, end) = state.supervisor.output_at(&SessionId::new(id))?;
    Ok(TerminalSnapshot {
        data: base64::engine::general_purpose::STANDARD.encode(kept_after(&bytes, end, from)),
        end,
    })
}

/// The kept `bytes` (ending at place `end`) after place `from`: all of
/// them without one, or when `from` is before the kept tail begins.
fn kept_after(bytes: &[u8], end: u64, from: Option<u64>) -> &[u8] {
    let Some(from) = from else { return bytes };
    let kept = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let skip = from.saturating_sub(end.saturating_sub(kept)).min(kept);
    &bytes[usize::try_from(skip).unwrap_or(bytes.len())..]
}

#[cfg(test)]
mod tests {
    use super::{TerminalChunk, chunk_after, kept_after};
    use base64::Engine as _;
    use juggler_core::model::SessionId;

    /// SK-23: a view that has shown up to a place asks for what's kept
    /// after it; one whose place is before the kept tail gets all of it.
    #[test]
    fn the_kept_output_after_a_place_is_the_bytes_past_it() {
        // "Do you want to proceed?" kept as bytes 100..123 of the output.
        let kept = b"Do you want to proceed?";
        assert_eq!(kept_after(kept, 123, None), kept);
        assert_eq!(kept_after(kept, 123, Some(112)), b"to proceed?");
        assert_eq!(kept_after(kept, 123, Some(123)), b"");
        assert_eq!(kept_after(kept, 123, Some(500)), b"");
        assert_eq!(kept_after(kept, 123, Some(40)), kept);
    }

    /// SK-23: the forwarder's chunks say where each ends, adding up from
    /// where its tap started, so a view can tell a chunk it already has
    /// (in the kept output) from one it hasn't.
    #[test]
    fn each_chunk_says_where_it_ends_from_where_the_tap_started() {
        let id = SessionId::new("s-1");
        let mut end = 120;
        let first = chunk_after(&id, &mut end, b"Do you want");
        let second = chunk_after(&id, &mut end, b" to proceed?");
        assert_eq!(first.end, 131);
        assert_eq!(second.end, 143);
        assert_eq!(end, 143);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&second.data)
            .unwrap();
        assert_eq!(decoded, b" to proceed?");
        assert_eq!(
            first,
            TerminalChunk {
                id: "s-1".into(),
                data: base64::engine::general_purpose::STANDARD.encode(b"Do you want"),
                end: 131,
            }
        );
    }
}

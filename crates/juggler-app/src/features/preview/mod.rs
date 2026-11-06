//! AJ-E415: the live preview of a session's window, per account and off by
//! default.

pub mod commands;
pub mod dto;

use std::sync::Arc;
use std::time::Duration;

use juggler_core::model::SessionId;

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::events::{AppEvent, EmitEvent as _};
use crate::features::desktop::desktop_window_identity;
use crate::features::preview::dto::{PreviewFrameInfo, PreviewStateInfo};
use crate::features::sessions::read_model::is_terminal;
use crate::state::AppState;

/// The one live preview session (Req.5: one at a time — the selected
/// Instance). Stop is a flag + a done channel (bounded join).
pub struct PreviewSession {
    /// The Instance whose window is mirrored.
    pub session_id: String,
    /// Stop flag (the loop polls it every tick).
    pub stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Completes when the loop thread ended.
    pub done: std::sync::mpsc::Receiver<()>,
}

/// Translates one driver event into the wire events (`preview://frame`
/// / `preview://state`) with the copy-exact phase texts.
fn emit_preview_event(app: &tauri::AppHandle, event: crate::preview::PreviewEvent) {
    match event {
        crate::preview::PreviewEvent::Frame {
            session_id,
            png_b64,
            at_ms,
            seq,
        } => {
            app.emit_event(
                AppEvent::PreviewFrame,
                PreviewFrameInfo {
                    session_id,
                    png_b64,
                    at_ms,
                    seq,
                },
            );
        }
        crate::preview::PreviewEvent::Phase { session_id, phase } => {
            let text = match phase {
                crate::preview::PreviewPhase::Live => String::new(),
                crate::preview::PreviewPhase::Paused => {
                    crate::preview::PAUSED_NOT_VISIBLE.to_string()
                }
                crate::preview::PreviewPhase::Denied => {
                    "Screen Recording permission needed — grant it in System Settings, then re-enable preview"
                        .to_string()
                }
                crate::preview::PreviewPhase::Unsupported => {
                    // D-17 unblocked macOS; this phase is now only the
                    // honest non-macOS answer.
                    "Live preview is not supported on this platform".to_string()
                }
            };
            app.emit_event(
                AppEvent::PreviewState,
                PreviewStateInfo {
                    session_id,
                    phase: phase.as_str().to_string(),
                    text,
                },
            );
        }
    }
}

/// Stops any running preview and waits ≤900 ms for the thread (the
/// poll cadence bounds the wait; a late exit leaks nothing — one
/// capture per poll, nothing persistent).
pub(crate) fn stop_preview_session(state: &Arc<AppState>) {
    let session = state
        .preview
        .lock()
        .map_err(|_| AppError::StatePoisoned)
        .ok()
        .and_then(|mut slot| slot.take());
    if let Some(session) = session {
        session
            .stop
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if session
            .done
            .recv_timeout(Duration::from_millis(900))
            .is_err()
        {
            tracing::warn!(
                "preview thread for {} outlived the stop budget",
                session.session_id
            );
        }
    }
}

/// [`preview_start`](crate::features::preview::commands::preview_start)'s work.
///
/// # Errors
///
/// As [`preview_start`](crate::features::preview::commands::preview_start).
pub fn preview_start_impl(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    session_id: String,
) -> AppResult<()> {
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc;

    let (record, identity) = {
        let storage = lock_storage_arc(state)?;
        let record = storage
            .sessions()
            .get(&SessionId::new(session_id.clone()))?;
        if record.kind != juggler_core::model::InstanceKind::Desktop {
            return Err(AppError::Validation {
                subject: "live preview",
                message: "preview mirrors desktop session windows".into(),
            });
        }
        if is_terminal(record.status) {
            return Ok(()); // nothing to mirror; the panel renders status only
        }
        let identity = desktop_window_identity(state, &record)?;
        (record, identity)
    };
    // The Account's explicit consent gate (Req.3) — default off.
    {
        let storage = lock_storage_arc(state)?;
        let recipe = storage.profiles().get(&record.profile)?;
        let account = storage.accounts().get(&recipe.account)?;
        if !account.preview_opt_in {
            return Err(AppError::Validation {
                subject: "live preview",
                message: format!(
                    "preview is off for {} — enable it in Account Actions",
                    account.label
                ),
            });
        }
    }
    stop_preview_session(state);

    let (done_tx, done_rx) = mpsc::channel();
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let session = session_id.clone();
    let live_id = SessionId::new(session_id.clone());
    let state_for_thread = Arc::clone(state);
    let app = app.clone();
    std::thread::spawn(move || {
        let emit = move |event: crate::preview::PreviewEvent| {
            emit_preview_event(&app, event);
        };
        // D-17: the REAL capture source on macOS — the audited init
        // shim (crates/juggler-scap-init) satisfies the binding's init
        // requirement inside the workspace's unsafe-free law. Other
        // platforms keep the honest unsupported source.
        #[cfg(target_os = "macos")]
        let source: std::sync::Arc<dyn crate::preview::CaptureSource> =
            std::sync::Arc::new(crate::preview::ScreenshotSource);
        #[cfg(not(target_os = "macos"))]
        let source: std::sync::Arc<dyn crate::preview::CaptureSource> =
            std::sync::Arc::new(crate::preview::UnsupportedSource);
        let driver = crate::preview::PreviewDriver {
            session_id: session.as_str(),
            identity,
            source: source.as_ref(),
            is_live: &|| state_for_thread.supervisor.is_supervised(&live_id),
            emit: &emit,
            stopped: stop_flag.as_ref(),
        };
        driver.run();
        let _ = done_tx.send(());
    });
    state
        .preview
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .replace(PreviewSession {
            session_id,
            stop,
            done: done_rx,
        });
    Ok(())
}

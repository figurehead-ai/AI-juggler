//! AJ-E412: running-model telemetry: the model a session's transcript says it
//! runs.

use std::sync::Arc;

use juggler_core::model::SessionId;

use crate::features::quality::transcript::line_model;
use crate::features::sessions::read_model::{emit_sessions, is_terminal};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// AJ-E412: running-model telemetry — the transcript-observed model.
// ---------------------------------------------------------------------------

/// Scans NEW transcript bytes (tracked per-Instance parse offset) for
/// the newest recorded turn model, and persists + classifies it on
/// change (`transcript` when it matches the launch overlay,
/// `in-session` when it differs — an observed `/model` change). Only
/// transcript lines are read: no Provider config files, no network
/// (Req.2/Req.7 — local-only by construction).
///
/// Returns true when any Instance's model changed (the caller emits
/// `sessions_updated`).
pub fn detect_running_models(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    only: Option<&[SessionId]>,
) -> bool {
    let changed = detect_running_models_core(state, only);
    if changed {
        let _ = emit_sessions(app, state);
    }
    changed
}

/// The storage-side core (tests call this directly — no app handle).
pub fn detect_running_models_core(state: &Arc<AppState>, only: Option<&[SessionId]>) -> bool {
    let sessions: Vec<(SessionId, Option<String>, std::path::PathBuf)> = {
        let Ok(storage) = state.storage.lock() else {
            return false;
        };
        storage
            .sessions()
            .list(None)
            .unwrap_or_default()
            .into_iter()
            .filter(|record| !is_terminal(record.status))
            .filter(|record| only.is_none_or(|ids| ids.contains(&record.id)))
            .filter_map(|record| {
                let overlay = storage
                    .profiles()
                    .get(&record.profile)
                    .ok()
                    .and_then(|recipe| recipe.settings_overlay.model);
                let path = record.transcript_ref.as_ref()?.path.clone();
                Some((record.id, overlay, path))
            })
            .collect()
    };
    let mut changed = false;
    for (session, launch_model, path) in sessions {
        if scan_one_transcript(state, &session, launch_model.as_ref(), &path) {
            changed = true;
        }
    }
    changed
}

/// The most of a transcript's end the model scan reads at once.
const MODEL_TAIL_BYTES: u64 = 8 * 1024 * 1024;

/// Tail-scans one Instance's transcript for the newest observed model;
/// true when the stored observation changed. Incremental by byte offset
/// (truncation resets to zero — re-parse, never assume).
fn scan_one_transcript(
    state: &Arc<AppState>,
    session: &SessionId,
    launch_model: Option<&String>,
    path: &std::path::Path,
) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(metadata) = std::fs::metadata(path) else {
        return false; // no transcript yet: not observed, never guessed
    };
    let len = metadata.len();
    let offset = state
        .model_offsets
        .lock()
        .ok()
        .and_then(|offsets| offsets.get(session).copied())
        .unwrap_or(0);
    if len < offset {
        // Truncated/rotated: reset and re-parse from the start.
        if let Ok(mut offsets) = state.model_offsets.lock() {
            offsets.insert(session.clone(), 0);
        }
        return scan_one_transcript(state, session, launch_model, path);
    }
    if len == offset {
        return false; // nothing new — one metadata read, no parse
    }
    // Read only the NEW bytes (plus a carry for the half-parsed line):
    // seek past the consumed prefix, never re-parse the head.
    // Only the newest model counts, so at most the file's last
    // MODEL_TAIL_BYTES: a first scan of a huge transcript never reads it
    // whole (a line cut at the start fails to parse and is skipped).
    let read_from = offset
        .saturating_sub(1)
        .max(len.saturating_sub(MODEL_TAIL_BYTES));
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    if file.seek(SeekFrom::Start(read_from)).is_err() {
        return false;
    }
    let mut bytes = Vec::with_capacity(usize::try_from(len - read_from).unwrap_or(0));
    if file
        .by_ref()
        .take(len - read_from)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return false;
    }
    if let Ok(mut offsets) = state.model_offsets.lock() {
        offsets.insert(session.clone(), len);
    }
    // The newest model recorded on a complete line wins.
    let mut observed: Option<String> = None;
    for line in String::from_utf8_lossy(&bytes).lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(model) = line_model(&value) {
            observed = Some(model);
        }
    }
    let Some(model) = observed else {
        return false;
    };
    let source = if launch_model == Some(&model) {
        juggler_core::model::ModelSource::Transcript
    } else {
        juggler_core::model::ModelSource::InSession
    };
    let next = juggler_core::model::RunningModel { model, source };
    persist_running_model(state, session, &next)
}

/// Persists an observed running model; false when unchanged (no write,
/// no emit) or when the row is gone.
fn persist_running_model(
    state: &Arc<AppState>,
    session: &SessionId,
    next: &juggler_core::model::RunningModel,
) -> bool {
    let Ok(storage) = state.storage.lock() else {
        return false;
    };
    let Ok(mut record) = storage.sessions().get(session) else {
        return false;
    };
    if record.running.as_ref() == Some(next) {
        return false; // unchanged: no write, no emit
    }
    record.running = Some(next.clone());
    let updated = storage.sessions().update(&record).is_ok();
    if updated && crate::usage_stats::recording() {
        // AJ-E1103: the model family only, never the name.
        let harness = crate::usage_stats::harness_of_recipe(&storage, &record.profile);
        crate::usage_stats::count(
            "instance.running_model",
            &[
                ("provider", crate::usage_stats::provider(&harness)),
                (
                    "model_family",
                    crate::usage_stats::model_family(&next.model),
                ),
            ],
        );
    }
    updated
}

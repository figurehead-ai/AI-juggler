//! AJ-E1530 Req.1 (ADR-0018 amendment, the maintainer's "Offer it,
//! opt-in", 2026-10-22): measuring limits for sessions started outside AI
//! Juggler, on an account used where it is.
//!
//! - Off unless the user turns it on, after a preview of the exact change.
//! - Only the tool's one key changes (Claude: `statusLine` in
//!   `settings.json`), spliced into the text so every other byte stays.
//! - The write is atomic, through a link to its target, with the file's
//!   permissions kept. A full copy from before is kept in AI Juggler's
//!   overlay folder, with the record of what changed. The wrapper and
//!   spool are there too, never in the user's folder.
//! - Turning it off puts the file back exactly when it's as AI Juggler
//!   left it; otherwise only AI Juggler's value is spliced out, and the
//!   user's edits stay.
//! - Unsafe states are refused in words; a value the user changed since is
//!   theirs, and left alone.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::jsonc;
use juggler_adapters::quota_ingest::{
    OUTSIDE_CAPTURE_BACKUP, OUTSIDE_CAPTURE_RECORD, OutsideCaptureRecord, ensure_launch_overlay,
    outside_capture_record, overlay_dir, overlay_wrapper_command,
};
use juggler_core::model::{Account, AccountId};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Where an account's capture stands.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CaptureStatusInfo {
    /// Whether it's offered at all: an account used where it is, of a
    /// tool that needs it.
    pub offered: bool,
    /// `off`, `on`, or `changed` (on, but the user's file no longer holds
    /// AI Juggler's value: theirs now, left alone).
    pub state: String,
    /// The file it changes, home-relative (`~/.claude/settings.json`).
    pub file: Option<String>,
    /// Why it isn't offered, in words.
    pub reason: Option<String>,
}

/// The preview: the exact change, before anything is written.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CapturePreviewInfo {
    /// The file, home-relative.
    pub file: String,
    /// The key's member as it is now (`"statusLine": …`), or `null` when
    /// there is none.
    pub before: Option<String>,
    /// The member as it would be.
    pub after: String,
    /// The file's fingerprint now: turning it on refuses when the file
    /// changed since this preview.
    pub fingerprint: String,
}

/// What turning it off did, in words.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CaptureOffInfo {
    /// `restored` (the file as it was), `spliced` (you'd edited it: only
    /// AI Juggler's value went), or `theirs` (your status line had changed:
    /// nothing of AI Juggler's was left in it).
    pub outcome: String,
}

fn refused(message: impl Into<String>) -> AppError {
    AppError::Refused(message.into())
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    crate::history::hex(&sha2::Sha256::digest(bytes))
}

fn home_relative(path: &Path) -> String {
    match dirs::home_dir().and_then(|home| path.strip_prefix(home).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// The account, its capture point and its settings file — or why it isn't
/// offered.
struct Target {
    account: Account,
    capture: juggler_adapter::LimitsCapture,
    adapter: juggler_adapter::SharedAdapter,
    file: PathBuf,
}

fn target(state: &Arc<AppState>, account_id: &str) -> AppResult<Result<Target, String>> {
    let account = lock_storage_arc(state)?
        .accounts()
        .get(&AccountId::new(account_id))?;
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.as_str().to_owned()))?;
    let Some(capture) = adapter.limits_capture() else {
        return Ok(Err("this tool reports its limits without it".into()));
    };
    if !account.in_place {
        return Ok(Err(
            "this account is AI Juggler's own copy: only AI Juggler runs sessions on it".into(),
        ));
    }
    let file = account.state_dir.join(capture.file);
    Ok(Ok(Target {
        account,
        capture,
        adapter,
        file,
    }))
}

/// The settings file's text, refused in words when it can't be changed
/// safely.
fn read_settings(file: &Path) -> AppResult<String> {
    let bytes = std::fs::read(file).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => refused(format!(
            "{} isn't there, so there's nothing to add it to",
            home_relative(file)
        )),
        _ => refused(format!("{} can't be read: {error}", home_relative(file))),
    })?;
    let text = String::from_utf8(bytes).map_err(|_| {
        refused(format!(
            "{} isn't text AI Juggler can read",
            home_relative(file)
        ))
    })?;
    jsonc::check(&text).map_err(|why| {
        refused(format!(
            "{} isn't valid JSON ({why}), so AI Juggler won't change it",
            home_relative(file)
        ))
    })?;
    Ok(text)
}

/// The member text `"key": value`, for the preview.
fn member(key: &str, value: &str) -> String {
    format!(
        "{}: {value}",
        serde_json::to_string(key).unwrap_or_default()
    )
}

/// AI Juggler's value for this account, as text, and the text the file
/// would have with it.
fn planned(
    state: &Arc<AppState>,
    target: &Target,
    text: &str,
) -> AppResult<(String, String, Option<String>)> {
    let overlays = state.overlays_dir();
    let original = jsonc::value_text(text, target.capture.key).map_err(refused)?;
    let original_value = original
        .as_deref()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok());
    let value = target
        .adapter
        .limits_capture_value(
            &overlay_wrapper_command(&overlays, target.account.id.as_str()),
            original_value.as_ref(),
        )
        .ok_or_else(|| refused("this tool has no value to set"))?;
    let ours = serde_json::to_string(&value).map_err(|error| refused(error.to_string()))?;
    let (new_text, before) = jsonc::set(text, target.capture.key, &ours).map_err(refused)?;
    Ok((ours, new_text, before))
}

/// Writes `bytes` over `file` atomically: through a link to its target,
/// keeping the file's permissions.
fn write_settings(file: &Path, bytes: &[u8]) -> AppResult<()> {
    let real = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    let permissions = std::fs::metadata(&real).map(|meta| meta.permissions()).ok();
    juggler_core::fs::atomic_write(&real, bytes).map_err(|error| AppError::Io {
        context: "write settings",
        message: error.to_string(),
    })?;
    if let Some(permissions) = permissions {
        let _ = std::fs::set_permissions(&real, permissions);
    }
    Ok(())
}

/// Where the account's capture stands.
///
/// # Errors
///
/// Storage errors; an unknown account.
pub fn status_impl(state: &Arc<AppState>, account_id: &str) -> AppResult<CaptureStatusInfo> {
    let target = match target(state, account_id)? {
        Ok(target) => target,
        Err(reason) => {
            return Ok(CaptureStatusInfo {
                offered: false,
                state: "off".into(),
                file: None,
                reason: Some(reason),
            });
        }
    };
    let record = outside_capture_record(&state.overlays_dir(), account_id);
    let state_word = match &record {
        None => "off",
        Some(record) => {
            let ours_still = std::fs::read_to_string(&target.file)
                .ok()
                .and_then(|text| jsonc::value_text(&text, target.capture.key).ok().flatten())
                .is_some_and(|value| value == record.ours);
            if ours_still { "on" } else { "changed" }
        }
    };
    Ok(CaptureStatusInfo {
        offered: true,
        state: state_word.into(),
        file: Some(home_relative(&target.file)),
        reason: None,
    })
}

/// The exact change, before anything is written.
///
/// # Errors
///
/// [`AppError::Refused`] in words: not offered, the file missing or not
/// valid JSON, or already on.
pub fn preview_impl(state: &Arc<AppState>, account_id: &str) -> AppResult<CapturePreviewInfo> {
    let target = target(state, account_id)?.map_err(refused)?;
    if outside_capture_record(&state.overlays_dir(), account_id).is_some() {
        return Err(refused("it's already on for this account"));
    }
    let text = read_settings(&target.file)?;
    let (ours, _, before) = planned(state, &target, &text)?;
    Ok(CapturePreviewInfo {
        file: home_relative(&target.file),
        before: before.map(|value| member(target.capture.key, &value)),
        after: member(target.capture.key, &ours),
        fingerprint: sha256(text.as_bytes()),
    })
}

/// Turns it on: the previewed change, if the file is still as previewed.
///
/// # Errors
///
/// [`AppError::Refused`] in words when the file changed since the
/// preview (or any reason the preview gives); write errors.
pub fn turn_on_impl(state: &Arc<AppState>, account_id: &str, fingerprint: &str) -> AppResult<()> {
    let target = target(state, account_id)?.map_err(refused)?;
    let overlays = state.overlays_dir();
    if outside_capture_record(&overlays, account_id).is_some() {
        return Err(refused("it's already on for this account"));
    }
    let text = read_settings(&target.file)?;
    if sha256(text.as_bytes()) != fingerprint {
        return Err(refused(format!(
            "{} changed since the preview: look at the change again first",
            home_relative(&target.file)
        )));
    }
    let (ours, new_text, before) = planned(state, &target, &text)?;
    // The wrapper first (it chains the user's own line, read now), then
    // the backup and the record, all in AI Juggler's folder; the user's
    // file last, in one atomic write.
    ensure_launch_overlay(&overlays, account_id, &target.account.state_dir).map_err(|error| {
        AppError::Io {
            context: "write the capture",
            message: error,
        }
    })?;
    let dir = overlay_dir(&overlays, account_id);
    let io = |error: std::io::Error| AppError::Io {
        context: "keep a copy of the settings",
        message: error.to_string(),
    };
    juggler_core::fs::atomic_write(&dir.join(OUTSIDE_CAPTURE_BACKUP), text.as_bytes())
        .map_err(io)?;
    let record = OutsideCaptureRecord {
        file: target.capture.file.to_owned(),
        key: target.capture.key.to_owned(),
        original: before,
        ours,
        written_sha256: sha256(new_text.as_bytes()),
        at_ms: crate::commands::now_millis(),
    };
    let record_bytes =
        serde_json::to_vec_pretty(&record).map_err(|error| refused(error.to_string()))?;
    juggler_core::fs::atomic_write(&dir.join(OUTSIDE_CAPTURE_RECORD), &record_bytes).map_err(io)?;
    if let Err(error) = write_settings(&target.file, new_text.as_bytes()) {
        let _ = std::fs::remove_file(dir.join(OUTSIDE_CAPTURE_RECORD));
        return Err(error);
    }
    // The launch overlay now chains from the record (the user's line as
    // it was), not from the file that holds AI Juggler's value.
    let _ = ensure_launch_overlay(&overlays, account_id, &target.account.state_dir);
    Ok(())
}

/// Turns it off: the file back exactly when it's as AI Juggler left it,
/// else only AI Juggler's value taken out; the record goes either way.
///
/// # Errors
///
/// [`AppError::Refused`] when it isn't on, or the file can't be read
/// safely; write errors.
pub fn turn_off_impl(state: &Arc<AppState>, account_id: &str) -> AppResult<CaptureOffInfo> {
    let target = target(state, account_id)?.map_err(refused)?;
    let overlays = state.overlays_dir();
    let dir = overlay_dir(&overlays, account_id);
    let record = outside_capture_record(&overlays, account_id)
        .ok_or_else(|| refused("it isn't on for this account"))?;
    let outcome = match std::fs::read(&target.file) {
        // Gone since: nothing of AI Juggler's is left to take out.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "theirs",
        Err(error) => {
            return Err(refused(format!(
                "{} can't be read: {error}",
                home_relative(&target.file)
            )));
        }
        Ok(bytes) if sha256(&bytes) == record.written_sha256 => {
            let backup =
                std::fs::read(dir.join(OUTSIDE_CAPTURE_BACKUP)).map_err(|error| AppError::Io {
                    context: "read the copy of the settings",
                    message: error.to_string(),
                })?;
            write_settings(&target.file, &backup)?;
            "restored"
        }
        Ok(bytes) => {
            let text = String::from_utf8(bytes).map_err(|_| {
                refused(format!(
                    "{} isn't text AI Juggler can read",
                    home_relative(&target.file)
                ))
            })?;
            match jsonc::unset(&text, &record.key, &record.ours, record.original.as_deref())
                .map_err(|why| {
                    refused(format!(
                        "{} isn't valid JSON now ({why}), so AI Juggler won't change it",
                        home_relative(&target.file)
                    ))
                })? {
                Some(new_text) => {
                    write_settings(&target.file, new_text.as_bytes())?;
                    "spliced"
                }
                None => "theirs",
            }
        }
    };
    let _ = std::fs::remove_file(dir.join(OUTSIDE_CAPTURE_RECORD));
    let _ = std::fs::remove_file(dir.join(OUTSIDE_CAPTURE_BACKUP));
    let _ = ensure_launch_overlay(&overlays, account_id, &target.account.state_dir);
    Ok(CaptureOffInfo {
        outcome: outcome.into(),
    })
}

/// Where the account's capture stands (read-only).
///
/// # Errors
///
/// As [`status_impl`].
#[tauri::command]
pub async fn quota_capture_status(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<CaptureStatusInfo> {
    crate::ipc::blocking(state.inner(), "read the capture", move |state| {
        status_impl(state, &account_id)
    })
    .await
}

/// The exact change, before anything is written (read-only).
///
/// # Errors
///
/// As [`preview_impl`].
#[tauri::command]
pub async fn quota_capture_preview(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<CapturePreviewInfo> {
    crate::ipc::blocking(state.inner(), "preview the capture", move |state| {
        preview_impl(state, &account_id)
    })
    .await
}

/// Turns it on, as previewed.
///
/// # Errors
///
/// As [`turn_on_impl`].
#[tauri::command]
pub async fn quota_capture_turn_on(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
    fingerprint: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "turn on the capture", move |state| {
        turn_on_impl(state, &account_id, &fingerprint)
    })
    .await
}

/// Turns it off.
///
/// # Errors
///
/// As [`turn_off_impl`].
#[tauri::command]
pub async fn quota_capture_turn_off(
    state: tauri::State<'_, Arc<AppState>>,
    account_id: String,
) -> AppResult<CaptureOffInfo> {
    crate::ipc::blocking(state.inner(), "turn off the capture", move |state| {
        turn_off_impl(state, &account_id)
    })
    .await
}

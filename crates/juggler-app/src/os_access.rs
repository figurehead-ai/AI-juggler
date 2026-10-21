//! D-25: the OS access this platform needs, one entry per thing it makes
//! possible — for the first-run step and for asking in context later.
//!
//! - **Reading never prompts.** [`os_access_list_impl`] only checks, so the
//!   UI can poll it while the step is open and flip an entry the moment the
//!   user says yes in the system dialog.
//! - **Allow does the asking.** [`os_access_request_impl`] raises the
//!   system's own prompts (on macOS: the Accessibility prompt that adds AI
//!   Juggler to the list, the Automation prompt for System Events, the
//!   Screen Recording prompt). When the user said no before, the system
//!   won't ask again, so the entry offers its settings pane instead.
//! - **Per platform.** macOS lists window focus (Accessibility + Automation:
//!   focus drives System Events) and the live preview (Screen Recording).
//!   Local Network is asked for by Fleet when it's turned on, not here.
//!   Other platforms list nothing yet, and the step is skipped.
//! - **Screen Recording needs a reopen.** macOS answers the Screen
//!   Recording check once per process: allowed in System Settings, it
//!   still reads "not allowed" until AI Juggler reopens (the 0.0.12
//!   report). Once asked and still not allowed, the entry offers
//!   [`os_access_reopen`], which relaunches the app (asking first when
//!   sessions are live). It never says "allowed" before the OS does.

// Tauri command signatures take State/values by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// One thing the OS must allow.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OsAccessInfo {
    /// Stable id: `focus` | `preview`.
    pub id: String,
    /// What it makes possible, as the user would say it.
    pub title: String,
    /// One line of why, and what it never does.
    pub why: String,
    /// The name the OS gives it (shown so the system dialog makes sense).
    pub system_name: String,
    /// `allowed` | `not_allowed` (asked, said no or not yet on) |
    /// `not_asked`.
    pub status: String,
    /// The next action: `prompt` (Allow raises the system dialog) |
    /// `settings` (only the settings pane can change it now) | `reopen`
    /// (allowed in the settings pane, macOS shows it only after AI
    /// Juggler reopens; the pane stays offered too) | `none`.
    pub action: String,
    /// A note for after the grant (e.g. "reopen AI Juggler"), when any.
    pub note: Option<String>,
}

const ASKED_PREFIX: &str = "os_access_asked:";
// Only macOS has entries that read these back.
#[cfg(target_os = "macos")]
const AUTOMATION_KEY: &str = "os_access_automation";

#[cfg(target_os = "macos")]
fn asked(state: &Arc<AppState>, id: &str) -> bool {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| {
            storage
                .app_settings()
                .get(&format!("{ASKED_PREFIX}{id}"))
                .ok()
                .flatten()
        })
        .is_some_and(|raw| raw == "1")
}

fn set_setting(state: &Arc<AppState>, key: &str, value: &str) -> AppResult<()> {
    crate::commands::lock_storage_arc(state)?
        .app_settings()
        .set(key, value)?;
    Ok(())
}

/// The status and next action of an entry, from what the OS reports
/// (`granted`) and whether AI Juggler has asked before.
#[must_use]
pub fn status_of(granted: bool, asked_before: bool) -> (&'static str, &'static str) {
    match (granted, asked_before) {
        (true, _) => ("allowed", "none"),
        (false, false) => ("not_asked", "prompt"),
        (false, true) => ("not_allowed", "settings"),
    }
}

/// The live preview's entry (Screen Recording), from what the OS reports
/// and whether AI Juggler asked. macOS's only prompt for it sends the user
/// to System Settings, and the answer it gives this process doesn't
/// change until AI Juggler reopens: so once asked, a "no" may be a stale
/// "yes", and the entry offers the reopen — in words that don't claim it.
#[must_use]
pub fn preview_state(
    granted: bool,
    asked_before: bool,
) -> (&'static str, &'static str, Option<&'static str>) {
    match status_of(granted, asked_before) {
        ("allowed", action) => ("allowed", action, None),
        ("not_allowed", _) => ("not_allowed", "reopen", Some(REOPEN_NOTE)),
        (status, action) => (
            status,
            action,
            Some("macOS may ask you to reopen AI Juggler after you allow it."),
        ),
    }
}

/// The preview entry's words once the user may have allowed it.
pub const REOPEN_NOTE: &str =
    "Allowed it in System Settings? macOS needs AI Juggler to reopen to see it.";

/// Why reopening AI Juggler has to be confirmed or wait, when it does.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ReopenBlockInfo {
    /// What reopening now would cut short, in words.
    pub reason: String,
    /// True when the user may reopen anyway (live sessions); false when
    /// it has to wait (a switch or a bring-in under way).
    pub can_reopen_anyway: bool,
}

/// What a Reopen click does: relaunch now, or say why not first.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReopenDecision {
    /// Nothing would be cut short, or the user said reopen anyway.
    Relaunch,
    /// Ask (or wait) first.
    Hold(ReopenBlockInfo),
}

/// The decision for what runs now (`busy`) and whether the user already
/// said "Reopen anyway". Live sessions can be overridden — a terminal
/// session ends when AI Juggler quits, so the user hears that first; a
/// switch or a bring-in can't be cut short.
#[must_use]
pub fn reopen_decision(busy: Option<crate::updates::Busy>, anyway: bool) -> ReopenDecision {
    use crate::updates::Busy;
    match busy {
        None => ReopenDecision::Relaunch,
        Some(Busy::Sessions(_)) if anyway => ReopenDecision::Relaunch,
        Some(Busy::Sessions(live)) => ReopenDecision::Hold(ReopenBlockInfo {
            reason: format!(
                "{live} {} running. Reopening AI Juggler ends the ones in its terminal; sessions in their own app keep running.",
                if live == 1 {
                    "session is"
                } else {
                    "sessions are"
                }
            ),
            can_reopen_anyway: true,
        }),
        Some(other) => ReopenDecision::Hold(ReopenBlockInfo {
            reason: other.words("Reopen AI Juggler"),
            can_reopen_anyway: false,
        }),
    }
}

/// Reads an osascript probe of System Events: `Some(true)` = allowed (the
/// script ran), `Some(false)` = refused (error -1743), `None` = unknown.
#[must_use]
pub fn automation_from_probe(ok: bool, stderr: &str) -> Option<bool> {
    if ok {
        Some(true)
    } else if stderr.contains("-1743") || stderr.to_lowercase().contains("not authorized") {
        Some(false)
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{AppState, Arc, OsAccessInfo};

    pub(super) fn accessibility_trusted() -> bool {
        macos_accessibility_client::accessibility::application_is_trusted()
    }

    pub(super) fn accessibility_prompt() -> bool {
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
    }

    pub(super) fn screen_recording_allowed() -> bool {
        core_graphics::access::ScreenCaptureAccess.preflight()
    }

    pub(super) fn screen_recording_request() -> bool {
        core_graphics::access::ScreenCaptureAccess.request()
    }

    /// Asks System Events for nothing but whether UI scripting is on:
    /// the first run raises the Automation prompt (and blocks until it's
    /// answered); later runs only report.
    pub(super) fn automation_probe() -> Option<bool> {
        let out = std::process::Command::new("/usr/bin/osascript")
            .args([
                "-e",
                "tell application \"System Events\" to return UI elements enabled",
            ])
            .output()
            .ok()?;
        super::automation_from_probe(out.status.success(), &String::from_utf8_lossy(&out.stderr))
    }

    pub(super) fn settings_url(id: &str) -> &'static str {
        match id {
            "preview" => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
            }
            _ => "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
        }
    }

    pub(super) fn entries(state: &Arc<AppState>) -> Vec<OsAccessInfo> {
        let automation_ok = super::stored_automation(state).unwrap_or(false);
        let focus_granted = accessibility_trusted() && automation_ok;
        let (status, action) = super::status_of(focus_granted, super::asked(state, "focus"));
        let (preview_status, preview_action, preview_note) =
            super::preview_state(screen_recording_allowed(), super::asked(state, "preview"));
        vec![
            OsAccessInfo {
                id: "focus".to_owned(),
                title: "Jump to a session's own window".to_owned(),
                why: "Brings the right terminal window to the front when you open a session."
                    .to_owned(),
                system_name: "Accessibility and Automation".to_owned(),
                status: status.to_owned(),
                action: action.to_owned(),
                note: None,
            },
            OsAccessInfo {
                id: "preview".to_owned(),
                title: "See a live preview of each session".to_owned(),
                why: "Shows a session's window inside AI Juggler, on this Mac only; nothing is recorded or sent."
                    .to_owned(),
                system_name: "Screen Recording".to_owned(),
                status: preview_status.to_owned(),
                action: preview_action.to_owned(),
                note: preview_note.map(str::to_owned),
            },
        ]
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{AppState, Arc, OsAccessInfo};

    pub(super) fn entries(_state: &Arc<AppState>) -> Vec<OsAccessInfo> {
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
fn stored_automation(state: &Arc<AppState>) -> Option<bool> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(AUTOMATION_KEY).ok().flatten())
        .map(|raw| raw == "1")
}

/// Core: what this OS needs, as it stands. Never prompts.
#[must_use]
pub fn os_access_list_impl(state: &Arc<AppState>) -> Vec<OsAccessInfo> {
    platform::entries(state)
}

/// Core: Allow — raises the system's prompts for one entry, records that
/// AI Juggler asked, and returns the list as it now stands.
///
/// # Errors
///
/// [`AppError::Validation`] for an id this platform doesn't list.
pub fn os_access_request_impl(state: &Arc<AppState>, id: &str) -> AppResult<Vec<OsAccessInfo>> {
    if !os_access_list_impl(state)
        .iter()
        .any(|entry| entry.id == id)
    {
        return Err(AppError::Validation {
            subject: "OS access",
            message: format!("{id:?} isn't needed on this computer"),
        });
    }
    set_setting(state, &format!("{ASKED_PREFIX}{id}"), "1")?;
    #[cfg(target_os = "macos")]
    match id {
        "focus" => {
            platform::accessibility_prompt();
            if let Some(allowed) = platform::automation_probe() {
                set_setting(state, AUTOMATION_KEY, if allowed { "1" } else { "0" })?;
            }
        }
        "preview" => {
            platform::screen_recording_request();
        }
        _ => {}
    }
    Ok(os_access_list_impl(state))
}

/// What this OS needs (the first-run step polls this).
#[tauri::command]
#[must_use]
pub fn os_access_list(state: tauri::State<'_, Arc<AppState>>) -> Vec<OsAccessInfo> {
    os_access_list_impl(state.inner())
}

/// Allow: raises the system prompts for one entry. Runs off the UI
/// thread: the Automation prompt blocks until it's answered.
///
/// # Errors
///
/// See [`os_access_request_impl`].
#[tauri::command]
pub async fn os_access_request(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<Vec<OsAccessInfo>> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || os_access_request_impl(&state, &id))
        .await
        .map_err(|error| AppError::Io {
            context: "ask for OS access",
            message: error.to_string(),
        })?
}

/// Reopens AI Juggler so macOS's new Screen Recording answer shows: at
/// once when nothing would be cut short or `anyway` is set after a live
/// session warning; otherwise returns why, without relaunching.
///
/// # Errors
///
/// Returns the typed storage error reading what runs now.
#[tauri::command]
pub fn os_access_reopen(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    anyway: bool,
) -> AppResult<Option<ReopenBlockInfo>> {
    let now = juggler_core::model::Timestamp::now().millis();
    match reopen_decision(crate::updates::busy_at(state.inner(), now)?, anyway) {
        ReopenDecision::Hold(block) => Ok(Some(block)),
        ReopenDecision::Relaunch => app.restart(),
    }
}

/// Opens the settings pane for one entry (the fixed link for that id —
/// never a caller-supplied URL).
///
/// # Errors
///
/// Returns the typed [`AppError`] when the pane cannot open, or for an id
/// this platform doesn't list.
#[tauri::command]
pub fn os_access_open_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> AppResult<()> {
    if !os_access_list_impl(state.inner())
        .iter()
        .any(|entry| entry.id == id)
    {
        return Err(AppError::Validation {
            subject: "OS access",
            message: format!("{id:?} isn't needed on this computer"),
        });
    }
    #[cfg(target_os = "macos")]
    {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url(platform::settings_url(&id), None::<&str>)
            .map_err(|e| AppError::Io {
                context: "open the system settings",
                message: e.to_string(),
            })?;
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_says_what_to_do_next() {
        assert_eq!(status_of(true, false), ("allowed", "none"));
        assert_eq!(status_of(true, true), ("allowed", "none"));
        assert_eq!(status_of(false, false), ("not_asked", "prompt"));
        assert_eq!(
            status_of(false, true),
            ("not_allowed", "settings"),
            "the system won't ask twice, so the settings pane is the way"
        );
    }

    /// The 0.0.12 report: allowed in System Settings, the preview card
    /// never turned to Allowed (macOS answers once per process).
    #[test]
    fn the_preview_offers_a_reopen_once_asked_and_says_allowed_only_when_the_os_does() {
        // Not asked yet: Allow raises the system prompt.
        let (status, action, note) = preview_state(false, false);
        assert_eq!((status, action), ("not_asked", "prompt"));
        assert!(note.is_some_and(|n| n.contains("reopen")));
        // Asked, back from System Settings, the OS still says no: offer
        // the reopen, and don't claim it's allowed.
        assert_eq!(
            preview_state(false, true),
            ("not_allowed", "reopen", Some(REOPEN_NOTE))
        );
        assert!(REOPEN_NOTE.starts_with("Allowed it in System Settings?"));
        // After the reopen, the OS says yes: Allowed, nothing to do.
        assert_eq!(preview_state(true, true), ("allowed", "none", None));
        assert_eq!(preview_state(true, false), ("allowed", "none", None));
    }

    /// The block a held decision carries (`None` for a relaunch).
    fn held(decision: ReopenDecision) -> Option<ReopenBlockInfo> {
        match decision {
            ReopenDecision::Hold(block) => Some(block),
            ReopenDecision::Relaunch => None,
        }
    }

    #[test]
    fn reopening_asks_first_when_sessions_are_live_and_waits_for_a_switch() {
        use crate::updates::Busy;
        assert_eq!(reopen_decision(None, false), ReopenDecision::Relaunch);
        let two = held(reopen_decision(Some(Busy::Sessions(2)), false))
            .expect("live sessions are said first");
        assert!(two.can_reopen_anyway);
        assert!(
            two.reason.starts_with("2 sessions are running."),
            "{}",
            two.reason
        );
        let one = held(reopen_decision(Some(Busy::Sessions(1)), false))
            .expect("one live session is said first");
        assert!(
            one.reason.starts_with("1 session is running."),
            "{}",
            one.reason
        );
        assert_eq!(
            reopen_decision(Some(Busy::Sessions(2)), true),
            ReopenDecision::Relaunch,
            "Reopen anyway, after the warning"
        );
        for busy in [Busy::Switch, Busy::BringIn] {
            let block = held(reopen_decision(Some(busy), true))
                .expect("a switch or a bring-in can't be cut short, even anyway");
            assert!(!block.can_reopen_anyway);
            assert!(
                block.reason.contains("Reopen AI Juggler"),
                "{}",
                block.reason
            );
        }
    }

    #[test]
    fn the_automation_probe_tells_allowed_refused_and_unknown_apart() {
        assert_eq!(automation_from_probe(true, ""), Some(true));
        assert_eq!(
            automation_from_probe(
                false,
                "execution error: Not authorized to send Apple events to System Events. (-1743)"
            ),
            Some(false)
        );
        assert_eq!(automation_from_probe(false, "some other failure"), None);
    }
}

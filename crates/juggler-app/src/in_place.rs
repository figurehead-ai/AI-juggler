//! ADR-0018: the isolation checklist's rows for an account used where it
//! is (moved from `commands.rs`, which only shrinks), with AJ-E1530's one
//! opt-in key.

use std::sync::Arc;

use juggler_core::model::Account;

use crate::commands::{
    IsolationRowInfo, launch_env_pinned, login_state_name, login_state_of, provider_display,
};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// ADR-0018: the checklist for an account that uses the user's folder
/// where it is. "Owner-only" and "marker present" are promises about AI
/// Juggler's own copies; here the promises are that AI Juggler put
/// nothing of its own in the folder and launches point only at it.
pub(crate) fn in_place_isolation_rows(
    state: &Arc<AppState>,
    account: &Account,
) -> AppResult<Vec<IsolationRowInfo>> {
    let dir = account.state_dir.clone();
    let mut rows = vec![IsolationRowInfo {
        id: "in_place_folder_present",
        label: "Uses your folder where it is".into(),
        pass: dir.is_dir(),
        detail: format!("{}", dir.display()),
    }];
    let ours = [
        juggler_adapters::claude_code::MANAGED_MARKER,
        "juggler-statusline.sh",
        "juggler-statusline-backup.json",
        "quota-spool",
    ];
    let untouched = ours.iter().all(|name| !dir.join(name).exists());
    rows.push(IsolationRowInfo {
        id: "in_place_no_juggler_files",
        label: "AI Juggler put nothing of its own in your folder".into(),
        pass: untouched,
        detail: "its launch settings and usage capture live in AI Juggler's own folder".into(),
    });
    let adapter = state
        .registry
        .get(&account.harness)
        .ok_or_else(|| AppError::UnknownHarness(account.harness.as_str().to_owned()))?;
    if let Some(row) = capture_row(state, account, adapter.as_ref()) {
        rows.push(row);
    }
    // Every pinned variable is the folder exactly, or — for the vendor's
    // own default folder — unset, as the vendor runs it.
    let pinned = adapter.pinned_env_keys();
    let default_folder = adapter
        .pinned_env(&dir)
        .iter()
        .all(|(_, value)| value.is_none());
    let env_ok = launch_env_pinned(adapter.as_ref(), account);
    let keys = pinned.join(" and ");
    rows.push(IsolationRowInfo {
        id: "launch_env_managed_only",
        label: "Launch environment points only at this account's folder".into(),
        pass: env_ok,
        detail: if default_folder {
            format!(
                "the default folder runs as {} runs it ({keys} unset)",
                provider_display(&state.registry, account.harness.as_str()).0
            )
        } else {
            format!(
                "{keys} {} this folder, exactly",
                if pinned.len() > 1 { "are" } else { "is" }
            )
        },
    });
    let probe_state = login_state_of(&state.registry, account);
    rows.push(IsolationRowInfo {
        id: "probe_independence",
        label: "Login state reads only this account's folder".into(),
        pass: !matches!(probe_state, juggler_adapters::LoginState::Unknown),
        detail: format!(
            "probe classified this dir as {}",
            login_state_name(probe_state)
        ),
    });
    Ok(rows)
}

/// AJ-E1530 Req.1: the one key the user can let AI Juggler set. While
/// it's on, that key is allowed and nothing else is; while it's off, the
/// settings must not call AI Juggler at all.
fn capture_row(
    state: &Arc<AppState>,
    account: &Account,
    adapter: &dyn juggler_adapter::HarnessAdapter,
) -> Option<IsolationRowInfo> {
    let capture = adapter.limits_capture()?;
    let file = account.state_dir.join(capture.file);
    let value = std::fs::read_to_string(&file).ok().and_then(|text| {
        juggler_adapter::jsonc::value_text(&text, capture.key)
            .ok()
            .flatten()
    });
    let record = juggler_adapters::quota_ingest::outside_capture_record(
        &state.overlays_dir(),
        account.id.as_str(),
    );
    let overlay =
        juggler_adapters::quota_ingest::overlay_dir(&state.overlays_dir(), account.id.as_str());
    let calls_us = value
        .as_deref()
        .is_some_and(|value| value.contains(&overlay.to_string_lossy().replace('\\', "\\\\")));
    Some(match record {
        Some(record) => IsolationRowInfo {
            id: "in_place_capture_only",
            label: format!(
                "Its only change to your folder is the {} you turned on",
                capture.key
            ),
            pass: value.as_deref() == Some(record.ours.as_str()) || !calls_us,
            detail: if value.as_deref() == Some(record.ours.as_str()) {
                format!(
                    "{} in {} measures limits for sessions started elsewhere",
                    capture.key, capture.file
                )
            } else {
                format!(
                    "you've changed {} since: AI Juggler leaves it as you set it",
                    capture.key
                )
            },
        },
        None => IsolationRowInfo {
            id: "in_place_settings_untouched",
            label: format!("Your {} doesn't call AI Juggler", capture.file),
            pass: !calls_us,
            detail: format!("{} is yours, as you set it", capture.key),
        },
    })
}

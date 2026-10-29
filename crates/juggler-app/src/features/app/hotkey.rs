//! The global "Go to instance…" hotkey (AJ-E414 Req.4): the binding the
//! user chose in Settings › Desktop apps, registered with the OS at start
//! and whenever it changes. There is no default (AJ-E2110).

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Rebinds the global "Go to instance…" hotkey (AJ-E414 Req.4). The
/// new binding is registered with the OS FIRST; only a successful
/// registration persists it — and a failed registration restores the
/// previous one, so the user is never left without the key they had.
/// An empty binding turns the hotkey off (AJ-E2110: there is no
/// default; the user picks one in Settings › Desktop apps).
#[tauri::command]
///
/// # Errors
///
/// Returns the typed [`AppError`] message for an OS registration
/// failure (conflict, unparseable accelerator) or a storage failure.
pub async fn app_go_to_hotkey_set(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    hotkey: String,
) -> AppResult<()> {
    crate::ipc::blocking(state.inner(), "go to hotkey set", move |state| {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        let hotkey = hotkey.trim().to_string();
        let previous = {
            let storage = crate::commands::lock_storage_arc(state)?;
            storage
                .app_settings()
                .get(crate::GO_TO_HOTKEY_KEY)
                .map_err(AppError::from)?
        };
        let shortcuts = app.global_shortcut();
        if let Some(previous) = &previous {
            let _ = shortcuts.unregister(previous.as_str());
        }
        // Empty: the key is off (and stays unset), nothing registered.
        let registered = if hotkey.is_empty() {
            Ok(())
        } else {
            shortcuts.register(hotkey.as_str())
        };
        match registered {
            Ok(()) => crate::commands::lock_storage_arc(state)?
                .app_settings()
                .set(crate::GO_TO_HOTKEY_KEY, &hotkey)
                .map_err(AppError::from),
            Err(error) => {
                // Best-effort restore of the previous binding.
                if let Some(previous) = &previous {
                    let _ = shortcuts.register(previous.as_str());
                }
                Err(AppError::Io {
                    context: "register hotkey",
                    message: format!("{hotkey}: {error}"),
                })
            }
        }
    })
    .await
}

/// Registers the "Go to instance…" hotkey the user chose (AJ-E414
/// Req.4). There is no default (AJ-E2110): `⌥Space` belongs to `ChatGPT`,
/// so nothing is registered until the user picks a key in Settings ›
/// Desktop apps. A conflict or a locked-down system key logs and
/// continues — the palette stays reachable in-app (⌘K).
pub(crate) fn register_go_to_hotkey(app: &tauri::AppHandle) {
    use tauri::Manager;
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let Some(hotkey) = app
        .state::<Arc<AppState>>()
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(crate::GO_TO_HOTKEY_KEY).ok())
        .flatten()
    else {
        return;
    };
    if let Err(error) = app.global_shortcut().register(hotkey.as_str()) {
        eprintln!("ai-juggler: hotkey {hotkey} not registered: {error}");
    }
}

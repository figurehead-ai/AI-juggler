//! D-127 for system prompts: whether this setup can use each entry, so
//! setup primes only those (PAT-OSP-2) and never one it can't use.

use std::sync::Arc;

use crate::state::AppState;

/// On a system with these prompts (macOS today).
#[must_use]
pub fn on_this_system(_state: &Arc<AppState>) -> bool {
    cfg!(target_os = "macos")
}

/// Every setup on a system with the prompt.
#[must_use]
pub fn always(state: &Arc<AppState>) -> bool {
    on_this_system(state)
}

/// None: an entry built never to be shown.
#[must_use]
pub const fn never(_state: &Arc<AppState>) -> bool {
    false
}

/// Selecting a terminal's tab is on (Settings › Desktop apps).
#[must_use]
pub fn terminal_tabs(state: &Arc<AppState>) -> bool {
    on_this_system(state) && crate::os_access::terminal_tabs_allowed(state)
}

/// Tabs are on and iTerm is installed.
#[must_use]
pub fn iterm_tabs(state: &Arc<AppState>) -> bool {
    terminal_tabs(state)
        && juggler_platform::apps::install_folders(dirs::home_dir().as_deref())
            .iter()
            .any(|folder| juggler_platform::apps::installed_app(folder, "iTerm").is_some())
}

/// A session lives in a desktop app (a link can open it there).
#[must_use]
pub fn desktop_apps(state: &Arc<AppState>) -> bool {
    on_this_system(state) && !crate::features::places::desktop_app_by_session(state).is_empty()
}

/// Opening at login is possible for this build (an installed, signed app).
#[must_use]
pub fn login_item(_state: &Arc<AppState>) -> bool {
    juggler_platform::os_prompt::login_item_status() != juggler_login_item::LoginItem::Unavailable
}

/// A registered tool can show a live preview.
#[must_use]
pub fn preview(state: &Arc<AppState>) -> bool {
    on_this_system(state) && crate::os_access::preview_supported(state)
}

/// Link (finding other computers) is turned on.
#[must_use]
pub fn link(state: &Arc<AppState>) -> bool {
    on_this_system(state) && crate::link::link_settings_enabled(state)
}

/// The folders of this setup's projects (the folders sessions run in).
fn project_roots(state: &Arc<AppState>) -> Vec<std::path::PathBuf> {
    state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.projects().list().ok())
        .unwrap_or_default()
        .into_iter()
        .map(|project| project.root)
        .collect()
}

fn in_folder(state: &Arc<AppState>, folder: juggler_platform::os_prompt::Folder) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    project_roots(state)
        .iter()
        .any(|root| juggler_platform::os_prompt::protected_folder(root, &home) == Some(folder))
}

/// A project lives in Documents.
#[must_use]
pub fn documents(state: &Arc<AppState>) -> bool {
    in_folder(state, juggler_platform::os_prompt::Folder::Documents)
}

/// A project lives on the Desktop.
#[must_use]
pub fn desktop(state: &Arc<AppState>) -> bool {
    in_folder(state, juggler_platform::os_prompt::Folder::Desktop)
}

/// A project lives in Downloads.
#[must_use]
pub fn downloads(state: &Arc<AppState>) -> bool {
    in_folder(state, juggler_platform::os_prompt::Folder::Downloads)
}

/// A project lives in another app's guarded data.
#[must_use]
pub fn app_data(state: &Arc<AppState>) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    project_roots(state)
        .iter()
        .any(|root| juggler_platform::os_prompt::other_apps_data(root, &home))
}

/// The system's own AI may be here, or a local model is installed: Bot's
/// helper can run.
#[must_use]
pub fn helper(_state: &Arc<AppState>) -> bool {
    juggler_platform::ai::system_model() == juggler_platform::ai::SystemModel::MayHave
        || !crate::features::ai::models::store::shared_installed_ids().is_empty()
}

/// Bot's helper can run here and a build with the helpers' old
/// identifiers ran on this computer (`super::helper_renamed`): macOS may
/// ask about the helper once after this update.
#[must_use]
pub fn helper_changed(state: &Arc<AppState>) -> bool {
    super::helper_renamed() && helper(state)
}

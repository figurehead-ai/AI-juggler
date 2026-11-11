//! Linking a desktop App to the tool folder its sessions use: by the
//! profile's own name (the profile launcher's convention), or (ADR-0032)
//! by the folder its host app declares for an app that runs one instance
//! (the Codex app's `~/.codex`). Moved out of `places` for its size.

use std::sync::Arc;

use super::with_book;
use crate::state::AppState;

/// The folder discovery found under a profile's own `name` (the profile
/// launcher's convention).
pub(super) fn folder_for_profile<'a>(
    name: &str,
    found: &'a [juggler_adapter::environment::DiscoveredInstall],
) -> Option<&'a juggler_adapter::environment::DiscoveredInstall> {
    found
        .iter()
        .find(|one| one.path.file_name().and_then(|n| n.to_str()) == Some(name))
}

/// The go sitting (2026-11-07, finding 1): a profile added after first
/// run listed none of its sessions, because the folder its tool writes
/// them to (the profile launcher's `ClaudeCodeProfiles/<name>` beside the
/// app's `ClaudeProfiles/<name>`) was never brought in. With the same
/// yes, the folder discovery finds under the profile's own name is
/// brought in where it is, as first run does (in place, never copied),
/// unless an Account holds it already; either way it's the profile's
/// folder from then on. That link is what tells the folder's tool sign-in
/// apart (the profile signs in inside its app, never in a terminal: the
/// rerun's "needs its sign-in" came from a folder brought in unlinked) and
/// gives the App page its Account. Only one an Account holds already
/// unless `may_bring_in` (Add's yes). Nothing when none matches; a
/// failure never fails the add. Returns whether the link is new.
pub(super) fn link_its_folder(state: &Arc<AppState>, id: &str, may_bring_in: bool) -> bool {
    // Its profile's own name, or (an app that runs one instance) none:
    // then the folder its host app declares (ADR-0032: the Codex app's
    // `~/.codex`).
    let Some((app_id, name)) = with_book(state, |book| {
        book.places.get(id).map(|place| {
            let name = place
                .instance_dir
                .as_deref()
                .and_then(std::path::Path::file_name)
                .and_then(|name| name.to_str())
                .map(str::to_owned);
            (place.app_id.clone(), name)
        })
    })
    .ok()
    .flatten() else {
        return false;
    };
    let home = juggler_platform::user::home().unwrap_or_default();
    let canonical =
        |path: &std::path::Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let accounts = crate::commands::lock_storage_arc(state)
        .ok()
        .and_then(|storage| storage.accounts().list().ok())
        .unwrap_or_default();
    for harness in state.registry.ids() {
        let Some(adapter) = state.registry.get(&harness) else {
            continue;
        };
        let Some(host) = adapter
            .host_apps()
            .into_iter()
            .find(|host| host.id == app_id)
        else {
            continue;
        };
        let installs = adapter.scan_environment();
        let found = match (&name, &host.default_folder) {
            (Some(name), _) => folder_for_profile(name, &installs),
            (None, Some(folder)) => {
                let want = canonical(&home.join(folder));
                installs.iter().find(|one| canonical(&one.path) == want)
            }
            (None, None) => None,
        };
        let Some(found) = found else {
            continue;
        };
        let name = name.clone().unwrap_or_else(|| {
            found
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        let held = accounts
            .iter()
            .find(|account| canonical(&account.state_dir) == canonical(&found.path))
            .map(|account| account.id.as_str().to_owned());
        let account = held.or_else(|| {
            if !may_bring_in {
                return None;
            }
            crate::features::account::import::account_import_mode_impl(
                state,
                harness.as_str(),
                &found.path.to_string_lossy(),
                &name,
                crate::features::account::import::ImportRequest {
                    origin: Some(found.origin.as_str().to_owned()),
                    in_place: true,
                    ..crate::features::account::import::ImportRequest::default()
                },
            )
            .map_err(|why| {
                tracing::warn!(
                    place = id,
                    "the added profile's folder wasn't brought in: {why}"
                );
            })
            .ok()
            .map(|done| done.account.id.as_str().to_owned())
        });
        return account.is_some_and(|account| {
            with_book(state, |book| {
                book.places.get_mut(id).is_some_and(|place| {
                    let new = place.account_id.as_deref() != Some(account.as_str());
                    place.account_id = Some(account);
                    new
                })
            })
            .unwrap_or(false)
        });
    }
    false
}

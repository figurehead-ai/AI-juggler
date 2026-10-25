//! D-38 (SETTLED 2026-10-25): the icons of the apps sessions run in, read
//! at run time from each app as it is installed on this Mac. The
//! maintainer: "Yes: show each app's icon as installed on my Mac, read at
//! run time; nothing shipped in the repo."
//!
//! - Only an app AI Juggler already knows is read ([`bundle::locate`]):
//!   AI Juggler's own, a place's running app, an app in the host registry.
//!   The UI asks by app id and process, never by path.
//! - Only a folder named `*.app`, not a link, whose icon files aren't
//!   links out of it ([`bundle::checked`]). Nothing outside it is read, and
//!   nothing reaches the network.
//! - The icon comes from macOS ([`render`]) at 32, 64 or 128 px, as a PNG,
//!   kept in memory and under `icons/` in AI Juggler's own folder until
//!   the app's bundle changes ([`cache`]).
//! - One app's profiles share its icon: the profile is the UI's badge.
//!
//! The tools' and companies' marks (`ToolMark`, AJ-E1526) are another
//! rule and never come from here.

pub mod bundle;
pub mod cache;
pub mod render;

#[cfg(test)]
mod tests;

pub use cache::IconCache;

use std::sync::Arc;

use base64::Engine;

use crate::error::AppResult;
use crate::features::conversations::places::HostRegistry;
use crate::state::AppState;

/// The sizes an icon is read at.
pub const SIZES: [u32; 3] = [32, 64, 128];

/// The icon of the app `ask` names, as a PNG `data:` URL; `None` when it
/// isn't installed, can't be read, or `px` isn't one of [`SIZES`].
#[must_use]
pub fn icon_data_url(
    cache: &IconCache,
    ask: &bundle::Ask,
    px: u32,
    hosts: &HostRegistry,
    lookup: &dyn bundle::Lookup,
) -> Option<String> {
    if !SIZES.contains(&px) {
        return None;
    }
    let found = bundle::locate(ask, hosts, lookup)?;
    let png = cache.get_or_render(&found, px, || render::icon_png(&found.path, px))?;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}

/// [`icon_data_url`] for the app's state and this Mac.
///
/// # Errors
///
/// None today; the result type keeps the command's shape.
pub fn read_impl(state: &Arc<AppState>, ask: &bundle::Ask, px: u32) -> AppResult<Option<String>> {
    let hosts = HostRegistry::of(&state.registry);
    Ok(icon_data_url(
        &state.app_icons,
        ask,
        px,
        &hosts,
        &bundle::System,
    ))
}

/// An app's icon as installed on this Mac, for the rail, Home and the
/// pages; `null` draws the app's letters.
///
/// # Errors
///
/// [`crate::error::AppError::Io`] when the read could not finish.
#[tauri::command]
pub async fn app_icons_read(
    state: tauri::State<'_, Arc<AppState>>,
    app_id: Option<String>,
    process: Option<u32>,
    px: u32,
) -> AppResult<Option<String>> {
    let ask = bundle::Ask {
        app_id,
        app_pid: process,
    };
    crate::ipc::blocking(state.inner(), "an app's icon", move |state| {
        read_impl(state, &ask, px)
    })
    .await
}

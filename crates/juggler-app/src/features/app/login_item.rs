//! AJ-E1509 Req.4 — "Open AI Juggler at login", opt-in, through macOS's
//! own login items (`SMAppService`, the audited `juggler-login-item`
//! crate, D-26). Off until the user turns it on (in Settings › Sessions or
//! the welcome's Make it smoother card); the system keeps the choice, so
//! AI Juggler reads it back rather than storing its own. Read at most once
//! a minute (the user can change it in System Settings too).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use juggler_login_item::LoginItem;

use crate::error::{AppError, AppResult};

/// How long a status read is reused.
const FRESH: Duration = Duration::from_secs(60);

static LAST: Mutex<Option<(Instant, LoginItem)>> = Mutex::new(None);

/// Whether AI Juggler opens at login, as the UI shows it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct LoginItemInfo {
    /// `on` | `off` | `needs_approval` (waiting in System Settings ›
    /// General › Login Items) | `unavailable` (a development build).
    pub state: String,
}

/// Pure: a status in the wire's words.
#[must_use]
pub fn words(item: LoginItem) -> &'static str {
    match item {
        LoginItem::On => "on",
        LoginItem::Off => "off",
        LoginItem::NeedsApproval => "needs_approval",
        LoginItem::Unavailable => "unavailable",
    }
}

/// Whether AI Juggler opens at login now (a read at most a minute old).
#[must_use]
pub fn status() -> LoginItem {
    if let Ok(last) = LAST.lock()
        && let Some((at, item)) = *last
        && at.elapsed() < FRESH
    {
        return item;
    }
    let item = juggler_login_item::status();
    if let Ok(mut last) = LAST.lock() {
        *last = Some((Instant::now(), item));
    }
    item
}

/// Whether AI Juggler opens at login.
#[tauri::command]
pub async fn app_login_item_get() -> LoginItemInfo {
    LoginItemInfo {
        state: words(status()).to_owned(),
    }
}

/// Turns opening at login on (AJ-E1547: through the gate, our sheet first
/// when it isn't primed, since the system shows its "login item added"
/// notice) or off (nothing is shown).
///
/// # Errors
///
/// [`AppError::Refused`] with the system's words when it refuses.
pub fn set_impl(on: bool) -> AppResult<LoginItem> {
    if !on {
        return juggler_platform::os_prompt::login_item_off().map_err(AppError::Refused);
    }
    match juggler_platform::os_prompt::login_item_on(
        crate::os_prompts::registry::LOGIN_ITEM,
        juggler_platform::os_prompt::user(),
    ) {
        juggler_platform::os_prompt::Gated::Called { value, .. } => {
            value.map_err(AppError::Refused)
        }
        // Not now: it stays as it was, and the next click asks again.
        _ => Ok(juggler_platform::os_prompt::login_item_status()),
    }
}

/// Turns opening at login on or off (off the main thread: the sheet may
/// wait for the user).
///
/// # Errors
///
/// [`AppError::Refused`] with the system's words when it refuses.
#[tauri::command]
pub async fn app_login_item_set(on: bool) -> AppResult<LoginItemInfo> {
    let item = tauri::async_runtime::spawn_blocking(move || set_impl(on))
        .await
        .map_err(|error| AppError::Io {
            context: "turn opening at login on or off",
            message: error.to_string(),
        })??;
    if let Ok(mut last) = LAST.lock() {
        *last = Some((Instant::now(), item));
    }
    Ok(LoginItemInfo {
        state: words(item).to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_status_has_its_word() {
        assert_eq!(words(LoginItem::On), "on");
        assert_eq!(words(LoginItem::Off), "off");
        assert_eq!(words(LoginItem::NeedsApproval), "needs_approval");
        assert_eq!(words(LoginItem::Unavailable), "unavailable");
    }

    #[test]
    fn a_test_build_never_reads_on() {
        assert_ne!(status(), LoginItem::On);
    }
}

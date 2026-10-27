//! AJ-E2507 — who the user menu says you are: your name as this computer
//! keeps it (its initial on the avatar), and what this system calls the
//! computer ("On this Mac", "My Macs"). Read through the platform seam;
//! nothing here is stored or sent.

use juggler_platform::system::{computer_noun, user_display_name};

/// You, as the user menu shows you.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AppUserInfo {
    /// Your first name (the first word of your full name), when known.
    pub name: Option<String>,
    /// The avatar's letter: your name's first, else "?".
    pub initial: String,
    /// What this system calls this computer ("Mac").
    pub computer: String,
    /// …and more than one ("Macs").
    pub computers: String,
}

/// Pure: the user menu's facts from a full name and the system's nouns.
#[must_use]
pub fn user_info(full_name: Option<&str>, nouns: (&str, &str)) -> AppUserInfo {
    let name = full_name
        .and_then(|full| full.split_whitespace().next())
        .map(str::to_owned);
    let initial = name
        .as_deref()
        .and_then(|name| name.chars().next())
        .map_or_else(|| "?".to_owned(), |first| first.to_uppercase().collect());
    AppUserInfo {
        name,
        initial,
        computer: nouns.0.to_owned(),
        computers: nouns.1.to_owned(),
    }
}

/// Who the user menu says you are.
#[tauri::command]
pub async fn app_user_get() -> AppUserInfo {
    user_info(user_display_name().as_deref(), computer_noun())
}

/// The user guides, public: where Help and guides goes (a fixed address;
/// the window passes nothing).
pub const GUIDES_URL: &str = "https://github.com/figurehead-ai/AI-juggler/tree/main/docs/guide";

/// Opens the user guides in the browser.
///
/// # Errors
///
/// [`crate::error::AppError::Io`] when the system can't open the browser.
#[tauri::command]
pub async fn app_help_open(app: tauri::AppHandle) -> crate::error::AppResult<()> {
    use tauri_plugin_opener::OpenerExt as _;
    app.opener()
        .open_url(GUIDES_URL, None::<&str>)
        .map_err(|e| crate::error::AppError::Io {
            context: "help",
            message: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_shows_your_first_name_and_its_initial() {
        let info = user_info(Some("Sam Rivera"), ("Mac", "Macs"));
        assert_eq!(info.name.as_deref(), Some("Sam"));
        assert_eq!(info.initial, "S");
        assert_eq!(info.computer, "Mac");
        let lower = user_info(Some("émile zola"), ("PC", "PCs"));
        assert_eq!(lower.initial, "É");
        let nobody = user_info(None, ("computer", "computers"));
        assert_eq!(nobody.name, None);
        assert_eq!(nobody.initial, "?");
        assert_eq!(user_info(Some("   "), ("Mac", "Macs")).name, None);
    }
}

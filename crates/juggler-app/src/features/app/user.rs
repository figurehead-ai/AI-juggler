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

/// The full name the menu starts from: the system's, except in a lab
/// (a Dev on a lab HOME, ADR-0027), where the user is a stand-in and the
/// Mac account's real name has no place on screen or in a walk's capture
/// (0.5.1 F24; "no real names in outputs").
#[must_use]
pub fn shown_full_name(in_a_lab: bool, system: impl FnOnce() -> Option<String>) -> Option<String> {
    if in_a_lab { None } else { system() }
}

/// Who the user menu says you are.
#[tauri::command]
pub async fn app_user_get() -> AppUserInfo {
    let full = shown_full_name(crate::lab_scope::in_a_lab_home(), user_display_name);
    user_info(full.as_deref(), computer_noun())
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
    crate::webview_guard::open_web(&app, GUIDES_URL, crate::webview_guard::Opened::ByApp)
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

    /// 0.5.1 F24: in a lab the menu never reads the Mac account's name.
    #[test]
    fn a_lab_never_shows_the_mac_accounts_name() {
        let read = std::cell::Cell::new(false);
        let system = || {
            read.set(true);
            Some("Sam Rivera".to_owned())
        };
        assert_eq!(shown_full_name(true, system), None);
        assert!(!read.get(), "not even read in a lab");
        assert_eq!(
            shown_full_name(false, || Some("Sam Rivera".to_owned())).as_deref(),
            Some("Sam Rivera")
        );
        let lab = user_info(shown_full_name(true, || None).as_deref(), ("Mac", "Macs"));
        assert_eq!((lab.name, lab.initial.as_str()), (None, "?"));
    }
}

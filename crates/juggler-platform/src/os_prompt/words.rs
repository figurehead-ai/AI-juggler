//! What each system prompt says, in the system's own words, and where its
//! answer can be changed later. macOS 14 and 15's words (Apple's dialogs
//! as they appear; a newer version may word them a little differently,
//! which the walks' dialog log catches). Other systems: none yet.

use super::{Dialog, Kind, Style};

const ALLOW: &str = "Allow";
const DONT_ALLOW: &str = "Don’t Allow";
const OPEN_SETTINGS: &str = "Open System Settings";
const DENY: &str = "Deny";
const OPEN: &str = "Open";
const CANCEL: &str = "Cancel";

/// The system's dialog for `kind`, `app` being the name it shows for AI
/// Juggler; `None` where this system shows none.
#[must_use]
pub fn dialog(kind: Kind, app: &str) -> Option<Dialog> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let alert =
        |title: String, body: &str, buttons: Vec<&'static str>, click: &'static str| Dialog {
            style: Style::Alert,
            title,
            body: body.to_owned(),
            buttons,
            click,
        };
    let to_settings = "Grant access to this application in Privacy & Security settings, located in System Settings.";
    Some(match kind {
        Kind::Accessibility => alert(
            format!("“{app}” would like to control this computer using accessibility features."),
            to_settings,
            vec![DENY, OPEN_SETTINGS],
            OPEN_SETTINGS,
        ),
        Kind::Automation(target) => alert(
            format!("“{app}” wants access to control “{}”.", target.name),
            &format!(
                "Allowing control will provide access to documents and data in “{}”, and to perform actions within that app.",
                target.name
            ),
            vec![DONT_ALLOW, ALLOW],
            ALLOW,
        ),
        Kind::Notifications => Dialog {
            style: Style::Banner,
            title: format!("“{app}” Notifications"),
            body: "Notifications may include alerts, sounds, and icon badges. These can be configured in Settings.".to_owned(),
            buttons: vec![ALLOW, DONT_ALLOW],
            click: ALLOW,
        },
        Kind::LoginItem => Dialog {
            style: Style::Banner,
            title: "Login Item Added".to_owned(),
            body: format!("“{app}” is an item that can run in the background. You can manage this in Login Items Settings."),
            buttons: Vec::new(),
            click: "",
        },
        Kind::ScreenRecording => alert(
            format!("“{app}” would like to record this computer’s screen and audio."),
            to_settings,
            vec![DENY, OPEN_SETTINGS],
            OPEN_SETTINGS,
        ),
        Kind::LocalNetwork => alert(
            format!("Allow “{app}” to find devices on local networks?"),
            "This will allow the app to discover, connect to, and collect data from devices on your networks.",
            vec![DONT_ALLOW, ALLOW],
            ALLOW,
        ),
        Kind::Folder(folder) => alert(
            format!("“{app}” would like to access files in your {} folder.", folder.name()),
            "",
            vec![DONT_ALLOW, ALLOW],
            ALLOW,
        ),
        Kind::AppData => alert(
            format!("“{app}” would like to access data from other apps."),
            "Keeping app data separate makes it easier to manage your privacy and security.",
            vec![DONT_ALLOW, ALLOW],
            ALLOW,
        ),
        Kind::Differs => alert(
            format!("“{app}” differs from previously opened versions. Are you sure you want to open it?"),
            "",
            vec![CANCEL, OPEN],
            OPEN,
        ),
        Kind::Keychain => alert(
            format!("{app} wants to use your confidential information stored in your keychain."),
            "To allow this, enter the login keychain password.",
            vec!["Always Allow", DENY, ALLOW],
            "Always Allow",
        ),
        Kind::FirstOpen => alert(
            "“…” is an app downloaded from the Internet. Are you sure you want to open it?".to_owned(),
            "",
            vec![CANCEL, OPEN],
            OPEN,
        ),
    })
}

/// Where the answer to `kind` can be changed later, in the system's words
/// (`None`: nowhere, it asks again each time).
#[must_use]
pub fn settings_place(kind: Kind) -> Option<&'static str> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    Some(match kind {
        Kind::Accessibility => "System Settings › Privacy & Security › Accessibility",
        Kind::Automation(_) => "System Settings › Privacy & Security › Automation",
        Kind::Notifications => "System Settings › Notifications",
        Kind::LoginItem => "System Settings › General › Login Items",
        Kind::ScreenRecording => "System Settings › Privacy & Security › Screen Recording",
        Kind::LocalNetwork => "System Settings › Privacy & Security › Local Network",
        Kind::Folder(_) => "System Settings › Privacy & Security › Files and Folders",
        Kind::AppData => "System Settings › Privacy & Security › App Data",
        Kind::FirstOpen => "System Settings › Privacy & Security (Open Anyway)",
        Kind::Differs | Kind::Keychain => return None,
    })
}

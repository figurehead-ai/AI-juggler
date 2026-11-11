//! What this system calls things, and what it can show outside a window
//! (AJ-E2501): the name its vendor gives this kind of computer, the
//! user's own name as the system keeps it, and whether the app's icon
//! can carry a count.
//!
//! - [`computer_noun`]: "Mac" on macOS, "PC" on Windows, "computer"
//!   elsewhere — so copy reads "On this Mac" where that is true, and the
//!   words follow the system everywhere else.
//! - [`user_display_name`]: the user's full name from the system's user
//!   record (macOS and Linux keep it in the account's GECOS field); none
//!   where it can't be read yet (Windows).
//! - [`app_badge`]: how a count shows on the app's icon. macOS badges the
//!   Dock icon. Windows draws an overlay on the taskbar button, which
//!   needs an icon drawn per count — not built yet. Linux has no common
//!   launcher badge — not built yet. Each "not yet" says so plainly.
//! - [`time_zone`] and [`zone_table`]: the system's time zone by name and
//!   the zone table naming each zone's place, for the Floor's sun
//!   (AJ-E3520: no Location permission, only the zone's coordinates).
//!   macOS and Linux link `/etc/localtime` into the zone database and keep
//!   `zone.tab` beside it; Windows names zones its own way — not built
//!   yet (none, and the Floor keeps the day's light).

/// Folders outside the home folder that say nothing about the person
/// (the system's own and installed apps'): a path in them is kept in a
/// bug report or log, where a personal one outside home is scrubbed.
#[must_use]
pub const fn system_roots() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &[
            "/Applications/",
            "/System/",
            "/Library/",
            "/usr/",
            "/bin/",
            "/sbin/",
            "/opt/",
            "/etc/",
            "/dev/",
        ]
    } else if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\",
            "C:\\Program Files\\",
            "C:\\Program Files (x86)\\",
            "C:\\ProgramData\\",
        ]
    } else {
        &[
            "/usr/",
            "/bin/",
            "/sbin/",
            "/opt/",
            "/etc/",
            "/dev/",
            "/lib/",
            "/lib64/",
            "/snap/",
            "/nix/store/",
        ]
    }
}

/// The noun this system's vendor uses for this computer, singular and
/// plural: what copy says after "this" ("On this Mac", "My Macs").
#[must_use]
pub const fn computer_noun() -> (&'static str, &'static str) {
    if cfg!(target_os = "macos") {
        ("Mac", "Macs")
    } else if cfg!(target_os = "windows") {
        ("PC", "PCs")
    } else {
        ("computer", "computers")
    }
}

/// AJ-E2904: where the user allows an app's notifications, in this
/// system's own words (Send test points there when nothing appeared).
#[must_use]
pub fn notification_settings_words() -> &'static str {
    notification_settings_words_on(std::env::consts::OS)
}

/// [`notification_settings_words`] on the system `os` names
/// (`std::env::consts::OS`).
#[must_use]
pub fn notification_settings_words_on(os: &str) -> &'static str {
    match os {
        "macos" => "System Settings › Notifications",
        "windows" => "Settings › System › Notifications",
        _ => "your desktop's notification settings",
    }
}

/// AJ-E2905 R7: a key combination the system (or every text field) keeps,
/// so a shortcut can't take it. `code` is the key's place as the window
/// reads it (`KeyboardEvent.code`); `primary` is ⌘ on macOS and Ctrl
/// elsewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReservedKey {
    /// The key's place (`KeyQ`, `Tab`, `F4`…).
    pub code: &'static str,
    /// With the primary modifier (⌘ on macOS, Ctrl elsewhere).
    pub primary: bool,
    /// With ⌥ / Alt.
    pub alt: bool,
    /// What the system does with it, after "is"/"does": `quits the app`.
    pub whose: &'static str,
}

const fn key(code: &'static str, primary: bool, alt: bool, whose: &'static str) -> ReservedKey {
    ReservedKey {
        code,
        primary,
        alt,
        whose,
    }
}

/// Every text field's own keys, on every system.
const EDITING: [ReservedKey; 4] = [
    key("KeyC", true, false, "copies"),
    key("KeyV", true, false, "pastes"),
    key("KeyX", true, false, "cuts"),
    key("KeyA", true, false, "selects all"),
];

const MACOS: [ReservedKey; 10] = [
    key("KeyQ", true, false, "quits the app"),
    key("KeyW", true, false, "closes the window"),
    key("KeyH", true, false, "hides the app"),
    key("KeyM", true, false, "minimises the window"),
    key("Tab", true, false, "switches apps"),
    key("Space", true, false, "is Spotlight"),
    EDITING[0],
    EDITING[1],
    EDITING[2],
    EDITING[3],
];

const WINDOWS: [ReservedKey; 7] = [
    key("Tab", false, true, "switches apps"),
    key("F4", false, true, "closes the window"),
    key("Space", false, true, "opens the window's menu"),
    EDITING[0],
    EDITING[1],
    EDITING[2],
    EDITING[3],
];

const LINUX: [ReservedKey; 7] = [
    key("Tab", false, true, "switches apps"),
    key("F4", false, true, "closes the window"),
    key("KeyT", true, true, "opens a terminal on many desktops"),
    EDITING[0],
    EDITING[1],
    EDITING[2],
    EDITING[3],
];

/// The keys this system keeps ([`ReservedKey`]).
#[must_use]
pub fn reserved_keys() -> &'static [ReservedKey] {
    reserved_keys_on(std::env::consts::OS)
}

/// [`reserved_keys`] on the system `os` names (`std::env::consts::OS`).
#[must_use]
pub fn reserved_keys_on(os: &str) -> &'static [ReservedKey] {
    match os {
        "macos" => &MACOS,
        "windows" => &WINDOWS,
        _ => &LINUX,
    }
}

/// How a count shows on the app's icon here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppBadge {
    /// The Dock icon carries the number (macOS).
    Dock,
    /// Not on this system yet: why, in plain words.
    NotYet(&'static str),
}

/// How a count shows on the app's icon on this system.
#[must_use]
pub const fn app_badge() -> AppBadge {
    if cfg!(target_os = "macos") {
        AppBadge::Dock
    } else if cfg!(target_os = "windows") {
        AppBadge::NotYet("the taskbar button's overlay isn't drawn yet")
    } else {
        AppBadge::NotYet("there is no common launcher badge here yet")
    }
}

/// The number a badge shows for `count`: none at zero (the badge goes).
#[must_use]
pub fn badge_value(count: u32) -> Option<i64> {
    (count > 0).then_some(i64::from(count))
}

/// The user's full name as the system keeps it, when it can be read.
#[cfg(unix)]
#[must_use]
pub fn user_display_name() -> Option<String> {
    let uid = nix::unistd::geteuid();
    let user = nix::unistd::User::from_uid(uid).ok().flatten()?;
    let gecos = user.gecos.to_string_lossy().into_owned();
    let full = gecos.split(',').next().unwrap_or("").trim().to_owned();
    if full.is_empty() {
        let login = user.name.trim().to_owned();
        (!login.is_empty()).then_some(login)
    } else {
        Some(full)
    }
}

/// The user's full name as the system keeps it, when it can be read:
/// not on this system yet.
#[cfg(not(unix))]
#[must_use]
pub fn user_display_name() -> Option<String> {
    None
}

/// A program's name for people when the system lists it by an identifier
/// (the 0.2 walk, F13: an alert named
/// `com.apple.Virtualization.VirtualMachine`). On macOS a service named in
/// reverse-DNS reads as its maker and its last part in words: "Virtual
/// Machine (part of macOS)" for the system's own, "Docker backend" for
/// another maker's. `None` when the name is plain already, and on a system
/// that doesn't name programs so.
#[must_use]
pub fn program_display_name(name: &str) -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let parts: Vec<&str> = name.split('.').collect();
    let plain = |part: &&str| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    };
    if parts.len() < 3
        || !parts.iter().all(plain)
        || !parts[0].chars().all(|c| c.is_ascii_lowercase())
    {
        return None;
    }
    let last = spaced(parts[parts.len() - 1]);
    if parts[0] == "com" && parts[1] == "apple" {
        return Some(format!("{last} (part of macOS)"));
    }
    let mut maker = parts[1].to_owned();
    if let Some(first) = maker.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    Some(format!("{maker} {last}"))
}

/// `VirtualMachine` → "Virtual Machine"; `helper-gpu` → "helper gpu".
fn spaced(word: &str) -> String {
    let mut out = String::new();
    let mut previous: Option<char> = None;
    for c in word.chars() {
        if c == '-' || c == '_' {
            out.push(' ');
        } else {
            if c.is_ascii_uppercase() && previous.is_some_and(|p| p.is_ascii_lowercase()) {
                out.push(' ');
            }
            out.push(c);
        }
        previous = Some(c);
    }
    out
}

/// The zone's name in a link into the zone database
/// (`/var/db/timezone/zoneinfo/Europe/Sarajevo` → `Europe/Sarajevo`).
#[must_use]
pub fn zone_of_link(path: &str) -> Option<String> {
    let name = path.rsplit_once("zoneinfo/")?.1.trim_matches('/');
    (!name.is_empty() && name.contains('/')).then(|| name.to_owned())
}

/// The system's time zone by name (`TZ` first, then what
/// `/etc/localtime` links to); none where it can't be read.
#[must_use]
pub fn time_zone() -> Option<String> {
    if let Ok(tz) = std::env::var("TZ") {
        let tz = tz.trim_start_matches(':');
        if tz.contains('/') && !tz.starts_with('/') {
            return Some(tz.to_owned());
        }
        if let Some(name) = zone_of_link(tz) {
            return Some(name);
        }
    }
    if cfg!(unix) {
        let link = std::fs::read_link("/etc/localtime").ok()?;
        zone_of_link(&link.to_string_lossy())
    } else {
        None
    }
}

/// The zone table (`zone.tab`: each zone's place in ISO 6709); none where
/// the system keeps none.
#[must_use]
pub fn zone_table() -> Option<String> {
    if cfg!(unix) {
        std::fs::read_to_string("/usr/share/zoneinfo/zone.tab").ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn each_system_keeps_its_own_keys_and_every_text_fields() {
        let mac = super::reserved_keys_on("macos");
        assert!(
            mac.iter()
                .any(|k| k.code == "KeyQ" && k.primary && k.whose == "quits the app")
        );
        let windows = super::reserved_keys_on("windows");
        assert!(
            windows
                .iter()
                .any(|k| k.code == "F4" && k.alt && !k.primary)
        );
        assert!(
            !windows.iter().any(|k| k.code == "KeyQ"),
            "Ctrl+Q isn't Windows' quit"
        );
        let linux = super::reserved_keys_on("linux");
        for list in [mac, windows, linux] {
            for code in ["KeyC", "KeyV", "KeyX", "KeyA"] {
                assert!(list.iter().any(|k| k.code == code && k.primary));
            }
        }
    }

    use super::*;

    #[test]
    fn a_service_named_in_reverse_dns_reads_in_words_on_macos() {
        let words = |name: &str| program_display_name(name);
        if cfg!(target_os = "macos") {
            assert_eq!(
                words("com.apple.Virtualization.VirtualMachine").as_deref(),
                Some("Virtual Machine (part of macOS)")
            );
            assert_eq!(
                words("com.docker.backend").as_deref(),
                Some("Docker backend")
            );
        } else {
            assert_eq!(words("com.apple.Virtualization.VirtualMachine"), None);
        }
        for plain in ["node", "next-server", "python3.11", "Docker", "my app.v2.x"] {
            assert_eq!(words(plain), None, "{plain}");
        }
    }

    #[test]
    fn a_badge_shows_a_count_and_goes_at_zero() {
        assert_eq!(badge_value(0), None);
        assert_eq!(badge_value(3), Some(3));
        assert_eq!(badge_value(u32::MAX), Some(i64::from(u32::MAX)));
    }

    /// AJ-E2904 R3: each system's own words for where notifications are
    /// allowed, and this one's are what Send test says.
    #[test]
    fn each_system_names_its_own_notification_settings() {
        assert_eq!(
            notification_settings_words_on("macos"),
            "System Settings › Notifications"
        );
        assert_eq!(
            notification_settings_words_on("windows"),
            "Settings › System › Notifications"
        );
        for other in ["linux", "freebsd"] {
            assert_eq!(
                notification_settings_words_on(other),
                "your desktop's notification settings"
            );
        }
        assert_eq!(
            notification_settings_words(),
            notification_settings_words_on(std::env::consts::OS)
        );
        if cfg!(target_os = "macos") {
            assert_eq!(
                notification_settings_words(),
                "System Settings › Notifications"
            );
        }
    }

    #[test]
    fn each_system_says_what_it_can_show() {
        let mac = std::hint::black_box(cfg!(target_os = "macos"));
        match app_badge() {
            AppBadge::Dock => assert!(mac, "the Dock is macOS's"),
            AppBadge::NotYet(why) => {
                assert!(!mac, "macOS has the Dock");
                assert!(why.contains("yet"));
            }
        }
        let (one, many) = computer_noun();
        assert!(!one.is_empty() && many.starts_with(one));
    }

    #[cfg(unix)]
    #[test]
    fn a_name_read_is_never_blank() {
        // A container's user may have no record at all: then there is none.
        if let Some(name) = user_display_name() {
            assert!(!name.trim().is_empty());
        }
    }

    #[test]
    fn a_zone_is_named_by_its_link() {
        use super::zone_of_link;
        assert_eq!(
            zone_of_link("/var/db/timezone/zoneinfo/America/New_York").as_deref(),
            Some("America/New_York")
        );
        assert_eq!(
            zone_of_link("/usr/share/zoneinfo/Europe/Sarajevo").as_deref(),
            Some("Europe/Sarajevo")
        );
        assert_eq!(zone_of_link("/usr/share/zoneinfo/UTC"), None);
        assert_eq!(zone_of_link("/etc/localtime"), None);
    }
}

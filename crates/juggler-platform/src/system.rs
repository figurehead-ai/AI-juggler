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

#[cfg(test)]
mod tests {
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
}

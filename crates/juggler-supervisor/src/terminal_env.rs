//! What a program AI Juggler starts in a terminal is told about that
//! terminal (0.4.1: every CLI in the embedded terminal drew in plain
//! white). A Mac app opened from the Finder or at login has no `TERM`,
//! `COLORTERM` or `LANG`: a tool started with the app's own environment
//! took its terminal for a dumb one and dropped every colour and style.
//!
//! The terminal is xterm.js: 256 colours and 24-bit colour, so `TERM` is
//! `xterm-256color` and `COLORTERM` is `truecolor`, whatever the app
//! inherited (a `TERM` from the terminal AI Juggler was started from
//! describes that terminal, not this one). `TERM_PROGRAM` names AI
//! Juggler. Without a UTF-8 locale, the system's is set (box drawing and
//! the tools' symbols are UTF-8).
//!
//! Not set: `FORCE_COLOR` (the terminal says what it can; a tool decides),
//! and a user's own `NO_COLOR` stays. The launch's overlay is applied after
//! this, so an adapter's value, or its unset marker, wins: an adapter
//! that removes one of these means it.
//!
//! Every PTY goes through [`pty_base_env`] — the supervisor's one PTY
//! spawn (`Supervisor::spawn_pty`) — and no other place opens one
//! (`tests/terminal_env.rs` holds both).

use std::ffi::{OsStr, OsString};

/// The terminal type xterm.js answers to.
pub const TERM: &str = "xterm-256color";
/// xterm.js draws 24-bit colour.
pub const COLORTERM: &str = "truecolor";
/// The name a tool sees for the terminal it runs in.
pub const TERM_PROGRAM: &str = "AI Juggler";
/// AI Juggler's version (one workspace version for every crate).
pub const TERM_PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The locale a terminal gets when neither the app nor the system gives a
/// UTF-8 one that exists on this Mac.
pub const FALLBACK_LOCALE: &str = "en_US.UTF-8";

/// Variables that name the terminal AI Juggler itself was started from
/// (the Dev build run from iTerm, Ghostty, …): inside AI Juggler's
/// terminal they lie, and a tool that reads them sends that terminal's
/// own sequences.
const PARENT_TERMINAL_KEYS: &[&str] = &[
    "TERM_SESSION_ID",
    "ITERM_SESSION_ID",
    "ITERM_PROFILE",
    "LC_TERMINAL",
    "LC_TERMINAL_VERSION",
    "KITTY_WINDOW_ID",
    "KITTY_PID",
    "GHOSTTY_RESOURCES_DIR",
    "GHOSTTY_BIN_DIR",
    "WEZTERM_PANE",
    "WEZTERM_EXECUTABLE",
    "ALACRITTY_WINDOW_ID",
    "ALACRITTY_SOCKET",
    "VTE_VERSION",
    "WT_SESSION",
    "TERMINAL_EMULATOR",
];

/// The environment a PTY child starts from: `inherited` (the app's own),
/// told it runs in AI Juggler's terminal. `system_locale` is the UTF-8
/// locale to use when `inherited` has none ([`system_utf8_locale`] in
/// production). The launch overlay goes on top of this.
#[must_use]
pub fn terminal_env<I>(inherited: I, system_locale: Option<&str>) -> Vec<(OsString, OsString)>
where
    I: IntoIterator<Item = (OsString, OsString)>,
{
    let mut env: Vec<(OsString, OsString)> = inherited
        .into_iter()
        .filter(|(key, _)| {
            !PARENT_TERMINAL_KEYS.iter().any(|k| key == *k)
                && !matches!(
                    key.to_str(),
                    Some("TERM" | "COLORTERM" | "TERM_PROGRAM" | "TERM_PROGRAM_VERSION")
                )
        })
        .collect();
    let locale = locale_to_set(&env, system_locale);
    for (key, value) in [
        ("TERM", TERM),
        ("COLORTERM", COLORTERM),
        ("TERM_PROGRAM", TERM_PROGRAM),
        ("TERM_PROGRAM_VERSION", TERM_PROGRAM_VERSION),
    ] {
        env.push((key.into(), value.into()));
    }
    if let Some((key, value)) = locale {
        env.push((key.into(), value.into()));
    }
    env
}

/// What a PTY child starts from: this process's environment through
/// [`terminal_env`], with the system's UTF-8 locale.
#[must_use]
pub fn pty_base_env() -> Vec<(OsString, OsString)> {
    terminal_env(std::env::vars_os(), system_utf8_locale())
}

/// The locale variable to add, if any: none when `LC_ALL` is set (the
/// user's word for every category) or `LC_CTYPE` / `LANG` is UTF-8 already;
/// `LANG` when there is none; else `LC_CTYPE`, leaving the user's `LANG`
/// for messages and formats.
fn locale_to_set(
    env: &[(OsString, OsString)],
    system_locale: Option<&str>,
) -> Option<(&'static str, String)> {
    let get = |name: &str| {
        env.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_os_str())
            .filter(|value| !value.is_empty())
    };
    if get("LC_ALL").is_some()
        || [get("LC_CTYPE"), get("LANG")]
            .into_iter()
            .flatten()
            .any(is_utf8)
    {
        return None;
    }
    let locale = system_locale.unwrap_or(FALLBACK_LOCALE).to_owned();
    Some((
        if get("LANG").is_none() {
            "LANG"
        } else {
            "LC_CTYPE"
        },
        locale,
    ))
}

/// True for a locale name in UTF-8 (`en_US.UTF-8`, `de_DE.utf8`, `UTF-8`).
fn is_utf8(value: &OsStr) -> bool {
    let lower = value.to_string_lossy().to_ascii_lowercase();
    lower.contains("utf-8") || lower.contains("utf8")
}

/// The system's own locale as a UTF-8 locale this Mac has (System
/// Settings › Language & Region, as Terminal sets it), or `None`: read
/// once per run.
#[must_use]
pub fn system_utf8_locale() -> Option<&'static str> {
    static LOCALE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    LOCALE
        .get_or_init(|| {
            utf8_locale_for(
                &system_locale_id()?,
                std::path::Path::new("/usr/share/locale"),
            )
        })
        .as_deref()
}

/// `id` (`de_DE`, `en_GB@rg=bazzzz`) as `<id>.UTF-8` when `locales`
/// holds it.
fn utf8_locale_for(id: &str, locales: &std::path::Path) -> Option<String> {
    let base = id.split('@').next()?.replace('-', "_");
    if base.is_empty() {
        return None;
    }
    let name = format!("{base}.UTF-8");
    locales.join(&name).is_dir().then_some(name)
}

#[cfg(target_os = "macos")]
fn system_locale_id() -> Option<String> {
    let id = objc2_foundation::NSLocale::currentLocale()
        .localeIdentifier()
        .to_string();
    (!id.is_empty()).then_some(id)
}

#[cfg(not(target_os = "macos"))]
fn system_locale_id() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    }

    fn get<'a>(env: &'a [(OsString, OsString)], key: &str) -> Option<&'a str> {
        env.iter()
            .rev()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.to_str())
    }

    #[test]
    fn a_finder_launched_app_gives_its_terminals_colour() {
        let out = terminal_env(env(&[("HOME", "/Users/me"), ("PATH", "/usr/bin")]), None);
        assert_eq!(get(&out, "TERM"), Some("xterm-256color"));
        assert_eq!(get(&out, "COLORTERM"), Some("truecolor"));
        assert_eq!(get(&out, "TERM_PROGRAM"), Some("AI Juggler"));
        assert_eq!(
            get(&out, "TERM_PROGRAM_VERSION"),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(get(&out, "LANG"), Some(FALLBACK_LOCALE));
        assert_eq!(get(&out, "HOME"), Some("/Users/me"));
        assert_eq!(get(&out, "FORCE_COLOR"), None, "never forced");
    }

    #[test]
    fn the_parent_terminal_is_not_this_one() {
        let out = terminal_env(
            env(&[
                ("TERM", "dumb"),
                ("TERM_PROGRAM", "iTerm.app"),
                ("ITERM_SESSION_ID", "w0t0p0"),
                ("LANG", "de_DE.UTF-8"),
            ]),
            Some("fr_FR.UTF-8"),
        );
        assert_eq!(out.iter().filter(|(k, _)| k == "TERM").count(), 1);
        assert_eq!(get(&out, "TERM"), Some("xterm-256color"));
        assert_eq!(get(&out, "TERM_PROGRAM"), Some("AI Juggler"));
        assert_eq!(get(&out, "ITERM_SESSION_ID"), None);
        assert_eq!(
            get(&out, "LANG"),
            Some("de_DE.UTF-8"),
            "a UTF-8 locale stays"
        );
    }

    #[test]
    fn the_users_no_color_stays() {
        let out = terminal_env(env(&[("NO_COLOR", "1")]), None);
        assert_eq!(get(&out, "NO_COLOR"), Some("1"));
        assert_eq!(get(&out, "FORCE_COLOR"), None);
    }

    #[test]
    fn the_locale_is_the_systems_and_never_over_the_users() {
        let out = terminal_env(env(&[]), Some("de_DE.UTF-8"));
        assert_eq!(get(&out, "LANG"), Some("de_DE.UTF-8"));
        // LANG chosen but not UTF-8: only the character type is added.
        let out = terminal_env(env(&[("LANG", "C")]), None);
        assert_eq!(get(&out, "LANG"), Some("C"));
        assert_eq!(get(&out, "LC_CTYPE"), Some(FALLBACK_LOCALE));
        // LC_ALL is the user's word for everything.
        let out = terminal_env(env(&[("LC_ALL", "C")]), None);
        assert_eq!(get(&out, "LANG"), None);
        assert_eq!(get(&out, "LC_CTYPE"), None);
        // UTF-8 character type already: nothing added.
        let out = terminal_env(env(&[("LC_CTYPE", "UTF-8")]), None);
        assert_eq!(get(&out, "LANG"), None);
    }

    #[test]
    fn a_system_locale_counts_only_when_this_mac_has_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("de_DE.UTF-8")).unwrap();
        assert_eq!(
            utf8_locale_for("de_DE@rg=atzzzz", dir.path()).as_deref(),
            Some("de_DE.UTF-8")
        );
        assert_eq!(utf8_locale_for("en_BA", dir.path()), None);
        assert_eq!(utf8_locale_for("", dir.path()), None);
    }
}

//! The system keychain, through one door (0.4.1). A run whose `HOME` isn't
//! this user's home (a lab, a replica, the Floor harness, a test), or one
//! started with `AIJ_NO_KEYCHAIN=1`, never touches a keychain: no `SecItem`
//! call and no `security` process. macOS looks for the login keychain under
//! `HOME`; finding none, it starts `SecurityAgent` and offers to create or
//! reset one, which once nearly reset the maintainer's real keychain.
//!
//! Every keychain use in the workspace goes through here or honours
//! [`off`] (`scripts/keychain_lint.py` holds it): the tools' sign-in
//! probes ([`item_exists`]) and AI Juggler's own secrets (the app keeps
//! them in private files when [`off`]).

use std::path::Path;
use std::sync::OnceLock;

/// Forces the no-keychain mode for a whole run (`AIJ_NO_KEYCHAIN=1`).
pub const ENV_OFF: &str = "AIJ_NO_KEYCHAIN";

/// Why this run must not touch a keychain, given its environment.
#[must_use]
pub fn off_because(
    forced: Option<&str>,
    home: Option<&Path>,
    record_home: Option<&Path>,
) -> Option<&'static str> {
    if forced.is_some_and(|value| !value.is_empty() && value != "0") {
        return Some("AIJ_NO_KEYCHAIN is set");
    }
    let canonical =
        |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    match (home, record_home) {
        (Some(home), Some(record)) if canonical(home) == canonical(record) => None,
        _ => Some("HOME isn't this user's home"),
    }
}

/// Why this run keeps away from every keychain, or `None` when it may use
/// the user's. Decided once, from `AIJ_NO_KEYCHAIN`, `HOME` and the user
/// record.
#[must_use]
pub fn off() -> Option<&'static str> {
    static OFF: OnceLock<Option<&'static str>> = OnceLock::new();
    *OFF.get_or_init(|| {
        let forced = std::env::var(ENV_OFF).ok();
        let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
        off_because(
            forced.as_deref(),
            home.as_deref(),
            crate::user::record_home().as_deref(),
        )
    })
}

/// Whether the keychain holds a generic password item for `service` (and
/// `account`, when given), without reading its secret (`-w` is never
/// passed). `false` when the keychain is [`off`]: nothing is run.
#[must_use]
pub fn item_exists(service: &str, account: Option<&str>) -> bool {
    if off().is_some() {
        return false;
    }
    item_exists_on_system(service, account)
}

#[cfg(target_os = "macos")]
fn item_exists_on_system(service: &str, account: Option<&str>) -> bool {
    let mut command = crate::process::helper(SECURITY);
    command.args(["find-generic-password", "-s", service]);
    if let Some(account) = account {
        command.args(["-a", account]);
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(target_os = "macos"))]
fn item_exists_on_system(_service: &str, _account: Option<&str>) -> bool {
    false
}

/// macOS's keychain tool, by absolute path (existence probes only).
#[cfg(target_os = "macos")]
const SECURITY: &str = "/usr/bin/security";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lab_or_a_forced_run_never_reaches_the_keychain() {
        let real = Path::new("/Users/someone");
        let replica = Path::new("/tmp/aij-replica");
        assert_eq!(
            off_because(None, Some(replica), Some(real)),
            Some("HOME isn't this user's home")
        );
        assert_eq!(
            off_because(None, None, Some(real)),
            Some("HOME isn't this user's home")
        );
        assert_eq!(
            off_because(None, Some(real), None),
            Some("HOME isn't this user's home")
        );
        assert_eq!(
            off_because(Some("1"), Some(real), Some(real)),
            Some("AIJ_NO_KEYCHAIN is set")
        );
        assert_eq!(off_because(Some("0"), Some(real), Some(real)), None);
        assert_eq!(off_because(None, Some(real), Some(real)), None);
    }
}

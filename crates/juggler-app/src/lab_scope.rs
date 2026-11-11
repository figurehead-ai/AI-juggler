//! ADR-0027: a lab sees only its own processes. In the test bridge's lab
//! write mode the app runs on a throwaway HOME, but this Mac's real apps
//! still run beside it; their names and folders would show on the lab's
//! screens (and in its walk captures). Once the lab mode is on
//! ([`set`], by `dev_bridge::lab` only), the process table the app
//! discovers apps and sessions from keeps the lab's own: a process whose
//! program or an argument names a path in the lab, or one this app
//! started (its descendants). Never a process's environment.
//!
//! The same holds for any run whose home ([`juggler_platform::user::home`])
//! isn't the user record's: a release candidate or replica started on
//! `HOME=<lab>` without the bridge (AJ-E3306: such a run offered this Mac's
//! running Claude app profiles, by name, beside the lab's). Off — no effect
//! at all — on the user's own home, which is every run from Finder or at
//! login.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::features::conversations::procs::Table;

static LAB: OnceLock<PathBuf> = OnceLock::new();

/// ADR-0027 condition 7: the real tool folder the lab uses where it is.
static ADOPTED: OnceLock<PathBuf> = OnceLock::new();

/// The lab uses `folder` (a real tool folder) where it is: of its
/// conversations, only those in the lab's folders are indexed ([`hides`]).
pub fn adopt(folder: &Path) {
    let _ = ADOPTED.set(folder.to_path_buf());
}

/// Whether a conversation of the Account on `account_dir`, in `cwd`, stays
/// out of a lab's index: one of the adopted real folder's that isn't in
/// the lab's folders (the review of condition 7: the real history would
/// show on the lab's screens and in its captures). Off — never hides —
/// outside a lab.
#[must_use]
pub fn hides(account_dir: &Path, cwd: Option<&Path>) -> bool {
    let (Some(lab), Some(adopted)) = (LAB.get(), ADOPTED.get()) else {
        return false;
    };
    hides_in(lab, adopted, account_dir, cwd)
}

/// [`hides`] for `lab` and `adopted` (pure, testable).
#[must_use]
pub fn hides_in(lab: &Path, adopted: &Path, account_dir: &Path, cwd: Option<&Path>) -> bool {
    let canonical =
        |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if canonical(account_dir) != canonical(adopted) {
        return false;
    }
    !cwd.is_some_and(|cwd| canonical(cwd).starts_with(canonical(lab)))
}

/// Scopes the process table to `lab` from now on (the lab mode's start).
pub fn set(lab: &Path) {
    let _ = LAB.set(lab.to_path_buf());
}

/// The folder the process table is scoped to: the bridge's lab, else
/// this run's home when it isn't the user record's (decided once).
fn scope() -> Option<&'static Path> {
    static OTHER_HOME: OnceLock<Option<PathBuf>> = OnceLock::new();
    LAB.get().map(PathBuf::as_path).or_else(|| {
        OTHER_HOME
            .get_or_init(juggler_platform::user::other_home)
            .as_deref()
    })
}

/// Keeps only the lab's processes in `table`, when a lab is set or this
/// run's home isn't the user's own.
pub fn keep(table: &mut Table) {
    let Some(lab) = scope() else {
        return;
    };
    keep_in(table, lab, std::process::id());
}

/// [`keep`] for `lab` and the app's own pid `me` (pure, testable).
pub fn keep_in(table: &mut Table, lab: &Path, me: u32) {
    let spellings = spellings(lab);
    let names_lab = |text: &str| {
        spellings
            .iter()
            .any(|lab| text.starts_with(lab.as_str()) || text.contains(&format!("={lab}")))
    };
    let descends = |pid: u32| {
        let mut at = Some(pid);
        for _ in 0..64 {
            match at {
                Some(found) if found == me => return true,
                Some(found) => at = table.get(&found).and_then(|process| process.ppid),
                None => return false,
            }
        }
        false
    };
    let ours: Vec<u32> = table
        .iter()
        .filter(|(pid, process)| {
            descends(**pid)
                || process
                    .exe
                    .as_deref()
                    .is_some_and(|exe| names_lab(&exe.to_string_lossy()))
                || process.args.iter().any(|arg| names_lab(arg))
        })
        .map(|(pid, _)| *pid)
        .collect();
    table.retain(|pid, _| ours.contains(pid));
}

/// The lab's path as processes may spell it (`/private/tmp/…` and `/tmp/…`).
fn spellings(lab: &Path) -> Vec<String> {
    let full = lab.to_string_lossy().into_owned();
    let mut out = vec![full.clone()];
    if let Some(short) = full.strip_prefix("/private") {
        out.push(short.to_owned());
    } else if full.starts_with("/tmp/") {
        out.push(format!("/private{full}"));
    }
    out
}

/// Whether HOME is a lab's (`/tmp/aij-lab…`, ADR-0027): a Dev there is its
/// own instance, apart from the Dev on the real HOME.
pub(crate) fn in_a_lab_home() -> bool {
    juggler_platform::user::home().is_some_and(|home| {
        home.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("aij-lab"))
    })
}

/// What the window keeps in its own storage is kept per data folder
/// (rc32): the webview's storage is one per app build, whatever HOME it
/// runs on, so a lab's Home said "Kimi Code is installed" from another
/// HOME's scan, and its Account pages opened on the tab another left.
/// The window's keys carry this scope: `None` on the user's own home
/// (their keys as they always were), else a short hash of `data_dir`.
#[must_use]
pub(crate) fn store_scope(data_dir: &Path, other_home: Option<&Path>) -> Option<String> {
    use sha2::{Digest, Sha256};
    other_home?;
    let digest = Sha256::digest(data_dir.to_string_lossy().as_bytes());
    Some(digest.iter().take(6).fold(String::new(), |mut out, b| {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
        out
    }))
}

/// The script the main window runs before its own: [`store_scope`] as
/// `window.__AIJ_STORE_SCOPE__` (`ui/src/lib/local-store.ts` reads it).
#[must_use]
pub(crate) fn store_scope_script(data_dir: &Path) -> String {
    let scope = store_scope(data_dir, juggler_platform::user::other_home().as_deref());
    format!(
        "window.__AIJ_STORE_SCOPE__ = \"{}\";",
        scope.unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::conversations::procs::Proc;

    #[test]
    fn the_adopted_folders_conversations_outside_the_lab_stay_out_of_its_index() {
        let lab = Path::new("/tmp/aij-lab-x");
        let real = Path::new("/Users/someone/.codex");
        // Its own conversation in the lab's folders: indexed.
        assert!(!hides_in(lab, real, real, Some(&lab.join("repos/web"))));
        // The real history elsewhere, or with no folder: kept out.
        assert!(hides_in(
            lab,
            real,
            real,
            Some(Path::new("/Users/someone/work/shop"))
        ));
        assert!(hides_in(lab, real, real, None));
        // Another Account (the lab's own folders): never hidden.
        let own = lab.join("profiles/codex-a");
        assert!(!hides_in(lab, real, &own, Some(Path::new("/elsewhere"))));
    }

    fn proc(pid: u32, ppid: u32, exe: &str, args: &[&str]) -> (u32, Proc) {
        (
            pid,
            Proc {
                pid,
                ppid: Some(ppid),
                exe: Some(PathBuf::from(exe)),
                args: args.iter().map(|arg| (*arg).to_owned()).collect(),
                start: None,
            },
        )
    }

    #[test]
    fn a_lab_keeps_its_own_processes_and_none_of_this_macs() {
        let mut table: Table = [
            // This app, and a session it started.
            proc(100, 1, "/build/debug/juggler-app", &[]),
            proc(101, 100, "/usr/local/bin/codex", &["codex"]),
            // A desktop app the lab runs on a folder in it.
            proc(
                200,
                1,
                "/Applications/Claude.app/Contents/MacOS/Claude",
                &["--user-data-dir=/tmp/aij-lab-x/profiles/work"],
            ),
            // This Mac's own: the real profile and an app with no folder.
            proc(
                300,
                1,
                "/Applications/Claude.app/Contents/MacOS/Claude",
                &["--user-data-dir=/Users/someone/Library/Profiles/real"],
            ),
            proc(
                301,
                1,
                "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT",
                &[],
            ),
            proc(
                302,
                301,
                "/Applications/ChatGPT.app/Contents/Resources/codex",
                &["codex", "app-server"],
            ),
        ]
        .into_iter()
        .collect();
        keep_in(&mut table, Path::new("/private/tmp/aij-lab-x"), 100);
        let mut kept: Vec<u32> = table.keys().copied().collect();
        kept.sort_unstable();
        assert_eq!(kept, [100, 101, 200]);
    }

    /// rc32: two labs keep apart what the window remembers; the user's
    /// own home keeps the keys it always had.
    #[test]
    fn each_lab_data_folder_has_its_own_window_storage() {
        let lab = Path::new("/tmp/aij-lab-a");
        let a = store_scope(Path::new("/tmp/aij-lab-a/data"), Some(lab)).unwrap();
        let b = store_scope(Path::new("/tmp/aij-lab-b/data"), Some(lab)).unwrap();
        assert_ne!(a, b);
        assert_eq!(a.len(), 12);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(store_scope(Path::new("/Users/me/data"), None), None);
    }

    #[test]
    fn without_a_lab_nothing_is_left_out() {
        let mut table: Table = [proc(
            1,
            0,
            "/Applications/Claude.app/Contents/MacOS/Claude",
            &[],
        )]
        .into_iter()
        .collect();
        keep(&mut table);
        assert_eq!(table.len(), 1);
    }
}

/// The real-accounts rule (ADR-0032 evidence, 2026-11-11): QA runs never
/// start a tool on the user's own tool folders. Agents' labs ran the
/// Claude CLI on two real profiles' folders (`ClaudeCodeProfiles/
/// bookauth-luka` and `-anela`), which made "CLI" Apps of them on the
/// user's screens. A lab may use only the sanctioned test profile's
/// folder (`ClaudeCodeProfiles/aij-test`), AI Juggler Dev's own managed
/// folders, or a folder under a throwaway lab home; this says why any
/// other `folder` (under `home`) is refused, `None` when it's allowed.
#[must_use]
pub fn refuses_real_folder(folder: &Path, home: &Path) -> Option<String> {
    let dev = home.join("Library/Application Support/AI Juggler Dev");
    let allowed = folder
        .starts_with(home.join("Library/Application Support/ClaudeCodeProfiles/aij-test"))
        || folder.starts_with(&dev)
        || !folder.starts_with(home);
    let real = [
        home.join(".claude"),
        home.join(".codex"),
        home.join("Library/Application Support/ClaudeCodeProfiles"),
        home.join("Library/Application Support/Claude"),
        home.join("Library/Application Support/ClaudeProfiles"),
    ];
    (!allowed && real.iter().any(|root| folder.starts_with(root))).then(|| {
        "a real tool folder of the user's: labs use only aij-test, AI Juggler Dev's own folders or a lab home"
            .to_owned()
    })
}

#[cfg(test)]
mod real_folder_tests {
    use super::refuses_real_folder;
    use std::path::Path;

    #[test]
    fn a_lab_never_runs_a_tool_on_the_users_own_folders() {
        let home = Path::new("/Users/you");
        let profiles = home.join("Library/Application Support/ClaudeCodeProfiles");
        assert!(refuses_real_folder(&profiles.join("bookauth-luka"), home).is_some());
        assert!(refuses_real_folder(&home.join(".claude"), home).is_some());
        assert!(refuses_real_folder(&home.join(".codex"), home).is_some());
        assert!(refuses_real_folder(&profiles.join("aij-test"), home).is_none());
        let dev = home
            .join("Library/Application Support/AI Juggler Dev/accounts/claude-code-361780292000");
        assert!(refuses_real_folder(&dev, home).is_none());
        assert!(refuses_real_folder(Path::new("/tmp/aij-lab-pd4/.claude"), home).is_none());
    }
}

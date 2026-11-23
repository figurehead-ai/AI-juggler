//! R11 (the 0.5.0 re-walk; the coordinating session's call, lab safety):
//! This Mac lists the Mac's real processes, by design: a normal build
//! always does. A lab's AI Juggler Dev (built with `lab-bridge`, its
//! `HOME` a throwaway `/tmp/aij-lab…`) is a guest on the Mac it runs on:
//! its Processes page shows and acts on only the lab's own processes,
//! those whose program or working folder is inside the lab's home, or
//! that the shared ownership module calls the lab's (an app instance or a
//! wrapper's program inside it, this app and what runs below those:
//! [`crate::lab_scope::keep_in`]). The walk's lab listed 150 of the Mac's
//! real Claude app processes, each with Stop… a press away.
//!
//! AJ-E2315 (the review's E12): never by a command line's text. An
//! argument's value (`--add-dir=<the lab>`: an agent driving the lab from
//! a real session) never makes a process the lab's, so it is never listed
//! or stopped from here.
//!
//! Two halves: the graph a lab builds leaves the rest of the Mac out
//! ([`super::model::scoped`]), and the stop paths refuse a process outside
//! the lab whatever asked ([`super::stop::check`], the ledger's stop):
//! hiding a row alone would leave the stop open to a pid typed by hand.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::features::conversations::procs::Table;

/// What the system says of a process that can tell it is the lab's: its
/// program and its working folder, as read now (never its arguments).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Whose {
    /// Its executable's path, when readable.
    pub exe: Option<String>,
    /// Its working folder, when readable.
    pub cwd: Option<String>,
}

/// The lab's home this AI Juggler is scoped to, or `None` (every normal
/// build, and a lab build whose `HOME` isn't a throwaway lab).
#[must_use]
pub fn home() -> Option<&'static Path> {
    static HOME: OnceLock<Option<PathBuf>> = OnceLock::new();
    HOME.get_or_init(lab_home).as_deref()
}

#[cfg(feature = "lab-bridge")]
fn lab_home() -> Option<PathBuf> {
    let home = juggler_platform::user::home()?;
    juggler_core::lab::throwaway_lab(&home).ok()
}

#[cfg(not(feature = "lab-bridge"))]
const fn lab_home() -> Option<PathBuf> {
    None
}

/// Whether `text` names `root` or a path under it (`/tmp/aij-lab-k3`,
/// `/tmp/aij-lab-k3/repos`, never `/tmp/aij-lab-k30`).
fn names(text: &str, root: &str) -> bool {
    text.match_indices(root).any(|(at, _)| {
        text[at + root.len()..]
            .chars()
            .next()
            .is_none_or(|next| matches!(next, '/' | ' ' | '"' | '\'' | '=' | ':'))
    })
}

/// Whether the process `whose` describes is the lab's at `home` by what
/// the system says of it (its program or its working folder inside it).
/// `home` is spelled as it resolves (`/private/tmp/aij-lab…`); `/tmp/…`
/// counts too.
#[must_use]
pub fn inside(home: &Path, whose: &Whose) -> bool {
    let resolved = home.to_string_lossy();
    let short = resolved
        .strip_prefix("/private")
        .unwrap_or(&resolved)
        .to_owned();
    let mut roots = vec![resolved.into_owned()];
    if short != roots[0] {
        roots.push(short);
    }
    [whose.exe.as_deref(), whose.cwd.as_deref()]
        .into_iter()
        .flatten()
        .any(|field| roots.iter().any(|root| names(field, root)))
}

/// One process's [`Whose`], read now; `None` when it's gone.
#[must_use]
pub fn read(pid: u32) -> Option<Whose> {
    let exe = super::stop::read_now(pid)?.exe;
    let (_, cwd) = juggler_supervisor::live_details(pid).unwrap_or_default();
    Some(Whose {
        exe,
        cwd: (!cwd.is_empty()).then_some(cwd),
    })
}

/// Whether `pid` is outside the lab this AI Juggler is scoped to (false
/// in a normal build). A process that can't be read is outside.
#[must_use]
pub fn outside(pid: u32) -> bool {
    home().is_some_and(|home| !keep(home, [pid]).get(&pid).copied().unwrap_or(false))
}

/// The lab's pids among `pids` (each read once).
#[must_use]
pub fn keep(home: &Path, pids: impl IntoIterator<Item = u32>) -> HashMap<u32, bool> {
    labs(
        home,
        &crate::features::conversations::procs::table(),
        std::process::id(),
        pids,
        read,
    )
}

/// [`keep`] over `table` with this app's pid `me`, each of `pids` read by
/// `read` (pure but for the instance lock its apps' processes may be
/// asked about): the lab's by the ownership module's evidence in `table`
/// ([`crate::lab_scope::keep_in`]), or by its program or working folder.
#[must_use]
pub fn labs(
    home: &Path,
    table: &Table,
    me: u32,
    pids: impl IntoIterator<Item = u32>,
    read: impl Fn(u32) -> Option<Whose>,
) -> HashMap<u32, bool> {
    let mut scoped = table.clone();
    crate::lab_scope::keep_in(&mut scoped, home, me);
    let owned: HashSet<u32> = scoped.into_keys().collect();
    pids.into_iter()
        .map(|pid| {
            let lab = owned.contains(&pid) || read(pid).is_some_and(|whose| inside(home, &whose));
            (pid, lab)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn whose(exe: &str, cwd: &str) -> Whose {
        Whose {
            exe: Some(exe.into()),
            cwd: Some(cwd.into()),
        }
    }

    #[test]
    fn a_process_is_the_labs_by_its_program_or_folder() {
        let home = Path::new("/private/tmp/aij-lab-k3");
        // The lab's stand-in, by its program.
        assert!(inside(
            home,
            &whose("/private/tmp/aij-lab-k3/.aij-lab-bin/claude", "/")
        ));
        // A child it left, by its folder (`/tmp` spelled short).
        assert!(inside(
            home,
            &whose("/bin/sleep", "/tmp/aij-lab-k3/repos/api")
        ));
        // The Mac's own: the real Claude app, a shell elsewhere, another lab.
        assert!(!inside(
            home,
            &whose("/Applications/Claude.app/Contents/MacOS/Claude", "/")
        ));
        assert!(!inside(home, &whose("/bin/zsh", "/Users/you/Dev")));
        assert!(!inside(home, &whose("/bin/sleep", "/tmp/aij-lab-k30")));
    }

    const CLAUDE: &str = "/Applications/Claude.app/Contents/MacOS/Claude";

    /// AJ-E2315 (E12): the lab's by the ownership module's evidence, never
    /// by an argument's value. A real session given `--add-dir=<the lab>`
    /// (an agent driving it) is never the lab's: not listed, never
    /// stoppable from the lab's Processes page.
    #[test]
    fn an_add_dir_into_the_lab_never_makes_a_process_the_labs() {
        use crate::features::conversations::procs::fixture;
        let home = Path::new("/tmp/aij-lab-k3");
        let table: Table = [
            // This app, and a session it started.
            fixture(100, 1, "/build/debug/juggler-app", &[]),
            fixture(101, 100, "/Users/you/.local/bin/claude", &["claude"]),
            // A lab Claude app profile, by its instance folder.
            fixture(
                200,
                1,
                CLAUDE,
                &[CLAUDE, "--user-data-dir=/tmp/aij-lab-k3/apps/lab"],
            ),
            // The Mac's own profile, and its Code tab's session given the
            // lab's folders, both forms.
            fixture(
                300,
                1,
                CLAUDE,
                &[CLAUDE, "--user-data-dir=/Users/you/Library/Profiles/work"],
            ),
            fixture(
                301,
                300,
                "/Users/you/.local/bin/claude",
                &["claude", "--add-dir=/tmp/aij-lab-k3/repos/web"],
            ),
            fixture(
                302,
                1,
                "/Users/you/.local/bin/claude",
                &["claude", "--add-dir", "/private/tmp/aij-lab-k3"],
            ),
        ]
        .into_iter()
        .collect();
        // What the system says of each: none is in the lab by its program
        // or folder (the agent works in its own repo).
        let read = |_: u32| Some(whose("/Users/you/.local/bin/claude", "/Users/you/Dev"));
        let lab = labs(home, &table, 100, [100, 101, 200, 300, 301, 302], read);
        for pid in [100, 101, 200] {
            assert!(lab[&pid], "{pid} is the lab's");
        }
        for pid in [300, 301, 302] {
            assert!(!lab[&pid], "{pid} isn't the lab's");
        }
        // A child the lab left behind (launchd took it), by its folder.
        let left = |_: u32| Some(whose("/bin/sleep", "/tmp/aij-lab-k3/repos/api"));
        assert!(labs(home, &table, 100, [999], left)[&999]);
        // Gone: not the lab's.
        assert!(!labs(home, &table, 100, [998], |_| None)[&998]);
    }

    #[test]
    fn a_normal_build_is_never_scoped() {
        #[cfg(not(feature = "lab-bridge"))]
        assert_eq!(home(), None);
        #[cfg(not(feature = "lab-bridge"))]
        assert!(!outside(1));
    }
}

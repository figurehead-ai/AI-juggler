//! R11 (the 0.5.0 re-walk; the coordinating session's call, lab safety):
//! This Mac lists the Mac's real processes, by design: a normal build
//! always does. A lab's AI Juggler Dev (built with `lab-bridge`, its
//! `HOME` a throwaway `/tmp/aij-lab…`) is a guest on the Mac it runs on:
//! its Processes page shows and acts on only the lab's own processes,
//! those whose program, working folder or command line is inside the
//! lab's home. The walk's lab listed 150 of the Mac's real Claude app
//! processes, each with Stop… a press away.
//!
//! Two halves: the graph a lab builds leaves the rest of the Mac out
//! ([`super::model::scoped`]), and the stop paths refuse a process outside
//! the lab whatever asked ([`super::stop::check`], the ledger's stop):
//! hiding a row alone would leave the stop open to a pid typed by hand.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// What tells whether a process is the lab's: its program, its working
/// folder and its command line, as read now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Whose {
    /// Its executable's path, when readable.
    pub exe: Option<String>,
    /// Its working folder, when readable.
    pub cwd: Option<String>,
    /// Its command line, joined.
    pub cmdline: String,
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
    let home = std::env::var_os("HOME")?;
    juggler_core::lab::throwaway_lab(Path::new(&home)).ok()
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

/// Whether the process `whose` describes is the lab's at `home` (its
/// program, its working folder or its command line inside it). `home` is
/// spelled as it resolves (`/private/tmp/aij-lab…`); `/tmp/…` counts too.
#[must_use]
pub fn inside(home: &Path, whose: &Whose) -> bool {
    let resolved = home.to_string_lossy();
    let short = resolved.strip_prefix("/private").unwrap_or(&resolved).to_owned();
    let mut roots = vec![resolved.into_owned()];
    if short != roots[0] {
        roots.push(short);
    }
    let fields = [whose.exe.as_deref(), whose.cwd.as_deref(), Some(whose.cmdline.as_str())];
    fields
        .into_iter()
        .flatten()
        .any(|field| roots.iter().any(|root| names(field, root)))
}

/// One process's [`Whose`], read now; `None` when it's gone.
#[must_use]
pub fn read(pid: u32) -> Option<Whose> {
    let exe = super::stop::read_now(pid)?.exe;
    let (cmdline, cwd) = juggler_supervisor::live_details(pid).unwrap_or_default();
    Some(Whose {
        exe,
        cwd: (!cwd.is_empty()).then_some(cwd),
        cmdline,
    })
}

/// Whether `pid` is outside the lab this AI Juggler is scoped to (false
/// in a normal build). A process that can't be read is outside.
#[must_use]
pub fn outside(pid: u32) -> bool {
    home().is_some_and(|home| read(pid).is_none_or(|whose| !inside(home, &whose)))
}

/// The lab's pids among `pids` (each read once).
#[must_use]
pub fn keep(home: &Path, pids: impl IntoIterator<Item = u32>) -> HashMap<u32, bool> {
    pids.into_iter()
        .map(|pid| (pid, read(pid).is_some_and(|whose| inside(home, &whose))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn whose(exe: &str, cwd: &str, cmdline: &str) -> Whose {
        Whose {
            exe: Some(exe.into()),
            cwd: Some(cwd.into()),
            cmdline: cmdline.into(),
        }
    }

    #[test]
    fn a_process_is_the_labs_by_its_program_folder_or_command_line() {
        let home = Path::new("/private/tmp/aij-lab-k3");
        // The lab's stand-in, by its program.
        assert!(inside(home, &whose("/private/tmp/aij-lab-k3/.aij-lab-bin/claude", "/", "claude")));
        // A child it left, by its folder (`/tmp` spelled short).
        assert!(inside(home, &whose("/bin/sleep", "/tmp/aij-lab-k3/repos/api", "sleep 600")));
        // A lab Claude app, by its data folder on the command line.
        assert!(inside(
            home,
            &whose("/Applications/Claude.app/Contents/MacOS/Claude", "/", "Claude --user-data-dir=/tmp/aij-lab-k3/apps/work")
        ));
        // The Mac's own: the real Claude app, a shell elsewhere, another lab.
        assert!(!inside(home, &whose("/Applications/Claude.app/Contents/MacOS/Claude", "/", "Claude")));
        assert!(!inside(home, &whose("/bin/zsh", "/Users/someone/Dev", "-zsh")));
        assert!(!inside(home, &whose("/bin/sleep", "/tmp/aij-lab-k30", "sleep 1")));
    }

    #[test]
    fn a_normal_build_is_never_scoped() {
        #[cfg(not(feature = "lab-bridge"))]
        assert_eq!(home(), None);
        #[cfg(not(feature = "lab-bridge"))]
        assert!(!outside(1));
    }
}

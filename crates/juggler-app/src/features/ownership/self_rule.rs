//! AJ-E2315 (the review's A1.5 and B1's self rule): a move never quits an
//! AI Juggler. Quitting a profile ends everything its main process runs.
//! When AI Juggler itself was started from inside that profile (the Dev
//! build run from a shell in its Code tab, as on the 0.4.6 rc3g walk), or
//! another AI Juggler was (the Dev build beside the installed one), the
//! move would quit AI Juggler mid-move, or wait for ever on the other one.
//! So the move refuses up front, before anything quits, and says why.
//!
//! An AI Juggler is a process running AI Juggler's own program (by its
//! file name: the Dev, Nightly and installed builds share it); its
//! ancestry is read up the process table from its pid, ours from our own.

use std::ffi::OsStr;
use std::path::Path;

use crate::features::conversations::procs::{Table, ancestors};

/// Which AI Juggler a move would quit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Juggler {
    /// This one: it runs below a process the move would quit.
    Ours,
    /// Another (its pid): it is one of the processes the move would
    /// quit, or runs below one.
    Another(u32),
}

/// Whether quitting `members` (a profile's processes, by pid) would quit
/// an AI Juggler: this one (`own`), or another, a process of `table`
/// running AI Juggler's own `program` (its file name). `None`: none.
#[must_use]
pub fn quits_juggler(table: &Table, members: &[u32], own: u32, program: &OsStr) -> Option<Juggler> {
    let quits = |pid: u32| {
        members.contains(&pid)
            || ancestors(table, pid)
                .iter()
                .any(|above| members.contains(&above.pid))
    };
    if ancestors(table, own)
        .iter()
        .any(|above| members.contains(&above.pid))
    {
        return Some(Juggler::Ours);
    }
    let mut others: Vec<u32> = table
        .values()
        .filter(|process| {
            process.pid != own && process.exe.as_deref().and_then(Path::file_name) == Some(program)
        })
        .map(|process| process.pid)
        .collect();
    others.sort_unstable();
    others
        .into_iter()
        .find(|pid| quits(*pid))
        .map(Juggler::Another)
}

/// Why `action` (Move, Undo) of `name` is refused for `juggler`, in
/// plain words: what was started from inside it, and how it clears.
#[must_use]
pub fn words(juggler: Juggler, name: &str, action: &str) -> String {
    match juggler {
        Juggler::Ours => format!(
            "AI Juggler itself was started from inside {name} (from its Code tab, say), so closing {name} would close AI Juggler too; {action} becomes available once you start AI Juggler from the Dock or Finder"
        ),
        Juggler::Another(_) => format!(
            "another AI Juggler was started from inside {name} (from its Code tab, say), so closing {name} would close it too; {action} becomes available once you quit it, or start it from the Dock or Finder"
        ),
    }
}

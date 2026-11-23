//! AJ-E2315 (the review's A2.4 and E7): which tool folder a desktop app
//! profile's sessions use, by evidence, never by a folder's name. The
//! profile launcher's convention (`ClaudeProfiles/<name>` beside
//! `ClaudeCodeProfiles/<name>`) linked a profile to a folder it may never
//! have used: one started by the user's own launcher with another
//! `CLAUDE_CONFIG_DIR` then had its history moved where it doesn't read,
//! and was restarted on another folder.
//!
//! The evidence is the tool's own: a running session writes a pid file in
//! the tool folder it uses (`<folder>/sessions/<pid>.json`, read by the
//! tool's adapter as [`LiveSession`]s), and that pid's parents are the
//! app's own: up from it, only programs of the app's bundle (its
//! wrappers), then a main process that runs this instance (its instance
//! flag read strictly, else the lock it holds, [`super::instance_of`]). A
//! session started from a shell inside the profile (a nested command
//! line, which may use any folder) has a shell above it, so it tells
//! nothing.

use std::path::{Path, PathBuf};

use juggler_adapter::live::{HostApp, LiveSession, same_start};
use juggler_platform::real_path::lexical;

use super::{InstanceKey, instance_of};
use crate::features::conversations::procs::{Table, ancestors};

/// The instance a folder's sessions are looked for under: its app, the
/// bundle it runs from, the home its default folder is under, and its
/// folder.
#[derive(Clone, Copy, Debug)]
pub struct Under<'a> {
    /// The app (its instance flag and lock).
    pub app: &'a HostApp,
    /// Its bundle (`/Applications/Claude.app`).
    pub bundle: &'a Path,
    /// The home its default instance is under.
    pub home: &'a Path,
    /// The instance's folder.
    pub instance: &'a InstanceKey,
}

/// Of `folders` (each tool folder with the live sessions its pid files
/// name), the one the sessions of `under` use (see the module's notes).
/// `None` when none of its sessions runs (nothing tells), or sessions in
/// two folders do (nothing is decided).
#[must_use]
pub fn folder_of_sessions<'f>(
    table: &Table,
    under: &Under<'_>,
    folders: &'f [(PathBuf, Vec<LiveSession>)],
) -> Option<&'f Path> {
    let mut found: Option<&Path> = None;
    for (folder, sessions) in folders {
        if sessions
            .iter()
            .any(|session| runs_in(table, under, session))
        {
            match found {
                Some(known) if known != folder.as_path() => return None,
                _ => found = Some(folder),
            }
        }
    }
    found
}

/// Whether `session`'s process runs in `under`'s instance, started by
/// the app itself.
fn runs_in(table: &Table, under: &Under<'_>, session: &LiveSession) -> bool {
    let Some(process) = session.pid.and_then(|pid| table.get(&pid)) else {
        return false;
    };
    // A pid the system gave another program since isn't the session.
    if !same_start(session.started_secs, process.start) {
        return false;
    }
    let bundle = lexical(under.bundle);
    let mains = bundle.join("Contents/MacOS");
    for above in ancestors(table, process.pid) {
        let Some(exe) = above.exe.as_deref().map(lexical) else {
            return false;
        };
        if !exe.starts_with(&bundle) {
            return false;
        }
        if exe.parent() == Some(mains.as_path()) {
            return instance_of(above, under.app, under.home)
                .ok()
                .and_then(|instance| instance.dir)
                .is_some_and(|dir| InstanceKey::of(&dir) == *under.instance);
        }
    }
    false
}

/// AJ-E2327: where a profile's tool-folder link comes from, strongest
/// first. No process's environment is ever read (D-37: the Claude app's
/// sessions carry a sign-in token there): what the Code child was given
/// is read from the pid file it writes in that folder itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkSource {
    /// Its running sessions' pid files, under its own main process
    /// ([`folder_of_sessions`]): the folder its Code child writes in now.
    Running,
    /// Its own records: the conversations the app lists for it, found as
    /// transcripts in one folder ([`folder_of_records`]).
    Records,
    /// AI Juggler's own: the folder it starts the profile with (its link
    /// so far), or the one the user chose for it.
    Recorded,
    /// The folder its app declares for its one instance (ADR-0032: the
    /// Codex app's `~/.codex`).
    Declared,
    /// The folder named like the profile (the profile launcher's
    /// convention): a fallback only, kept until evidence confirms or
    /// corrects it.
    Named,
}

impl LinkSource {
    /// Its stored word (and the window's).
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Records => "records",
            Self::Recorded => "recorded",
            Self::Declared => "declared",
            Self::Named => "named",
        }
    }

    /// From its stored word.
    #[must_use]
    pub fn of_word(word: &str) -> Option<Self> {
        [
            Self::Running,
            Self::Records,
            Self::Recorded,
            Self::Declared,
            Self::Named,
        ]
        .into_iter()
        .find(|one| one.word() == word)
    }

    /// Whether it proves the folder (what the profile's own sessions or
    /// records show), rather than recalls or guesses it.
    #[must_use]
    pub const fn is_evidence(self) -> bool {
        matches!(self, Self::Running | Self::Records | Self::Declared)
    }
}

/// What tells which tool folder a profile uses, each `None` when it
/// tells nothing ([`decide`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Evidence<'f> {
    /// [`LinkSource::Running`].
    pub running: Option<&'f Path>,
    /// [`LinkSource::Records`].
    pub records: Option<&'f Path>,
    /// [`LinkSource::Recorded`].
    pub recorded: Option<&'f Path>,
    /// [`LinkSource::Declared`].
    pub declared: Option<&'f Path>,
    /// [`LinkSource::Named`].
    pub named: Option<&'f Path>,
}

/// AJ-E2327: the folder a profile uses, by the strongest evidence that
/// tells: its running sessions, then its own records, then the folder its
/// app declares, then AI Juggler's own record, then (a fallback only) the
/// folder named like it. `None` when nothing tells.
#[must_use]
pub fn decide<'f>(evidence: &Evidence<'f>) -> Option<(&'f Path, LinkSource)> {
    [
        (evidence.running, LinkSource::Running),
        (evidence.records, LinkSource::Records),
        (evidence.declared, LinkSource::Declared),
        (evidence.recorded, LinkSource::Recorded),
        (evidence.named, LinkSource::Named),
    ]
    .into_iter()
    .find_map(|(folder, source)| folder.map(|folder| (folder, source)))
}

/// AJ-E2327 (for AJ-E2317's Move-time refusal): the folder evidence says
/// a profile uses when it isn't the one it's `linked` to, and what says
/// so. Only proof contradicts a link (its running sessions, its records,
/// its app's declared folder); a recalled or guessed folder never does.
#[must_use]
pub fn contradicts<'f>(evidence: &Evidence<'f>, linked: &Path) -> Option<(&'f Path, LinkSource)> {
    let linked = InstanceKey::of(linked);
    decide(&Evidence {
        recorded: None,
        named: None,
        ..*evidence
    })
    .filter(|(folder, _)| InstanceKey::of(folder) != linked)
}

/// AJ-E2327: of `folders` (each tool folder with the conversation ids its
/// transcripts hold), the one holding the most of `ids` (the profile's own
/// records' conversations), so a profile that isn't running is linked by
/// where its sessions are. A switch copies a conversation into another
/// folder under the same id, so a tie isn't decided by counting: one of
/// `prefer` among the tied (its link so far, the folder named like it)
/// is confirmed, else nothing is decided. `None` when none holds any.
#[must_use]
pub fn folder_of_records<'f>(
    ids: &[String],
    folders: &'f [(PathBuf, std::collections::HashSet<String>)],
    prefer: &[&Path],
) -> Option<&'f Path> {
    let counts: Vec<(&Path, usize)> = folders
        .iter()
        .map(|(folder, held)| {
            let n = ids.iter().filter(|id| held.contains(*id)).count();
            (folder.as_path(), n)
        })
        .collect();
    let most = counts.iter().map(|(_, n)| *n).max().filter(|n| *n > 0)?;
    let tied: Vec<&Path> = counts
        .iter()
        .filter(|(_, n)| *n == most)
        .map(|(folder, _)| *folder)
        .collect();
    if let [one] = tied.as_slice() {
        return Some(one);
    }
    prefer.iter().find_map(|want| {
        let want = InstanceKey::of(want);
        tied.iter()
            .find(|folder| InstanceKey::of(folder) == want)
            .copied()
    })
}

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

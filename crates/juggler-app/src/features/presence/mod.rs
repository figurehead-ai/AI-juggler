//! ADR-0032 (D-100): presence comes from the process table.
//!
//! One place decides what runs and who owns a session; every list, count
//! and the rail read what it says:
//!
//! - a desktop App runs while its main process does (its bundle, its
//!   instance folder, the same start time), found in the process table
//!   now ([`running`]), never from a session's last activity or a pid
//!   remembered from an earlier scan;
//! - a live session belongs to the App its own process runs in
//!   ([`live_owner`]): the desktop App of its own tool when one is on its
//!   process's chain, else its folder's CLI App. No folder guesses for a
//!   live session: its process says;
//! - an ended one to the App it was last seen live in, else the one its
//!   tool's records name, else its folder's CLI App
//!   ([`crate::features::places::app_of`]).
//!
//! [`check`] holds the invariants; debug builds log any mismatch.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::features::conversations::places::{HostRegistry, RunningApp, running_apps};
use crate::features::conversations::procs::Table;
use crate::features::conversations::scan::Place;

/// The App ids of the desktop Apps running in `table`, each with its main
/// process (pid, start, program).
#[must_use]
pub fn running(table: &Table, hosts: &HostRegistry, home: &Path) -> BTreeMap<String, RunningApp> {
    running_apps(table, hosts, home)
        .into_iter()
        .map(|app| (app_id_of(&app), app))
        .collect()
}

/// A running desktop app's App id (`app:<host app>:<instance folder>`).
#[must_use]
pub fn app_id_of(app: &RunningApp) -> String {
    crate::features::places::book::place_id(&app.app_id, app.instance.as_deref())
}

/// The App a live session belongs to: the desktop App of its own tool
/// (`tool_hosts`: the host app ids its adapter declares) its process runs
/// in, else the CLI App of `folder` (the tool folder its files are in).
/// A terminal, an editor, tmux or no app on its chain is a CLI session's
/// detail, never a desktop App's.
#[must_use]
pub fn live_owner(tool_hosts: &[String], place: Option<&Place>, folder: &str) -> String {
    place
        .and_then(|place| place.host.as_ref())
        .filter(|host| host.kind == Some(juggler_adapter::live::HostKind::Desktop))
        .and_then(|host| {
            let app = host.app_id.as_deref()?;
            tool_hosts
                .iter()
                .any(|own| own == app)
                .then(|| crate::features::places::book::place_id(app, host.instance_dir.as_deref()))
        })
        .unwrap_or_else(|| juggler_core::storage::cli_app_id(folder))
}

/// The host app ids `tool`'s adapter declares (`claude-app` for
/// `claude-code`, `codex-app` for `codex`).
#[must_use]
pub fn tool_hosts(registry: &juggler_adapter::AdapterRegistry, tool: &str) -> Vec<String> {
    registry
        .ids()
        .into_iter()
        .filter(|id| id.to_string() == tool)
        .filter_map(|id| registry.get(&id))
        .flat_map(|adapter| adapter.host_apps())
        .map(|app| app.id)
        .collect()
}

/// ADR-0032 amendment 5: an App's one identity, as every surface names it
/// (the rail's data, Accounts, the Floor's signs): never built twice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppIdentity {
    /// Its App id (`app:<host>:<instance folder>`, `cli:<folder>`).
    pub id: String,
    /// Desktop or CLI: said in words and by a badge everywhere.
    pub desktop: bool,
    /// Its app's word: "Claude app", "Codex app", "Claude Code CLI".
    pub word: String,
    /// Its profile: a desktop App's Account name (its instance folder's
    /// name when another desktop App shares that folder), none for the
    /// app's default instance; a CLI App's folder name.
    pub profile: Option<String>,
    /// "Claude app · acme-east", or the name the user gave it.
    pub label: String,
}

/// What [`identity`] is made from.
#[derive(Clone, Copy, Debug)]
pub struct Naming<'a> {
    /// The App id.
    pub id: &'a str,
    /// A desktop App (else CLI).
    pub desktop: bool,
    /// A desktop App's host word ("Claude app"), or a CLI App's tool name
    /// ("Claude Code").
    pub app: &'a str,
    /// A desktop App's instance folder; whether it's the app's default.
    pub instance_dir: Option<&'a Path>,
    /// Whether that's the app's default instance.
    pub default_instance: bool,
    /// Its folder's Account name.
    pub folder_label: Option<&'a str>,
    /// Another desktop App holds the same folder.
    pub folder_shared: bool,
    /// The user's own name for it.
    pub named: Option<&'a str>,
}

/// An App's one identity ([`AppIdentity`]).
#[must_use]
pub fn identity(naming: Naming<'_>) -> AppIdentity {
    let word = if naming.desktop || naming.app.ends_with(" CLI") {
        naming.app.to_owned()
    } else {
        format!("{} CLI", naming.app)
    };
    let own = || {
        naming
            .instance_dir
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
    };
    let profile = if naming.desktop {
        if naming.default_instance || naming.instance_dir.is_none() {
            None
        } else if naming.folder_shared {
            own()
        } else {
            naming.folder_label.map(str::to_owned).or_else(own)
        }
    } else {
        naming.folder_label.map(str::to_owned)
    };
    let label = naming
        .named
        .filter(|name| !name.trim().is_empty())
        .map_or_else(
            || {
                profile
                    .as_ref()
                    .map_or_else(|| word.clone(), |p| format!("{word} · {p}"))
            },
            str::to_owned,
        );
    AppIdentity {
        id: naming.id.to_owned(),
        desktop: naming.desktop,
        word,
        profile,
        label,
    }
}

/// What the invariants are checked against: the Apps known with whether
/// each runs, the running set from the process table, and each live
/// session's owner.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    /// Each known desktop App id with the pid the list gives it.
    pub listed: BTreeMap<String, Option<u32>>,
    /// The desktop App ids running now, from the process table.
    pub running: BTreeSet<String>,
    /// Each live session (`<account>/<tool>/<id>`) with its owner.
    pub live_owners: BTreeMap<String, String>,
}

/// The invariants, broken ones in words (empty when all hold):
///
/// - a known desktop App reads running exactly when the process table
///   has it;
/// - a live session's desktop owner is running.
#[must_use]
pub fn check(facts: &Facts) -> Vec<String> {
    let mut out = Vec::new();
    for (id, pid) in &facts.listed {
        match (pid.is_some(), facts.running.contains(id)) {
            (true, false) => out.push(format!("{id} reads running but no process runs it")),
            (false, true) => out.push(format!("{id} runs but reads closed")),
            _ => {}
        }
    }
    for (session, owner) in &facts.live_owners {
        if owner.starts_with("app:") && !facts.running.contains(owner) {
            out.push(format!(
                "live session {session} is owned by {owner}, which isn't running"
            ));
        }
    }
    out
}

/// Debug builds: log each broken invariant (`presence: invariant …`).
pub fn self_check(facts: &Facts) {
    if cfg!(debug_assertions) {
        for broken in check(facts) {
            tracing::warn!(target: "presence", "presence: invariant: {broken}");
        }
    }
}

#[cfg(test)]
mod tests;

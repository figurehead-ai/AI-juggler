//! AJ-E3013 (D-116): AI Juggler's connector in a desktop App's own
//! config folder, as that App's adapter describes it.
//!
//! The connector is how a desktop app's sessions join a pool: the agent
//! bridge as an MCP server (and, for Claude, the plugin's skill and inbox
//! hooks). Each desktop adapter says what it is as data
//! ([`DesktopConnector::plan`]): whole files of AI Juggler's own, or one
//! table merged into the tool's own config file. One writer for every
//! tool (`juggler_adapters::connector_files`) previews, adds, removes and
//! checks it, with nothing written outside that App's config folder,
//! nothing through a link, and no secret anywhere in it.

use std::path::{Path, PathBuf};

use crate::inventory::GetIt;

/// The name every tool's inventory lists AI Juggler's connector under
/// (a plugin, [`crate::inventory::Origin::Provided`] by
/// [`CONNECTOR_BY`]): never a user's skill or server to copy, only
/// something the other App turns on.
pub const CONNECTOR_ITEM: &str = "ai-juggler";

/// Who provides the connector, in words.
pub const CONNECTOR_BY: &str = "AI Juggler";

/// One part of the connector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectorPart {
    /// A whole file of AI Juggler's own (never merged: created, and on an
    /// upgrade replaced only while it is still as AI Juggler wrote it).
    File {
        /// Its path inside the App's config folder (relative; no `..`).
        path: PathBuf,
        /// Its text.
        text: String,
    },
    /// One TOML table merged into the tool's own config file; every
    /// other byte of the file is kept (comments, order, spacing).
    TomlTable {
        /// The config file's path inside the App's config folder.
        path: PathBuf,
        /// The table's dotted path (`["mcp_servers", "juggler"]`).
        table: Vec<String>,
        /// The table as TOML, its header and its sub-tables included
        /// (`[mcp_servers.juggler]` …).
        text: String,
    },
}

impl ConnectorPart {
    /// The file it writes, relative to the App's config folder.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::File { path, .. } | Self::TomlTable { path, .. } => path,
        }
    }
}

/// What the connector is for one App.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectorPlan {
    /// Its parts, in the order they're written.
    pub parts: Vec<ConnectorPart>,
    /// When the App's sessions pick it up, in a sentence.
    pub picked_up: String,
    /// How the user adds it themselves instead (the guide): the steps,
    /// the one command or snippet to copy, and what happens after.
    pub guide: GetIt,
}

/// What a plan is made from: the edition of AI Juggler doing the Add.
#[derive(Clone, Copy, Debug)]
pub struct ConnectorInput<'a> {
    /// The bridge program (`…/Contents/MacOS/juggler-agent`), by its
    /// absolute path.
    pub agent: &'a Path,
    /// The edition's bridge socket (a path, not a secret).
    pub socket: &'a Path,
    /// AI Juggler's version, for the plugin's manifest.
    pub version: &'a str,
}

/// A desktop App's connector, as data (Claude: its plugin folder; Codex:
/// one `[mcp_servers.juggler]` table).
pub trait DesktopConnector: Send + Sync {
    /// The connector for this edition of AI Juggler. Holds no secret.
    fn plan(&self, input: &ConnectorInput<'_>) -> ConnectorPlan;
}

/// Whether `path` is a plain relative path that stays inside the folder
/// it's joined to: no root, no `..`, no `.` steps, not empty.
#[must_use]
pub fn stays_inside(path: &Path) -> bool {
    use std::path::Component;
    path.components().next().is_some()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::stays_inside;
    use std::path::Path;

    #[test]
    fn a_part_stays_inside_its_folder() {
        assert!(stays_inside(Path::new(
            "skills/ai-juggler/.claude-plugin/plugin.json"
        )));
        assert!(stays_inside(Path::new("config.toml")));
        for outside in ["", "/etc/hosts", "../x", "skills/../../x", "./x"] {
            assert!(!stays_inside(Path::new(outside)), "{outside}");
        }
    }
}

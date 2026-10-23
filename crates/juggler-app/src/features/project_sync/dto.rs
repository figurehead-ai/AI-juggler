//! The project sync's wire types (AJ-E1507).

/// One rules file in the project, beside the master.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RulesFileInfo {
    /// Its name in the project folder.
    pub name: String,
    /// The tools that read it, by name.
    pub tools: Vec<String>,
    /// `master`, `same`, `reads`, `link`, `differs`, `missing`, `broken`
    /// (it reads a rules file that isn't there), or `unreadable` (not a
    /// text file, or too big to compare).
    pub standing: String,
    /// How it stands, in a few words ("Reads AGENTS.md", "12 lines differ").
    pub words: String,
    /// When it last changed, epoch ms; `None` when it isn't there.
    #[ts(type = "number | null")]
    pub modified_ms: Option<i64>,
    /// It can be the master: it's there, with its own text.
    pub can_be_master: bool,
    /// Applying the master to it is possible (it isn't a link, and it's
    /// text).
    pub can_apply: bool,
    /// The project's summary counts it as differing; the sheet opens on
    /// the first one that does.
    pub counts: bool,
    /// The line that would make it read the master instead of repeating it,
    /// when its tools have one.
    pub import_line: Option<String>,
    /// Its content's hash, to apply against exactly what was shown; `None`
    /// when it isn't there.
    pub sha256: Option<String>,
}

/// A project's rules files and how they stand.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RulesSyncInfo {
    /// The master compared with: the one picked, or the newest file with
    /// its own text; `None` when there's none.
    pub master: Option<String>,
    /// Every rules file the project's tools read.
    pub files: Vec<RulesFileInfo>,
    /// Every file's tools see the master's rules.
    pub in_sync: bool,
    /// For the project's header: "Rules in sync", "1 rules file differs";
    /// `None` when there's nothing to compare (no rules file at all).
    pub summary: Option<String>,
}

/// One line of a diff.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DiffLineInfo {
    /// `kept`, `added` or `removed`.
    pub change: String,
    /// The line's text.
    pub text: String,
    /// Its number in the file now; `None` for an added line.
    pub old_number: Option<u32>,
    /// Its number after applying; `None` for a removed line.
    pub new_number: Option<u32>,
}

/// What applying the master to one file would change.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RulesDiffInfo {
    /// The file.
    pub file: String,
    /// The master.
    pub master: String,
    /// `copy` (the master's text) or `read` (the import line).
    pub how: String,
    /// Each change with the lines around it.
    pub groups: Vec<Vec<DiffLineInfo>>,
    /// Lines added.
    pub added: u32,
    /// Lines removed.
    pub removed: u32,
}

/// One account's MCP servers, for the project.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ServerAccountInfo {
    /// The account.
    pub account: String,
    /// Its name.
    pub label: String,
    /// Its tool, by name.
    pub tool: String,
    /// Its servers' names, this project's own included.
    pub servers: Vec<String>,
}

/// A server one account has and another lacks.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ServerCopyInfo {
    /// The server's name.
    pub server: String,
    /// The account it would be copied from.
    pub from: String,
    /// That account's name.
    pub from_label: String,
    /// The account that lacks it.
    pub to: String,
    /// That account's name.
    pub to_label: String,
    /// Pass back to copy it.
    pub offer: String,
    /// `copy`, `translate`, `needs-you`, `impossible`.
    pub verdict: String,
    /// What the verdict means for this one, in a sentence.
    pub note: Option<String>,
}

/// MCP servers across the accounts, for one project.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ServersSyncInfo {
    /// Each account and its servers.
    pub accounts: Vec<ServerAccountInfo>,
    /// What one account could get from another, the ones that can be
    /// copied first.
    pub copies: Vec<ServerCopyInfo>,
}

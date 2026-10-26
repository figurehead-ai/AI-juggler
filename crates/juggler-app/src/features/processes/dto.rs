//! What the Processes page sees (AJ-E2401..E2406).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::model::Counts;

/// Counts over the whole graph.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProcessCountsInfo {
    /// Processes shown (AI Juggler's own hooks and tools excluded).
    pub processes: u32,
    /// Sessions.
    pub sessions: u32,
    /// Processes needing the user (Needs attention).
    pub attention: u32,
    /// To clean up: left behind or didn't stop, not kept or snoozed (the
    /// top bar's and This Mac's count).
    pub to_clean: u32,
    /// Left behind.
    pub left_behind: u32,
    /// Finished, waiting for their parent.
    pub zombies: u32,
    /// Didn't stop.
    pub didnt_stop: u32,
    /// Won't be touched.
    pub wont_touch: u32,
}

impl ProcessCountsInfo {
    /// From the graph's counts.
    #[must_use]
    pub const fn of(counts: &Counts) -> Self {
        Self {
            processes: counts.processes,
            sessions: counts.sessions,
            attention: counts.attention,
            to_clean: counts.to_clean,
            left_behind: counts.left_behind,
            zombies: counts.zombies,
            didnt_stop: counts.didnt_stop,
            wont_touch: counts.wont_touch,
        }
    }
}

/// Totals over a session's or an app's processes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RollupInfo {
    /// Processes.
    pub processes: u32,
    /// Memory, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
    /// CPU, percent of one core.
    pub cpu_pct: f64,
    /// Listening ports (at most 8).
    pub ports: Vec<u16>,
    /// Processes needing the user.
    pub attention: u32,
    /// Processes to clean up.
    pub to_clean: u32,
    /// The most urgent state among them, when any.
    pub worst: Option<String>,
}

/// An app's row.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AppRowInfo {
    /// The registry's id, when known (its icon is read from the app).
    pub app_id: Option<String>,
    /// Its name.
    pub name: String,
    /// Its main process, when known.
    pub pid: Option<u32>,
    /// AI Juggler itself.
    pub ours: bool,
    /// `own`, `desktop`, `terminal` or `editor`, when known (its icon).
    pub kind: Option<String>,
    /// Its sessions.
    pub sessions: u32,
}

/// A session's row.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProcessSessionInfo {
    /// The session's id (the supervisor's for ours, the tool's for theirs).
    pub session_id: String,
    /// `ours` or `theirs`.
    pub origin: String,
    /// Its tool.
    pub tool: String,
    /// Its account.
    pub account_id: Option<String>,
    /// Its project.
    pub project_id: Option<String>,
    /// Its title.
    pub title: String,
    /// Its folder's name, when known (never the path).
    pub folder_name: Option<String>,
    /// `working`, `idle` or `ended`.
    pub state: String,
    /// Its own process still runs.
    pub alive: bool,
    /// Its own process, while it runs.
    pub root_pid: Option<u32>,
    /// Folded to one line (its processes not listed).
    pub folded: bool,
}

/// A process's row.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProcRowInfo {
    /// Its pid.
    pub pid: u32,
    /// Its start time, epoch seconds (with the pid, its identity).
    #[ts(type = "number")]
    pub start_time: i64,
    /// Its short name (`node (vite)`).
    pub name: String,
    /// Its role (`dev_server`, `watcher`, `mcp_server`, `tool`, `hook`, …).
    pub role: String,
    /// The role in words ("dev server").
    pub role_words: String,
    /// The role is known, not guessed.
    pub role_sure: bool,
    /// What started it, from the tool's own events ("Started by Bash: npm
    /// run dev"), when known.
    pub label: Option<String>,
    /// Recent CPU, oldest first (percent of one core).
    pub spark: Vec<f32>,
    /// The most memory seen, bytes.
    #[ts(type = "number")]
    pub peak_rss: u64,
    /// Age, ms.
    #[ts(type = "number")]
    pub age_ms: i64,
    /// CPU time used, ms.
    #[ts(type = "number")]
    pub cpu_time_ms: u64,
    /// For a zombie: the parent only it can wait for (pid, name).
    pub waits_for_pid: Option<u32>,
    /// That parent's name.
    pub waits_for_name: Option<String>,
    /// `ours` (the kill sheet), `observed` (D-41b) or `never`.
    pub stop: String,
    /// Why it isn't offered, for `never`.
    pub stop_reason: Option<String>,
    /// `keep` or `snooze`, when the user marked it.
    pub mark: Option<String>,
    /// For a reminder: until when, epoch ms.
    #[ts(type = "number | null")]
    pub mark_until_ms: Option<i64>,
    /// Heavy (the hog rules).
    pub heavy: bool,
    /// Why its last stop failed, when it did.
    pub failed: Option<String>,
    /// AI Juggler's own (a hook, its tools): hidden unless asked for.
    pub internal: bool,
}

/// One row of the page: an app, a session or a process.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProcessGraphRowInfo {
    /// Unique in the page (`a:…`, `s:…`, `p:<pid>:<start>`).
    pub key: String,
    /// `app`, `session` or `process`.
    pub kind: String,
    /// Indent: 0 an app, 1 a session, 2 and on a process and its
    /// subprocesses.
    pub depth: u8,
    /// The row it hangs under, when any.
    pub parent_key: Option<String>,
    /// It has rows under it (expandable).
    pub has_children: bool,
    /// Its state word (a process's; a session's `working|idle|ended`).
    pub state: String,
    /// `ours` or `theirs`.
    pub origin: String,
    /// Whose, in words ("Started by AI Juggler", "In a session Claude runs").
    pub owner_words: String,
    /// CPU now, percent of one core.
    pub cpu_pct: Option<f64>,
    /// Memory now, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
    /// Listening ports.
    pub ports: Vec<u16>,
    /// The app's details, for an app row.
    pub app: Option<AppRowInfo>,
    /// The session's details, for a session row.
    pub session: Option<ProcessSessionInfo>,
    /// The process's details, for a process row.
    pub process: Option<ProcRowInfo>,
    /// Totals, for an app or a session row.
    pub rollup: Option<RollupInfo>,
    /// The session the row belongs to (`s:…`), for a process row.
    pub session_key: Option<String>,
}

/// Which rows the page asks for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)] // filters: each switch is its own choice
pub struct ProcessQueryInput {
    /// `attention` (default), `running`, `ended`.
    pub view: String,
    /// Only this session (its id), for a session's Processes tab.
    pub session_id: Option<String>,
    /// Only these states (words), when any.
    pub states: Vec<String>,
    /// Only this app (its row key or name).
    pub app: Option<String>,
    /// Only this account (profile).
    pub account_id: Option<String>,
    /// Only this project.
    pub project_id: Option<String>,
    /// Only processes listening on a port.
    pub has_port: bool,
    /// Only heavy processes.
    pub heavy: bool,
    /// Also AI Juggler's own hooks and tools.
    pub show_internal: bool,
    /// Words to find (`port:5173`, `pid:123`, a name, a role, a title).
    pub search: String,
    /// Sessions unfolded by the user (keys).
    pub expanded: Vec<String>,
    /// Sessions folded by the user (keys).
    pub collapsed: Vec<String>,
}

/// The view's state beside its rows: the graph's version and counts.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProcessStatusInfo {
    /// The graph's version the rows are of.
    #[ts(type = "number")]
    pub version: u64,
    /// The counts now.
    pub counts: ProcessCountsInfo,
    /// When the graph was built, epoch ms (0: not yet).
    #[ts(type = "number")]
    pub built_ms: i64,
    /// The ledger can't be written: what AI Juggler started can be shown
    /// but not stopped until it restarts.
    pub ledger_degraded: bool,
}

/// One cleanup in the audit, for the Cleanup history view.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CleanupEventInfo {
    /// Its id.
    #[ts(type = "number")]
    pub id: i64,
    /// When, epoch ms.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// `user`, `user:observed`, `auto:tier-1`, `auto:tier-2`, `system`.
    pub actor: String,
    /// Done by an automatic rule.
    pub automatic: bool,
    /// The process.
    pub pid: u32,
    /// Its program's name.
    pub name: String,
    /// The session it was in.
    pub session_id: String,
    /// Its tool.
    pub tool: String,
    /// Its account.
    pub account_id: Option<String>,
    /// Its project.
    pub project_id: Option<String>,
    /// The outcome in words (`exited on TERM`, `needed KILL`, `already
    /// gone`, `refused`, or not known yet).
    pub outcome: String,
    /// Ports it held before.
    pub ports_before: Option<String>,
    /// Ports still held after.
    pub ports_after: Option<String>,
}

/// What an automatic rule would do now (the dry run).
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RulePreviewInfo {
    /// The account.
    pub account_id: String,
    /// `1` (tidy left-behind dev servers) or `2` (also after a crash).
    pub tier: u8,
    /// What it would tidy now, and what later.
    pub items: Vec<RulePreviewItemInfo>,
    /// The sentence to show ("Would tidy vite :5173, esbuild --watch now").
    pub words: String,
}

/// One process a rule would or wouldn't tidy.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RulePreviewItemInfo {
    /// Its pid.
    pub pid: u32,
    /// Its short name with its ports (`vite :5173`).
    pub name: String,
    /// The session it was in.
    pub session_id: String,
    /// `now`, `later` (its grace hasn't passed) or `never`.
    pub when: String,
    /// Why not, in words, for `never`; for `later`, when.
    pub reason: Option<String>,
}

/// An account's automatic cleanup, as the page shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct AccountPolicyInfo {
    /// The account.
    pub account_id: String,
    /// Its name.
    pub label: String,
    /// Tidies left-behind dev servers and watchers.
    pub tier1: bool,
    /// Also after a crash.
    pub tier2: bool,
}

/// The automatic cleanup across accounts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CleanupPolicyInfo {
    /// Each account's rules.
    pub accounts: Vec<AccountPolicyInfo>,
    /// Paused for every account.
    pub paused: bool,
    /// Held after a crash until this time, epoch ms.
    #[ts(type = "number | null")]
    pub suspended_until_ms: Option<i64>,
}

/// A process snapshot to save (names and hashed command lines only).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ProcessSnapshotInfo {
    /// A file name to offer.
    pub file_name: String,
    /// The snapshot, JSON.
    pub json: String,
}

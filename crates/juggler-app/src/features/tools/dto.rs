//! The tools catalog's wire types.

/// One of first run's groups for a tool's setups.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SetupGroupInfo {
    /// Stable within the tool.
    pub id: String,
    /// The group's heading.
    pub title: String,
    /// Matches a setup whose path contains this.
    pub path_contains: Option<String>,
    /// Matches a setup with this scan origin.
    pub origin: Option<String>,
    /// The heading says which version is installed.
    pub shows_install: bool,
}

/// Where a tool sits in the app's flows.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ToolFlowInfo {
    /// First run brings its setups in.
    pub first_run: bool,
    /// The tool a fresh account is for when nothing else says.
    pub primary: bool,
}

/// A tool's own terms, and one line on several accounts under them.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ToolTermsInfo {
    /// The terms' name ("Anthropic's Consumer Terms").
    pub name: String,
    /// Where they are, on the vendor's own site.
    pub url: String,
    /// What several accounts mean under them.
    pub accounts: String,
    /// The day the line was checked against them (`YYYY-MM-DD`).
    pub checked: String,
}

/// What a screen needs to know a tool can do.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ToolSupportsInfo {
    /// ADR-0018: its setups can be used where they are.
    pub in_place: bool,
    /// It reports its usage limits.
    pub limits: bool,
}

/// One tool, as every screen reads it (ADR-0019 §1).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ToolSpecInfo {
    /// The tool's id.
    pub id: String,
    /// Its name on every screen.
    pub display_name: String,
    /// Its short name where room is tight.
    pub short_name: String,
    /// Who makes it; empty when unknown.
    pub vendor: String,
    /// The tool's mark id; `null` draws the letter mark.
    pub mark: Option<String>,
    /// The vendor's mark, where it differs.
    pub vendor_mark: Option<String>,
    /// Where to get it.
    pub install_url: Option<String>,
    /// A one-line install command, where the vendor documents one.
    pub install_command: Option<String>,
    /// Its own sign-in, first mention on a surface.
    pub sign_in_own: String,
    /// Its own sign-in, in a list of tools.
    pub sign_in_short: String,
    /// What "the same sign-in, settings and history" is the same as.
    pub same_as: String,
    /// The desktop app that can hold the same setup open.
    pub desktop_app: Option<String>,
    /// D-38: the host registry's id for the tool's own app (the first of
    /// its adapter's host apps), whose icon stands for the tool.
    #[ts(optional = nullable)]
    pub host_app: Option<String>,
    /// `cli` and/or `desktop`.
    pub kinds: Vec<String>,
    /// The project rules files it reads.
    pub rules_files: Vec<String>,
    /// First run's words while it looks for this tool's setups.
    pub scan_hint: String,
    /// First run's groups, in order.
    pub setup_groups: Vec<SetupGroupInfo>,
    /// The name its default setup is brought in as.
    pub default_label: Option<String>,
    /// Where it sits in the app's flows.
    pub flow: ToolFlowInfo,
    /// Its vendor's terms, shown when an account is added.
    pub terms: Option<ToolTermsInfo>,
    /// The environment keys that point it at an account's folder.
    pub home_env: Vec<String>,
    /// What it can do that screens show.
    pub supports: ToolSupportsInfo,
}

//! The tool update check's wire types.

/// What a tool's page says about its versions (AJ-E1611).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is one fact the page states"
)]
pub struct ToolUpdateInfo {
    /// The tool's id.
    pub tool: String,
    /// The version on this Mac, when it's installed and says.
    pub installed: Option<String>,
    /// The newest version the last check found.
    pub latest: Option<String>,
    /// `latest` is newer than `installed`.
    pub newer: bool,
    /// `latest` is one AI Juggler was checked with.
    pub verified: bool,
    /// When the last check was, epoch ms.
    #[ts(type = "number | null")]
    pub checked_ms: Option<i64>,
    /// The command the user runs to update it (never run by AI Juggler).
    pub update_command: Option<String>,
    /// The daily check is on.
    pub checks_on: bool,
    /// The tool has a public list of its versions to check.
    pub has_feed: bool,
}

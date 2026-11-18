//! The notices' wire types (ADR-0035 §2, AJ-E2023).

/// How loud a notice is: D-117's attention levels, never louder than its
/// targeting justifies ([`super::cap`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum RemoteNoticeLevel {
    /// L1, a note.
    Info,
    /// L2, needs you.
    NeedsYou,
    /// L3, broken.
    Broken,
}

/// One notice from the remote configuration, to show.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteNoticeInfo {
    /// Its id: what a dismissal remembers.
    pub id: String,
    /// How loud it is.
    pub level: RemoteNoticeLevel,
    /// One plain line.
    pub title: String,
    /// A plain sentence or two, if any.
    pub body: Option<String>,
    /// An `https://` link to our website, if any.
    pub link: Option<String>,
    /// The link's words, if the file gives them (else the window's own).
    pub link_label: Option<String>,
    /// The user may dismiss it.
    pub dismissable: bool,
}

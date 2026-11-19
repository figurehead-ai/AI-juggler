//! The Models screen's wire types (AJ-E3803, AJ-E3813; the screen is
//! AJ-E3812): the models on this Mac, the download, and the answers to a
//! Download click and an approval. Every word the screen shows comes from
//! the words file; these carry keys and numbers only.

use super::download::Outcome;
use super::store::{Installed, Refusal};
use super::transfer::{Failure, Phase, Progress, Wait};
use super::{InstalledState, Listing, installed_state};

/// Where an installed model stands against the list (ADR-0035 §11.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum InstalledStanding {
    /// Listed: works, with the list's current verdicts.
    Listed,
    /// The list can't be read: works from its copy, with no note.
    Unknown,
    /// No longer listed: works until removed, with the plain note.
    Withdrawn,
}

/// One model on this Mac.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct InstalledModelInfo {
    /// Its id.
    pub id: String,
    /// Its display name.
    pub name: String,
    /// Its disk use, bytes.
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// When it was installed, milliseconds since the epoch.
    #[ts(type = "number")]
    pub installed_at_ms: i64,
    /// Its licence's name.
    pub licence_name: Option<String>,
    /// Where it stands against the list.
    pub standing: InstalledStanding,
}

impl InstalledModelInfo {
    /// The row for `installed`, against `listing`.
    #[must_use]
    pub fn of(installed: &Installed, listing: &Listing) -> Self {
        let (standing, entry) = match installed_state(&installed.entry, listing) {
            InstalledState::Listed(entry) => (InstalledStanding::Listed, entry),
            InstalledState::Unknown(entry) => (InstalledStanding::Unknown, entry),
            InstalledState::Withdrawn(entry) => (InstalledStanding::Withdrawn, entry),
        };
        Self {
            id: installed.entry.id.clone(),
            name: entry.name,
            size_bytes: installed.size_bytes,
            installed_at_ms: installed.installed_at_ms,
            licence_name: installed
                .entry
                .download
                .as_ref()
                .map(|download| download.licence_name.clone()),
            standing,
        }
    }
}

/// Where a download stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum DownloadState {
    /// Bytes are arriving.
    Running,
    /// Waiting for this Mac (`wait` says why).
    Waiting,
    /// Every byte is in; checking it.
    Verifying,
    /// Verified and installed.
    Installed,
    /// Cancelled; nothing is left behind.
    Cancelled,
    /// It failed (`failure` says why); nothing is left behind.
    Failed,
}

/// Why a download failed: a key for the words file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum DownloadFailure {
    /// The file wasn't the one tested (its hash differed): deleted.
    NotTheTestedFile,
    /// The host sent a different size: stopped.
    WrongSize,
    /// The host sent it somewhere it may not go: stopped.
    Redirected,
    /// The disk filled up while it ran: the part was deleted.
    DiskFull,
    /// The network kept failing.
    Network,
    /// The file couldn't be written.
    Disk,
}

impl DownloadFailure {
    /// The key for `failure` (`None` for a cancel).
    #[must_use]
    pub fn of(failure: &Failure) -> Option<Self> {
        Some(match failure {
            Failure::Cancelled => return None,
            Failure::HashMismatch => Self::NotTheTestedFile,
            Failure::SizeMismatch | Failure::TooLong => Self::WrongSize,
            Failure::UrlRefused(_) | Failure::RedirectRefused | Failure::TooManyRedirects => {
                Self::Redirected
            }
            Failure::DiskFull => Self::DiskFull,
            Failure::Network(_) => Self::Network,
            Failure::Disk(_) => Self::Disk,
        })
    }
}

/// A download: running, or how the last one ended.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct DownloadInfo {
    /// The model.
    pub id: String,
    /// Bytes on disk.
    #[ts(type = "number")]
    pub done_bytes: u64,
    /// The file's size.
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// Where it stands.
    pub state: DownloadState,
    /// Why it waits, when it does.
    pub wait: Option<Wait>,
    /// Why it failed, when it did.
    pub failure: Option<DownloadFailure>,
}

impl DownloadInfo {
    /// A running download.
    #[must_use]
    pub fn running(id: &str, progress: Progress) -> Self {
        let (state, wait) = match progress.phase {
            Phase::Running => (DownloadState::Running, None),
            Phase::Waiting(wait) => (DownloadState::Waiting, Some(wait)),
            Phase::Verifying => (DownloadState::Verifying, None),
        };
        Self {
            id: id.to_owned(),
            done_bytes: progress.done,
            size_bytes: progress.size,
            state,
            wait,
            failure: None,
        }
    }

    /// An ended download (`size_bytes` as its entry says).
    #[must_use]
    pub fn ended(id: &str, outcome: &Outcome, size_bytes: u64) -> Self {
        let (state, failure, done) = match outcome {
            Outcome::Installed => (DownloadState::Installed, None, size_bytes),
            Outcome::Failed(Failure::Cancelled) => (DownloadState::Cancelled, None, 0),
            Outcome::Failed(failure) => (DownloadState::Failed, DownloadFailure::of(failure), 0),
        };
        Self {
            id: id.to_owned(),
            done_bytes: done,
            size_bytes,
            state,
            wait: None,
            failure,
        }
    }
}

/// The models on this Mac, for Settings › AI › Models.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ModelsInfo {
    /// Each model on this Mac, in the order installed.
    pub installed: Vec<InstalledModelInfo>,
    /// The disk they use together, bytes (from the index, no walk).
    #[ts(type = "number")]
    pub total_bytes: u64,
    /// The running download, else how the last one ended (since the app
    /// started).
    pub download: Option<DownloadInfo>,
    /// The models whose approval card the user declined.
    pub declined: Vec<String>,
}

/// Why a Download click didn't start one: a key for the words file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum StartRefusal {
    /// Not a model the app offers now.
    NotOffered,
    /// Apple's model: nothing to download.
    NothingToFetch,
    /// Already on this Mac.
    AlreadyInstalled,
    /// Another download is running.
    Busy,
    /// A model is already here: show the approval card.
    NeedsApproval,
    /// Not enough free disk (`needed_bytes`, `free_bytes`).
    NotEnoughDisk,
    /// The disk's free space can't be read.
    DiskUnknown,
}

/// The answer to a Download click.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct StartInfo {
    /// It started.
    pub started: bool,
    /// Why not, when it didn't.
    pub refusal: Option<StartRefusal>,
    /// The model's size (the approval card).
    #[ts(type = "number | null")]
    pub size_bytes: Option<u64>,
    /// What AI models would take with it (the approval card).
    #[ts(type = "number | null")]
    pub total_after_bytes: Option<u64>,
    /// The free disk it needs (not enough disk).
    #[ts(type = "number | null")]
    pub needed_bytes: Option<u64>,
    /// The free disk now (not enough disk).
    #[ts(type = "number | null")]
    pub free_bytes: Option<u64>,
}

impl StartInfo {
    /// It started.
    #[must_use]
    pub const fn started() -> Self {
        Self {
            started: true,
            refusal: None,
            size_bytes: None,
            total_after_bytes: None,
            needed_bytes: None,
            free_bytes: None,
        }
    }

    /// It didn't, for `refusal`.
    #[must_use]
    pub fn refused(refusal: &Refusal) -> Self {
        let mut info = Self {
            started: false,
            refusal: None,
            size_bytes: None,
            total_after_bytes: None,
            needed_bytes: None,
            free_bytes: None,
        };
        info.refusal = Some(match *refusal {
            Refusal::NotOffered => StartRefusal::NotOffered,
            Refusal::NothingToFetch => StartRefusal::NothingToFetch,
            Refusal::AlreadyInstalled => StartRefusal::AlreadyInstalled,
            Refusal::Busy => StartRefusal::Busy,
            Refusal::NeedsApproval {
                size_bytes,
                total_after_bytes,
            } => {
                info.size_bytes = Some(size_bytes);
                info.total_after_bytes = Some(total_after_bytes);
                StartRefusal::NeedsApproval
            }
            Refusal::NotEnoughDisk {
                needed_bytes,
                free_bytes,
            } => {
                info.needed_bytes = Some(needed_bytes);
                info.free_bytes = Some(free_bytes);
                StartRefusal::NotEnoughDisk
            }
            Refusal::DiskUnknown => StartRefusal::DiskUnknown,
        });
        info
    }
}

/// The user approved a further model on its card: the one-time token the
/// Download click then carries.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct ApprovalInfo {
    /// The model.
    pub id: String,
    /// The token for one Download of it, within 10 minutes.
    pub token: String,
    /// Its size.
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// What AI models would take with it.
    #[ts(type = "number")]
    pub total_after_bytes: u64,
}
